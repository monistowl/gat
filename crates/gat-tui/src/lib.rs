use anyhow::Result;
use iocraft::terminal::{Terminal, get_terminal_size};
use iocraft::input::RawModeGuard;
use std::io::{self, Read};

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
        let (width, height) = get_terminal_size();
        let output = self.shell.render_with_size(width, height);

        // Truncate output to fit terminal height, preserving header and footer
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() > height as usize {
            // Keep header (first line), show content, add scrolling indicator
            let available = height as usize - 2; // Reserve space for header and indicator
            let truncated = lines.iter()
                .take(available)
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            format!("{}\n... ({} more lines, use scroll/pagination to view) ...",
                    truncated,
                    lines.len() - available)
        } else {
            output
        }
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
        // Enable raw mode and ensure it's restored on exit
        let _raw_mode = RawModeGuard::enable()?;

        self.run_event_loop()
    }

    fn run_event_loop(&mut self) -> Result<()> {
        let mut terminal = Terminal::new()?;
        let mut stdin = io::stdin();
        let mut buffer = [0; 1];

        // Initial render
        terminal.clear()?;
        terminal.render(&self.render())?;
        terminal.flush()?;

        // Main event loop
        loop {
            match stdin.read(&mut buffer) {
                Ok(0) => {
                    // Shouldn't happen with VMIN=1, but handle it
                    break;
                }
                Ok(_) => {
                    let c = buffer[0] as char;
                    if c == 'q' {
                        break;
                    }
                    self.select_menu_item(c);
                    terminal.clear()?;
                    terminal.render(&self.render())?;
                    terminal.flush()?;
                }
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    break;
                }
            }
        }

        terminal.clear()?;
        terminal.flush()?;
        Ok(())
    }
}

