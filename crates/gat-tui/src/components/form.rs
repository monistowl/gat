// Form component system
//
// Provides two-tier component architecture:
// 1. Stateless rendering functions - pure functions that render widgets
// 2. Optional state wrappers - manage form state and interaction

use std::collections::HashMap;
use tuirealm::ratatui::style::{Color, Modifier, Style};
use tuirealm::ratatui::text::{Line, Span};

/// A form field configuration with label and value
#[derive(Debug, Clone)]
pub enum FormField {
    TextInput {
        label: String,
        value: String,
        placeholder: Option<String>,
    },
    Checkbox {
        label: String,
        checked: bool,
    },
    Select {
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    TextArea {
        label: String,
        value: String,
        line_count: usize,
    },
}

impl FormField {
    pub fn label(&self) -> &str {
        match self {
            FormField::TextInput { label, .. } => label,
            FormField::Checkbox { label, .. } => label,
            FormField::Select { label, .. } => label,
            FormField::TextArea { label, .. } => label,
        }
    }
}

/// A section of related form fields
#[derive(Debug, Clone)]
pub struct FormSection {
    pub title: String,
    pub fields: Vec<FormField>,
}

/// State management for interactive forms
#[derive(Debug, Clone)]
pub struct ConfigFormState {
    pub sections: Vec<FormSection>,
    pub focused_field: (usize, usize), // (section_idx, field_idx)
    pub errors: HashMap<(usize, usize), String>,
    pub dirty: bool,
}

impl ConfigFormState {
    pub fn new(sections: Vec<FormSection>) -> Self {
        Self {
            sections,
            focused_field: (0, 0),
            errors: HashMap::new(),
            dirty: false,
        }
    }

    /// Move focus to the next field
    pub fn focus_next(&mut self) {
        let (mut section_idx, mut field_idx) = self.focused_field;

        // Try to move to next field in current section
        if field_idx + 1 < self.sections[section_idx].fields.len() {
            field_idx += 1;
        } else if section_idx + 1 < self.sections.len() {
            // Move to first field of next section
            section_idx += 1;
            field_idx = 0;
        }
        // Otherwise stay at the end

        self.focused_field = (section_idx, field_idx);
    }

    /// Move focus to the previous field
    pub fn focus_prev(&mut self) {
        let (mut section_idx, mut field_idx) = self.focused_field;

        if field_idx > 0 {
            field_idx -= 1;
        } else if section_idx > 0 {
            section_idx -= 1;
            field_idx = self.sections[section_idx].fields.len() - 1;
        }
        // Otherwise stay at the beginning

        self.focused_field = (section_idx, field_idx);
    }

    /// Get mutable reference to the currently focused field
    pub fn get_focused_field_mut(&mut self) -> &mut FormField {
        let (section_idx, field_idx) = self.focused_field;
        &mut self.sections[section_idx].fields[field_idx]
    }

    /// Get immutable reference to the currently focused field
    pub fn get_focused_field(&self) -> &FormField {
        let (section_idx, field_idx) = self.focused_field;
        &self.sections[section_idx].fields[field_idx]
    }

    /// Input a character to the currently focused field
    pub fn input_char(&mut self, c: char) {
        match self.get_focused_field_mut() {
            FormField::TextInput { value, .. } => {
                value.push(c);
                self.dirty = true;
            }
            FormField::TextArea { value, .. } => {
                value.push(c);
                self.dirty = true;
            }
            _ => {} // Checkboxes and selects don't take character input
        }
    }

    /// Delete the last character from the focused field
    pub fn delete_char(&mut self) {
        match self.get_focused_field_mut() {
            FormField::TextInput { value, .. } => {
                value.pop();
                self.dirty = true;
            }
            FormField::TextArea { value, .. } => {
                value.pop();
                self.dirty = true;
            }
            _ => {}
        }
    }

    /// Toggle checkbox state
    pub fn toggle_checkbox(&mut self) {
        if let FormField::Checkbox { checked, .. } = self.get_focused_field_mut() {
            *checked = !*checked;
            self.dirty = true;
        }
    }

    /// Move to next option in select field
    pub fn select_next(&mut self) {
        if let FormField::Select {
            options,
            selected,
            ..
        } = self.get_focused_field_mut()
        {
            if *selected + 1 < options.len() {
                *selected += 1;
                self.dirty = true;
            }
        }
    }

    /// Move to previous option in select field
    pub fn select_prev(&mut self) {
        if let FormField::Select { selected, .. } = self.get_focused_field_mut() {
            if *selected > 0 {
                *selected -= 1;
                self.dirty = true;
            }
        }
    }

    /// Collect all form values into a map
    pub fn collect_values(&self) -> HashMap<String, String> {
        let mut values = HashMap::new();
        for section in &self.sections {
            for field in &section.fields {
                match field {
                    FormField::TextInput { label, value, .. } => {
                        values.insert(label.clone(), value.clone());
                    }
                    FormField::Checkbox { label, checked } => {
                        values.insert(label.clone(), checked.to_string());
                    }
                    FormField::Select {
                        label,
                        options,
                        selected,
                    } => {
                        if *selected < options.len() {
                            values.insert(label.clone(), options[*selected].clone());
                        }
                    }
                    FormField::TextArea { label, value, .. } => {
                        values.insert(label.clone(), value.clone());
                    }
                }
            }
        }
        values
    }

