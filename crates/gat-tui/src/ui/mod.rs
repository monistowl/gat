use std::fmt::Write;

mod ansi;
mod components;
mod grid_components;
mod layout;
mod modal;
mod navigation;
mod registry;
mod theme;

pub use ansi::{StyledText, COLOR_RED, COLOR_GREEN, COLOR_YELLOW, COLOR_CYAN, BOLD, DIM, REVERSE};

/// The root container for the terminal experience.
pub use components::{
    config_form, file_browser_table, job_queue_table, manifest_preview, metrics_table,
    progress_bar, ConfigField, ConfigFieldType, FileInfo, Job, JobStatus, MetricStatus,
    MetricValue,
};
pub use grid_components::{GridBrowserState, GridInfo, GridLoadState, GridStatus};
pub use layout::{PaneLayout, ResponsiveRules, Sidebar, SubTabs};
pub use modal::{CommandModal, ExecutionMode};
pub use navigation::{ContextButton, MenuItem, NavMenu};
pub use registry::{PaneContext, PaneView, PanelRegistry};
pub use theme::{EmptyState, Theme, THEME};

pub struct AppShell {
    pub title: String,
    pub menu: NavMenu,
    pub tooltip: Option<Tooltip>,
    pub modal: Option<CommandModal>,
    viewport: (u16, u16),
}

impl AppShell {
    pub fn new(title: impl Into<String>, menu: NavMenu) -> Self {
        // Get actual terminal size, fall back to sensible defaults if detection fails
        let (width, height) = crossterm::terminal::size()
            .unwrap_or((80, 24));

        Self {
            title: title.into(),
            menu,
            tooltip: None,
            modal: None,
            viewport: (width, height),
        }
    }

    pub fn with_tooltip(mut self, tooltip: Tooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    pub fn with_modal(mut self, modal: CommandModal) -> Self {
        self.modal = Some(modal);
        self
    }

    pub fn with_viewport(mut self, width: u16, height: u16) -> Self {
        self.viewport = (width, height);
        self
    }

    pub fn select_menu_item(&mut self, hotkey: char) {
        self.menu.select_by_hotkey(hotkey);
    }

    pub fn render(&self) -> String {
        self.render_with_size(self.viewport.0, self.viewport.1)
    }

    pub fn render_with_size(&self, width: u16, height: u16) -> String {
        let mut output = String::new();

        // Render frame title with cyan and bold styling
        let title_text = THEME.frame_title(&self.title);
        let styled_title = StyledText::new()
            .color(COLOR_CYAN)
            .bold()
            .apply(title_text);
        let _ = writeln!(&mut output, "{}", styled_title);

        // Render menu bar
        let _ = writeln!(&mut output, "{}", self.menu.render_menu_bar());

        // Render active pane layout
        self.menu
            .render_active_layout_into(&mut output, width, height);

        // Render tooltip if present (dim styling)
        let tooltip = self
            .menu
            .active_tooltip()
            .cloned()
            .or_else(|| self.tooltip.clone());
        if let Some(tooltip) = tooltip {
            let tooltip_text = tooltip.render();
            let styled_tooltip = StyledText::new()
                .color(COLOR_YELLOW)
                .dim()
                .apply(tooltip_text);
            let _ = writeln!(&mut output, "\n{}", styled_tooltip);
        }

        // Render modal if present (with reverse video for emphasis)
        if let Some(modal) = &self.modal {
            let modal_text = modal.render();
            let styled_modal = StyledText::new()
                .color(COLOR_YELLOW)
                .bold()
                .apply(modal_text);
            let _ = writeln!(&mut output, "\n{}", styled_modal);
        }

        output
    }
}

/// Flexible content container that can hold nested panes.
#[derive(Clone, Debug)]
pub struct Pane {
    pub title: String,
    pub body: Vec<String>,
    pub children: Vec<Pane>,
    pub tabs: Option<Tabs>,
    pub table: Option<TableView>,
    pub collapsible: Option<Collapsible>,
    pub visual: bool,
    pub empty: Option<EmptyState>,
}

impl Pane {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: Vec::new(),
            children: Vec::new(),
            tabs: None,
            table: None,
            collapsible: None,
            visual: false,
            empty: None,
        }
    }

    pub fn body(mut self, lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.body = lines.into_iter().map(|l| l.into()).collect();
        self
    }

    pub fn mark_visual(mut self) -> Self {
        self.visual = true;
        self
    }

    pub fn with_child(mut self, child: Pane) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_tabs(mut self, tabs: Tabs) -> Self {
        self.tabs = Some(tabs);
        self
    }

    pub fn with_table(mut self, table: TableView) -> Self {
        self.table = Some(table);
        self
    }

    pub fn with_collapsible(mut self, collapsible: Collapsible) -> Self {
        self.collapsible = Some(collapsible);
        self
    }

    pub fn with_empty_state(mut self, empty: EmptyState) -> Self {
        self.empty = Some(empty);
        self
    }

    fn render_into(&self, output: &mut String, indent: usize, expanded: bool) {
        let pad = THEME.indent(indent);
        let visual_label = if self.visual && expanded {
            " (expanded)"
        } else if self.visual {
            " (visualizer)"
        } else if expanded {
            " (wide)"
        } else {
            ""
        };
        let _ = writeln!(output, "{}▶ {}{}", pad, self.title, visual_label);

        if let Some(collapsible) = &self.collapsible {
            let _ = writeln!(output, "{}  {}", pad, collapsible.render());
        }

        if self.body.is_empty()
            && self.children.is_empty()
            && self.table.as_ref().map_or(true, |table| !table.has_rows())
        {
            if let Some(empty) = &self.empty {
                for line in empty.render_lines(&THEME) {
                    let _ = writeln!(output, "{}  {}", pad, line);
                }
            }
        } else {
            for line in &self.body {
                let _ = writeln!(output, "{}  {}", pad, line);
            }
        }

        if let Some(tabs) = &self.tabs {
            let _ = writeln!(output, "{}  {}", pad, tabs.render());
        }

        if let Some(table) = &self.table {
            for line in table.render_lines() {
                let _ = writeln!(output, "{}  {}", pad, line);
            }
        }

        for child in &self.children {
            child.render_into(output, indent, expanded);
        }
    }
}

