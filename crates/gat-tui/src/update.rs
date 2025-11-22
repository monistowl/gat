/// State update logic (Elm-style reducer)
///
/// Pure function that takes current state and a message, producing new state.
/// This is where all business logic happens.

use crate::message::*;
use crate::models::*;

/// Update function: processes messages and returns new state + side effects
pub fn update(mut state: AppState, msg: Message) -> (AppState, Vec<SideEffect>) {
    let mut effects = Vec::new();

    match msg {
        // Navigation
        Message::SwitchPane(pane_id) => {
            state.active_pane = pane_id;
        }
        Message::SelectTab(pane_id, tab_idx) => {
            let pane_state = state
                .pane_states
                .entry(pane_id)
                .or_insert_with(PaneState::default);
            pane_state.selected_tab = tab_idx;
        }

        // Modal control
        Message::OpenModal(modal_msg) => {
            state.modal_state = Some(ModalState::from_message(modal_msg));
        }
        Message::CloseModal => {
            state.modal_state = Some(ModalState::None);
        }

        // Input handling
        Message::TextInput(component_id, text) => {
            let pane_state = state
                .pane_states
                .entry(state.active_pane)
                .or_insert_with(PaneState::default);
            pane_state.form_values.insert(component_id, text);
        }
        Message::SelectionChange(component_id, idx) => {
            let pane_state = state
                .pane_states
                .entry(state.active_pane)
                .or_insert_with(PaneState::default);
            pane_state.form_values.insert(component_id, idx.to_string());
        }
        Message::CheckboxToggle(_component_id, _value) => {
            // TODO: Store checkbox state
        }

        // Pane-specific handlers
        Message::Dashboard(msg) => {
            handle_dashboard(&mut state, msg, &mut effects);
        }
        Message::Commands(msg) => {
            handle_commands(&mut state, msg, &mut effects);
        }
        Message::Datasets(msg) => {
            handle_datasets(&mut state, msg, &mut effects);
        }
        Message::Pipeline(msg) => {
            handle_pipeline(&mut state, msg, &mut effects);
        }
        Message::Operations(msg) => {
            handle_operations(&mut state, msg, &mut effects);
        }

        // System messages
        Message::Tick => {
            // Periodic updates
        }
        Message::Resize(w, h) => {
            state.terminal_width = w;
            state.terminal_height = h;
        }
        Message::Settings(msg) => {
            handle_settings(&mut state, msg);
        }

        // Async task completion
        Message::TaskCompleted(task_id, result) => {
            state.async_tasks.remove(&task_id);
            // Handle task results and update relevant pane state
            match result {
                TaskResult::Success(_output) => {
                    state.add_notification(
                        "Task completed successfully",
                        NotificationKind::Success,
                    );
                }
                TaskResult::Failure(err) => {
                    state.add_notification(&format!("Task failed: {}", err), NotificationKind::Error);
                }
                TaskResult::Output(_output) => {
                    // Update pane with output
                }
            }
        }
        Message::TaskFailed(task_id, error) => {
            state.async_tasks.remove(&task_id);
            state.add_notification(&error, NotificationKind::Error);
        }

        // Keyboard shortcuts
        Message::KeyShortcut(shortcut) => {
            handle_shortcut(&mut state, shortcut);
        }
    }

    (state, effects)
}

fn handle_dashboard(_state: &mut AppState, _msg: DashboardMessage, _effects: &mut Vec<SideEffect>) {
    // TODO: Implement dashboard logic
}

fn handle_commands(
    state: &mut AppState,
    msg: CommandsMessage,
    effects: &mut Vec<SideEffect>,
) {
    match msg {
        CommandsMessage::ExecuteCommand(cmd) => {
            let task_id = format!("cmd_{}", state.async_tasks.len());
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);
            effects.push(SideEffect::ExecuteCommand {
                task_id,
                command: cmd,
            });
        }
        CommandsMessage::SearchCommands(query) => {
            let pane_state = state
                .pane_states
                .entry(PaneId::Commands)
                .or_insert_with(PaneState::default);
            pane_state.form_values.insert("search".to_string(), query);
        }
        _ => {
            // Other command messages
        }
    }
}

fn handle_datasets(
    state: &mut AppState,
    msg: DatasetsMessage,
    effects: &mut Vec<SideEffect>,
) {
    match msg {
        DatasetsMessage::UploadDataset(path) => {
            let task_id = format!("dataset_{}", state.async_tasks.len());
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);
            effects.push(SideEffect::UploadDataset {
                task_id,
                file_path: path,
            });
        }
        _ => {}
    }
}

fn handle_pipeline(_state: &mut AppState, _msg: PipelineMessage, _effects: &mut Vec<SideEffect>) {
    // TODO: Implement pipeline logic
}

fn handle_operations(_state: &mut AppState, _msg: OperationsMessage, _effects: &mut Vec<SideEffect>) {
    // TODO: Implement operations logic
}

fn handle_settings(state: &mut AppState, msg: SettingsMessage) {
    match msg {
        SettingsMessage::UpdateTheme(theme_str) => {
            if theme_str == "light" {
                state.settings.theme = Theme::Light;
            } else {
                state.settings.theme = Theme::Dark;
            }
        }
        SettingsMessage::UpdateCliPath(path) => {
            state.settings.gat_cli_path = path;
        }
        SettingsMessage::UpdateTimeout(secs) => {
            state.settings.command_timeout_secs = secs;
        }
        SettingsMessage::UpdateAutoSave(enabled) => {
            state.settings.auto_save_on_pane_switch = enabled;
        }
        SettingsMessage::UpdateConfirmDelete(enabled) => {
            state.settings.confirm_on_delete = enabled;
        }
        SettingsMessage::SaveSettings => {
            // Persist settings to file
        }
        SettingsMessage::ResetToDefaults => {
            state.settings = AppSettings::default();
        }
    }
}

fn handle_shortcut(state: &mut AppState, shortcut: KeyShortcut) {
    match shortcut {
        KeyShortcut::Quit => {
            state.should_quit = true;
        }
        KeyShortcut::Help => {
            state.active_pane = PaneId::Help;
        }
        KeyShortcut::PaneSwitch(ch) => {
            let pane = match ch {
                '1' => PaneId::Dashboard,
                '2' => PaneId::Operations,
                '3' => PaneId::Datasets,
                '4' => PaneId::Pipeline,
                '5' => PaneId::Commands,
                'h' => PaneId::Help,
                _ => return,
            };
            state.active_pane = pane;
        }
        _ => {}
    }
}

/// Side effects that should be executed outside the state machine
#[derive(Clone, Debug)]
pub enum SideEffect {
    ExecuteCommand { task_id: String, command: String },
    UploadDataset { task_id: String, file_path: String },
    FetchMetrics { task_id: String },
    SaveSettings(AppSettings),
    // Add more as needed
}

