use std::collections::HashMap;

use crate::panes::{
    analytics::AnalyticsPane, commands::CommandsPane, dashboard::DashboardPane,
    datasets::DatasetsPane, operations::OperationsPane, pipeline::PipelinePane,
    quickstart::QuickstartPane,
};
use crate::ui::modal::{CommandTemplate, CommandTemplateParameter, ExecutionMode};
use crate::utils::ConfigManager;

use super::{AppShell, CommandModal, PaneContext, PanelRegistry, Tooltip};

struct StatusLine {
    active: String,
    execution_mode: ExecutionMode,
    last_result: String,
}

impl StatusLine {
    fn new(active: impl Into<String>, execution_mode: ExecutionMode) -> Self {
        Self {
            active: active.into(),
            execution_mode,
            last_result: "no runs yet".to_string(),
        }
    }

    fn render(&self, width: usize, preset_count: usize) -> String {
        let base = format!(
            "Status: {} | Mode: {} | Last: {}",
            self.active,
            self.execution_mode.as_label(),
            self.last_result
        );

        let mut line = if preset_count > 0 {
            format!("{base} | Presets: {preset_count} ready")
        } else {
            base
        };

        if line.len() > width {
            let mut truncated: String = line.chars().take(width.saturating_sub(1)).collect();
            truncated.push('…');
            line = truncated;
        }

        line
    }
}

/// Lightweight application shell used by the test harnesses and documentation
/// examples.
///
/// It wires panes into the navigation menu, injects the shared command modal,
/// and renders a compact status bar for quick state inspection.
pub struct App {
    shell: AppShell,
    status: StatusLine,
    recent_parameters: HashMap<String, Vec<String>>,
    config: ConfigManager,
    viewport: (u16, u16),
}

impl App {
    pub fn new() -> Self {
        let config = ConfigManager::load().unwrap_or_default();
        let recent_parameters = config.config().ui.recent_parameters.clone();
        let viewport = Self::detect_viewport();

        let mut context = PaneContext::new().with_tooltip(Tooltip::new(
            "Hotkeys stay unique across panes: [1-6] + [h] for help; contextual actions show next to the menu.",
        ));
        let command_modal = CommandModal::new(
            "Run command",
            "Enter a gat-cli command (one flag per line)",
            'r',
        )
        .with_help(Tooltip::new(
            "Dry-run prints the normalized invocation; Full streams live output into this modal.",
        ))
        .with_command_text(["gat-cli datasets list", "--format table", "--limit 5"])
        .with_templates(Self::command_templates());
        context = context.with_modal(command_modal);

        let registry = PanelRegistry::new(context)
            .register(DashboardPane)
            .register(OperationsPane)
            .register(DatasetsPane)
            .register(PipelinePane)
            .register(CommandsPane)
            .register(AnalyticsPane)
            .register(QuickstartPane);

        let mut shell = registry.into_shell("GAT Terminal UI");
        shell.menu.ensure_unique_hotkeys();

        let status = StatusLine::new(
            shell.active_menu_label().unwrap_or("Dashboard"),
            ExecutionMode::DryRun,
        );

        Self {
            shell,
            status,
            recent_parameters,
            config,
            viewport,
        }
    }

    pub fn select_menu_item(&mut self, hotkey: char) {
        self.shell.select_menu_item(hotkey);
        if let Some(label) = self.shell.active_menu_label() {
            self.status.active = label.to_string();
        }
    }

    pub fn active_menu_label(&self) -> Option<&str> {
        self.shell.active_menu_label()
    }

    pub fn render(&self) -> String {
        let (width, height) = self.viewport;
        let mut output = self.shell.render_with_size(width, height);
        let preset_count = self
            .recent_parameters
            .get(&self.status.active.to_lowercase())
            .map(|v| v.len())
            .unwrap_or(0);
        let status_line = self.status.render(width as usize, preset_count);
        output.push('\n');
        output.push_str(&status_line);

        Self::fit_to_viewport(output, width as usize, height as usize)
    }

    pub fn record_command_result(&mut self, summary: impl Into<String>) {
        self.status.last_result = summary.into();
    }

    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        self.status.execution_mode = mode;
    }

    pub fn store_recent_parameters(&mut self, pane: impl Into<String>, params: Vec<String>) {
        let pane_key = pane.into();
        if params.is_empty() {
            return;
        }
        self.recent_parameters
            .insert(pane_key.clone(), params.clone());
        let _ = self.config.record_recent_parameters(pane_key, params);
    }

    fn detect_viewport() -> (u16, u16) {
        let width = std::env::var("COLUMNS")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .or_else(|| crossterm::terminal::size().ok().map(|s| s.0))
            .filter(|w| *w > 0)
            .unwrap_or(80);
        let height = std::env::var("LINES")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .or_else(|| crossterm::terminal::size().ok().map(|s| s.1))
            .filter(|h| *h > 0)
            .unwrap_or(24);

        (width.max(60).min(96), height.max(20).min(40))
    }

    fn fit_to_viewport(output: String, width: usize, height: usize) -> String {
        let mut lines: Vec<String> = output
            .lines()
            .map(|line| {
                if line.chars().count() > width {
                    let mut truncated: String =
                        line.chars().take(width.saturating_sub(1)).collect();
                    truncated.push('…');
                    truncated
                } else {
                    line.to_string()
                }
            })
            .collect();

        if lines.len() > height {
            if height > 0 {
                lines.truncate(height.saturating_sub(1));
                lines.push("...".to_string());
            }
        }

        lines.join("\n")
    }

    fn command_templates() -> Vec<CommandTemplate> {
        vec![
            CommandTemplate::new(
                "Grid import with limits",
                "Load a grid and enforce branch limits for a dry-run preview.",
                "gat-cli import matpower --file <grid> --limits <limits.csv>",
                [
                    CommandTemplateParameter::new(
                        "Grid file",
                        "Open grid file picker for Arrow/MATPOWER inputs",
                        Some("picker: [g] grid file"),
                    ),
                    CommandTemplateParameter::new(
                        "Limits CSV",
                        "Select limits/ratings file to apply",
                        Some("selector: limits.csv"),
                    ),
                ],
            ),
            CommandTemplate::new(
                "Batch run from manifest",
                "Kick off batch PF/OPF from a manifest with job caps.",
                "gat-cli batch powerflow --manifest <manifest.json> --max-jobs 4 --out <dir>",
                [
                    CommandTemplateParameter::new(
                        "Manifest",
                        "Pick a manifest.json file",
                        Some("selector: manifest"),
                    ),
                    CommandTemplateParameter::new(
                        "Output",
                        "Choose destination directory for run artifacts",
                        Some("picker: [o] output dir"),
                    ),
                ],
            ),
            CommandTemplate::new(
                "Analytics with constraints",
                "Run reliability analytics with limits and scenario selection.",
                "gat-cli analytics reliability --grid <grid> --limits <limits.csv> --manifest <manifest>",
                [
                    CommandTemplateParameter::new(
                        "Grid",
                        "Pick grid/Arrow source",
                        Some("picker: grid"),
                    ),
                    CommandTemplateParameter::new(
                        "Limits",
                        "Apply optional branch limits for realism",
                        Some("selector: limits"),
                    ),
                    CommandTemplateParameter::new(
                        "Manifest",
                        "Scenario manifest or run-set identifier",
                        Some("selector: manifest"),
                    ),
                ],
            ),
        ]
    }
}
