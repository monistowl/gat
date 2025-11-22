use std::collections::HashMap;
use std::sync::Arc;
use crate::{QueryBuilder, QueryError, DatasetEntry};
use crate::data::{Workflow, SystemMetrics};
use crate::services::{GridService, GatCoreQueryBuilder};

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

    // Grid management (Phase 2)
    pub grid_service: GridService,
    pub gat_core_query_builder: Option<GatCoreQueryBuilder>,
    pub current_grid_id: Option<String>,

    // Workflow tracking (Phase 3)
    pub executed_workflows: Vec<Workflow>,

    // Async task tracking
    pub datasets_loading: bool,
    pub workflows_loading: bool,
    pub metrics_loading: bool,
    pub pipeline_loading: bool,
    pub commands_loading: bool,

    // Results cache
    pub datasets: Option<Result<Vec<DatasetEntry>, QueryError>>,
    pub workflows: Option<Result<Vec<Workflow>, QueryError>>,
    pub metrics: Option<Result<SystemMetrics, QueryError>>,
    pub pipeline_config: Option<Result<String, QueryError>>,
    pub commands: Option<Result<Vec<String>, QueryError>>,
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
            .field("current_grid_id", &self.current_grid_id)
            .field("executed_workflows", &self.executed_workflows.len())
            .field("datasets_loading", &self.datasets_loading)
            .field("workflows_loading", &self.workflows_loading)
            .field("metrics_loading", &self.metrics_loading)
            .field("pipeline_loading", &self.pipeline_loading)
            .field("commands_loading", &self.commands_loading)
            .field("datasets", &self.datasets)
            .field("workflows", &self.workflows)
            .field("metrics", &self.metrics)
            .field("pipeline_config", &self.pipeline_config)
            .field("commands", &self.commands)
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

        // Initialize with MockQueryBuilder (fixtures) by default
        let query_builder = Arc::new(MockQueryBuilder);

        // Initialize grid service for real data integration (Phase 2)
        let grid_service = GridService::new();

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
            grid_service,
            gat_core_query_builder: None,
            current_grid_id: None,
            executed_workflows: Vec::new(),
            datasets_loading: false,
            workflows_loading: false,
            metrics_loading: false,
            pipeline_loading: false,
            commands_loading: false,
            datasets: None,
            workflows: None,
            metrics: None,
            pipeline_config: None,
            commands: None,
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

    /// Fetch all datasets asynchronously
    pub async fn fetch_datasets(&mut self) {
        self.datasets_loading = true;
        self.datasets = Some(self.query_builder.get_datasets().await);
        self.datasets_loading = false;
    }

    /// Fetch all workflows asynchronously
    pub async fn fetch_workflows(&mut self) {
        self.workflows_loading = true;
        self.workflows = Some(self.query_builder.get_workflows().await);
        self.workflows_loading = false;
    }

    /// Fetch system metrics asynchronously
    pub async fn fetch_metrics(&mut self) {
        self.metrics_loading = true;
        self.metrics = Some(self.query_builder.get_metrics().await);
        self.metrics_loading = false;
    }

    /// Fetch pipeline configuration asynchronously
    pub async fn fetch_pipeline_config(&mut self) {
        self.pipeline_loading = true;
        self.pipeline_config = Some(self.query_builder.get_pipeline_config().await);
        self.pipeline_loading = false;
    }

    /// Fetch available commands asynchronously
    pub async fn fetch_commands(&mut self) {
        self.commands_loading = true;
        self.commands = Some(self.query_builder.get_commands().await);
        self.commands_loading = false;
    }

    /// Load a grid from an Arrow file and set it as current
    ///
    /// Returns the grid ID on success, or error message on failure.
    /// Invalidates cached results when grid changes.
    pub fn load_grid(&mut self, file_path: &str) -> Result<String, String> {
        match self.grid_service.load_grid_from_arrow(file_path) {
            Ok(grid_id) => {
                self.set_current_grid(grid_id.clone());
                Ok(grid_id)
            }
            Err(e) => Err(format!("{:?}", e)),
        }
    }

    /// Set the current active grid and switch to GatCoreQueryBuilder
    ///
    /// Invalidates cached results and updates the active query builder
    /// to use real data from the grid instead of fixtures.
    pub fn set_current_grid(&mut self, grid_id: String) {
        self.current_grid_id = Some(grid_id.clone());

        // Create/update GatCoreQueryBuilder with the new grid
        let mut gat_core_qb = GatCoreQueryBuilder::new(self.grid_service.clone());
        gat_core_qb.set_current_grid(grid_id);
        self.gat_core_query_builder = Some(gat_core_qb);

        // Switch to GatCoreQueryBuilder as the active query builder
        // In future, this could be made switchable with MockQueryBuilder for testing
        self.query_builder = Arc::new(
            self.gat_core_query_builder.clone().unwrap_or_else(|| {
                GatCoreQueryBuilder::new(self.grid_service.clone())
            })
        );

        // Invalidate cached results so they refresh with new grid data
        self.invalidate_caches();
    }

    /// Unload the current grid
    pub fn unload_current_grid(&mut self) -> Result<(), String> {
        if let Some(grid_id) = &self.current_grid_id {
            match self.grid_service.unload_grid(grid_id) {
                Ok(_) => {
                    self.current_grid_id = None;
                    self.gat_core_query_builder = None;
                    self.invalidate_caches();
                    Ok(())
                }
                Err(e) => Err(format!("{:?}", e)),
            }
        } else {
            Err("No grid currently loaded".to_string())
        }
    }

    /// Get list of all loaded grid IDs
    pub fn list_grids(&self) -> Vec<String> {
        self.grid_service.list_grids()
    }

    /// Invalidate all cached results
    ///
    /// Called when grid changes to force refresh of metrics, datasets, etc.
    fn invalidate_caches(&mut self) {
        self.datasets = None;
        self.workflows = None;
        self.metrics = None;
        self.pipeline_config = None;
        self.commands = None;

        self.datasets_loading = false;
        self.workflows_loading = false;
        self.metrics_loading = false;
        self.pipeline_loading = false;
        self.commands_loading = false;
    }

    /// Add a workflow execution record (Phase 3)
    pub fn add_workflow(&mut self, workflow: Workflow) {
        self.executed_workflows.push(workflow);

        // Keep only the last 100 workflows to manage memory
        if self.executed_workflows.len() > 100 {
            self.executed_workflows.remove(0);
        }

        // Update cache with new workflow list
        self.workflows = Some(Ok(self.executed_workflows.clone()));
    }

    /// Get all executed workflows
    pub fn get_workflows(&self) -> Vec<Workflow> {
        self.executed_workflows.clone()
    }

    /// Get workflows filtered by grid (if applicable)
    pub fn get_workflows_for_grid(&self, grid_id: &str) -> Vec<Workflow> {
        self.executed_workflows
            .iter()
            .filter(|w| w.name.contains(grid_id))
            .cloned()
            .collect()
    }

    /// Clear all workflow history
    pub fn clear_workflows(&mut self) {
        self.executed_workflows.clear();
        self.workflows = Some(Ok(Vec::new()));
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
