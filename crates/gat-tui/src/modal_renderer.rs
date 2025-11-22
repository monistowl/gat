/// Modal rendering system for tuirealm-based TUI
///
/// Provides rendering functions for all modal types (confirmation, info, commands, settings).
/// Modals are rendered as overlays on top of the current pane, centered on screen.

use crate::models::{
    ModalState, CommandModalState, SettingsModalState, ConfirmationState, InfoState,
    ExecutionMode, Theme,
};

/// Central modal renderer
pub struct ModalRenderer;

impl ModalRenderer {
    /// Render the appropriate modal based on state
    pub fn render(modal_state: &ModalState, width: u16, height: u16) -> Option<String> {
        match modal_state {
            ModalState::None => None,
            ModalState::CommandExecution(state) => Some(Self::render_command_modal(state, width, height)),
            ModalState::Settings(state) => Some(Self::render_settings_modal(state, width, height)),
            ModalState::Confirmation(state) => Some(Self::render_confirmation_modal(state, width, height)),
            ModalState::Info(state) => Some(Self::render_info_modal(state, width, height)),
        }
    }

    /// Render command execution modal
    fn render_command_modal(state: &CommandModalState, width: u16, height: u16) -> String {
        let modal_width = (width as usize).saturating_sub(4).max(40);
        let modal_height = (height as usize).saturating_sub(4);
        let start_col = ((width as usize).saturating_sub(modal_width)) / 2;
        let start_row = ((height as usize).saturating_sub(modal_height)) / 2;

        let mut output = String::new();
        output.push_str(&Self::render_backdrop(width, height, start_row, start_col, modal_height, modal_width));
        output.push_str(&Self::render_command_content(state, start_row, start_col, modal_width, modal_height));

        output
    }

    /// Render command modal content
    fn render_command_content(
        state: &CommandModalState,
        _start_row: usize,
        start_col: usize,
        width: usize,
        height: usize,
    ) -> String {
        let mut lines = Vec::new();

        // Header
        lines.push(format!(
            "{}┌─ Command Execution {}─┐",
            Self::spaces(start_col),
            Self::horizontal(width.saturating_sub(24))
        ));

        // Command text (centered, scrollable)
        lines.push(format!(
            "{}│ Command:                                     │",
            Self::spaces(start_col)
        ));

        let cmd_lines = state.command_text.lines().take(height.saturating_sub(8)).collect::<Vec<_>>();
        for line in cmd_lines {
            let truncated = if line.len() > width.saturating_sub(4) {
                format!("{}...", &line[..width.saturating_sub(7)])
            } else {
                line.to_string()
            };
            lines.push(format!(
                "{}│ > {}{}│",
                Self::spaces(start_col),
                truncated,
                Self::spaces(width.saturating_sub(5).saturating_sub(truncated.len()))
            ));
        }

        // Execution mode
        lines.push(format!(
            "{}│                                             │",
            Self::spaces(start_col)
        ));
        let mode_indicator = match state.execution_mode {
            ExecutionMode::DryRun => "◯ Dry-run",
            ExecutionMode::Full => "● Full",
        };
        lines.push(format!(
            "{}│ Mode: {}{}│",
            Self::spaces(start_col),
            mode_indicator,
            Self::spaces(width.saturating_sub(10).saturating_sub(mode_indicator.len()))
        ));

        // Output section
        lines.push(format!(
            "{}│ Output:                                     │",
            Self::spaces(start_col)
        ));

        let output_lines = state.output.iter().take(height.saturating_sub(12)).collect::<Vec<_>>();
        for line in output_lines {
            let truncated = if line.len() > width.saturating_sub(4) {
                format!("{}...", &line[..width.saturating_sub(7)])
            } else {
                line.to_string()
            };
            lines.push(format!(
                "{}│ {}{}│",
                Self::spaces(start_col),
                truncated,
                Self::spaces(width.saturating_sub(5).saturating_sub(truncated.len()))
            ));
        }

        // Footer with controls
        lines.push(format!(
            "{}├─ [Enter] Execute │ [Tab] Mode │ [Esc] Close {}┤",
            Self::spaces(start_col),
            Self::horizontal(width.saturating_sub(45))
        ));

        lines.push(format!(
            "{}└{}┘",
            Self::spaces(start_col),
            Self::horizontal(width)
        ));

        lines.join("\n")
    }

