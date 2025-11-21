use anyhow::Result;
use iocraft::terminal::Terminal;

pub mod panes;
pub mod ui;

use panes::dashboard::DashboardPane;
use ui::{
    AppShell, ContextButton, MenuItem, Modal, NavMenu, Pane, PaneLayout, ResponsiveRules, Sidebar,
    SubTabs, TableView, Tabs, Tooltip,
};

/// High-level application state for the terminal UI.
pub struct App {
    shell: AppShell,
}

impl App {
    pub fn new() -> Self {
        let dashboard_layout = DashboardPane::layout();

        let workflow_table = TableView::new(["Workflow", "Status", "Updated"])
            .add_row(["Ingest", "Ready", "just now"])
            .add_row(["Transform", "Idle", "1m ago"])
            .add_row(["Solve", "Pending", "3m ago"]);

        let operations_layout = PaneLayout::new(
            Pane::new("Operations")
                .body([
                    "DERMS + ADMS actions",
                    "Queue new studies and review topology",
                ])
                .with_tabs(Tabs::new(["DERMS", "ADMS", "State"], 0))
                .with_child(
                    Pane::new("DERMS queue").body(["2 queued envelopes", "1 stress-test running"]),
                ),
        )
        .with_secondary(
            Pane::new("ADMS control")
                .body(["Switching plans", "Voltage watchdogs"])
                .mark_visual(),
        )
        .with_sidebar(Sidebar::new("Operator notes", true).lines(["Next: reload feeders"]))
        .with_subtabs(SubTabs::new(["Switching", "Outage", "Settings"], 2));

        let datasets_layout = PaneLayout::new(
            Pane::new("Data catalog")
                .body([
                    "Public data connectors",
                    "OPSD snapshot",
                    "Airtravel tutorial",
                ])
                .with_table(workflow_table)
                .with_child(Pane::new("Downloads").body(["Ready to fetch"].into_iter()))
                .mark_visual(),
        )
        .with_sidebar(Sidebar::new("Metadata", false).lines(["Retained: 30d", "Backups: nightly"]))
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 80,
            tall_threshold: 24,
            expand_visuals_on_wide: true,
        });

        let nav_menu = NavMenu::new(
            vec![
                MenuItem::new("dashboard", "Dashboard", '1', dashboard_layout)
                    .with_context_buttons([
                        ContextButton::new('g', "[g] Go to quick actions"),
                        ContextButton::new('v', "[v] Show layout visualizer"),
                    ]),
                MenuItem::new("operations", "Operations", '2', operations_layout)
                    .with_context_buttons([
                        ContextButton::new('d', "[d] Dispatch action"),
                        ContextButton::new('s', "[s] Schedule study"),
                    ]),
                MenuItem::new("datasets", "Datasets", '3', datasets_layout).with_context_buttons([
                    ContextButton::new('f', "[f] Fetch dataset"),
                    ContextButton::new('i', "[i] Inspect schema"),
                ]),
            ],
            0,
        );

        let tooltip =
            Tooltip::new("Use menu hotkeys to change focus; layouts swap with selection.");
        let modal = Modal::new(
            "Prototype shell",
            "Menu-driven panes, responsive defaults, and contextual actions are active.",
        );

        let shell = AppShell::new("GAT Terminal UI", nav_menu)
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
