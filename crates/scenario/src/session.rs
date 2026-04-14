use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use portable_pty::PtySize;
use regex::Regex;

use crate::error::Error;
use crate::key::Key;
use crate::output::Output;
use crate::screen::ScreenBuffer;

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
    rows: u16,
    cols: u16,
}

impl Session {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        writer: Box<dyn Write + Send>,
        child: Box<dyn portable_pty::Child + Send + Sync>,
        master: Box<dyn portable_pty::MasterPty + Send>,
        state: Arc<Mutex<ReaderState>>,
        reader_thread: JoinHandle<()>,
        timeout: Duration,
        rows: u16,
        cols: u16,
    ) -> Self {
        Session {
            writer: Some(writer),
            child,
            master,
            state,
            reader_thread: Some(reader_thread),
            timeout,
            expect_pos: 0,
            rows,
            cols,
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

    /// Send multiple terminal keys in order to the process.
    pub fn send_keys<I>(&mut self, keys: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Key>,
    {
        for key in keys {
            self.send_key(key)?;
        }
        Ok(())
    }

    /// Send a single terminal key to the process.
    pub fn press(&mut self, key: Key) -> Result<(), Error> {
        self.send_key(key)
    }

    /// Simulate pressing Enter.
    pub fn enter(&mut self) -> Result<(), Error> {
        self.send_key(Key::Enter)
    }

    /// Simulate pressing Ctrl+C.
    pub fn ctrl_c(&mut self) -> Result<(), Error> {
        self.send_key(Key::CtrlC)
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

    /// Assert that a literal pattern does NOT appear in the unseen output.
    ///
    /// Waits for 500ms (the default quiet period), checking periodically.
    /// Fails if the pattern is found at any point during that period.
    /// Does not advance `expect_pos`.
    pub fn expect_not(&self, pattern: &str) -> Result<(), Error> {
        self.expect_not_timeout(pattern, Duration::from_millis(500))
    }

    /// Assert that a literal pattern does NOT appear within the given duration.
    ///
    /// Checks the unseen output (after `expect_pos`) periodically. Returns
    /// `Ok(())` if the pattern never appears before the deadline or the process
    /// exits. Returns `Err(Error::UnexpectedPattern)` if the pattern is found.
    pub fn expect_not_timeout(&self, pattern: &str, duration: Duration) -> Result<(), Error> {
        let deadline = Instant::now() + duration;
        loop {
            let unseen = self.unseen_text();
            if unseen.contains(pattern) {
                return Err(Error::UnexpectedPattern {
                    pattern: pattern.to_string(),
                    buffer: unseen,
                });
            }
            if Instant::now() >= deadline || self.is_done() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Assert that a regex pattern does NOT match in the unseen output.
    ///
    /// Waits for 500ms (the default quiet period), checking periodically.
    /// Fails if the pattern matches at any point during that period.
    /// Does not advance `expect_pos`.
    pub fn expect_not_regex(&self, pattern: &str) -> Result<(), Error> {
        self.expect_not_regex_timeout(pattern, Duration::from_millis(500))
    }

    /// Assert that a regex pattern does NOT match within the given duration.
    ///
    /// Checks the unseen output (after `expect_pos`) periodically. Returns
    /// `Ok(())` if the pattern never matches before the deadline or the process
    /// exits. Returns `Err(Error::UnexpectedPattern)` if a match is found.
    pub fn expect_not_regex_timeout(&self, pattern: &str, duration: Duration) -> Result<(), Error> {
        let re = Regex::new(pattern)?;
        let deadline = Instant::now() + duration;
        loop {
            let unseen = self.unseen_text();
            if re.is_match(&unseen) {
                return Err(Error::UnexpectedPattern {
                    pattern: pattern.to_string(),
                    buffer: unseen,
                });
            }
            if Instant::now() >= deadline || self.is_done() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Pty(e.to_string()))?;
        self.rows = rows;
        self.cols = cols;
        Ok(())
    }

    /// Get the current PTY dimensions as `(rows, cols)`.
    pub fn pty_size(&self) -> (usize, usize) {
        (self.rows as usize, self.cols as usize)
    }

    /// Get the current terminal screen content as a grid of lines.
    ///
    /// Interprets ANSI escape sequences to determine what would actually
    /// be visible on screen. Returns one `String` per terminal row.
    pub fn visible_screen(&self) -> Vec<String> {
        let state = self.state.lock().unwrap();
        let (rows, cols) = self.pty_size();
        let mut screen = ScreenBuffer::new(rows, cols);
        screen.process(&state.raw);
        screen.lines()
    }

    /// Get the current terminal screen content as newline-delimited text.
    pub fn visible_text(&self) -> String {
        self.visible_screen().join("\n")
    }

    /// Wait until the current visible screen contains the given string.
    pub fn expect_screen(&self, pattern: &str) -> Result<(), Error> {
        self.expect_screen_match(pattern, |visible, pattern| visible.contains(pattern))
    }

    /// Wait until the current visible screen matches the given regex.
    pub fn expect_screen_regex(&self, pattern: &str) -> Result<(), Error> {
        let re = Regex::new(pattern)?;
        self.expect_screen_match(pattern, |visible, _| re.is_match(visible))
    }

    /// Assert that the current visible screen does not contain the given string.
    pub fn expect_screen_not(&self, pattern: &str) -> Result<(), Error> {
        self.expect_screen_not_timeout(pattern, Duration::from_millis(500))
    }

    /// Assert that the current visible screen does not contain the given string within the given duration.
    pub fn expect_screen_not_timeout(
        &self,
        pattern: &str,
        duration: Duration,
    ) -> Result<(), Error> {
        self.expect_screen_not_match(pattern, duration, |visible, pattern| {
            visible.contains(pattern)
        })
    }

    /// Assert that the current visible screen does not match the given regex.
    pub fn expect_screen_not_regex(&self, pattern: &str) -> Result<(), Error> {
        self.expect_screen_not_regex_timeout(pattern, Duration::from_millis(500))
    }

    /// Assert that the current visible screen does not match the given regex within the given duration.
    pub fn expect_screen_not_regex_timeout(
        &self,
        pattern: &str,
        duration: Duration,
    ) -> Result<(), Error> {
        let re = Regex::new(pattern)?;
        self.expect_screen_not_match(pattern, duration, |visible, _| re.is_match(visible))
    }

    /// Wait until the visible screen is unchanged for the requested quiet period.
    pub fn wait_for_screen_stable(&self, quiet_period: Duration) -> Result<(), Error> {
        let deadline = Instant::now() + self.timeout;
        let mut last_visible = self.visible_text();
        let mut last_raw_len = self.raw_len();
        let mut stable_since = if Self::screen_has_content(&last_visible) || last_raw_len > 0 {
            Some(Instant::now())
        } else {
            None
        };

        loop {
            thread::sleep(Duration::from_millis(10));
            let visible = self.visible_text();
            let raw_len = self.raw_len();
            if visible != last_visible {
                last_visible = visible;
                stable_since = Some(Instant::now());
            } else if raw_len != last_raw_len {
                stable_since = Some(Instant::now());
            } else if let Some(stable_since) = stable_since
                && Instant::now().duration_since(stable_since) >= quiet_period
            {
                return Ok(());
            }
            last_raw_len = raw_len;

            if Instant::now() >= deadline {
                return Err(Error::Timeout(self.timeout));
            }
        }
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

    fn expect_screen_match<F>(&self, pattern: &str, matches: F) -> Result<(), Error>
    where
        F: Fn(&str, &str) -> bool,
    {
        let deadline = Instant::now() + self.timeout;
        loop {
            let visible = self.visible_text();
            if matches(&visible, pattern) {
                return Ok(());
            }
            if self.is_done() {
                return Err(Error::ExpectTimeout {
                    pattern: pattern.to_string(),
                    timeout: self.timeout,
                    buffer: visible,
                });
            }
            if Instant::now() >= deadline {
                return Err(Error::ExpectTimeout {
                    pattern: pattern.to_string(),
                    timeout: self.timeout,
                    buffer: visible,
                });
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn expect_screen_not_match<F>(
        &self,
        pattern: &str,
        quiet_period: Duration,
        matches: F,
    ) -> Result<(), Error>
    where
        F: Fn(&str, &str) -> bool,
    {
        let deadline = Instant::now() + quiet_period;
        loop {
            let visible = self.visible_text();
            if matches(&visible, pattern) {
                return Err(Error::UnexpectedPattern {
                    pattern: pattern.to_string(),
                    buffer: visible,
                });
            }
            if Instant::now() >= deadline || self.is_done() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
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

    fn raw_len(&self) -> usize {
        self.state.lock().unwrap().raw.len()
    }

    fn screen_has_content(visible: &str) -> bool {
        visible.lines().any(|line| !line.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Scenario, Terminal};

    #[test]
    fn expect_not_succeeds_when_pattern_absent() {
        let mut session = Scenario::new("echo")
            .arg("hello world")
            .terminal(Terminal::pty(80, 24))
            .spawn()
            .unwrap();

        session.expect("hello").unwrap();
        // "goodbye" never appears in the output
        session.expect_not("goodbye").unwrap();
        let output = session.wait().unwrap();
        assert!(output.success());
    }

    #[test]
    fn expect_not_fails_when_pattern_present() {
        let session = Scenario::new("echo")
            .arg("hello world")
            .terminal(Terminal::pty(80, 24))
            .spawn()
            .unwrap();

        // "hello" will appear in the output
        let result = session.expect_not("hello");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("expect_not found unexpected pattern")
        );
        let _ = session.wait();
    }

    #[test]
    fn expect_not_regex_succeeds_when_pattern_absent() {
        let mut session = Scenario::new("echo")
            .arg("hello world")
            .terminal(Terminal::pty(80, 24))
            .spawn()
            .unwrap();

        session.expect("hello").unwrap();
        // No digits in "hello world"
        session.expect_not_regex(r"\d+").unwrap();
        let output = session.wait().unwrap();
        assert!(output.success());
    }

    #[test]
    fn expect_not_regex_fails_when_pattern_matches() {
        let session = Scenario::new("echo")
            .arg("version 42")
            .terminal(Terminal::pty(80, 24))
            .spawn()
            .unwrap();

        // Digits will appear in the output
        let result = session.expect_not_regex(r"\d+");
        assert!(result.is_err());
        let _ = session.wait();
    }
}
