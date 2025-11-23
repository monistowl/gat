/// All messages/actions in the application (Elm-inspired)
///
/// Messages represent user actions and system events that drive state changes.
/// The update function processes messages to produce new state.
use crate::models::{ModalState, PaneId};
use crate::{DatasetEntry, QueryError};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Message {
    // Navigation
    SwitchPane(PaneId),
    SelectTab(PaneId, usize),

    // Modal control
    OpenModal(ModalMessage),
    CloseModal,

    // Input/Form handling
    TextInput(String, String),      // component_id, text_value
    SelectionChange(String, usize), // component_id, selected_index
    CheckboxToggle(String, bool),   // component_id, new_value

    // Pane-specific actions
    Dashboard(DashboardMessage),
    Commands(CommandsMessage),
    Datasets(DatasetsMessage),
    Pipeline(PipelineMessage),
    Operations(OperationsMessage),

    // System
    Tick,
    Resize(u16, u16), // width, height
    Settings(SettingsMessage),

    // Async task completion
    TaskCompleted(String, TaskResult), // task_id, result
    TaskFailed(String, String),        // task_id, error

    // Keyboard shortcuts
    KeyShortcut(KeyShortcut),
}

#[derive(Clone, Debug, Copy)]
pub enum KeyShortcut {
    Quit,
    Help,
    PaneSwitch(char), // '1'-'5' or 'h'
    NextTab,
    PrevTab,
    Search,
}

#[derive(Clone, Debug)]
pub enum ModalMessage {
    CommandExecution,
    Settings,
    ConfirmAction(String),
    Info(String, String), // title, message
    FilePicker,
}

#[derive(Clone, Debug)]
pub enum DashboardMessage {
    RefreshMetrics,
    ClickMetric(String),
    FetchMetrics,
    MetricsLoaded(Result<crate::data::SystemMetrics, QueryError>),
}

#[derive(Clone, Debug)]
pub enum CommandsMessage {
    SelectCommand(usize),
    ExecuteCommand(String),
    CancelExecution,
    SearchCommands(String),
    ClearHistory,
    FetchCommands,
    CommandsLoaded(Result<Vec<String>, QueryError>),
}

#[derive(Clone, Debug)]
pub enum DatasetsMessage {
    SelectDataset(usize),
    UploadDataset(String), // file path
    DeleteDataset(usize),
    SearchDatasets(String),
    RefreshList,
    // Async data fetching
    FetchDatasets,
    DatasetsLoaded(Result<Vec<DatasetEntry>, QueryError>),

    // Grid management (Phase 3)
    LoadGrid(String),   // file_path
    UnloadGrid(String), // grid_id
    SwitchGrid(String), // grid_id
    RefreshGrids,
    GridLoaded(String),     // grid_id
    GridLoadFailed(String), // error message
}

#[derive(Clone, Debug)]
pub enum PipelineMessage {
    SelectNode(usize),
    AddTransform(String),
    RemoveTransform(usize),
    UpdateConfig(HashMap<String, String>),
    RunPipeline,
    FetchPipeline,
    PipelineLoaded(Result<String, QueryError>),
}

#[derive(Clone, Debug)]
pub enum OperationsMessage {
    SelectTab(usize),             // 0=Batch, 1=Alloc, 2=Reliability
    ConfigChange(String, String), // key, value
    Execute,
    CancelRun,
    FetchOperations,
    OperationsLoaded(Result<Vec<crate::data::Workflow>, QueryError>),

    // Command execution (Phase 4)
    ExecuteCommand(String),                          // command line
    CommandOutput(String),                           // output chunk
    CommandCompleted(Result<CommandResult, String>), // result or error
    CancelCommand,                                   // stop running command
}

#[derive(Clone, Debug)]
pub enum SettingsMessage {
    UpdateTheme(String),
    UpdateCliPath(String),
    UpdateTimeout(u64),
    UpdateAutoSave(bool),
    UpdateConfirmDelete(bool),
    SaveSettings,
    ResetToDefaults,
}

#[derive(Clone, Debug)]
pub enum TaskResult {
    Success(String),
    Failure(String),
    Output(String),
}

/// Command execution result (Phase 4)
#[derive(Clone, Debug)]
pub struct CommandResult {
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

impl ModalState {
    pub fn from_message(msg: ModalMessage) -> Self {
        match msg {
            ModalMessage::CommandExecution => ModalState::CommandExecution(Default::default()),
            ModalMessage::Settings => ModalState::Settings(Default::default()),
            ModalMessage::ConfirmAction(msg) => {
                ModalState::Confirmation(crate::models::ConfirmationState {
                    message: msg,
                    yes_label: "Yes".to_string(),
                    no_label: "No".to_string(),
                })
            }
            ModalMessage::Info(title, message) => ModalState::Info(crate::models::InfoState {
                title,
                message,
                details: None,
            }),
            ModalMessage::FilePicker => ModalState::None, // TODO: Implement file picker modal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_result_creation() {
        let result = CommandResult {
            command: "gat-cli datasets list".to_string(),
            exit_code: 0,
            stdout: "dataset1\ndataset2".to_string(),
            stderr: String::new(),
            duration_ms: 150,
            timed_out: false,
        };
        assert_eq!(result.exit_code, 0);
        assert!(!result.timed_out);
        assert_eq!(result.duration_ms, 150);
    }

    #[test]
    fn test_command_result_with_error() {
        let result = CommandResult {
            command: "gat-cli invalid".to_string(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "Unknown command: invalid".to_string(),
            duration_ms: 50,
            timed_out: false,
        };
        assert_ne!(result.exit_code, 0);
        assert!(!result.stderr.is_empty());
    }

    #[test]
    fn test_command_result_timeout() {
        let result = CommandResult {
            command: "gat-cli long-running".to_string(),
            exit_code: -1,
            stdout: "partial output".to_string(),
            stderr: "Command timed out".to_string(),
            duration_ms: 300000,
            timed_out: true,
        };
        assert!(result.timed_out);
    }

    #[test]
    fn test_operations_message_execute_command() {
        let msg = OperationsMessage::ExecuteCommand("gat-cli datasets list".to_string());
        match msg {
            OperationsMessage::ExecuteCommand(cmd) => {
                assert_eq!(cmd, "gat-cli datasets list");
            }
            _ => panic!("Wrong message variant"),
        }
    }

    #[test]
    fn test_operations_message_command_output() {
        let msg = OperationsMessage::CommandOutput("dataset1".to_string());
        match msg {
            OperationsMessage::CommandOutput(output) => {
                assert_eq!(output, "dataset1");
            }
            _ => panic!("Wrong message variant"),
        }
    }

    #[test]
    fn test_operations_message_command_completed_success() {
        let result = CommandResult {
            command: "echo test".to_string(),
            exit_code: 0,
            stdout: "test".to_string(),
            stderr: String::new(),
            duration_ms: 10,
            timed_out: false,
        };
        let msg = OperationsMessage::CommandCompleted(Ok(result));
        match msg {
            OperationsMessage::CommandCompleted(Ok(r)) => {
                assert_eq!(r.exit_code, 0);
                assert_eq!(r.stdout, "test");
            }
            _ => panic!("Wrong message variant"),
        }
    }
}
