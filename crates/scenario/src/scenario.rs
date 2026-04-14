use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::error::Error;
use crate::output::Output;
use crate::project::Project;
use crate::session::{ReaderState, Session};

/// Terminal conditions for a scenario.
#[derive(Debug, Clone, Default)]
pub enum Terminal {
    /// Piped stdio — `is_terminal()` returns `false` in the child process.
    #[default]
    Piped,
    /// Real pseudo-terminal with specific dimensions.
    /// The child process sees `is_terminal() == true`.
    Pty { cols: u16, rows: u16 },
}

impl Terminal {
    /// Create a PTY terminal with the given column and row count.
    pub fn pty(cols: u16, rows: u16) -> Self {
        Terminal::Pty { cols, rows }
    }
}

/// Reusable session settings for interactive scenarios.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    terminal: Terminal,
    timeout: Duration,
}

impl SessionConfig {
    /// Create a PTY session profile with the given column and row count.
    pub fn pty(cols: u16, rows: u16) -> Self {
        SessionConfig {
            terminal: Terminal::pty(cols, rows),
            timeout: Duration::from_secs(30),
        }
    }

    /// Set the timeout for this session profile.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Builder for defining a CLI scenario.
///
/// A scenario describes how to run a CLI command under specific terminal
/// conditions. Use [`run()`](Scenario::run) for non-interactive commands or
/// [`spawn()`](Scenario::spawn) for interactive sessions.
///
/// # Example
///
/// ```no_run
/// use scenario::{Scenario, Terminal};
///
/// let output = Scenario::new("echo")
///     .arg("hello")
///     .terminal(Terminal::pty(80, 24))
///     .run()
///     .unwrap();
///
/// assert!(output.success());
/// assert!(output.stdout().contains("hello"));
/// ```
pub struct Scenario {
    program: OsString,
    args: Vec<OsString>,
    envs: Vec<(OsString, OsString)>,
    env_removals: Vec<OsString>,
    env_clear: bool,
    current_dir: Option<PathBuf>,
    terminal: Terminal,
    stdin_data: Option<Vec<u8>>,
    timeout: Duration,
}

impl Scenario {
    /// Create a new scenario for the given program.
    pub fn new(program: impl Into<OsString>) -> Self {
        Scenario {
            program: program.into(),
            args: Vec::new(),
            envs: Vec::new(),
            env_removals: Vec::new(),
            env_clear: false,
            current_dir: None,
            terminal: Terminal::default(),
            stdin_data: None,
            timeout: Duration::from_secs(30),
        }
    }

    /// Add a single argument.
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set an environment variable.
    pub fn env(mut self, key: impl Into<OsString>, val: impl Into<OsString>) -> Self {
        self.envs.push((key.into(), val.into()));
        self
    }

    /// Remove an environment variable.
    pub fn env_remove(mut self, key: impl Into<OsString>) -> Self {
        self.env_removals.push(key.into());
        self
    }

    /// Clear all inherited environment variables.
    pub fn env_clear(mut self) -> Self {
        self.env_clear = true;
        self
    }

    /// Set the working directory.
    pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    /// Set the working directory to a [`Project`]'s path.
    ///
    /// This is convenience sugar for `.current_dir(project.path())`.
    /// The `Project` must outlive the scenario execution.
    pub fn project(self, project: &Project) -> Self {
        self.current_dir(project.path())
    }

    /// Set the terminal conditions.
    pub fn terminal(mut self, terminal: Terminal) -> Self {
        self.terminal = terminal;
        self
    }

    /// Apply a reusable session profile.
    pub fn session_config(mut self, config: &SessionConfig) -> Self {
        self.terminal = config.terminal.clone();
        self.timeout = config.timeout;
        self
    }

    /// Provide data to be written to stdin (piped mode only).
    pub fn stdin(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.stdin_data = Some(data.into());
        self
    }

    /// Set the timeout for the scenario. Default is 30 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Run the scenario and capture its output.
    ///
    /// Works in both piped and PTY modes.
    pub fn run(self) -> Result<Output, Error> {
        match self.terminal {
            Terminal::Piped => self.run_piped(),
            Terminal::Pty { cols, rows } => self.run_pty(cols, rows),
        }
    }

