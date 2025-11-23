/// Command execution UI components
///
/// Provides rendering helpers for displaying command results and output.
use crate::message::CommandResult;

/// Command result modal renderer
pub struct CommandResultModal;

impl CommandResultModal {
    /// Render command result as formatted text
    /// Returns lines of formatted output suitable for display
    pub fn render(result: &CommandResult) -> Vec<String> {
        let mut lines = Vec::new();

        // Header box
        lines
            .push("╔════════════════════════════════════════════════════════════════╗".to_string());
        lines
            .push("║                    COMMAND EXECUTION RESULT                    ║".to_string());
        lines
            .push("╚════════════════════════════════════════════════════════════════╝".to_string());
        lines.push(String::new());

        // Command info
        lines.push("Command:".to_string());
        lines.push(format!("  {}", result.command));
        lines.push(String::new());

        // Exit code with color indicator
        let status_indicator = if result.exit_code == 0 {
            "✓ Success"
        } else if result.timed_out {
            "⏱ Timeout"
        } else {
            "✗ Failed"
        };
        lines.push(format!(
            "Status: {} (exit code: {})",
            status_indicator, result.exit_code
        ));

        // Duration
        let duration_sec = result.duration_ms as f64 / 1000.0;
        lines.push(format!("Duration: {:.2}s", duration_sec));

        // Timeout indicator
        if result.timed_out {
            lines.push("⚠ Command timed out".to_string());
        }

        lines.push(String::new());
        lines.push("─".repeat(66));
        lines.push(String::new());

        // Standard output section
        if !result.stdout.is_empty() {
            lines.push("STDOUT:".to_string());
            for line in result.stdout.lines() {
                lines.push(format!("  {}", line));
            }
            lines.push(String::new());
        }

        // Standard error section
        if !result.stderr.is_empty() {
            lines.push("STDERR:".to_string());
            for line in result.stderr.lines() {
                lines.push(format!("  {}", line));
            }
            lines.push(String::new());
        }

        // Empty output indicator
        if result.stdout.is_empty() && result.stderr.is_empty() {
            lines.push("(No output)".to_string());
            lines.push(String::new());
        }

        // Footer
        lines.push("─".repeat(66));
        lines.push("Press [Enter] to dismiss or [C] to copy output".to_string());

        lines
    }

    /// Render result as single formatted text block
    pub fn render_text(result: &CommandResult) -> String {
        Self::render(result).join("\n")
    }
}

/// Command output viewer with scrolling support
pub struct CommandOutputViewer {
    pub lines: Vec<String>,
    pub scroll_position: usize,
}

