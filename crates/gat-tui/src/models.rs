use std::collections::HashMap;
use std::sync::Arc;
use crate::{QueryBuilder, QueryError, DatasetEntry};
use crate::data::{Workflow, SystemMetrics};

/// Global application state
#[derive(Clone)]
pub struct AppState {
    pub active_pane: PaneId,
    pub pane_states: HashMap<PaneId, PaneState>,
    pub command_queue: Vec<Command>,
    pub notifications: Vec<Notification>,
    pub settings: AppSettings,
    pub error_state: Option<ErrorInfo>,
    pub modal_state: Option<ModalState>,
    pub should_quit: bool,
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub async_tasks: HashMap<String, AsyncTaskState>,

    // Query service
    pub query_builder: Arc<dyn QueryBuilder>,

    // Async task tracking
    pub datasets_loading: bool,
    pub workflows_loading: bool,
    pub metrics_loading: bool,

    // Results cache
    pub datasets: Option<Result<Vec<DatasetEntry>, QueryError>>,
    pub workflows: Option<Result<Vec<Workflow>, QueryError>>,
    pub metrics: Option<Result<SystemMetrics, QueryError>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("active_pane", &self.active_pane)
            .field("pane_states", &self.pane_states)
            .field("command_queue", &self.command_queue)
            .field("notifications", &self.notifications)
            .field("settings", &self.settings)
            .field("error_state", &self.error_state)
            .field("modal_state", &self.modal_state)
            .field("should_quit", &self.should_quit)
            .field("terminal_width", &self.terminal_width)
            .field("terminal_height", &self.terminal_height)
            .field("async_tasks", &self.async_tasks)
            .field("datasets_loading", &self.datasets_loading)
            .field("workflows_loading", &self.workflows_loading)
            .field("metrics_loading", &self.metrics_loading)
            .field("datasets", &self.datasets)
            .field("workflows", &self.workflows)
            .field("metrics", &self.metrics)
            .finish()
    }
}

/// Pane identifier
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum PaneId {
    Dashboard = 0,
    Operations = 1,
    Datasets = 2,
    Pipeline = 3,
    Commands = 4,
    Help = 5,
}

impl PaneId {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaneId::Dashboard => "dashboard",
            PaneId::Operations => "operations",
            PaneId::Datasets => "datasets",
            PaneId::Pipeline => "pipeline",
            PaneId::Commands => "commands",
            PaneId::Help => "help",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PaneId::Dashboard => "Dashboard",
            PaneId::Operations => "Operations",
            PaneId::Datasets => "Datasets",
            PaneId::Pipeline => "Pipeline",
            PaneId::Commands => "Commands",
            PaneId::Help => "Help",
        }
    }

    pub fn hotkey(&self) -> char {
        match self {
            PaneId::Dashboard => '1',
            PaneId::Operations => '2',
            PaneId::Datasets => '3',
            PaneId::Pipeline => '4',
            PaneId::Commands => '5',
            PaneId::Help => 'h',
        }
    }
}

/// Per-pane state (scroll position, selection, form values, etc)
#[derive(Clone, Debug)]
pub struct PaneState {
    pub scroll_position: usize,
    pub selected_row: usize,
    pub selected_tab: usize,
    pub form_values: HashMap<String, String>,
    pub focus_field: Option<String>,
}

impl Default for PaneState {
    fn default() -> Self {
        PaneState {
            scroll_position: 0,
            selected_row: 0,
            selected_tab: 0,
            form_values: HashMap::new(),
            focus_field: None,
        }
    }
}

/// A command queued for execution
#[derive(Clone, Debug)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub command_line: String,
    pub status: CommandStatus,
    pub output: Vec<String>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

