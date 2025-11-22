use anyhow::Result;
use iocraft::terminal::Terminal;

pub mod data;
mod command_runner;
pub use command_runner::CommandHandle;
pub mod panes;
pub mod ui;

use panes::{
    commands::CommandsPane, dashboard::DashboardPane, datasets::DatasetsPane,
    operations::OperationsPane, pipeline::PipelinePane, quickstart::QuickstartPane,
};
use ui::{AppShell, CommandModal, ExecutionMode, PaneContext, PanelRegistry, Tooltip};

/// High-level application state for the terminal UI.
pub struct App {
    shell: AppShell,
}

impl App {
    pub fn new() -> Self {
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
        let context = PaneContext::new()
            .with_tooltip(Tooltip::new(
                "Use menu hotkeys to change focus; layouts swap with selection.",
            ))
            .with_modal(modal);

        let registry = PanelRegistry::new(context)
            .register(DashboardPane)
            .register(OperationsPane)
            .register(DatasetsPane)
            .register(PipelinePane)
            .register(CommandsPane)
            .register(QuickstartPane);

        let shell = registry.into_shell("GAT Terminal UI");

        Self { shell }
    }

    pub fn render(&self) -> String {
        self.shell.render()
    }

    pub fn select_menu_item(&mut self, hotkey: char) {
        self.shell.select_menu_item(hotkey);
    }

    pub fn active_menu_label(&self) -> Option<&str> {
        self.shell
            .menu
            .active_item()
            .map(|item| item.label.as_str())
    }

    pub fn run(&self) -> Result<()> {
        let mut terminal = Terminal::new()?;
        terminal.clear()?;
        terminal.render(&self.render())?;
        terminal.flush()?;
        Ok(())
    }
}