/// A simple collapsible text block.
#[derive(Clone, Debug)]
pub struct Collapsible {
    pub label: String,
    pub expanded: bool,
    pub content: Vec<String>,
}

impl Collapsible {
    pub fn new(label: impl Into<String>, expanded: bool) -> Self {
        Self {
            label: label.into(),
            expanded,
            content: Vec::new(),
        }
    }

    pub fn content(mut self, lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.content = lines.into_iter().map(|l| l.into()).collect();
        self
    }

    pub fn render(&self) -> String {
        if self.expanded {
            format!("▼ {}", self.content.join(" | "))
        } else {
            format!("▶ {}", self.label)
        }
    }
}

/// Tab collection used to segment content areas.
#[derive(Clone, Debug)]
pub struct Tabs {
    pub labels: Vec<String>,
    pub active: usize,
}

impl Tabs {
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>, active: usize) -> Self {
        Self {
            labels: labels.into_iter().map(|l| l.into()).collect(),
            active,
        }
    }

    pub fn render(&self) -> String {
        let rendered: Vec<String> = self
            .labels
            .iter()
            .enumerate()
            .map(|(idx, label)| {
                if idx == self.active {
                    format!("[{}]", label)
                } else {
                    label.to_string()
                }
            })
            .collect();
        rendered.join("  ")
    }
}

/// A compact table presentation for small datasets.
#[derive(Clone, Debug)]
pub struct TableView {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub empty: Option<EmptyState>,
}

impl TableView {
    pub fn new(headers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            headers: headers.into_iter().map(|h| h.into()).collect(),
            rows: Vec::new(),
            empty: None,
        }
    }

    pub fn add_row(mut self, row: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.rows.push(row.into_iter().map(|c| c.into()).collect());
        self
    }

    pub fn with_empty_state(mut self, empty: EmptyState) -> Self {
        self.empty = Some(empty);
        self
    }

    pub fn has_rows(&self) -> bool {
        !self.rows.is_empty()
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if self.rows.is_empty() {
            if let Some(empty) = &self.empty {
                return empty.render_lines(&THEME);
            }
        }

        if !self.headers.is_empty() {
            lines.push(self.headers.join(THEME.table_gap));
            lines.push(
                self.headers
                    .iter()
                    .map(|h| THEME.divider(h.len()))
                    .collect::<Vec<_>>()
                    .join(THEME.table_gap),
            );
        }
        for row in &self.rows {
            lines.push(row.join(THEME.table_gap));
        }
        lines
    }
}

/// Inline helper used to annotate controls or data.
#[derive(Clone, Debug)]
pub struct Tooltip {
    pub message: String,
}

impl Tooltip {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn render(&self) -> String {
        format!("💡 {}", self.message)
    }
}
