/// Commands Pane - Authoring and executing gat-cli commands
///
/// The commands pane provides:
/// - Command snippet library with templates
/// - Custom command editor
/// - Dry-run vs full execution modes
/// - Command history with results
/// - Output modal for viewing results
use crate::components::*;
use crate::services::TuiServiceLayer;
use crate::ui::{
    ContextButton, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar, TableView,
    Tooltip,
};

/// Execution mode for commands
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionMode {
    DryRun,
    Full,
}

impl ExecutionMode {
    pub fn label(&self) -> &'static str {
        match self {
            ExecutionMode::DryRun => "Dry-run",
            ExecutionMode::Full => "Full",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            ExecutionMode::DryRun => ExecutionMode::Full,
            ExecutionMode::Full => ExecutionMode::DryRun,
        }
    }
}

/// A command snippet template
#[derive(Clone, Debug)]
pub struct CommandSnippet {
    pub id: String,
    pub command: String,
    pub description: String,
    pub category: String,
}

/// Historical command execution result
#[derive(Clone, Debug)]
pub struct CommandResult {
    pub id: String,
    pub command: String,
    pub mode: ExecutionMode,
    pub status: CommandStatus,
    pub output_lines: usize,
    pub timestamp: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    Success,
    Failed,
    Running,
}

impl CommandStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            CommandStatus::Success => "✓",
            CommandStatus::Failed => "✗",
            CommandStatus::Running => "⟳",
        }
    }
}

/// Commands pane state
#[derive(Clone, Debug)]
pub struct CommandsPaneState {
    // Snippet library
    pub snippets: Vec<CommandSnippet>,
    pub selected_snippet: usize,

    // Custom command editor
    pub custom_command: String,
    pub execution_mode: ExecutionMode,

    // Command history/results
    pub history: Vec<CommandResult>,
    pub selected_history: usize,

    // Component states
    pub snippets_table: TableWidget,
    pub history_list: ListWidget,
    pub command_input: InputWidget,
    pub output_display: ParagraphWidget,
    pub mode_indicator: StatusWidget,

    // UI state
    pub modal_open: bool,
    pub search_filter: String,
}

