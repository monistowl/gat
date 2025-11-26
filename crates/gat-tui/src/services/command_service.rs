use crate::message::CommandResult;
/// Command execution service for running system commands with timeout and output streaming
///
/// Handles subprocess management, output capture, timeout enforcement, and graceful termination.
///
/// # Security
///
/// This module implements defense-in-depth against command injection attacks:
/// - **Program allowlisting**: Only `gat` and `gat-cli` binaries can be executed
/// - **Argument sanitization**: Shell metacharacters are rejected in arguments
/// - **Structured command building**: Use `SecureCommandBuilder` instead of string interpolation
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Shell metacharacters that could enable command injection
const DANGEROUS_CHARS: &[char] = &[
    ';',  // Command chaining
    '|',  // Pipes
    '&',  // Background/chaining
    '$',  // Variable expansion
    '`',  // Command substitution
    '(',  // Subshell
    ')',  // Subshell
    '{',  // Brace expansion
    '}',  // Brace expansion
    '<',  // Input redirection
    '>',  // Output redirection
    '\n', // Newline (command separator)
    '\r', // Carriage return
    '\0', // Null byte
];

/// Programs allowed to be executed by CommandService
const ALLOWED_PROGRAMS: &[&str] = &["gat", "gat-cli"];

/// Check if a string contains dangerous shell metacharacters
fn contains_dangerous_chars(s: &str) -> bool {
    s.chars().any(|c| DANGEROUS_CHARS.contains(&c))
}

/// Validate that an argument is safe for command execution
fn validate_argument(arg: &str) -> Result<(), CommandError> {
    if contains_dangerous_chars(arg) {
        return Err(CommandError::SecurityViolation(format!(
            "Argument contains forbidden shell metacharacters: {}",
            arg.chars()
                .filter(|c| DANGEROUS_CHARS.contains(c))
                .collect::<String>()
        )));
    }
    Ok(())
}

/// Validate that a program is in the allowlist
fn validate_program(program: &str) -> Result<(), CommandError> {
    // Extract just the program name (handle paths like /usr/bin/gat)
    let program_name = std::path::Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);

    if !ALLOWED_PROGRAMS.contains(&program_name) {
        return Err(CommandError::SecurityViolation(format!(
            "Program '{}' is not in the allowlist. Only {:?} are permitted.",
            program, ALLOWED_PROGRAMS
        )));
    }
    Ok(())
}

/// Secure command builder for constructing gat-cli commands safely
///
/// Use this instead of string interpolation to prevent command injection.
///
/// # Example
/// ```ignore
/// let cmd = SecureCommandBuilder::new("gat")
///     .subcommand("dataset")
///     .subcommand("public")
///     .subcommand("describe")
///     .arg(user_provided_id)?  // Validates the argument
///     .flag("--format", "json")
///     .build()?;
/// ```
#[derive(Debug, Clone)]
pub struct SecureCommandBuilder {
    program: String,
    args: Vec<String>,
}

impl SecureCommandBuilder {
    /// Create a new secure command builder with the specified program
    ///
    /// # Errors
    /// Returns `CommandError::SecurityViolation` if the program is not in the allowlist
    pub fn new(program: &str) -> Result<Self, CommandError> {
        validate_program(program)?;
        Ok(Self {
            program: program.to_string(),
            args: Vec::new(),
        })
    }

    /// Add a subcommand (trusted, not user-provided)
    pub fn subcommand(mut self, subcmd: &str) -> Self {
        self.args.push(subcmd.to_string());
        self
    }

    /// Add a user-provided argument with validation
    ///
    /// # Errors
    /// Returns `CommandError::SecurityViolation` if the argument contains dangerous characters
    pub fn arg(mut self, value: &str) -> Result<Self, CommandError> {
        validate_argument(value)?;
        self.args.push(value.to_string());
        Ok(self)
    }

    /// Add a flag with a trusted name and user-provided value
    ///
    /// # Errors
    /// Returns `CommandError::SecurityViolation` if the value contains dangerous characters
    pub fn flag(mut self, name: &str, value: &str) -> Result<Self, CommandError> {
        validate_argument(value)?;
        self.args.push(name.to_string());
        self.args.push(value.to_string());
        Ok(self)
    }

    /// Add a boolean flag (no value)
    pub fn bool_flag(mut self, name: &str) -> Self {
        self.args.push(name.to_string());
        self
    }