    /// Spawn an interactive session.
    ///
    /// Requires `Terminal::Pty`. Returns a [`Session`] that supports
    /// `send_line()`, `expect()`, and other interactive operations.
    pub fn spawn(self) -> Result<Session, Error> {
        match self.terminal {
            Terminal::Piped => Err(Error::SpawnRequiresPty),
            Terminal::Pty { cols, rows } => self.spawn_pty(cols, rows),
        }
    }

    fn run_piped(self) -> Result<Output, Error> {
        let mut cmd = std::process::Command::new(&self.program);
        cmd.args(&self.args);
        if self.env_clear {
            cmd.env_clear();
        }
        for key in &self.env_removals {
            cmd.env_remove(key);
        }
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        if let Some(dir) = &self.current_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        if self.stdin_data.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }

        let mut child = cmd.spawn()?;

        // Write stdin data if provided
        if let Some(data) = &self.stdin_data
            && let Some(mut stdin) = child.stdin.take()
        {
            use std::io::Write;
            let _ = stdin.write_all(data);
        }

        // Read stdout/stderr in separate threads to avoid pipe deadlock
        let stdout_handle = child.stdout.take().unwrap();
        let stderr_handle = child.stderr.take().unwrap();

        let stdout_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            let mut reader = stdout_handle;
            let _ = reader.read_to_end(&mut buf);
            buf
        });
        let stderr_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            let mut reader = stderr_handle;
            let _ = reader.read_to_end(&mut buf);
            buf
        });

        // Wait for child with timeout
        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait()? {
                Some(status) => break status,
                None => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        return Err(Error::Timeout(self.timeout));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        };

        let stdout_bytes = stdout_thread.join().expect("stdout reader panicked");
        let stderr_bytes = stderr_thread.join().expect("stderr reader panicked");

        Ok(Output::from_piped(std::process::Output {
            status,
            stdout: stdout_bytes,
            stderr: stderr_bytes,
        }))
    }

    fn run_pty(self, cols: u16, rows: u16) -> Result<Output, Error> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Pty(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&self.program);
        cmd.args(self.args.iter().map(OsStr::new));
        if self.env_clear {
            cmd.env_clear();
        }
        for key in &self.env_removals {
            cmd.env_remove(key);
        }
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        if let Some(dir) = &self.current_dir {
            cmd.cwd(dir);
        }

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| Error::Pty(e.to_string()))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| Error::Pty(e.to_string()))?;

        // Write stdin data via the PTY master if provided
        if let Some(data) = &self.stdin_data {
            let mut writer = pair
                .master
                .take_writer()
                .map_err(|e| Error::Pty(e.to_string()))?;
            use std::io::Write;
            let _ = writer.write_all(data);
            drop(writer);
        }

        // Read all output in a separate thread (read_to_end blocks until EOF)
        let reader_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = reader.read_to_end(&mut buf);
            buf
        });

        // Wait for child with timeout
        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait()? {
                Some(status) => break status,
                None => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        return Err(Error::Timeout(self.timeout));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        };

        // Drop master to ensure reader gets EOF
        drop(pair.master);

        let raw = reader_thread.join().expect("reader thread panicked");
        Ok(Output::from_pty(raw, status))
    }

    fn spawn_pty(self, cols: u16, rows: u16) -> Result<Session, Error> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Pty(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&self.program);
        cmd.args(self.args.iter().map(OsStr::new));
        if self.env_clear {
            cmd.env_clear();
        }
        for key in &self.env_removals {
            cmd.env_remove(key);
        }
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        if let Some(dir) = &self.current_dir {
            cmd.cwd(dir);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| Error::Pty(e.to_string()))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| Error::Pty(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| Error::Pty(e.to_string()))?;

        // Start reader thread
        let state = Arc::new(Mutex::new(ReaderState::new()));
        let reader_state = Arc::clone(&state);
        let reader_thread = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        reader_state.lock().unwrap().done = true;
                        break;
                    }
                    Ok(n) => {
                        reader_state
                            .lock()
                            .unwrap()
                            .raw
                            .extend_from_slice(&buf[..n]);
                    }
                    Err(_) => {
                        // EIO is expected on macOS when the child exits
                        reader_state.lock().unwrap().done = true;
                        break;
                    }
                }
            }
        });

        Ok(Session::new(
            writer,
            child,
            pair.master,
            state,
            reader_thread,
            self.timeout,
            rows,
            cols,
        ))
    }
}
