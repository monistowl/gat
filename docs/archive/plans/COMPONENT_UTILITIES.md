# gat-tui Component Utilities Documentation

## Overview

The component utilities system provides a two-tier architecture for building reusable UI components in gat-tui:

1. **Stateless Rendering Functions** - Pure functions that convert data into displayable text
2. **Optional State Wrappers** - Structs that manage interactive state and business logic

This separation enables:
- Easy testing of rendering logic
- Clean separation of concerns
- Reusability across different panes
- Consistency in UI patterns

## Architecture

### Two-Tier Design

```
┌─────────────────────────────────────┐
│  Application State (main.rs)        │
│  Holds all mutable state            │
└──────────────┬──────────────────────┘
               │
       ┌───────▼────────────┐
       │  State Wrappers    │
       │  (ConfigFormState, │
       │   FileBrowserState)│
       └───────┬────────────┘
               │
       ┌───────▼─────────────────┐
       │ Rendering Functions     │
       │ (Pure, deterministic)   │
       └─────────────────────────┘
               │
               ▼
      ┌─────────────────────┐
      │  ratatui Widgets    │
      │  (Paragraph, Table) │
      └─────────────────────┘
```

**Key Principle:** State management is optional. Components can use just rendering functions without state wrappers for simple display cases.

## Form Components

Located in: `crates/gat-tui/src/components/form.rs`

### Data Structures

#### FormField
Represents a single form input element.

```rust
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
```

#### FormSection
Groups related fields together.

```rust
pub struct FormSection {
    pub title: String,
    pub fields: Vec<FormField>,
}
```

#### ConfigFormState
Manages interactive form behavior and validation.

```rust
pub struct ConfigFormState {
    pub sections: Vec<FormSection>,
    pub focused_field: (usize, usize),      // (section_idx, field_idx)
    pub errors: HashMap<(usize, usize), String>,
    pub dirty: bool,                        // Has form been modified?
}
```

### State Methods

**Navigation:**
- `focus_next()` - Move to next field
- `focus_prev()` - Move to previous field
- `get_focused_field()` - Read current field
- `get_focused_field_mut()` - Modify current field

**Interaction:**
- `input_char(c: char)` - Add character to text fields
- `delete_char()` - Remove last character
- `toggle_checkbox()` - Toggle checkbox state
- `select_next()` / `select_prev()` - Navigate select options

**Validation & Collection:**
- `collect_values() -> HashMap<String, String>` - Extract all form data
- `reset_dirty()` - Mark form as unmodified
- `clear_errors()` - Remove validation errors

### Rendering Functions

**Individual Field Rendering:**

```rust
pub fn render_text_input(
    label: &str,
    value: &str,
    is_focused: bool,
    is_error: bool,
    placeholder: Option<&str>,
) -> String

pub fn render_checkbox(
    label: &str,
    is_checked: bool,
    is_focused: bool,
) -> String

pub fn render_select(
    label: &str,
    options: &[String],
    selected_index: usize,
    is_focused: bool,
) -> String

pub fn render_text_area(
    label: &str,
    value: &str,
    is_focused: bool,
    line_count: usize,
) -> String
```

**Section Rendering:**

```rust
pub fn render_form_section(
    title: &str,
    field_widgets: Vec<String>,
) -> (String, Vec<String>)

pub fn render_form_section_with_fields<'a>(
    section: &'a FormSection,
    form_state: &'a ConfigFormState,
) -> Vec<Line<'a>>
```

### Usage Example

```rust
use gat_tui::components::{
    FormField, FormSection, ConfigFormState,
    render_form_section_with_fields,
};

// Create form structure
let sections = vec![
    FormSection {
        title: "Source Configuration".to_string(),
        fields: vec![
            FormField::TextInput {
                label: "Dataset Name".to_string(),
                value: String::new(),
                placeholder: Some("e.g., my_dataset".to_string()),
            },
            FormField::Select {
                label: "Source Type".to_string(),
                options: vec![
                    "OPSD".to_string(),
                    "Matpower".to_string(),
                    "CSV".to_string(),
                ],
                selected: 0,
            },
        ],
    },
    FormSection {
        title: "Processing Options".to_string(),
        fields: vec![
            FormField::Checkbox {
                label: "Parallel Processing".to_string(),
                checked: false,
            },
        ],
    },
];

// Create interactive state
let mut form_state = ConfigFormState::new(sections);

// In event loop:
match key_event {
    KeyEvent::Char(c) => form_state.input_char(c),
    KeyEvent::Tab | KeyEvent::Down => form_state.focus_next(),
    KeyEvent::BackTab | KeyEvent::Up => form_state.focus_prev(),
    _ => {}
}

// For rendering, use rendering functions:
let display_lines = render_form_section_with_fields(
    &form_state.sections[0],
    &form_state,
);

// Extract values when done:
let values = form_state.collect_values();
```

