/// ANSI escape code generation for terminal styling.
///
/// This module provides helpers to generate ANSI escape sequences for colors,
/// text styles, and formatting in terminal output.

/// ANSI escape code sequences
#[derive(Debug, Clone, Copy)]
pub struct AnsiCode(&'static str);

impl AnsiCode {
    /// Apply this code to text
    pub fn apply<S: AsRef<str>>(self, text: S) -> String {
        format!("{}{}\x1b[0m", self.0, text.as_ref())
    }
}

// Foreground colors
pub const COLOR_RED: AnsiCode = AnsiCode("\x1b[31m");
pub const COLOR_GREEN: AnsiCode = AnsiCode("\x1b[32m");
pub const COLOR_YELLOW: AnsiCode = AnsiCode("\x1b[33m");
#[allow(dead_code)]
pub const COLOR_BLUE: AnsiCode = AnsiCode("\x1b[34m");
#[allow(dead_code)]
pub const COLOR_MAGENTA: AnsiCode = AnsiCode("\x1b[35m");
pub const COLOR_CYAN: AnsiCode = AnsiCode("\x1b[36m");
#[allow(dead_code)]
pub const COLOR_WHITE: AnsiCode = AnsiCode("\x1b[37m");

// Bright foreground colors
#[allow(dead_code)]
pub const COLOR_BRIGHT_RED: AnsiCode = AnsiCode("\x1b[91m");
#[allow(dead_code)]
pub const COLOR_BRIGHT_GREEN: AnsiCode = AnsiCode("\x1b[92m");
#[allow(dead_code)]
pub const COLOR_BRIGHT_YELLOW: AnsiCode = AnsiCode("\x1b[93m");
#[allow(dead_code)]
pub const COLOR_BRIGHT_CYAN: AnsiCode = AnsiCode("\x1b[96m");
#[allow(dead_code)]
pub const COLOR_BRIGHT_WHITE: AnsiCode = AnsiCode("\x1b[97m");

// Text styles
pub const BOLD: AnsiCode = AnsiCode("\x1b[1m");
pub const DIM: AnsiCode = AnsiCode("\x1b[2m");
#[allow(dead_code)]
pub const ITALIC: AnsiCode = AnsiCode("\x1b[3m");
#[allow(dead_code)]
pub const UNDERLINE: AnsiCode = AnsiCode("\x1b[4m");
pub const REVERSE: AnsiCode = AnsiCode("\x1b[7m");

// Reset
pub const RESET: &str = "\x1b[0m";

/// Combine multiple styles
pub struct StyledText {
    codes: Vec<&'static str>,
}

impl StyledText {
    pub fn new() -> Self {
        Self {
            codes: Vec::new(),
        }
    }

    pub fn color(mut self, code: AnsiCode) -> Self {
        self.codes.push(code.0);
        self
    }

    pub fn bold(mut self) -> Self {
        self.codes.push(BOLD.0);
        self
    }

    pub fn dim(mut self) -> Self {
        self.codes.push(DIM.0);
        self
    }

    pub fn reverse(mut self) -> Self {
        self.codes.push(REVERSE.0);
        self
    }

    pub fn apply<S: AsRef<str>>(self, text: S) -> String {
        let prefix = self.codes.join("");
        format!("{}{}{}", prefix, text.as_ref(), RESET)
    }
}

impl Default for StyledText {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_code_application() {
        let result = COLOR_RED.apply("error");
        assert!(result.contains("\x1b[31m"));
        assert!(result.contains("error"));
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_styled_text() {
        let result = StyledText::new()
            .color(COLOR_CYAN)
            .bold()
            .apply("Dashboard");

        assert!(result.contains("\x1b[36m")); // Cyan
        assert!(result.contains("\x1b[1m"));  // Bold
        assert!(result.contains("Dashboard"));
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_multiple_codes() {
        let result = StyledText::new()
            .color(COLOR_GREEN)
            .dim()
            .apply("inactive");

        assert!(result.contains("\x1b[32m")); // Green
        assert!(result.contains("\x1b[2m"));  // Dim
        assert!(result.contains("inactive"));
    }
}