    /// Mark form as pristine (not dirty)
    pub fn reset_dirty(&mut self) {
        self.dirty = false;
    }

    /// Clear all errors
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }
}

// ============================================================================
// STATELESS RENDERING FUNCTIONS
// ============================================================================

/// Render a text input field
pub fn render_text_input(
    label: &str,
    value: &str,
    _is_focused: bool,
    _is_error: bool,
    placeholder: Option<&str>,
) -> String {
    let display_value = if value.is_empty() {
        placeholder.unwrap_or("").to_string()
    } else {
        value.to_string()
    };

    format!("{}: {}", label, display_value)
}

/// Render a checkbox field
pub fn render_checkbox(label: &str, is_checked: bool, _is_focused: bool) -> String {
    let check_char = if is_checked { "☑" } else { "☐" };
    format!("{} {}", check_char, label)
}

/// Render a select field with options
pub fn render_select(
    label: &str,
    options: &[String],
    selected_index: usize,
    _is_focused: bool,
) -> String {
    let selected_option = options
        .get(selected_index)
        .map(|s| s.as_str())
        .unwrap_or("(none)");

    format!("{}: [{}]", label, selected_option)
}

/// Render a multi-line text area
pub fn render_text_area(
    label: &str,
    value: &str,
    _is_focused: bool,
    _line_count: usize,
) -> String {
    // Simple representation for text area
    let line_count = value.lines().count();
    format!("{}: ({} lines)", label, line_count)
}

/// Render a form section with title and fields
pub fn render_form_section(
    title: &str,
    field_widgets: Vec<String>,
) -> (String, Vec<String>) {
    (format!("▶ {}", title), field_widgets)
}

/// Render a complete form with multiple sections
pub fn render_form_section_with_fields<'a>(
    section: &'a FormSection,
    form_state: &'a ConfigFormState,
) -> Vec<Line<'a>> {
    let (section_idx, _) = form_state.focused_field;
    let is_section_focused = {
        // Find section index
        form_state
            .sections
            .iter()
            .position(|s| s.title == section.title)
            .map(|idx| idx == section_idx)
            .unwrap_or(false)
    };

    let mut lines = vec![];

    // Section title
    let title_style = if is_section_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    lines.push(Line::from(Span::styled(
        format!("▶ {}", section.title),
        title_style,
    )));

    // Fields
    for (field_idx, field) in section.fields.iter().enumerate() {
        let is_focused =
            is_section_focused && form_state.focused_field.1 == field_idx;

        let error_msg = form_state
            .errors
            .get(&(section_idx, field_idx))
            .map(|e| format!(" ({})", e))
            .unwrap_or_default();

        let field_line = match field {
            FormField::TextInput {
                label,
                value,
                placeholder,
            } => {
                let indicator = if is_focused { "▶" } else { " " };
                let styled_value = if value.is_empty() {
                    placeholder.as_deref().unwrap_or("").to_string()
                } else {
                    value.clone()
                };

                let style = if is_focused {
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(vec![
                    Span::raw(format!("  {} ", indicator)),
                    Span::styled(format!("{}: ", label), style),
                    Span::styled(styled_value, style),
                    Span::styled(error_msg, Style::default().fg(Color::Red)),
                ])
            }
            FormField::Checkbox { label, checked } => {
                let indicator = if is_focused { "▶" } else { " " };
                let check_char = if *checked { "☑" } else { "☐" };
                let style = if is_focused {
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(vec![
                    Span::raw(format!("  {} {} ", indicator, check_char)),
                    Span::styled(label, style),
                    Span::styled(error_msg, Style::default().fg(Color::Red)),
                ])
            }
            FormField::Select {
                label,
                options,
                selected,
            } => {
                let indicator = if is_focused { "▶" } else { " " };
                let selected_option = options
                    .get(*selected)
                    .map(|s| s.as_str())
                    .unwrap_or("(none)");
                let style = if is_focused {
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(vec![
                    Span::raw(format!("  {} ", indicator)),
                    Span::styled(format!("{}: [{}]", label, selected_option), style),
                    Span::styled(error_msg, Style::default().fg(Color::Red)),
                ])
            }
            FormField::TextArea {
                label,
                value: _,
                line_count,
            } => {
                let indicator = if is_focused { "▶" } else { " " };
                let style = if is_focused {
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(vec![
                    Span::raw(format!("  {} ", indicator)),
                    Span::styled(format!("{} ({} lines)", label, line_count), style),
                    Span::styled(error_msg, Style::default().fg(Color::Red)),
                ])
            }
        };

        lines.push(field_line);
    }

    lines.push(Line::from("")); // Spacing
    lines
}
