use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// Shell metacharacters that could enable command injection
const SHELL_METACHARACTERS: &[char] = &[';', '|', '&', '$', '`', '(', ')', '{', '}', '<', '>', '\n', '\r'];

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

    pub fn from_messages(lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let (tx, rx) = mpsc::channel();
        for line in lines {
            let _ = tx.send(line.into());
        }
        Self { receiver: rx }
    }
}

/// Validate that the command is safe to execute.
///
/// Only allows:
/// - `gat` subcommands (the GAT CLI tool)
/// - `echo` (for dry-run mode)
///
/// Arguments are validated to not contain shell metacharacters.
fn validate_command(cmd: &[String]) -> Result<()> {
    if cmd.is_empty() {
        return Err(anyhow!("Empty command"));
    }

    let command_name = &cmd[0];

    // Only allow 'gat' or 'echo' commands
    // Extract just the binary name, not the full path
    let binary_name = std::path::Path::new(command_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(command_name);

    if binary_name != "gat" && binary_name != "echo" {
        return Err(anyhow!(
            "Command not allowed: '{}'. Only 'gat' commands are permitted.",
            command_name
        ));
    }

    // Validate arguments don't contain shell metacharacters
    for (idx, arg) in cmd.iter().enumerate().skip(1) {
        if arg.chars().any(|c| SHELL_METACHARACTERS.contains(&c)) {
            return Err(anyhow!(
                "Invalid argument at position {}: shell metacharacters are not allowed",
                idx
            ));
        }
    }

    Ok(())
}

pub fn spawn_command(cmd: Vec<String>) -> Result<CommandHandle> {
    // Validate command before execution
    validate_command(&cmd)?;

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
            if let Some(reader) = stdout {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_gat_command() {
        assert!(validate_command(&["gat".to_string(), "pf".to_string()]).is_ok());
        assert!(validate_command(&["gat".to_string(), "--help".to_string()]).is_ok());
    }

    #[test]
    fn test_validate_echo_command() {
        assert!(validate_command(&["echo".to_string(), "hello".to_string()]).is_ok());
    }

    #[test]
    fn test_reject_arbitrary_commands() {
        assert!(validate_command(&["sh".to_string(), "-c".to_string(), "ls".to_string()]).is_err());
        assert!(validate_command(&["rm".to_string(), "-rf".to_string()]).is_err());
        assert!(validate_command(&["cat".to_string(), "/etc/passwd".to_string()]).is_err());
    }

    #[test]
    fn test_reject_shell_metacharacters() {
        assert!(validate_command(&["gat".to_string(), "pf; rm -rf /".to_string()]).is_err());
        assert!(validate_command(&["gat".to_string(), "pf | cat".to_string()]).is_err());
        assert!(validate_command(&["gat".to_string(), "pf && ls".to_string()]).is_err());
        assert!(validate_command(&["gat".to_string(), "$(whoami)".to_string()]).is_err());
        assert!(validate_command(&["gat".to_string(), "`id`".to_string()]).is_err());
    }

    #[test]
    fn test_reject_empty_command() {
        assert!(validate_command(&[]).is_err());
    }

    #[test]
    fn test_allow_valid_gat_arguments() {
        assert!(validate_command(&[
            "gat".to_string(),
            "pf".to_string(),
            "--input".to_string(),
            "case.arrow".to_string(),
            "--output".to_string(),
            "results.parquet".to_string(),
        ]).is_ok());
    }
}
