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
    use std::io;
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
}
