use anyhow::Result;
use iocraft::terminal::Terminal;

mod command_runner;
pub mod panes;
pub mod ui;

use panes::dashboard::DashboardPane;
use ui::{
    AppShell, CommandModal, ContextButton, ExecutionMode, MenuItem, NavMenu, Pane, PaneLayout,
    ResponsiveRules, Sidebar, SubTabs, TableView, Tabs, Tooltip,
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

        let commands_layout = PaneLayout::new(
            Pane::new("Commands workspace")
                .body([
                    "Author gat-cli commands, stack them as multi-line snippets, and run with a hotkey.",
                    "Dry-runs print the normalized invocation; full runs stream into the modal output.",
                ])
                .with_table(
                    TableView::new(["Snippet", "Purpose"])
                        .add_row([
                            "gat-cli datasets list --limit 5",
                            "Verify dataset catalogue connectivity",
                        ])
                        .add_row([
                            "gat-cli derms envelope --grid-file <case>",
                            "Preview flexibility envelope inputs",
                        ])
                        .add_row([
                            "gat-cli dist import matpower --m <file>",
                            "Convert MATPOWER test cases before ADMS runs",
                        ]),
                )
                .with_child(
                    Pane::new("Hotkeys")
                        .body([
                            "[r] Run custom… opens the modal",
                            "[d] Toggle dry-run vs full execution",
                            "[esc] Close modal after reviewing output",
                        ])
                        .mark_visual(),
                ),
        )
        .with_sidebar(
            Sidebar::new("Recent command results", false).lines([
                "✔ dry-run datasets list (5 rows)",
                "✔ envelope preview (synthetic)",
                "… output scrollable inside modal",
            ]),
        )
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 88,
            tall_threshold: 24,
            expand_visuals_on_wide: false,
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
                MenuItem::new("commands", "Commands", '4', commands_layout)
                    .with_context_buttons([ContextButton::new('r', "[r] Run custom…")]),
            ],
            0,
        );

        let tooltip =
            Tooltip::new("Use menu hotkeys to change focus; layouts swap with selection.");
        let mut modal = CommandModal::new(
            "Run custom gat-cli command",
            "Paste multi-line gat-cli snippets, switch between dry-run/full, then stream output below.",
            'r',
        )
        .with_help(Tooltip::new(
            "Syntax: gat-cli <domain> <verb> [flags]. Use new lines for long arguments and include sample files from test_data/.",
        ))
        .with_command_text([
            "gat-cli datasets list --format table",
            "--limit 5",
        ])
        .with_mode(ExecutionMode::DryRun);

        let _ = modal.submit();

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
