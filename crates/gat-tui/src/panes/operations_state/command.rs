//! Command execution state management
//!
//! Handles command input, output buffering, and execution lifecycle.

/// Maximum number of output lines to retain
const MAX_OUTPUT_LINES: usize = 1000;

/// State for command execution
#[derive(Clone, Debug, Default)]
pub struct CommandState {
    /// Current command input text
    input: String,
    /// Command output lines
    output: Vec<String>,
    /// Whether a command is currently executing
    executing: bool,
    /// Whether the current input has been validated
    validated: bool,
    /// Exit code from last command execution
    last_exit_code: Option<i32>,
    /// Duration of last command execution in milliseconds
    last_duration_ms: Option<u64>,
}

impl CommandState {
    /// Create a new empty CommandState
    pub fn new() -> Self {
        Self::default()
    }

    // ============================================================================
    // Input management
    // ============================================================================

    /// Set the command input text
    pub fn set_input(&mut self, input: String) {
        self.input = input;
        self.validated = false;
    }

    /// Add a character to the command input
    pub fn add_char(&mut self, ch: char) {
        self.input.push(ch);
        self.validated = false;
    }

    /// Remove the last character from command input
    pub fn backspace(&mut self) {
        self.input.pop();
        self.validated = false;
    }

    /// Clear the command input
    pub fn clear_input(&mut self) {
        self.input.clear();
        self.validated = false;
    }

    /// Get the current command input
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Check if input is empty
    pub fn is_input_empty(&self) -> bool {
        self.input.is_empty()
    }

    // ============================================================================
    // Validation
    // ============================================================================

    /// Mark the command as validated
    pub fn set_validated(&mut self, validated: bool) {
        self.validated = validated;
    }

    /// Check if the command has been validated
    pub fn is_validated(&self) -> bool {
        self.validated
    }

    // ============================================================================
    // Execution lifecycle
    // ============================================================================

    /// Start command execution
    pub fn start_execution(&mut self) {
        self.executing = true;
        self.output.clear();
        self.last_exit_code = None;
        self.last_duration_ms = None;
    }

    /// Add an output line from the running command
    pub fn add_output(&mut self, line: String) {
        self.output.push(line);
        // Limit output to prevent memory issues
        if self.output.len() > MAX_OUTPUT_LINES {
            self.output.remove(0);
        }
    }

    /// Complete command execution with result
    pub fn complete_execution(&mut self, exit_code: i32, duration_ms: u64) {
        self.executing = false;
        self.last_exit_code = Some(exit_code);
        self.last_duration_ms = Some(duration_ms);
    }

    /// Check if a command is currently executing
    pub fn is_executing(&self) -> bool {
        self.executing
    }

    // ============================================================================
    // Output access
    // ============================================================================

    /// Get all output lines
    pub fn output(&self) -> &[String] {
        &self.output
    }

    /// Get output as a single joined string
    pub fn output_text(&self) -> String {
        self.output.join("\n")
    }

    /// Get the number of output lines
    pub fn output_line_count(&self) -> usize {
        self.output.len()
    }

    /// Clear all output
    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    // ============================================================================
    // Result access
    // ============================================================================

    /// Get the last exit code
    pub fn last_exit_code(&self) -> Option<i32> {
        self.last_exit_code
    }

    /// Get the last command duration in milliseconds
    pub fn last_duration_ms(&self) -> Option<u64> {
        self.last_duration_ms
    }