impl Default for CommandsPaneState {
    fn default() -> Self {
        let snippets = vec![
            // Datasets operations
            CommandSnippet {
                id: "list-datasets".into(),
                command: "gat-cli datasets list --limit 5".into(),
                description: "Verify dataset catalogue connectivity".into(),
                category: "Datasets".into(),
            },
            CommandSnippet {
                id: "describe-dataset".into(),
                command: "gat-cli datasets describe --id <id> --format json".into(),
                description: "Inspect dataset metadata from the catalogue".into(),
                category: "Datasets".into(),
            },
            CommandSnippet {
                id: "fetch-dataset".into(),
                command: "gat-cli datasets fetch --id <id> --out data/public".into(),
                description: "Fetch public dataset to local cache".into(),
                category: "Datasets".into(),
            },
            CommandSnippet {
                id: "upload-dataset".into(),
                command: "gat-cli datasets upload --file <path> --name <name>".into(),
                description: "Upload new dataset to catalogue".into(),
                category: "Datasets".into(),
            },
            CommandSnippet {
                id: "validate-dataset".into(),
                command: "gat-cli datasets validate --id <id>".into(),
                description: "Validate dataset for integrity and completeness".into(),
                category: "Datasets".into(),
            },
            // DERMS operations
            CommandSnippet {
                id: "preview-envelope".into(),
                command: "gat-cli derms envelope --grid-file <case>".into(),
                description: "Preview flexibility envelope inputs".into(),
                category: "DERMS".into(),
            },
            CommandSnippet {
                id: "derms-opf".into(),
                command: "gat-cli derms opf --grid <grid> --dataset <id>".into(),
                description: "Run optimal power flow with DERMS".into(),
                category: "DERMS".into(),
            },
            // Distribution operations
            CommandSnippet {
                id: "import-matpower".into(),
                command: "gat-cli dist import matpower --m <file>".into(),
                description: "Convert MATPOWER test cases before ADMS runs".into(),
                category: "Distribution".into(),
            },
            CommandSnippet {
                id: "import-pandapower".into(),
                command: "gat-cli import pandapower --file test_data/adjacent/pandapower/case14.json --out grid.arrow".into(),
                description: "Load pandapower JSON using new import path".into(),
                category: "Distribution".into(),
            },
            CommandSnippet {
                id: "import-auto-detect".into(),
                command: "gat-cli import format --file <path> --out grid.arrow".into(),
                description: "Auto-detect format across MATPOWER/PSS/E/CIM/pandapower".into(),
                category: "Distribution".into(),
            },
            CommandSnippet {
                id: "dist-powerflow".into(),
                command: "gat-cli dist powerflow --network <file> --demand <file>".into(),
                description: "Run distribution network power flow analysis".into(),
                category: "Distribution".into(),
            },
            CommandSnippet {
                id: "dist-flisr".into(),
                command:
                    "gat-cli dist flisr --network <file> --steps 5 --preview switching_plan.json".into(),
                description: "Simulate FLISR switching sequence with preview output".into(),
                category: "Distribution".into(),
            },
            CommandSnippet {
                id: "dist-vvo".into(),
                command: "gat-cli dist vvo --network <file> --targets <file> --export voltage_profiles.parquet".into(),
                description: "Run Volt/VAR optimization and export voltage profile snapshots".into(),
                category: "Distribution".into(),
            },
            CommandSnippet {
                id: "dist-automation-analytics".into(),
                command:
                    "gat-cli analytics automation --steps switching_plan.json --voltage-band 0.95,1.05".into(),
                description: "Analyze switching steps and voltage windows for grid automation".into(),
                category: "Analytics".into(),
            },
            // Scenario operations
            CommandSnippet {
                id: "scenario-solve".into(),
                command: "gat-cli scenarios solve --config <path>".into(),
                description: "Run scenario analysis with configuration".into(),
                category: "Scenarios".into(),
            },
            CommandSnippet {
                id: "scenario-validate".into(),
                command: "gat-cli scenarios validate --template <path>".into(),
                description: "Validate scenario template syntax and completeness".into(),
                category: "Scenarios".into(),
            },
            CommandSnippet {
                id: "scenario-materialize".into(),
                command: "gat-cli scenarios materialize --template <path> --output <dir>".into(),
                description: "Materialize scenarios from template".into(),
                category: "Scenarios".into(),
            },
            // Analytics operations
            CommandSnippet {
                id: "reliability-analysis".into(),
                command: "gat-cli analytics reliability --grid grid.arrow --outages contingencies.yaml".into(),
                description: "Run reliability metrics (LOLE, EUE)".into(),
                category: "Analytics".into(),
            },
            CommandSnippet {
                id: "deliverability-score".into(),
                command: "gat-cli analytics deliverability --dataset <id> --grid <grid>".into(),
                description: "Calculate deliverability score".into(),
                category: "Analytics".into(),
            },
            CommandSnippet {
                id: "elcc-estimation".into(),
                command: "gat-cli analytics elcc --dataset <id> --grid <grid>".into(),
                description: "Run ELCC resource adequacy estimation".into(),
                category: "Analytics".into(),
            },
            CommandSnippet {
                id: "powerflow-study".into(),
                command: "gat-cli analytics powerflow --dataset <id> --grid <grid> --cases <count>"
                    .into(),
                description: "Run comprehensive power flow study".into(),
                category: "Analytics".into(),
            },
            CommandSnippet {
                id: "der-analytics".into(),
                command: "gat-cli analytics der --dataset <id> --window 24h".into(),
                description: "Analyze DER fleet performance and flexibility bands".into(),
                category: "Analytics".into(),
            },
            CommandSnippet {
                id: "der-headroom".into(),
                command: "gat-cli analytics der --dataset <id> --metric hosting-headroom".into(),
                description: "Quantify DER hosting headroom with safety bands".into(),
                category: "Analytics".into(),
            },
            // Batch operations
            CommandSnippet {
                id: "batch-powerflow".into(),
                command: "gat-cli batch powerflow --manifest <file> --max-jobs 4 --output <dir>"
                    .into(),
                description: "Run batch power flow across multiple scenarios".into(),
                category: "Batch".into(),
            },
            CommandSnippet {
                id: "batch-opf".into(),
                command: "gat-cli batch opf --manifest <file> --max-jobs 4 --solver ipopt".into(),
                description: "Run batch optimal power flow".into(),
                category: "Batch".into(),
            },
            CommandSnippet {
                id: "batch-status".into(),
                command: "gat-cli batch status --job-id <id>".into(),
                description: "Check status of batch job".into(),
                category: "Batch".into(),
            },
            // Runs
            CommandSnippet {
                id: "list-runs".into(),
                command: "gat-cli runs list --root runs --format table".into(),
                description: "List saved runs".into(),
                category: "Runs".into(),
            },
            CommandSnippet {
                id: "describe-run".into(),
                command: "gat-cli runs describe run.json --format json".into(),
                description: "Describe run from manifest (shows outputs)".into(),
                category: "Runs".into(),
            },
            CommandSnippet {
                id: "resume-run".into(),
                command: "gat-cli runs resume run.json --execute".into(),
                description: "Re-run saved manifest with same arguments".into(),
                category: "Runs".into(),
            },
            // Utilities
            CommandSnippet {
                id: "geo-join".into(),
                command: "gat-cli geo join --left <file> --right <file> --output <file>".into(),
                description: "Perform geographic join of datasets".into(),
                category: "Utilities".into(),
            },
            CommandSnippet {
                id: "health-check".into(),
                command: "gat-cli health check --verbose".into(),
                description: "Run system health check".into(),
                category: "Utilities".into(),
            },
            // Hosting capacity & DER scheduling
            CommandSnippet {
                id: "der-schedule".into(),
                command: "gat-cli derms schedule --dataset <id> --horizon 24".into(),
                description: "Optimize DER schedules for the next 24 hours".into(),
                category: "DERMS".into(),
            },
            CommandSnippet {
                id: "der-dayahead".into(),
                command: "gat-cli derms schedule --dataset <id> --horizon 24 --mode dayahead".into(),
                description: "Publish day-ahead DER schedules with telemetry pins".into(),
                category: "DERMS".into(),
            },
            CommandSnippet {
                id: "hosting-capacity".into(),
                command: "gat-cli derms hosting-capacity --grid <grid> --der-type solar --out hosting.parquet"
                    .into(),
                description: "Estimate DER hosting capacity by bus with voltage bands".into(),
                category: "DERMS".into(),
            },
            CommandSnippet {
                id: "hosting-capacity-analytics".into(),
                command:
                    "gat-cli analytics hosting --grid <grid> --zones <file> --out hosting_by_zone.parquet".into(),
                description: "Summarize hosting capacity analytics by zone with flags".into(),
                category: "Analytics".into(),
            },
            CommandSnippet {
                id: "hosting-capacity-zones".into(),
                command: "gat-cli derms hosting-capacity --grid <grid> --zones <file> --voltage-band 0.95,1.05"
                    .into(),
                description: "Summarize hosting capacity by zone with limit flags".into(),
                category: "Distribution".into(),
            },
        ];

        let mut snippets_table = TableWidget::new("commands_snippets");
        snippets_table.columns = vec![
            Column {
                header: "Snippet".into(),
                width: 40,
            },
            Column {
                header: "Purpose".into(),
                width: 40,
            },
        ];

        let mut history_list = ListWidget::new("commands_history");

        let history = vec![
            CommandResult {
                id: "cmd_001".into(),
                command: "gat-cli datasets list --limit 5".into(),
                mode: ExecutionMode::DryRun,
                status: CommandStatus::Success,
                output_lines: 5,
                timestamp: "2024-11-21 14:30:00".into(),
            },
            CommandResult {
                id: "cmd_002".into(),
                command: "gat-cli derms envelope --grid-file synthetic".into(),
                mode: ExecutionMode::Full,
                status: CommandStatus::Success,
                output_lines: 12,
                timestamp: "2024-11-21 14:25:00".into(),
            },
        ];

        // Populate history list
        for result in &history {
            history_list.add_item(
                format!(
                    "{} {} ({})",
                    result.status.symbol(),
                    result.command,
                    result.mode.label()
                ),
                result.id.clone(),
            );
        }

        let mut mode_indicator = StatusWidget::new("execution_mode");
        mode_indicator = mode_indicator.set_info("Dry-run mode");

        CommandsPaneState {
            snippets,
            selected_snippet: 0,
            custom_command: String::new(),
            execution_mode: ExecutionMode::DryRun,
            history,
            selected_history: 0,
            snippets_table,
            history_list,
            command_input: InputWidget::new("command_editor")
                .with_placeholder("Type custom command or select snippet..."),
            output_display: ParagraphWidget::new("command_output"),
            mode_indicator,
            modal_open: false,
            search_filter: String::new(),
        }
    }
}

