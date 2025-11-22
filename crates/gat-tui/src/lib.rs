use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use iocraft::terminal::Terminal;
use std::io;

pub mod data;
pub mod modals;
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

    pub fn run(mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture
        )?;

        let result = self.event_loop();

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            stdout,
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;

        result
    }

    fn event_loop(&mut self) -> Result<()> {
        let mut terminal = Terminal::new()?;
        terminal.clear()?;
        terminal.render(&self.render())?;
        terminal.flush()?;

        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match handle_key(key) {
                        Some('q') => break,
                        Some(c) => {
                            self.select_menu_item(c);
                            terminal.clear()?;
                            terminal.render(&self.render())?;
                            terminal.flush()?;
                        }
                        None => {}
                    }
                }
            }
        }

        Ok(())
    }
}

/// Convert crossterm KeyEvent to char, filtering for printable keys.
/// Returns 'q' for Ctrl+C or ESC, None for non-printable keys.
fn handle_key(key: KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(c) => Some(c),
        KeyCode::Esc | KeyCode::F(0) => Some('q'), // ESC to quit
        _ => None,
    }
}
