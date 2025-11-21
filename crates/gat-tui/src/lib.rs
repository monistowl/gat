use anyhow::Result;
use iocraft::terminal::Terminal;

pub mod ui;

use ui::{AppShell, Collapsible, Modal, Pane, TableView, Tabs, Tooltip};

/// High-level application state for the terminal UI.
pub struct App {
    shell: AppShell,
}

impl App {
    pub fn new() -> Self {
        let workflow_table = TableView::new(["Workflow", "Status", "Updated"])
            .add_row(["Ingest", "Ready", "just now"])
            .add_row(["Transform", "Idle", "1m ago"])
            .add_row(["Solve", "Pending", "3m ago"]);

        let details = Collapsible::new("Solver settings", true).content([
            "Solver: Gauss",
            "Poll: 1s",
            "Verbose: false",
        ]);

        let root = Pane::new("Overview")
            .body([
                "Minimal terminal shell built on iocraft primitives.",
                "Replace this placeholder data with live workflow state as needed.",
            ])
            .with_tabs(Tabs::new(["Overview", "Runs", "Config"], 0))
            .with_table(workflow_table)
            .with_collapsible(details)
            .with_child(
                Pane::new("Recent activity")
                    .body(["No live events captured; connect gat-core to stream updates."]),
            );

        let tooltip = Tooltip::new("Use Ctrl+C to exit once integrated with an event loop.");
        let modal = Modal::new(
            "Prototype shell",
            "This layout demonstrates the new primitives without ratatui.",
        );

        let shell = AppShell::new("GAT Terminal UI", root)
            .with_tooltip(tooltip)
            .with_modal(modal);

        Self { shell }
    }

    pub fn render(&self) -> String {
        self.shell.render()
    }

    pub fn run(&self) -> Result<()> {
        let mut terminal = Terminal::new()?;
        terminal.clear()?;
        terminal.render(&self.render())?;
        terminal.flush()?;
        Ok(())
    }
}
