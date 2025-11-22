use std::io::{self, Write};

pub mod terminal {
    use super::*;

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
            self.stdout.write_all(b"\x1B[2J\x1B[H")
        }

        pub fn render(&mut self, body: &str) -> io::Result<()> {
            self.stdout.write_all(body.as_bytes())
        }

        pub fn flush(&mut self) -> io::Result<()> {
            self.stdout.flush()
        }
    }
}

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
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    Ok(Event::Tick)
                }
                Err(e) => Err(e),
            }
        }
    }
}
