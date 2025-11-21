use std::fmt::Write;

mod layout;
mod navigation;

/// The root container for the terminal experience.
pub use layout::{PaneLayout, ResponsiveRules, Sidebar, SubTabs};
pub use navigation::{ContextButton, MenuItem, NavMenu};

pub struct AppShell {
    pub title: String,
    pub menu: NavMenu,
    pub tooltip: Option<Tooltip>,
    pub modal: Option<Modal>,
    viewport: (u16, u16),
}

impl AppShell {
    pub fn new(title: impl Into<String>, menu: NavMenu) -> Self {
        Self {
            title: title.into(),
            menu,
            tooltip: None,
            modal: None,
            viewport: (110, 32),
        }
    }

    pub fn with_tooltip(mut self, tooltip: Tooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    pub fn with_modal(mut self, modal: Modal) -> Self {
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
        let _ = writeln!(&mut output, "┏━━━━━━━━ {} ━━━━━━━━┓", self.title);
        let _ = writeln!(&mut output, "{}", self.menu.render_menu_bar());
        self.menu
            .render_active_layout_into(&mut output, width, height);
        if let Some(tooltip) = &self.tooltip {
            let _ = writeln!(&mut output, "\n{}", tooltip.render());
        }
        if let Some(modal) = &self.modal {
            let _ = writeln!(&mut output, "\n{}", modal.render());
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

    fn render_into(&self, output: &mut String, indent: usize, expanded: bool) {
        let pad = " ".repeat(indent * 2);
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

        for line in &self.body {
            let _ = writeln!(output, "{}  {}", pad, line);
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
            child.render_into(output, indent + 1, expanded);
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
}

impl TableView {
    pub fn new(headers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            headers: headers.into_iter().map(|h| h.into()).collect(),
            rows: Vec::new(),
        }
    }

    pub fn add_row(mut self, row: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.rows.push(row.into_iter().map(|c| c.into()).collect());
        self
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.headers.is_empty() {
            lines.push(self.headers.join(" | "));
            lines.push(
                self.headers
                    .iter()
                    .map(|h| "-".repeat(h.len()))
                    .collect::<Vec<_>>()
                    .join("-+-"),
            );
        }
        for row in &self.rows {
            lines.push(row.join(" | "));
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

/// Modal overlay that highlights blocking information.
#[derive(Clone, Debug)]
pub struct Modal {
    pub title: String,
    pub body: String,
}

impl Modal {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }

    pub fn render(&self) -> String {
        format!("[{}]\n{}", self.title, self.body)
    }
}
