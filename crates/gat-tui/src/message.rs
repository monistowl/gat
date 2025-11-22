/// All messages/actions in the application (Elm-inspired)
///
/// Messages represent user actions and system events that drive state changes.
/// The update function processes messages to produce new state.

use crate::models::{PaneId, ModalState};
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
    TextInput(String, String), // component_id, text_value
    SelectionChange(String, usize), // component_id, selected_index
    CheckboxToggle(String, bool), // component_id, new_value

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
    TaskFailed(String, String), // task_id, error

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
}

#[derive(Clone, Debug)]
pub enum CommandsMessage {
    SelectCommand(usize),
    ExecuteCommand(String),
    CancelExecution,
    SearchCommands(String),
    ClearHistory,
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
}

#[derive(Clone, Debug)]
pub enum PipelineMessage {
    SelectNode(usize),
    AddTransform(String),
    RemoveTransform(usize),
    UpdateConfig(HashMap<String, String>),
    RunPipeline,
}

#[derive(Clone, Debug)]
pub enum OperationsMessage {
    SelectTab(usize), // 0=Batch, 1=Alloc, 2=Reliability
    ConfigChange(String, String), // key, value
    Execute,
    CancelRun,
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

impl ModalState {
    pub fn from_message(msg: ModalMessage) -> Self {
        match msg {
            ModalMessage::CommandExecution => ModalState::CommandExecution(Default::default()),
            ModalMessage::Settings => ModalState::Settings(Default::default()),
            ModalMessage::ConfirmAction(msg) => ModalState::Confirmation(crate::models::ConfirmationState {
                message: msg,
                yes_label: "Yes".to_string(),
                no_label: "No".to_string(),
            }),
            ModalMessage::Info(title, message) => ModalState::Info(crate::models::InfoState {
                title,
                message,
                details: None,
            }),
            ModalMessage::FilePicker => ModalState::None, // TODO: Implement file picker modal
        }
    }
}