    /// Get a status string for the current state
    pub fn status(&self) -> &'static str {
        if self.executing {
            "Running..."
        } else if let Some(code) = self.last_exit_code {
            if code == 0 {
                "Success"
            } else {
                "Failed"
            }
        } else {
            "Ready"
        }
    }

    /// Check if the last command was successful
    pub fn was_successful(&self) -> bool {
        self.last_exit_code == Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_state_init() {
        let state = CommandState::new();
        assert!(state.input().is_empty());
        assert!(state.output().is_empty());
        assert!(!state.is_executing());
        assert!(!state.is_validated());
        assert_eq!(state.status(), "Ready");
    }

    #[test]
    fn test_input_management() {
        let mut state = CommandState::new();

        state.set_input("gat-cli datasets list".to_string());
        assert_eq!(state.input(), "gat-cli datasets list");

        state.clear_input();
        assert!(state.is_input_empty());
    }

    #[test]
    fn test_char_input() {
        let mut state = CommandState::new();

        state.add_char('e');
        state.add_char('c');
        state.add_char('h');
        state.add_char('o');

        assert_eq!(state.input(), "echo");
    }

    #[test]
    fn test_backspace() {
        let mut state = CommandState::new();
        state.set_input("hello".to_string());

        state.backspace();
        assert_eq!(state.input(), "hell");

        state.backspace();
        state.backspace();
        assert_eq!(state.input(), "he");
    }

    #[test]
    fn test_validation_flag() {
        let mut state = CommandState::new();
        assert!(!state.is_validated());

        state.set_validated(true);
        assert!(state.is_validated());

        // Input change should reset validation
        state.set_input("new command".to_string());
        assert!(!state.is_validated());
    }

    #[test]
    fn test_char_input_resets_validation() {
        let mut state = CommandState::new();
        state.set_validated(true);
        assert!(state.is_validated());

        state.add_char('x');
        assert!(!state.is_validated());
    }

    #[test]
    fn test_execution_lifecycle() {
        let mut state = CommandState::new();

        // Initially not executing
        assert!(!state.is_executing());
        assert!(state.last_exit_code().is_none());
        assert_eq!(state.status(), "Ready");

        // Start execution
        state.start_execution();
        assert!(state.is_executing());
        assert_eq!(state.status(), "Running...");

        // Add output
        state.add_output("Line 1".to_string());
        state.add_output("Line 2".to_string());
        assert_eq!(state.output_line_count(), 2);

        // Complete execution
        state.complete_execution(0, 150);
        assert!(!state.is_executing());
        assert_eq!(state.last_exit_code(), Some(0));
        assert_eq!(state.last_duration_ms(), Some(150));
        assert_eq!(state.status(), "Success");
        assert!(state.was_successful());
    }

    #[test]
    fn test_output_accumulation() {
        let mut state = CommandState::new();
        state.start_execution();

        for i in 0..10 {
            state.add_output(format!("Output line {}", i));
        }

        assert_eq!(state.output_line_count(), 10);
        assert_eq!(
            state.output_text(),
            "Output line 0\nOutput line 1\nOutput line 2\nOutput line 3\nOutput line 4\n\
             Output line 5\nOutput line 6\nOutput line 7\nOutput line 8\nOutput line 9"
        );
    }

    #[test]
    fn test_output_limit() {
        let mut state = CommandState::new();
        state.start_execution();

        // Add more than MAX_OUTPUT_LINES
        for i in 0..1100 {
            state.add_output(format!("Line {}", i));
        }

        // Should only keep last MAX_OUTPUT_LINES
        assert_eq!(state.output_line_count(), MAX_OUTPUT_LINES);
        assert!(state.output()[0].contains("100")); // First kept line
    }

    #[test]
    fn test_clear_output() {
        let mut state = CommandState::new();
        state.start_execution();
        state.add_output("Some output".to_string());

        assert!(!state.output().is_empty());
        state.clear_output();
        assert!(state.output().is_empty());
    }

    #[test]
    fn test_status_strings() {
        let mut state = CommandState::new();

        assert_eq!(state.status(), "Ready");

        state.start_execution();
        assert_eq!(state.status(), "Running...");

        state.complete_execution(0, 100);
        assert_eq!(state.status(), "Success");

        state.start_execution();
        state.complete_execution(1, 50);
        assert_eq!(state.status(), "Failed");
        assert!(!state.was_successful());
    }

    #[test]
    fn test_failed_execution() {
        let mut state = CommandState::new();

        state.start_execution();
        state.add_output("Error message".to_string());
        state.complete_execution(127, 200);

        assert!(!state.is_executing());
        assert_eq!(state.last_exit_code(), Some(127));
        assert_eq!(state.status(), "Failed");
    }
}
