/// Commands Pane - Authoring and executing gat-cli commands
///
/// The commands pane provides:
/// - Command snippet library with templates
/// - Custom command editor
/// - Dry-run vs full execution modes
/// - Command history with results
/// - Output modal for viewing results

use crate::components::*;

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
            CommandSnippet {
                id: "list-datasets".into(),
                command: "gat-cli datasets list --limit 5".into(),
                description: "Verify dataset catalogue connectivity".into(),
                category: "Datasets".into(),
            },
            CommandSnippet {
                id: "preview-envelope".into(),
                command: "gat-cli derms envelope --grid-file <case>".into(),
                description: "Preview flexibility envelope inputs".into(),
                category: "DERMS".into(),
            },
            CommandSnippet {
                id: "import-matpower".into(),
                command: "gat-cli dist import matpower --m <file>".into(),
                description: "Convert MATPOWER test cases before ADMS runs".into(),
                category: "Distribution".into(),
            },
            CommandSnippet {
                id: "scenario-solve".into(),
                command: "gat-cli scenarios solve --config <path>".into(),
                description: "Run scenario analysis with configuration".into(),
                category: "Scenarios".into(),
            },
        ];

        let mut snippets_table = TableWidget::new("commands_snippets");
        snippets_table.columns = vec![
            Column { header: "Snippet".into(), width: 40 },
            Column { header: "Purpose".into(), width: 40 },
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
                format!("{} {} ({})", result.status.symbol(), result.command, result.mode.label()),
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
        self.mode_indicator = StatusWidget::new("execution_mode")
            .set_info(msg);
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
            format!("{} {} ({})", result.status.symbol(), result.command, result.mode.label()),
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
        let success = self.history.iter().filter(|r| r.status == CommandStatus::Success).count();
        let failed = self.history.iter().filter(|r| r.status == CommandStatus::Failed).count();
        let running = self.history.iter().filter(|r| r.status == CommandStatus::Running).count();
        (success, failed, running)
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
        assert_eq!(state.snippet_count(), 4);
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
        state.load_snippet_to_editor(1);
        assert_eq!(state.custom_command, "gat-cli derms envelope --grid-file <case>");
        assert_eq!(state.selected_snippet, 1);
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
        assert_eq!(filtered.len(), 1); // Only "list-datasets" matches
        assert_eq!(filtered[0].id, "list-datasets");
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