/// Toast notification
#[derive(Clone, Debug)]
pub struct Notification {
    pub message: String,
    pub kind: NotificationKind,
    pub timestamp: std::time::SystemTime,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

/// Global application settings
#[derive(Clone, Debug)]
pub struct AppSettings {
    pub theme: Theme,
    pub auto_save_on_pane_switch: bool,
    pub confirm_on_delete: bool,
    pub gat_cli_path: String,
    pub command_timeout_secs: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            theme: Theme::Dark,
            auto_save_on_pane_switch: true,
            confirm_on_delete: true,
            gat_cli_path: "gat-cli".to_string(),
            command_timeout_secs: 300,
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

/// Error information for display
#[derive(Clone, Debug)]
pub struct ErrorInfo {
    pub message: String,
    pub details: Option<String>,
    pub recoverable: bool,
}

/// Modal state
#[derive(Clone, Debug)]
pub enum ModalState {
    None,
    CommandExecution(CommandModalState),
    Settings(SettingsModalState),
    Confirmation(ConfirmationState),
    Info(InfoState),
}

#[derive(Clone, Debug)]
pub struct CommandModalState {
    pub command_text: String,
    pub execution_mode: ExecutionMode,
    pub output: Vec<String>,
}

impl Default for CommandModalState {
    fn default() -> Self {
        CommandModalState {
            command_text: String::new(),
            execution_mode: ExecutionMode::Full,
            output: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    DryRun,
    Full,
}

#[derive(Clone, Debug)]
pub struct SettingsModalState {
    pub selected_field: usize,
    pub theme: Theme,
    pub auto_save: bool,
    pub confirm_delete: bool,
}

impl Default for SettingsModalState {
    fn default() -> Self {
        SettingsModalState {
            selected_field: 0,
            theme: Theme::Dark,
            auto_save: true,
            confirm_delete: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConfirmationState {
    pub message: String,
    pub yes_label: String,
    pub no_label: String,
}

#[derive(Clone, Debug)]
pub struct InfoState {
    pub title: String,
    pub message: String,
    pub details: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        use crate::services::MockQueryBuilder;

        let mut pane_states = HashMap::new();
        pane_states.insert(PaneId::Dashboard, PaneState::default());
        pane_states.insert(PaneId::Operations, PaneState::default());
        pane_states.insert(PaneId::Datasets, PaneState::default());
        pane_states.insert(PaneId::Pipeline, PaneState::default());
        pane_states.insert(PaneId::Commands, PaneState::default());
        pane_states.insert(PaneId::Help, PaneState::default());

        let query_builder = Arc::new(MockQueryBuilder);

        AppState {
            active_pane: PaneId::Dashboard,
            pane_states,
            command_queue: Vec::new(),
            notifications: Vec::new(),
            settings: AppSettings::default(),
            error_state: None,
            modal_state: Some(ModalState::None),
            should_quit: false,
            terminal_width: 80,
            terminal_height: 24,
            async_tasks: HashMap::new(),
            query_builder,
            datasets_loading: false,
            workflows_loading: false,
            metrics_loading: false,
            datasets: None,
            workflows: None,
            metrics: None,
        }
    }

    pub fn current_pane_state(&self) -> PaneState {
        self.pane_states
            .get(&self.active_pane)
            .cloned()
            .unwrap_or_default()
    }

    pub fn current_pane_state_mut(&mut self) -> &mut PaneState {
        self.pane_states
            .entry(self.active_pane)
            .or_insert_with(PaneState::default)
    }

    pub fn add_notification(&mut self, message: &str, kind: NotificationKind) {
        self.notifications.push(Notification {
            message: message.to_string(),
            kind,
            timestamp: std::time::SystemTime::now(),
        });
    }

    pub fn is_modal_open(&self) -> bool {
        !matches!(self.modal_state, Some(ModalState::None) | None)
    }

    pub fn show_confirmation(&mut self, message: String, yes_label: String, no_label: String) {
        self.modal_state = Some(ModalState::Confirmation(ConfirmationState {
            message,
            yes_label,
            no_label,
        }));
    }

    pub fn show_info(&mut self, title: String, message: String, details: Option<String>) {
        self.modal_state = Some(ModalState::Info(InfoState {
            title,
            message,
            details,
        }));
    }

    pub fn show_command_modal(&mut self, command: String) {
        self.modal_state = Some(ModalState::CommandExecution(CommandModalState {
            command_text: command,
            execution_mode: ExecutionMode::DryRun,
            output: Vec::new(),
        }));
    }

    pub fn close_modal(&mut self) {
        self.modal_state = Some(ModalState::None);
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState::new()
    }
}

/// Async task state tracking
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum AsyncTaskState {
    Running,
    Pending,
    Completed,
    Failed,
}