    /// Build the command string
    pub fn build(self) -> String {
        let mut parts = vec![self.program];
        parts.extend(self.args);
        shell_words::join(&parts)
    }

    /// Build and return the program and arguments separately for direct Command construction
    pub fn build_parts(self) -> (String, Vec<String>) {
        (self.program, self.args)
    }
}

/// Parse a command string into program and arguments, respecting shell quoting rules.
///
/// Handles:
/// - Double-quoted strings: `"hello world"` → `hello world`
/// - Single-quoted strings: `'hello world'` → `hello world`
/// - Escaped spaces: `hello\ world` → `hello world`
/// - Mixed quoting: `"hello 'nested' world"` → `hello 'nested' world`
///
/// Returns an error if the command is empty or contains only whitespace.
pub fn parse_command(command: &str) -> Result<Vec<String>, CommandError> {
    let parts = shell_words::split(command)
        .map_err(|e| CommandError::ExecutionFailed(format!("Failed to parse command: {}", e)))?;

    if parts.is_empty() {
        return Err(CommandError::ExecutionFailed("Empty command".to_string()));
    }

    Ok(parts)
}

/// Error types for command execution
#[derive(Debug, Clone)]
pub enum CommandError {
    /// Command execution failed
    ExecutionFailed(String),
    /// Command timed out
    TimedOut,
    /// Failed to spawn process
    SpawnFailed(String),
    /// IO error during execution
    IoError(String),
    /// Security violation (injection attempt, disallowed program, etc.)
    SecurityViolation(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            CommandError::TimedOut => write!(f, "Command timed out"),
            CommandError::SpawnFailed(msg) => write!(f, "Failed to spawn process: {}", msg),
            CommandError::IoError(msg) => write!(f, "IO error: {}", msg),
            CommandError::SecurityViolation(msg) => write!(f, "Security violation: {}", msg),
        }
    }
}

impl std::error::Error for CommandError {}

/// Configuration for a command execution
#[derive(Clone, Debug)]
pub struct CommandExecution {
    /// The command line to execute (e.g., "gat-cli datasets list")
    pub command: String,
    /// Working directory (optional)
    pub working_dir: Option<String>,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Maximum lines of output to capture
    pub max_output_lines: usize,
}

impl CommandExecution {
    pub fn new(command: String, timeout_secs: u64) -> Self {
        Self {
            command,
            working_dir: None,
            timeout_secs,
            max_output_lines: 10000,
        }
    }

    pub fn with_working_dir(mut self, dir: String) -> Self {
        self.working_dir = Some(dir);
        self
    }

    pub fn with_max_output_lines(mut self, max_lines: usize) -> Self {
        self.max_output_lines = max_lines;
        self
    }
}

/// Service for executing system commands with async support
pub struct CommandService {
    default_timeout: u64,
    max_output_lines: usize,
}

impl CommandService {
    /// Create a new CommandService with default settings
    pub fn new(default_timeout: u64) -> Self {
        Self {
            default_timeout,
            max_output_lines: 10000,
        }
    }

    /// Set maximum output lines to capture
    pub fn with_max_output_lines(mut self, max_lines: usize) -> Self {
        self.max_output_lines = max_lines;
        self
    }

    /// Execute a command synchronously (blocking)
    /// Returns command result with exit code, stdout, stderr, and duration
    ///
    /// # Security
    ///
    /// This method enforces security constraints:
    /// - Only programs in `ALLOWED_PROGRAMS` (gat, gat-cli) can be executed
    /// - Arguments are validated to reject shell metacharacters
    ///
    /// Use `SecureCommandBuilder` to construct commands safely from user input.
    pub async fn execute(&self, exec: CommandExecution) -> Result<CommandResult, CommandError> {
        let start = Instant::now();
        let timeout = std::time::Duration::from_secs(exec.timeout_secs);

        // Parse command into program and args using shell-words for proper quoting
        let parts = parse_command(&exec.command)?;
        let program = &parts[0];
        let args = &parts[1..];

        // SECURITY: Validate program is in allowlist
        validate_program(program)?;

        // SECURITY: Validate all arguments for injection attempts
        for arg in args {
            validate_argument(arg)?;
        }

        // Build command
        let mut cmd = Command::new(program);
        cmd.args(args);

        // Set working directory if provided
        if let Some(ref dir) = exec.working_dir {
            cmd.current_dir(dir);
        }

        // Capture stdout and stderr
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd
            .spawn()
            .map_err(|e| CommandError::SpawnFailed(format!("{}: {}", program, e)))?;

        // Collect output
        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut stdout_lines = 0;
        let mut stderr_lines = 0;

        // Read stdout
        if let Some(out) = child.stdout.take() {
            let reader = BufReader::new(out);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if stdout_lines >= exec.max_output_lines {
                    stdout.push_str(&format!(
                        "\n[Output truncated - exceeded {} lines]",
                        exec.max_output_lines
                    ));
                    break;
                }
                if !stdout.is_empty() {
                    stdout.push('\n');
                }
                stdout.push_str(&line);
                stdout_lines += 1;
            }
        }