impl CommandOutputViewer {
    /// Create new viewer from lines
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            lines,
            scroll_position: 0,
        }
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self) {
        let max_scroll = self.lines.len().saturating_sub(1);
        if self.scroll_position < max_scroll {
            self.scroll_position += 1;
        }
    }

    /// Scroll to end of output
    pub fn scroll_to_end(&mut self) {
        self.scroll_position = self.lines.len().saturating_sub(1);
    }

    /// Render visible lines for display
    /// height: number of lines available for display
    pub fn render(&self, height: usize) -> Vec<String> {
        if self.lines.is_empty() {
            return vec!["(No output)".to_string()];
        }

        let start = self
            .scroll_position
            .min(self.lines.len().saturating_sub(height));
        let end = (start + height).min(self.lines.len());

        self.lines[start..end].to_vec()
    }

    /// Get total number of lines
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Check if at end of output
    pub fn is_at_end(&self) -> bool {
        self.scroll_position >= self.lines.len().saturating_sub(1)
    }

    /// Get current position as percentage (0-100)
    pub fn scroll_percentage(&self) -> u32 {
        if self.lines.is_empty() {
            return 100;
        }
        ((self.scroll_position * 100) / self.lines.len().saturating_sub(1)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_result_modal_success() {
        let result = CommandResult {
            command: "echo test".to_string(),
            exit_code: 0,
            stdout: "test output".to_string(),
            stderr: String::new(),
            duration_ms: 150,
            timed_out: false,
        };

        let lines = CommandResultModal::render(&result);
        let text = lines.join("\n");

        assert!(text.contains("COMMAND EXECUTION RESULT"));
        assert!(text.contains("echo test"));
        assert!(text.contains("✓ Success"));
        assert!(text.contains("exit code: 0"));
        assert!(text.contains("test output"));
    }

    #[test]
    fn test_command_result_modal_failure() {
        let result = CommandResult {
            command: "false".to_string(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "error occurred".to_string(),
            duration_ms: 50,
            timed_out: false,
        };

        let lines = CommandResultModal::render(&result);
        let text = lines.join("\n");

        assert!(text.contains("✗ Failed"));
        assert!(text.contains("exit code: 1"));
        assert!(text.contains("error occurred"));
    }

    #[test]
    fn test_command_result_modal_timeout() {
        let result = CommandResult {
            command: "sleep 10".to_string(),
            exit_code: -1,
            stdout: "partial output".to_string(),
            stderr: String::new(),
            duration_ms: 5000,
            timed_out: true,
        };

        let lines = CommandResultModal::render(&result);
        let text = lines.join("\n");

        assert!(text.contains("⏱ Timeout"));
        assert!(text.contains("timed out"));
        assert!(text.contains("5000 ms") || text.contains("5.00s"));
    }

    #[test]
    fn test_command_result_modal_no_output() {
        let result = CommandResult {
            command: "true".to_string(),
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 10,
            timed_out: false,
        };

        let lines = CommandResultModal::render(&result);
        let text = lines.join("\n");

        assert!(text.contains("(No output)"));
    }

    #[test]
    fn test_command_result_modal_multiline_output() {
        let result = CommandResult {
            command: "echo multiline".to_string(),
            exit_code: 0,
            stdout: "line 1\nline 2\nline 3".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            timed_out: false,
        };

        let lines = CommandResultModal::render(&result);
        let text = lines.join("\n");

        assert!(text.contains("line 1"));
        assert!(text.contains("line 2"));
        assert!(text.contains("line 3"));
    }

    #[test]
    fn test_output_viewer_creation() {
        let lines = vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
        ];
        let viewer = CommandOutputViewer::new(lines.clone());

        assert_eq!(viewer.line_count(), 3);
        assert_eq!(viewer.scroll_position, 0);
        assert!(!viewer.is_at_end());
    }

    #[test]
    fn test_output_viewer_scroll_up_down() {
        let lines = vec![
            "L1".to_string(),
            "L2".to_string(),
            "L3".to_string(),
            "L4".to_string(),
        ];
        let mut viewer = CommandOutputViewer::new(lines);

        // Start at beginning
        assert_eq!(viewer.scroll_position, 0);

        // Scroll down
        viewer.scroll_down();
        assert_eq!(viewer.scroll_position, 1);

        viewer.scroll_down();
        assert_eq!(viewer.scroll_position, 2);

        // Scroll up
        viewer.scroll_up();
        assert_eq!(viewer.scroll_position, 1);

        viewer.scroll_up();
        assert_eq!(viewer.scroll_position, 0);

        // Can't scroll up from beginning
        viewer.scroll_up();
        assert_eq!(viewer.scroll_position, 0);
    }

    #[test]
    fn test_output_viewer_scroll_to_end() {
        let lines = vec!["L1".to_string(), "L2".to_string(), "L3".to_string()];
        let mut viewer = CommandOutputViewer::new(lines);

        assert_eq!(viewer.scroll_position, 0);
        viewer.scroll_to_end();
        assert_eq!(viewer.scroll_position, 2);
        assert!(viewer.is_at_end());
    }

    #[test]
    fn test_output_viewer_render() {
        let lines = vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
            "Line 4".to_string(),
            "Line 5".to_string(),
        ];
        let viewer = CommandOutputViewer::new(lines);

        let rendered = viewer.render(3);
        assert_eq!(rendered.len(), 3);
        assert_eq!(rendered[0], "Line 1");
        assert_eq!(rendered[1], "Line 2");
        assert_eq!(rendered[2], "Line 3");
    }

    #[test]
    fn test_output_viewer_render_with_scroll() {
        let lines = vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
            "Line 4".to_string(),
            "Line 5".to_string(),
        ];
        let mut viewer = CommandOutputViewer::new(lines);

        viewer.scroll_down();
        viewer.scroll_down();

        let rendered = viewer.render(3);
        assert_eq!(rendered.len(), 3);
        assert_eq!(rendered[0], "Line 3");
        assert_eq!(rendered[1], "Line 4");
        assert_eq!(rendered[2], "Line 5");
    }

    #[test]
    fn test_output_viewer_empty() {
        let viewer = CommandOutputViewer::new(vec![]);

        let rendered = viewer.render(10);
        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0], "(No output)");
    }

    #[test]
    fn test_output_viewer_scroll_percentage() {
        let lines = vec![
            "L1".to_string(),
            "L2".to_string(),
            "L3".to_string(),
            "L4".to_string(),
        ];
        let mut viewer = CommandOutputViewer::new(lines);

        assert_eq!(viewer.scroll_percentage(), 0);

        viewer.scroll_down();
        viewer.scroll_down();
        viewer.scroll_down();

        assert_eq!(viewer.scroll_percentage(), 100);
    }

    #[test]
    fn test_output_viewer_large_output() {
        let lines: Vec<String> = (0..1000).map(|i| format!("Line {}", i)).collect();

        let mut viewer = CommandOutputViewer::new(lines);
        assert_eq!(viewer.line_count(), 1000);

        viewer.scroll_to_end();
        assert!(viewer.is_at_end());

        let rendered = viewer.render(50);
        assert_eq!(rendered.len(), 50);
        assert!(rendered[0].contains("Line"));
    }

    #[test]
    fn test_command_result_modal_text() {
        let result = CommandResult {
            command: "echo hello".to_string(),
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            timed_out: false,
        };

        let text = CommandResultModal::render_text(&result);
        assert!(text.contains("COMMAND EXECUTION RESULT"));
        assert!(text.contains("echo hello"));
        assert!(text.contains("hello"));
    }
}
