use std::io::{self, Write};

pub mod terminal {
    use super::*;

    /// Get terminal size from environment or use reasonable defaults
    pub fn get_terminal_size() -> (u16, u16) {
        // Try to get size from COLUMNS and LINES environment variables
        let width = std::env::var("COLUMNS")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(110);

        let height = std::env::var("LINES")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(100); // Large default for testing/CI

        (width, height)
    }

    /// Minimal terminal wrapper that writes directly to stdout.
    pub struct Terminal {
        stdout: io::Stdout,
    }

    impl Terminal {
        pub fn new() -> io::Result<Self> {
            Ok(Self {
                stdout: io::stdout(),
            })
        }

        pub fn clear(&mut self) -> io::Result<()> {
            // Clear screen and move cursor to home
            self.stdout.write_all(b"\x1B[2J\x1B[H")?;
            Ok(())
        }

        pub fn render(&mut self, body: &str) -> io::Result<()> {
            self.stdout.write_all(body.as_bytes())
        }

        pub fn flush(&mut self) -> io::Result<()> {
            self.stdout.flush()
        }
    }
}

#[cfg(unix)]
pub mod input {
    use std::io::{self, Read};
    use std::os::unix::io::AsRawFd;
    use std::time::Duration;
    use termios::{tcsetattr, Termios, ECHO, ICANON, TCSANOW};

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Event {
        Quit,
        Tick,
        Key(char),
    }

    pub trait EventSource {
        fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
            Ok(false)
        }

        fn read(&mut self) -> io::Result<Event> {
            Ok(Event::Quit)
        }
    }

    /// Manages terminal raw mode setup and teardown.
    /// Works safely even when stdin is not a real terminal (no-op in that case).
    pub struct RawModeGuard {
        original_termios: Option<Termios>,
    }

    impl RawModeGuard {
        /// Enable raw mode on stdin and return a guard that will restore it on drop.
        /// If stdin is not a terminal, this succeeds but does nothing (no-op guard).
        pub fn enable() -> io::Result<Self> {
            // Try to get termios settings
            match Termios::from_fd(io::stdin().as_raw_fd()) {
                Ok(original_termios) => {
                    let mut raw_termios = original_termios;

                    // Disable canonical mode and echo
                    raw_termios.c_lflag &= !(ICANON | ECHO);
                    // VMIN=1, VTIME=0 means: wait for at least 1 character with no timeout
                    raw_termios.c_cc[termios::VMIN] = 1;
                    raw_termios.c_cc[termios::VTIME] = 0;

                    // Apply raw mode
                    tcsetattr(io::stdin().as_raw_fd(), TCSANOW, &raw_termios)?;

                    Ok(Self {
                        original_termios: Some(original_termios),
                    })
                }
                Err(e) => {
                    // Check if it's a "not a terminal" error (expected, not fatal)
                    if e.raw_os_error() == Some(25) || e.raw_os_error() == Some(19) {
                        // ENOTTY (25) or ENODEV (19) - stdin is not a TTY
                        // This is OK - we'll return a no-op guard
                        Ok(Self {
                            original_termios: None,
                        })
                    } else {
                        // Other errors are real problems
                        Err(e)
                    }
                }
            }
        }
    }

    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            // Only restore if we actually set raw mode
            if let Some(original) = self.original_termios.as_ref() {
                let _ = tcsetattr(io::stdin().as_raw_fd(), TCSANOW, original);
            }
        }
    }

    /// Simple stdin-based event source that reads single characters.
    pub struct StdinEventSource {
        buffer: [u8; 1],
    }

    impl StdinEventSource {
        pub fn new() -> Self {
            Self { buffer: [0; 1] }
        }
    }

    impl Default for StdinEventSource {
        fn default() -> Self {
            Self::new()
        }
    }

    impl EventSource for StdinEventSource {
        fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
            // Simple polling: try to read one byte without blocking
            // In a real implementation, we'd use select() or poll()
            // For now, always return true (data available) and let read() handle blocking
            Ok(true)
        }

        fn read(&mut self) -> io::Result<Event> {
            let stdin = io::stdin();
            let mut handle = stdin.lock();

            match handle.read(&mut self.buffer) {
                Ok(0) => Ok(Event::Quit), // EOF
                Ok(_) => {
                    let c = self.buffer[0] as char;
                    match c {
                        'q' => Ok(Event::Quit),
                        c => Ok(Event::Key(c)),
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => Ok(Event::Tick),
                Err(e) => Err(e),
            }
        }
    }
}

#[cfg(windows)]
pub mod input {
    use std::io::{self, Read};
    use std::time::Duration;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Event {
        Quit,
        Tick,
        Key(char),
    }

    pub trait EventSource {
        fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
            Ok(false)
        }

        fn read(&mut self) -> io::Result<Event> {
            Ok(Event::Quit)
        }
    }

    /// No-op guard on Windows: console mode is managed by the OS/console host.
    pub struct RawModeGuard;

    impl RawModeGuard {
        pub fn enable() -> io::Result<Self> {
            Ok(Self)
        }
    }

    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            // No-op: Windows console modes are not modified here.
        }
    }

    /// Simple stdin-based event source; blocks for one character.
    pub struct StdinEventSource {
        buffer: [u8; 1],
    }

    impl StdinEventSource {
        pub fn new() -> Self {
            Self { buffer: [0; 1] }
        }
    }

    impl Default for StdinEventSource {
        fn default() -> Self {
            Self::new()
        }
    }

    impl EventSource for StdinEventSource {
        fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
            Ok(true)
        }

        fn read(&mut self) -> io::Result<Event> {
            let stdin = io::stdin();
            let mut handle = stdin.lock();

            match handle.read(&mut self.buffer) {
                Ok(0) => Ok(Event::Quit),
                Ok(_) => {
                    let c = self.buffer[0] as char;
                    match c {
                        'q' => Ok(Event::Quit),
                        c => Ok(Event::Key(c)),
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => Ok(Event::Tick),
                Err(e) => Err(e),
            }
        }
    }
}