    /// Render confirmation dialog
    fn render_confirmation_modal(state: &ConfirmationState, width: u16, height: u16) -> String {
        let modal_width = 50;
        let modal_height = 10;
        let start_col = ((width as usize).saturating_sub(modal_width)) / 2;
        let start_row = ((height as usize).saturating_sub(modal_height)) / 2;

        let mut output = String::new();
        output.push_str(&Self::render_backdrop(width, height, start_row, start_col, modal_height, modal_width));

        let mut lines = Vec::new();
        lines.push(format!(
            "{}┌─ Confirmation {}─┐",
            Self::spaces(start_col),
            Self::horizontal(modal_width.saturating_sub(20))
        ));

        // Message (word-wrapped)
        let wrapped = Self::word_wrap(&state.message, modal_width.saturating_sub(4));
        for line in wrapped {
            lines.push(format!(
                "{}│ {}{}│",
                Self::spaces(start_col),
                line,
                Self::spaces(modal_width.saturating_sub(4).saturating_sub(line.len()))
            ));
        }

        lines.push(format!(
            "{}│                                                  │",
            Self::spaces(start_col)
        ));

        // Buttons
        let buttons = format!("[Y] {} │ [N] {}", state.yes_label, state.no_label);
        let button_pad = modal_width.saturating_sub(4).saturating_sub(buttons.len());
        lines.push(format!(
            "{}│ {}{}│",
            Self::spaces(start_col),
            buttons,
            Self::spaces(button_pad)
        ));

        lines.push(format!(
            "{}└{}┘",
            Self::spaces(start_col),
            Self::horizontal(modal_width)
        ));

        output.push_str(&lines.join("\n"));
        output
    }

    /// Render info/alert dialog
    fn render_info_modal(state: &InfoState, width: u16, height: u16) -> String {
        let modal_width = 60;
        let modal_height = 12;
        let start_col = ((width as usize).saturating_sub(modal_width)) / 2;
        let start_row = ((height as usize).saturating_sub(modal_height)) / 2;

        let mut output = String::new();
        output.push_str(&Self::render_backdrop(width, height, start_row, start_col, modal_height, modal_width));

        let mut lines = Vec::new();
        lines.push(format!(
            "{}┌─ {} {}─┐",
            Self::spaces(start_col),
            state.title,
            Self::horizontal(modal_width.saturating_sub(state.title.len() + 8))
        ));

        // Message
        let wrapped = Self::word_wrap(&state.message, modal_width.saturating_sub(4));
        for line in wrapped.iter().take(3) {
            lines.push(format!(
                "{}│ {}{}│",
                Self::spaces(start_col),
                line,
                Self::spaces(modal_width.saturating_sub(4).saturating_sub(line.len()))
            ));
        }

        // Details if present
        if let Some(details) = &state.details {
            lines.push(format!(
                "{}│                                                        │",
                Self::spaces(start_col)
            ));
            let detail_wrapped = Self::word_wrap(details, modal_width.saturating_sub(6));
            for line in detail_wrapped.iter().take(2) {
                lines.push(format!(
                    "{}│   {}{}│",
                    Self::spaces(start_col),
                    line,
                    Self::spaces(modal_width.saturating_sub(6).saturating_sub(line.len()))
                ));
            }
        }

        lines.push(format!(
            "{}│                                                        │",
            Self::spaces(start_col)
        ));
        lines.push(format!(
            "{}├─ [Enter] OK {}─┤",
            Self::spaces(start_col),
            Self::horizontal(modal_width.saturating_sub(18))
        ));

        lines.push(format!(
            "{}└{}┘",
            Self::spaces(start_col),
            Self::horizontal(modal_width)
        ));

        output.push_str(&lines.join("\n"));
        output
    }

    /// Render settings modal
    fn render_settings_modal(state: &SettingsModalState, width: u16, height: u16) -> String {
        let modal_width = 65;
        let modal_height = 16;
        let start_col = ((width as usize).saturating_sub(modal_width)) / 2;

        let mut output = String::new();
        output.push_str(&Self::render_backdrop(width, height, 3, start_col, modal_height, modal_width));

        let mut lines = Vec::new();
        lines.push(format!(
            "{}┌─ Settings {}─┐",
            Self::spaces(start_col),
            Self::horizontal(modal_width.saturating_sub(18))
        ));

        // Theme setting
        let theme_indicator = match state.theme {
            Theme::Dark => "●",
            Theme::Light => "○",
        };
        lines.push(format!(
            "{}│ {} Theme: Dark  ○ Light                     │",
            Self::spaces(start_col),
            theme_indicator
        ));

        // Auto-save setting
        let auto_save_indicator = if state.auto_save { "✓" } else { "✗" };
        lines.push(format!(
            "{}│ {} Auto-save on pane switch                  │",
            Self::spaces(start_col),
            auto_save_indicator
        ));

        // Confirm delete setting
        let confirm_indicator = if state.confirm_delete { "✓" } else { "✗" };
        lines.push(format!(
            "{}│ {} Confirm on delete                         │",
            Self::spaces(start_col),
            confirm_indicator
        ));

        lines.push(format!(
            "{}│                                                 │",
            Self::spaces(start_col)
        ));

        lines.push(format!(
            "{}├─ [↑↓] Navigate │ [Space] Toggle │ [Esc] Close {}─┤",
            Self::spaces(start_col),
            Self::horizontal(modal_width.saturating_sub(53))
        ));

        lines.push(format!(
            "{}└{}┘",
            Self::spaces(start_col),
            Self::horizontal(modal_width)
        ));

        output.push_str(&lines.join("\n"));
        output
    }