impl CommandsPaneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select_next_snippet(&mut self) {
        if self.selected_snippet < self.snippets.len().saturating_sub(1) {
            self.selected_snippet += 1;
        }
    }

    pub fn select_prev_snippet(&mut self) {
        if self.selected_snippet > 0 {
            self.selected_snippet -= 1;
        }
    }

    pub fn selected_snippet(&self) -> Option<&CommandSnippet> {
        self.snippets.get(self.selected_snippet)
    }

    pub fn load_snippet_to_editor(&mut self, index: usize) {
        if let Some(snippet) = self.snippets.get(index) {
            self.custom_command = snippet.command.clone();
            self.command_input.set_value(snippet.command.clone());
            self.selected_snippet = index;
        }
    }

    pub fn clear_editor(&mut self) {
        self.custom_command.clear();
        self.command_input.clear();
    }

    pub fn toggle_execution_mode(&mut self) {
        self.execution_mode = self.execution_mode.toggle();
        let msg = format!("{} mode", self.execution_mode.label());
        self.mode_indicator = StatusWidget::new("execution_mode").set_info(msg);
    }

    pub fn open_modal(&mut self) {
        self.modal_open = true;
        if self.custom_command.is_empty() {
            if let Some(snippet) = self.selected_snippet() {
                self.custom_command = snippet.command.clone();
            }
        }
    }

    pub fn close_modal(&mut self) {
        self.modal_open = false;
    }

    pub fn add_to_history(&mut self, result: CommandResult) {
        self.history.insert(0, result.clone());
        self.history_list.add_item(
            format!(
                "{} {} ({})",
                result.status.symbol(),
                result.command,
                result.mode.label()
            ),
            result.id,
        );
    }

    pub fn selected_result(&self) -> Option<&CommandResult> {
        self.history.get(self.selected_history)
    }

    pub fn select_next_result(&mut self) {
        if self.selected_history < self.history.len().saturating_sub(1) {
            self.selected_history += 1;
        }
    }

    pub fn select_prev_result(&mut self) {
        if self.selected_history > 0 {
            self.selected_history -= 1;
        }
    }

    pub fn filter_snippets(&mut self, query: String) {
        self.search_filter = query;
    }

    pub fn filtered_snippets(&self) -> Vec<&CommandSnippet> {
        if self.search_filter.is_empty() {
            self.snippets.iter().collect()
        } else {
            let query = self.search_filter.to_lowercase();
            self.snippets
                .iter()
                .filter(|s| {
                    s.command.to_lowercase().contains(&query)
                        || s.description.to_lowercase().contains(&query)
                        || s.category.to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    pub fn snippet_count(&self) -> usize {
        self.snippets.len()
    }

    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.history_list.clear();
        self.selected_history = 0;
    }

    pub fn execution_summary(&self) -> (usize, usize, usize) {
        let success = self
            .history
            .iter()
            .filter(|r| r.status == CommandStatus::Success)
            .count();
        let failed = self
            .history
            .iter()
            .filter(|r| r.status == CommandStatus::Failed)
            .count();
        let running = self
            .history
            .iter()
            .filter(|r| r.status == CommandStatus::Running)
            .count();
        (success, failed, running)
    }

    /// Execute the custom command and add result to history
    /// This integrates with TuiServiceLayer for real gat-cli execution
    pub async fn execute_command(
        &mut self,
        service_layer: &TuiServiceLayer,
    ) -> Result<CommandResult, String> {
        if self.custom_command.is_empty() {
            return Err("No command to execute".to_string());
        }

        let id = format!("cmd_{:06}", self.history.len() + 1);
        let response = service_layer
            .execute_custom_command(
                &self.custom_command,
                self.execution_mode == ExecutionMode::DryRun,
            )
            .await
            .map_err(|e| e.to_string())?;

        let output_lines = response["output_lines"].as_u64().unwrap_or(0) as usize;
        let status = match response["exit_code"].as_i64().unwrap_or(1) {
            0 => CommandStatus::Success,
            _ => CommandStatus::Failed,
        };

        let result = CommandResult {
            id,
            command: self.custom_command.clone(),
            mode: self.execution_mode,
            status,
            output_lines,
            timestamp: format!("{:?}", std::time::SystemTime::now()),
        };

        self.add_to_history(result.clone());
        Ok(result)
    }

    /// Execute the currently selected snippet by routing through the service layer
    pub async fn execute_selected_snippet(
        &mut self,
        service_layer: &TuiServiceLayer,
    ) -> Result<CommandResult, String> {
        let snippet_command = self
            .selected_snippet()
            .map(|snippet| snippet.command.clone());

        if let Some(command) = snippet_command {
            self.custom_command = command.clone();
            self.command_input.set_value(command);
        }

        self.execute_command(service_layer).await
    }
}

/// Quick action shortcuts for commands pane
pub struct CommandAction {
    pub key: char,
    pub label: String,
    pub description: String,
}

impl CommandAction {
    pub fn all() -> Vec<Self> {
        vec![
            CommandAction {
                key: 'r',
                label: "[r]".into(),
                description: "Run custom command".into(),
            },
            CommandAction {
                key: 'd',
                label: "[d]".into(),
                description: "Toggle dry-run/full mode".into(),
            },
            CommandAction {
                key: 's',
                label: "[s]".into(),
                description: "Select and load snippet".into(),
            },
            CommandAction {
                key: 'c',
                label: "[c]".into(),
                description: "Clear history".into(),
            },
            CommandAction {
                key: 'f',
                label: "[f]".into(),
                description: "Filter snippets".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands_init() {
        let state = CommandsPaneState::new();
        assert!(state.snippet_count() >= 30);
        assert_eq!(state.history_count(), 2);
        assert_eq!(state.execution_mode, ExecutionMode::DryRun);
    }

    #[test]
    fn test_snippet_selection() {
        let mut state = CommandsPaneState::new();
        state.select_next_snippet();
        assert_eq!(state.selected_snippet, 1);
        state.select_prev_snippet();
        assert_eq!(state.selected_snippet, 0);
    }

    #[test]
    fn test_load_snippet() {
        let mut state = CommandsPaneState::new();
        let index = state
            .snippets
            .iter()
            .position(|s| s.id == "preview-envelope")
            .expect("preview envelope snippet present");
        state.load_snippet_to_editor(index);
        assert_eq!(
            state.custom_command,
            "gat-cli derms envelope --grid-file <case>"
        );
        assert_eq!(state.selected_snippet, index);
    }

    #[test]
    fn test_load_analytics_snippet() {
        let mut state = CommandsPaneState::new();
        let index = state
            .snippets
            .iter()
            .position(|s| s.id == "reliability-analysis")
            .expect("reliability snippet present");
        state.load_snippet_to_editor(index);
        assert!(state.custom_command.contains("reliability"));
        assert_eq!(state.selected_snippet, index);
    }

    #[test]
    fn test_load_batch_snippet() {
        let mut state = CommandsPaneState::new();
        let index = state
            .snippets
            .iter()
            .position(|s| s.id == "batch-powerflow")
            .expect("batch powerflow snippet present");
        state.load_snippet_to_editor(index);
        assert!(state.custom_command.contains("batch powerflow"));
        assert_eq!(state.selected_snippet, index);
    }

    #[test]
    fn test_execution_mode_toggle() {
        let mut state = CommandsPaneState::new();
        assert_eq!(state.execution_mode, ExecutionMode::DryRun);
        state.toggle_execution_mode();
        assert_eq!(state.execution_mode, ExecutionMode::Full);
        state.toggle_execution_mode();
        assert_eq!(state.execution_mode, ExecutionMode::DryRun);
    }

    #[test]
    fn test_history_management() {
        let mut state = CommandsPaneState::new();
        let initial_count = state.history_count();

        let result = CommandResult {
            id: "cmd_003".into(),
            command: "gat-cli test".into(),
            mode: ExecutionMode::DryRun,
            status: CommandStatus::Success,
            output_lines: 10,
            timestamp: "2024-11-21 14:35:00".into(),
        };

        state.add_to_history(result);
        assert_eq!(state.history_count(), initial_count + 1);
    }

    #[test]
    fn test_filter_snippets() {
        let mut state = CommandsPaneState::new();
        state.filter_snippets("dataset".into());
        let filtered = state.filtered_snippets();
        assert!(filtered.len() >= 3); // Multiple dataset snippets match
        assert!(filtered.iter().any(|s| s.id == "list-datasets"));
        assert!(filtered.iter().any(|s| s.id == "upload-dataset"));
        assert!(filtered.iter().any(|s| s.id == "validate-dataset"));
    }

    #[test]
    fn test_filter_analytics_snippets() {
        let mut state = CommandsPaneState::new();
        state.filter_snippets("analytics".into());
        let filtered = state.filtered_snippets();
        assert!(filtered.len() >= 4); // Multiple analytics snippets
        assert!(filtered.iter().all(|s| s.category == "Analytics"));
    }

    #[test]
    fn test_filter_batch_snippets() {
        let mut state = CommandsPaneState::new();
        state.filter_snippets("batch".into());
        let filtered = state.filtered_snippets();
        assert!(filtered.len() >= 3); // Multiple batch snippets
        assert!(filtered.iter().all(|s| s.category == "Batch"));
    }

    #[tokio::test]
    async fn test_execute_selected_snippet_routes_to_service() {
        use crate::services::MockQueryBuilder;
        use std::sync::Arc;

        let mut state = CommandsPaneState::new();
        let service = TuiServiceLayer::new(Arc::new(MockQueryBuilder));

        let result = state
            .execute_selected_snippet(&service)
            .await
            .expect("snippet should execute");

        assert_eq!(result.status, CommandStatus::Success);
        assert_eq!(state.history_count(), 3);
    }

    #[test]
    fn test_execution_summary() {
        let state = CommandsPaneState::new();
        let (success, failed, running) = state.execution_summary();
        assert_eq!(success, 2); // 2 successful commands in history
        assert_eq!(failed, 0);
        assert_eq!(running, 0);
    }

    #[test]
    fn test_command_actions() {
        let actions = CommandAction::all();
        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0].key, 'r');
        assert_eq!(actions[1].key, 'd');
    }

    #[test]
    fn test_execution_mode_label() {
        assert_eq!(ExecutionMode::DryRun.label(), "Dry-run");
        assert_eq!(ExecutionMode::Full.label(), "Full");
    }

    #[test]
    fn test_command_status_symbol() {
        assert_eq!(CommandStatus::Success.symbol(), "✓");
        assert_eq!(CommandStatus::Failed.symbol(), "✗");
        assert_eq!(CommandStatus::Running.symbol(), "⟳");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PaneView Implementation
// ─────────────────────────────────────────────────────────────────────────────

/// Commands pane for the TUI registry
pub struct CommandsPane;

impl CommandsPane {
    pub fn layout() -> PaneLayout {
        PaneLayout::new(
            Pane::new("Commands workspace")
                .body([
                    "Author gat-cli commands, stack them as multi-line snippets, and run with a hotkey.",
                    "Dry-runs print the normalized invocation; full runs stream into the modal output.",
                    "Run custom gat-cli command straight from the modal",
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
                        ])
                        .add_row([
                            "gat-cli dist flisr --network <file> --steps 5",
                            "Stage FLISR switching steps before dispatch",
                        ])
                        .add_row([
                            "gat-cli derms hosting-capacity --grid <grid> --der-type solar",
                            "Estimate hosting capacity with voltage headroom",
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
            collapse_secondary_first: true,
        })
    }
}

impl PaneView for CommandsPane {
    fn id(&self) -> &'static str {
        "commands"
    }

    fn label(&self) -> &'static str {
        "Commands"
    }

    fn hotkey(&self) -> char {
        '5'
    }

    fn layout(&self, _context: &PaneContext) -> PaneLayout {
        Self::layout()
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Author and run gat-cli snippets; modal output stays linked to this pane.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![ContextButton::new('r', "[r] Run custom…")]
    }
}