        // Read stderr
        if let Some(err) = child.stderr.take() {
            let reader = BufReader::new(err);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if stderr_lines >= exec.max_output_lines {
                    stderr.push_str(&format!(
                        "\n[Error output truncated - exceeded {} lines]",
                        exec.max_output_lines
                    ));
                    break;
                }
                if !stderr.is_empty() {
                    stderr.push('\n');
                }
                stderr.push_str(&line);
                stderr_lines += 1;
            }
        }

        // Wait for process with timeout
        let wait_result = tokio::time::timeout(timeout, child.wait()).await;

        let (exit_code, timed_out) = match wait_result {
            Ok(Ok(status)) => {
                let code = status.code().unwrap_or(-1);
                (code, false)
            }
            Ok(Err(e)) => {
                return Err(CommandError::ExecutionFailed(e.to_string()));
            }
            Err(_) => {
                // Timeout occurred - kill the child process
                let _ = child.kill().await;
                (-1, true)
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(CommandResult {
            command: exec.command,
            exit_code,
            stdout,
            stderr,
            duration_ms,
            timed_out,
        })
    }

    /// Execute a command with output streaming callback
    /// Calls the provided closure as each line is produced
    ///
    /// # Security
    ///
    /// This method enforces the same security constraints as `execute()`.
    pub async fn execute_with_streaming<F>(
        &self,
        exec: CommandExecution,
        mut on_output: F,
    ) -> Result<CommandResult, CommandError>
    where
        F: FnMut(String) + Send,
    {
        let start = Instant::now();
        let timeout = std::time::Duration::from_secs(exec.timeout_secs);

        // Parse command using shell-words for proper quoting
        let parts = parse_command(&exec.command)?;
        let program = &parts[0];
        let args = &parts[1..];

        // SECURITY: Validate program is in allowlist
        validate_program(program)?;

        // SECURITY: Validate all arguments for injection attempts
        for arg in args {
            validate_argument(arg)?;
        }

        // Build command
        let mut cmd = Command::new(program);
        cmd.args(args);

        if let Some(ref dir) = exec.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn
        let mut child = cmd
            .spawn()
            .map_err(|e| CommandError::SpawnFailed(format!("{}: {}", program, e)))?;

        let mut all_stdout = String::new();
        let mut all_stderr = String::new();
        let mut stdout_lines = 0;
        let mut stderr_lines = 0;

        // Stream stdout
        if let Some(out) = child.stdout.take() {
            let reader = BufReader::new(out);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if stdout_lines >= exec.max_output_lines {
                    let truncated = format!(
                        "[Output truncated - exceeded {} lines]",
                        exec.max_output_lines
                    );
                    on_output(truncated.clone());
                    all_stdout.push('\n');
                    all_stdout.push_str(&truncated);
                    break;
                }
                on_output(line.clone());
                if !all_stdout.is_empty() {
                    all_stdout.push('\n');
                }
                all_stdout.push_str(&line);
                stdout_lines += 1;
            }
        }

        // Stream stderr
        if let Some(err) = child.stderr.take() {
            let reader = BufReader::new(err);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if stderr_lines >= exec.max_output_lines {
                    let truncated = format!(
                        "[Error output truncated - exceeded {} lines]",
                        exec.max_output_lines
                    );
                    on_output(truncated.clone());
                    all_stderr.push('\n');
                    all_stderr.push_str(&truncated);
                    break;
                }
                on_output(line.clone());
                if !all_stderr.is_empty() {
                    all_stderr.push('\n');
                }
                all_stderr.push_str(&line);
                stderr_lines += 1;
            }
        }

        // Wait with timeout
        let wait_result = tokio::time::timeout(timeout, child.wait()).await;

        let (exit_code, timed_out) = match wait_result {
            Ok(Ok(status)) => {
                let code = status.code().unwrap_or(-1);
                (code, false)
            }
            Ok(Err(e)) => {
                return Err(CommandError::ExecutionFailed(e.to_string()));
            }
            Err(_) => {
                let _ = child.kill().await;
                (-1, true)
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(CommandResult {
            command: exec.command,
            exit_code,
            stdout: all_stdout,
            stderr: all_stderr,
            duration_ms,
            timed_out,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_service_creation() {
        let service = CommandService::new(300);
        assert_eq!(service.default_timeout, 300);
        assert_eq!(service.max_output_lines, 10000);
    }

    #[tokio::test]
    async fn test_execute_rejects_echo_not_in_allowlist() {
        // SECURITY: "echo" is not in the allowlist, should be rejected
        let service = CommandService::new(10);
        let exec = CommandExecution::new("echo hello".to_string(), 10);

        let result = service.execute(exec).await;
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[tokio::test]
    async fn test_execute_rejects_false_not_in_allowlist() {
        // SECURITY: "false" is not in the allowlist, should be rejected
        let service = CommandService::new(10);
        let exec = CommandExecution::new("false".to_string(), 10);

        let result = service.execute(exec).await;
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[tokio::test]
    async fn test_execute_rejects_sleep_not_in_allowlist() {
        // SECURITY: "sleep" is not in the allowlist, should be rejected
        let service = CommandService::new(1);
        let exec = CommandExecution::new("sleep".to_string(), 1);

        let result = service.execute(exec).await;
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[tokio::test]
    async fn test_execute_rejects_nonexistent_command() {
        // SECURITY: Any nonexistent command is also not in the allowlist
        let service = CommandService::new(10);
        let exec = CommandExecution::new("nonexistent_command_xyz_abc".to_string(), 10);

        let result = service.execute(exec).await;
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[tokio::test]
    async fn test_execute_empty_command() {
        let service = CommandService::new(10);
        let exec = CommandExecution::new(String::new(), 10);

        let result = service.execute(exec).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CommandError::ExecutionFailed(msg) => {
                assert!(msg.contains("Empty"))
            }
            _ => panic!("Expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn test_streaming_rejects_disallowed_program() {
        // SECURITY: execute_with_streaming should also reject disallowed programs
        let service = CommandService::new(10);
        let exec = CommandExecution::new("echo test".to_string(), 10);

        let result = service.execute_with_streaming(exec, |_line| {}).await;

        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[test]
    fn test_parse_command_unit() {
        // Unit test for the parse_command helper function
        use super::parse_command;

        // Simple command
        let result = parse_command("echo hello").unwrap();
        assert_eq!(result, vec!["echo", "hello"]);

        // Quoted argument
        let result = parse_command(r#"echo "hello world""#).unwrap();
        assert_eq!(result, vec!["echo", "hello world"]);

        // Single quoted argument
        let result = parse_command("echo 'hello world'").unwrap();
        assert_eq!(result, vec!["echo", "hello world"]);

        // Escaped space
        let result = parse_command(r"echo hello\ world").unwrap();
        assert_eq!(result, vec!["echo", "hello world"]);

        // Mixed quotes
        let result = parse_command(r#"echo "hello 'nested' world""#).unwrap();
        assert_eq!(result, vec!["echo", "hello 'nested' world"]);

        // Multiple quoted args
        let result = parse_command(r#"cmd "arg one" "arg two""#).unwrap();
        assert_eq!(result, vec!["cmd", "arg one", "arg two"]);

        // Empty string should fail
        assert!(parse_command("").is_err());

        // Whitespace only should fail
        assert!(parse_command("   ").is_err());
    }

    // ==========================================================================
    // Security Tests
    // ==========================================================================

    #[test]
    fn test_validate_program_allowlist() {
        // Allowed programs
        assert!(validate_program("gat").is_ok());
        assert!(validate_program("gat-cli").is_ok());
        assert!(validate_program("/usr/bin/gat").is_ok());
        assert!(validate_program("/usr/local/bin/gat-cli").is_ok());

        // Disallowed programs
        assert!(matches!(
            validate_program("echo"),
            Err(CommandError::SecurityViolation(_))
        ));
        assert!(matches!(
            validate_program("bash"),
            Err(CommandError::SecurityViolation(_))
        ));
        assert!(matches!(
            validate_program("sh"),
            Err(CommandError::SecurityViolation(_))
        ));
        assert!(matches!(
            validate_program("rm"),
            Err(CommandError::SecurityViolation(_))
        ));
        assert!(matches!(
            validate_program("/bin/sh"),
            Err(CommandError::SecurityViolation(_))
        ));
    }

    #[test]
    fn test_validate_argument_safe() {
        // Safe arguments
        assert!(validate_argument("dataset-id-123").is_ok());
        assert!(validate_argument("output.json").is_ok());
        assert!(validate_argument("--format").is_ok());
        assert!(validate_argument("/path/to/file.json").is_ok());
        assert!(validate_argument("my_dataset").is_ok());
    }

    #[test]
    fn test_validate_argument_injection_attempts() {
        // Command chaining with semicolon
        assert!(matches!(
            validate_argument("foo; rm -rf /"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Pipe injection
        assert!(matches!(
            validate_argument("foo | cat /etc/passwd"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Background execution
        assert!(matches!(
            validate_argument("foo & malicious_cmd"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Command substitution with $()
        assert!(matches!(
            validate_argument("$(whoami)"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Command substitution with backticks
        assert!(matches!(
            validate_argument("`id`"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Output redirection
        assert!(matches!(
            validate_argument("foo > /etc/passwd"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Input redirection
        assert!(matches!(
            validate_argument("foo < /etc/shadow"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Newline injection
        assert!(matches!(
            validate_argument("foo\nrm -rf /"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Null byte injection
        assert!(matches!(
            validate_argument("foo\0bar"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Variable expansion
        assert!(matches!(
            validate_argument("$HOME"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Brace expansion
        assert!(matches!(
            validate_argument("{a,b}"),
            Err(CommandError::SecurityViolation(_))
        ));

        // Subshell
        assert!(matches!(
            validate_argument("(echo foo)"),
            Err(CommandError::SecurityViolation(_))
        ));
    }

    #[test]
    fn test_secure_command_builder_basic() {
        let builder = SecureCommandBuilder::new("gat").unwrap();
        let cmd = builder.subcommand("dataset").subcommand("list").build();
        assert_eq!(cmd, "gat dataset list");
    }

    #[test]
    fn test_secure_command_builder_with_args() {
        let builder = SecureCommandBuilder::new("gat-cli").unwrap();
        let cmd = builder
            .subcommand("dataset")
            .subcommand("describe")
            .arg("my-dataset-id")
            .unwrap()
            .flag("--format", "json")
            .unwrap()
            .build();
        assert_eq!(cmd, "gat-cli dataset describe my-dataset-id --format json");
    }

    #[test]
    fn test_secure_command_builder_rejects_disallowed_program() {
        let result = SecureCommandBuilder::new("rm");
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[test]
    fn test_secure_command_builder_rejects_dangerous_arg() {
        let builder = SecureCommandBuilder::new("gat").unwrap();
        let result = builder.subcommand("dataset").arg("foo; rm -rf /");
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[test]
    fn test_secure_command_builder_rejects_dangerous_flag_value() {
        let builder = SecureCommandBuilder::new("gat").unwrap();
        let result = builder.subcommand("dataset").flag("--out", "$(malicious)");
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[tokio::test]
    async fn test_execute_rejects_disallowed_program() {
        let service = CommandService::new(10);
        let exec = CommandExecution::new("rm -rf /".to_string(), 10);

        let result = service.execute(exec).await;
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[tokio::test]
    async fn test_execute_rejects_injection_in_args() {
        let service = CommandService::new(10);
        // Even if gat is the program, injection in args should be blocked
        let exec = CommandExecution::new("gat dataset 'foo; rm -rf /'".to_string(), 10);

        let result = service.execute(exec).await;
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }

    #[tokio::test]
    async fn test_execute_rejects_pipe_injection() {
        let service = CommandService::new(10);
        let exec = CommandExecution::new("gat dataset 'foo | cat /etc/passwd'".to_string(), 10);

        let result = service.execute(exec).await;
        assert!(matches!(result, Err(CommandError::SecurityViolation(_))));
    }
}
