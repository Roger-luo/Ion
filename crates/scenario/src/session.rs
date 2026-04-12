use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use portable_pty::PtySize;
use regex::Regex;

use crate::error::Error;
use crate::key::Key;
use crate::output::Output;

/// Shared state between the reader thread and the session.
pub(crate) struct ReaderState {
    pub(crate) raw: Vec<u8>,
    pub(crate) done: bool,
}

impl ReaderState {
    pub(crate) fn new() -> Self {
        ReaderState {
            raw: Vec::new(),
            done: false,
        }
    }
}

/// An interactive session with a process running in a PTY.
///
/// Created by [`Scenario::spawn()`](crate::Scenario::spawn). Provides methods
/// to send input and wait for expected output patterns.
///
/// # Example
///
/// ```no_run
/// use scenario::{Scenario, Terminal};
///
/// let mut session = Scenario::new("my-cli")
///     .args(["init"])
///     .terminal(Terminal::pty(80, 24))
///     .spawn()
///     .unwrap();
///
/// session.expect("Choose template:").unwrap();
/// session.send_line("default").unwrap();
/// session.expect("Created").unwrap();
///
/// let output = session.wait().unwrap();
/// assert!(output.success());
/// ```
pub struct Session {
    writer: Option<Box<dyn Write + Send>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    state: Arc<Mutex<ReaderState>>,
    reader_thread: Option<JoinHandle<()>>,
    timeout: Duration,
    expect_pos: usize,
}

impl Session {
    pub(crate) fn new(
        writer: Box<dyn Write + Send>,
        child: Box<dyn portable_pty::Child + Send + Sync>,
        master: Box<dyn portable_pty::MasterPty + Send>,
        state: Arc<Mutex<ReaderState>>,
        reader_thread: JoinHandle<()>,
        timeout: Duration,
    ) -> Self {
        Session {
            writer: Some(writer),
            child,
            master,
            state,
            reader_thread: Some(reader_thread),
            timeout,
            expect_pos: 0,
        }
    }

    /// Send a line of text followed by a carriage return (simulates pressing Enter).
    pub fn send_line(&mut self, line: &str) -> Result<(), Error> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| Error::Pty("session writer already closed".into()))?;
        writer.write_all(line.as_bytes())?;
        writer.write_all(b"\r")?;
        writer.flush()?;
        Ok(())
    }

    /// Send a terminal key (arrow keys, Ctrl+C, etc.) to the process.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use scenario::{Key, Scenario, Terminal};
    ///
    /// let mut session = Scenario::new("my-cli")
    ///     .terminal(Terminal::pty(80, 24))
    ///     .spawn()
    ///     .unwrap();
    /// session.send_key(Key::Down).unwrap();
    /// session.send_key(Key::Enter).unwrap();
    /// ```
    pub fn send_key(&mut self, key: Key) -> Result<(), Error> {
        self.send(&key.to_bytes())
    }

    /// Send raw bytes to the process.
    pub fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| Error::Pty("session writer already closed".into()))?;
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    /// Wait until the terminal output contains the given string.
    ///
    /// Matches against ANSI-stripped text. Only searches output that hasn't
    /// been consumed by a previous `expect` call.
    pub fn expect(&mut self, pattern: &str) -> Result<(), Error> {
        let deadline = Instant::now() + self.timeout;
        loop {
            if self.poll_literal(pattern) {
                return Ok(());
            }
            if self.is_done() {
                // Final check after child exit
                if self.poll_literal(pattern) {
                    return Ok(());
                }
                return Err(Error::ExpectTimeout {
                    pattern: pattern.to_string(),
                    timeout: self.timeout,
                    buffer: self.unseen_text(),
                });
            }
            if Instant::now() >= deadline {
                return Err(Error::ExpectTimeout {
                    pattern: pattern.to_string(),
                    timeout: self.timeout,
                    buffer: self.unseen_text(),
                });
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Wait until the terminal output matches the given regex.
    ///
    /// Matches against ANSI-stripped text. Only searches output that hasn't
    /// been consumed by a previous `expect` call.
    pub fn expect_regex(&mut self, pattern: &str) -> Result<(), Error> {
        let re = Regex::new(pattern)?;
        let deadline = Instant::now() + self.timeout;
        loop {
            if self.poll_regex(&re) {
                return Ok(());
            }
            if self.is_done() {
                if self.poll_regex(&re) {
                    return Ok(());
                }
                return Err(Error::ExpectTimeout {
                    pattern: pattern.to_string(),
                    timeout: self.timeout,
                    buffer: self.unseen_text(),
                });
            }
            if Instant::now() >= deadline {
                return Err(Error::ExpectTimeout {
                    pattern: pattern.to_string(),
                    timeout: self.timeout,
                    buffer: self.unseen_text(),
                });
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Resize the terminal.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), Error> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Pty(e.to_string()))
    }

    /// Wait for the process to exit and return the captured output.
    ///
    /// Drops the writer (sending EOF to the child), waits for exit, and
    /// collects all remaining output.
    pub fn wait(mut self) -> Result<Output, Error> {
        // Drop writer to signal EOF
        self.writer.take();

        // Wait for child with timeout
        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match self.child.try_wait()? {
                Some(status) => break status,
                None => {
                    if Instant::now() >= deadline {
                        let _ = self.child.kill();
                        return Err(Error::Timeout(self.timeout));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        };

        // Wait for reader thread to finish collecting output.
        // The reader will get EOF/EIO once the child exits (slave side
        // was already dropped in spawn_pty).
        if let Some(handle) = self.reader_thread.take() {
            let reader_deadline = Instant::now() + Duration::from_secs(5);
            while !self.is_done() && Instant::now() < reader_deadline {
                thread::sleep(Duration::from_millis(10));
            }
            let _ = handle.join();
        }

        let raw = self.state.lock().unwrap().raw.clone();
        Ok(Output::from_pty(raw, status))
    }

    /// Get the full captured output so far, with ANSI codes stripped.
    pub fn current_output(&self) -> String {
        let state = self.state.lock().unwrap();
        strip_ansi_escapes::strip_str(String::from_utf8_lossy(&state.raw))
    }

    fn poll_literal(&mut self, pattern: &str) -> bool {
        let stripped = self.current_stripped();
        if self.expect_pos >= stripped.len() {
            return false;
        }
        let unseen = &stripped[self.expect_pos..];
        if let Some(pos) = unseen.find(pattern) {
            self.expect_pos += pos + pattern.len();
            true
        } else {
            false
        }
    }

    fn poll_regex(&mut self, re: &Regex) -> bool {
        let stripped = self.current_stripped();
        if self.expect_pos >= stripped.len() {
            return false;
        }
        let unseen = &stripped[self.expect_pos..];
        if let Some(m) = re.find(unseen) {
            self.expect_pos += m.end();
            true
        } else {
            false
        }
    }

    fn current_stripped(&self) -> String {
        let state = self.state.lock().unwrap();
        strip_ansi_escapes::strip_str(String::from_utf8_lossy(&state.raw))
    }

    fn unseen_text(&self) -> String {
        let stripped = self.current_stripped();
        if self.expect_pos >= stripped.len() {
            String::new()
        } else {
            stripped[self.expect_pos..].to_string()
        }
    }

    fn is_done(&self) -> bool {
        self.state.lock().unwrap().done
    }
}