---

## File Browser Components

Located in: `crates/gat-tui/src/components/file_browser.rs`

### Data Structures

#### FileEntry
Represents a file or directory in listing.

```rust
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: SystemTime,
    pub path: PathBuf,
}
```

#### TreeEntry
Used for hierarchical tree display (future enhancement).

```rust
pub struct TreeEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub children_count: usize,
    pub path: PathBuf,
}
```

#### FileBrowserState
Manages directory traversal and file selection.

```rust
pub struct FileBrowserState {
    pub root_path: PathBuf,
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub tree_expanded: HashSet<PathBuf>,
    pub selected_index: usize,
    pub show_details: bool,
    pub filter: Option<String>,  // e.g., "*.csv"
}
```

### State Methods

**Initialization:**
- `new(root_path: PathBuf) -> Result<Self>` - Create browser from directory

**Navigation:**
- `navigate_to(path: &Path) -> Result<()>` - Change to directory
- `parent_directory() -> Result<()>` - Go to parent
- `select_next()` - Select next file
- `select_prev()` - Select previous file
- `enter_selected() -> Result<Option<PathBuf>>` - Enter directory or select file

**Filtering:**
- `apply_filter(pattern: &str)` - Show only matching files (e.g., "*.csv")
- `clear_filter()` - Remove filter
- `get_breadcrumb() -> Vec<String>` - Get path components for breadcrumb

**Tree Interaction:**
- `toggle_expanded(path: &Path)` - Expand/collapse directory

**Queries:**
- `get_selected_file() -> Option<&FileEntry>` - Current selection

### Rendering Functions

**Individual Elements:**

```rust
pub fn render_tree_node(
    name: &str,
    is_dir: bool,
    is_selected: bool,
    is_expanded: bool,
    indent_level: usize,
) -> String

pub fn render_file_entry(
    name: &str,
    is_dir: bool,
    is_selected: bool,
    show_details: bool,
    size: u64,
) -> String
```

**Layout:**

```rust
pub fn render_breadcrumb(
    path_components: &[String],
    separator: &str,
) -> String

pub fn render_selection_info(
    current_path: &str,
    selected_file: Option<&str>,
    file_count: usize,
) -> String
```

### Features

**Smart Filtering:**
- Wildcard patterns: `*.csv` matches files ending with `.csv`
- Substring matching: `data` shows all files containing "data"
- Useful for: dataset selection, config file browsing, output directory choice

**Intelligent Sorting:**
- Directories listed first
- Alphabetical within each group

**Human-Readable Sizes:**
- 512 B, 1.2 KB, 42.5 MB, etc.

### Usage Example

```rust
use gat_tui::components::FileBrowserState;

// Initialize browser at home directory
let mut browser = FileBrowserState::new(
    PathBuf::from(std::env::var("HOME").unwrap())
)?;

// Filter to only show CSV files
browser.apply_filter("*.csv");

// In event loop:
match key_event {
    KeyEvent::Down => browser.select_next(),
    KeyEvent::Up => browser.select_prev(),
    KeyEvent::Enter => {
        match browser.enter_selected() {
            Ok(Some(file_path)) => {
                // File selected
                println!("Selected: {:?}", file_path);
            },
            Ok(None) => {
                // Entered directory, entries updated
            },
            Err(e) => eprintln!("Error: {}", e),
        }
    },
    KeyEvent::Backspace => {
        let _ = browser.parent_directory();
    },
    _ => {}
}

// For rendering:
let breadcrumb = render_breadcrumb(&browser.get_breadcrumb(), " > ");
for (idx, entry) in browser.entries.iter().enumerate() {
    let line = render_file_entry(
        &entry.name,
        entry.is_dir,
        idx == browser.selected_index,
        browser.show_details,
        entry.size,
    );
    println!("{}", line);
}

let info = render_selection_info(
    &browser.current_path.to_string_lossy(),
    browser.get_selected_file().map(|f| f.name.as_str()),
    browser.entries.len(),
);
println!("{}", info);
```