    /// Render semi-transparent backdrop
    fn render_backdrop(
        width: u16,
        height: u16,
        modal_start_row: usize,
        modal_start_col: usize,
        modal_height: usize,
        modal_width: usize,
    ) -> String {
        let mut lines = Vec::new();

        // Rows above modal (partial transparency effect with dim text)
        for _ in 0..modal_start_row {
            lines.push(format!("{}\x1b[2m{}\x1b[0m", "", "".repeat(width as usize)));
        }

        // Rows covering modal sides (dim)
        for _ in 0..modal_height {
            let before = format!("\x1b[2m{}\x1b[0m", "▓".repeat(modal_start_col));
            let after = format!("\x1b[2m{}\x1b[0m", "▓".repeat((width as usize).saturating_sub(modal_start_col + modal_width)));
            lines.push(format!("{}{}", before, after));
        }

        // Rows below modal (partial transparency)
        let remaining = (height as usize).saturating_sub(modal_start_row + modal_height);
        for _ in 0..remaining {
            lines.push(format!("{}\x1b[2m{}\x1b[0m", "", "".repeat(width as usize)));
        }

        lines.join("\n")
    }

    /// Helper: create spaces for padding
    fn spaces(count: usize) -> String {
        " ".repeat(count)
    }

    /// Helper: create horizontal line
    fn horizontal(count: usize) -> String {
        "─".repeat(count)
    }

    /// Helper: word-wrap text to fit within width
    fn word_wrap(text: &str, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_no_modal() {
        let result = ModalRenderer::render(&ModalState::None, 80, 24);
        assert!(result.is_none());
    }

    #[test]
    fn test_render_confirmation() {
        let state = ConfirmationState {
            message: "Delete this item?".to_string(),
            yes_label: "Delete".to_string(),
            no_label: "Cancel".to_string(),
        };
        let result = ModalRenderer::render(&ModalState::Confirmation(state), 80, 24);
        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Delete this item?"));
        assert!(output.contains("Delete"));
        assert!(output.contains("Cancel"));
    }

    #[test]
    fn test_render_info() {
        let state = InfoState {
            title: "Error".to_string(),
            message: "Something went wrong".to_string(),
            details: Some("Details here".to_string()),
        };
        let result = ModalRenderer::render(&ModalState::Info(state), 80, 24);
        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Error"));
        assert!(output.contains("Something went wrong"));
        assert!(output.contains("Details here"));
    }

    #[test]
    fn test_render_command_modal() {
        let state = CommandModalState {
            command_text: "gat-cli test".to_string(),
            execution_mode: ExecutionMode::DryRun,
            output: vec!["output line 1".to_string()],
        };
        let result = ModalRenderer::render(&ModalState::CommandExecution(state), 100, 30);
        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Command Execution"));
        assert!(output.contains("gat-cli test"));
    }

    #[test]
    fn test_render_settings_modal() {
        let state = SettingsModalState {
            selected_field: 0,
            theme: Theme::Dark,
            auto_save: true,
            confirm_delete: false,
        };
        let result = ModalRenderer::render(&ModalState::Settings(state), 80, 24);
        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Settings"));
        assert!(output.contains("Theme"));
        assert!(output.contains("Auto-save"));
    }

    #[test]
    fn test_word_wrap() {
        let text = "This is a long message that needs to be wrapped";
        let wrapped = ModalRenderer::word_wrap(text, 15);
        assert!(!wrapped.is_empty());
        for line in wrapped {
            assert!(line.len() <= 15);
        }
    }

    #[test]
    fn test_backdrop_dimensions() {
        let result = ModalRenderer::render_backdrop(80, 24, 5, 10, 10, 60);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 24); // Total height
    }
}
