use anyhow::Result;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub struct CommandHandle {
    receiver: Receiver<String>,
}

impl CommandHandle {
    pub fn poll(&self) -> Vec<String> {
        let mut lines = Vec::new();
        while let Ok(line) = self.receiver.try_recv() {
            lines.push(line);
        }
        lines
    }
}

pub fn spawn_command(cmd: Vec<String>) -> Result<CommandHandle> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        if cmd.is_empty() {
            return;
        }
        let mut c = Command::new(&cmd[0]);
        if cmd.len() > 1 {
            c.args(&cmd[1..]);
        }
        if let Ok(mut child) = c.stdout(Stdio::piped()).spawn() {
            let stdout = child.stdout.take();
            if let Some(mut reader) = stdout {
                use std::io::{BufRead, BufReader};
                let buf = BufReader::new(reader);
                for line in buf.lines().flatten() {
                    let _ = tx.send(line);
                }
            }
            let _ = child.wait();
        }
    });
    Ok(CommandHandle { receiver: rx })
}