---

## Progress Indicators (Future)

Planned for Phase 2:

```rust
pub fn render_progress_bar(
    label: &str,
    progress: f32,  // 0.0 to 1.0
    width: usize,
    show_percentage: bool,
) -> String

pub fn render_spinner(
    label: &str,
    frame: usize,  // Animated frame index
) -> String
```

These will be stateless rendering functions. State (current frame, current progress) will be managed in the main application loop.

---

## Integration Pattern

### In main.rs

```rust
use gat_tui::components::{ConfigFormState, FileBrowserState};

#[derive(Default)]
pub struct AppState {
    pub config_form: Option<ConfigFormState>,
    pub file_browser: Option<FileBrowserState>,
    // ... other state
}

// In event loop:
fn handle_event(state: &mut AppState, event: KeyEvent) {
    if let Some(form) = &mut state.config_form {
        match event {
            KeyEvent::Tab | KeyEvent::Down => form.focus_next(),
            KeyEvent::BackTab | KeyEvent::Up => form.focus_prev(),
            KeyEvent::Char(c) => form.input_char(c),
            KeyEvent::Backspace => form.delete_char(),
            _ => {}
        }
    }

    if let Some(browser) = &mut state.file_browser {
        match event {
            KeyEvent::Down => browser.select_next(),
            KeyEvent::Up => browser.select_prev(),
            KeyEvent::Enter => {
                if let Ok(Some(file)) = browser.enter_selected() {
                    // Handle file selection
                }
            },
            _ => {}
        }
    }
}
```

### Rendering

Use the stateless rendering functions directly:

```rust
fn render_ui(state: &AppState, frame: &mut Frame, area: Rect) {
    if let Some(form) = &state.config_form {
        for section in &form.sections {
            let lines = render_form_section_with_fields(section, form);
            // Render lines to frame
        }
    }

    if let Some(browser) = &state.file_browser {
        let breadcrumb = render_breadcrumb(&browser.get_breadcrumb(), " > ");
        // Render breadcrumb

        for (idx, entry) in browser.entries.iter().enumerate() {
            let line = render_file_entry(
                &entry.name,
                entry.is_dir,
                idx == browser.selected_index,
                browser.show_details,
                entry.size,
            );
            // Render line
        }
    }
}
```

---

## Design Principles

1. **Rendering is stateless** - Rendering functions take all parameters explicitly
2. **State is optional** - Use just rendering functions for simple displays
3. **Composition over inheritance** - Stack components together
4. **Validation at boundaries** - Only validate user input, trust internal code
5. **Pure functions** - No side effects in rendering functions
6. **Error handling** - State methods use `Result<T>` for fallible operations
7. **Keyboard-first** - All interactions via keyboard events

---

## Testing

Both rendering functions and state logic are designed to be easily testable:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_navigation() {
        let mut form = ConfigFormState::new(vec![
            FormSection {
                title: "Test".to_string(),
                fields: vec![
                    FormField::TextInput { /* ... */ },
                    FormField::Checkbox { /* ... */ },
                ],
            },
        ]);

        assert_eq!(form.focused_field, (0, 0));
        form.focus_next();
        assert_eq!(form.focused_field, (0, 1));
        form.focus_prev();
        assert_eq!(form.focused_field, (0, 0));
    }

    #[test]
    fn test_file_filter() {
        let path = PathBuf::from("data.csv");
        assert!(matches_filter(&path, "*.csv"));
        assert!(!matches_filter(&path, "*.json"));
    }
}
```

---

## Future Enhancements

1. **Modal dialogs** - Confirmation, text input modals
2. **Advanced validation** - Custom validators, cross-field validation
3. **Rich styling** - Theme system integration
4. **Async operations** - File loading, API calls with spinners
5. **Keyboard shortcuts** - Single-key actions (R for retry, E for edit)
6. **Tab completion** - Smart suggestion for text inputs
7. **Help tooltips** - Context-sensitive help text
