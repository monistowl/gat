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
                    state
                        .add_notification("Task completed successfully", NotificationKind::Success);
                }
                TaskResult::Failure(err) => {
                    state.add_notification(
                        &format!("Task failed: {}", err),
                        NotificationKind::Error,
                    );
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

fn handle_dashboard(state: &mut AppState, msg: DashboardMessage, effects: &mut Vec<SideEffect>) {
    match msg {
        DashboardMessage::RefreshMetrics | DashboardMessage::FetchMetrics => {
            let task_id = "fetch_metrics".to_string();
            state.metrics_loading = true;
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);
            effects.push(SideEffect::FetchMetrics { task_id });
        }
        DashboardMessage::MetricsLoaded(result) => {
            state.metrics = Some(result.clone());
            state.metrics_loading = false;
            state.async_tasks.remove("fetch_metrics");

            match result {
                Ok(_metrics) => {
                    state
                        .add_notification("Metrics loaded successfully", NotificationKind::Success);
                }
                Err(e) => {
                    state.add_notification(
                        &format!("Failed to load metrics: {}", e),
                        NotificationKind::Error,
                    );
                }
            }
        }
        DashboardMessage::ClickMetric(_) => {
            // Local handling - no async
        }
    }
}

fn handle_commands(state: &mut AppState, msg: CommandsMessage, effects: &mut Vec<SideEffect>) {
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
        CommandsMessage::FetchCommands => {
            let task_id = "fetch_commands".to_string();
            state.commands_loading = true;
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);
            effects.push(SideEffect::FetchCommands { task_id });
        }
        CommandsMessage::CommandsLoaded(result) => {
            state.commands = Some(result.clone());
            state.commands_loading = false;
            state.async_tasks.remove("fetch_commands");

            match result {
                Ok(commands) => {
                    state.add_notification(
                        &format!("Loaded {} commands", commands.len()),
                        NotificationKind::Success,
                    );
                }
                Err(e) => {
                    state.add_notification(
                        &format!("Failed to load commands: {}", e),
                        NotificationKind::Error,
                    );
                }
            }
        }
        _ => {
            // Other command messages
        }
    }
}

fn handle_datasets(state: &mut AppState, msg: DatasetsMessage, effects: &mut Vec<SideEffect>) {
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
        DatasetsMessage::FetchDatasets => {
            // Spawn async fetch task
            let task_id = "fetch_datasets".to_string();
            state.datasets_loading = true;
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);
            effects.push(SideEffect::FetchDatasets { task_id });
        }
        DatasetsMessage::DatasetsLoaded(result) => {
            // Handle fetch completion
            state.datasets = Some(result.clone());
            state.datasets_loading = false;
            state.async_tasks.remove("fetch_datasets");

            match result {
                Ok(datasets) => {
                    state.add_notification(
                        &format!("Loaded {} datasets", datasets.len()),
                        NotificationKind::Success,
                    );
                }
                Err(e) => {
                    state.add_notification(
                        &format!("Failed to load datasets: {}", e),
                        NotificationKind::Error,
                    );
                }
            }
        }

        // Grid management (Phase 3)
        DatasetsMessage::LoadGrid(file_path) => {
            // Load grid from file path
            let task_id = format!("load_grid_{}", state.async_tasks.len());
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);
            effects.push(SideEffect::LoadGrid { task_id, file_path });
        }

        DatasetsMessage::UnloadGrid(_grid_id) => {
            // Unload grid and refresh
            match state.unload_current_grid() {
                Ok(_) => {
                    state.add_notification(&format!("Grid unloaded"), NotificationKind::Success);
                    // Trigger refresh of datasets
                    effects.push(SideEffect::FetchDatasets {
                        task_id: "fetch_datasets_after_unload".to_string(),
                    });
                }
                Err(e) => {
                    state.add_notification(
                        &format!("Failed to unload grid: {}", e),
                        NotificationKind::Error,
                    );
                }
            }
        }

        DatasetsMessage::SwitchGrid(grid_id) => {
            // Switch to a different grid
            state.set_current_grid(grid_id.clone());
            state.add_notification(
                &format!("Switched to grid {}", grid_id),
                NotificationKind::Success,
            );
            // Trigger metrics refresh for new grid
            let msg = Message::Dashboard(DashboardMessage::FetchMetrics);
            effects.push(SideEffect::SendMessage(Box::new(msg)));
        }

        DatasetsMessage::RefreshGrids => {
            // Refresh the list of loaded grids
            effects.push(SideEffect::FetchDatasets {
                task_id: "fetch_grids".to_string(),
            });
        }

        DatasetsMessage::GridLoaded(grid_id) => {
            // Grid load succeeded
            state.add_notification(
                &format!("Grid '{}' loaded successfully", grid_id),
                NotificationKind::Success,
            );
            // Refresh datasets list and metrics
            effects.push(SideEffect::FetchDatasets {
                task_id: "fetch_datasets_after_load".to_string(),
            });
            let msg = Message::Dashboard(DashboardMessage::FetchMetrics);
            effects.push(SideEffect::SendMessage(Box::new(msg)));
        }

        DatasetsMessage::GridLoadFailed(error) => {
            // Grid load failed
            state.add_notification(
                &format!("Failed to load grid: {}", error),
                NotificationKind::Error,
            );
        }

        // Other existing messages
        DatasetsMessage::SelectDataset(_) => {}
        DatasetsMessage::DeleteDataset(_) => {}
        DatasetsMessage::SearchDatasets(_) => {}
        DatasetsMessage::RefreshList => {}
    }
}

fn handle_pipeline(state: &mut AppState, msg: PipelineMessage, effects: &mut Vec<SideEffect>) {
    match msg {
        PipelineMessage::FetchPipeline => {
            let task_id = "fetch_pipeline".to_string();
            state.pipeline_loading = true;
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);
            effects.push(SideEffect::FetchPipeline { task_id });
        }
        PipelineMessage::PipelineLoaded(result) => {
            state.pipeline_config = Some(result.clone());
            state.pipeline_loading = false;
            state.async_tasks.remove("fetch_pipeline");

            match result {
                Ok(_config) => {
                    state.add_notification(
                        "Pipeline configuration loaded",
                        NotificationKind::Success,
                    );
                }
                Err(e) => {
                    state.add_notification(
                        &format!("Failed to load pipeline: {}", e),
                        NotificationKind::Error,
                    );
                }
            }
        }
        _ => {
            // TODO: Implement other pipeline operations
        }
    }
}

fn handle_operations(state: &mut AppState, msg: OperationsMessage, effects: &mut Vec<SideEffect>) {
    match msg {
        OperationsMessage::FetchOperations => {
            let task_id = "fetch_operations".to_string();
            state.workflows_loading = true;
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);
            effects.push(SideEffect::FetchOperations { task_id });
        }
        OperationsMessage::OperationsLoaded(result) => {
            state.workflows = Some(result.clone());
            state.workflows_loading = false;
            state.async_tasks.remove("fetch_operations");

            match result {
                Ok(workflows) => {
                    state.add_notification(
                        &format!("Loaded {} operations", workflows.len()),
                        NotificationKind::Success,
                    );
                }
                Err(e) => {
                    state.add_notification(
                        &format!("Failed to load operations: {}", e),
                        NotificationKind::Error,
                    );
                }
            }
        }
        // Command execution (Phase 4)
        OperationsMessage::ExecuteCommand(command) => {
            let task_id = format!("cmd_{}", chrono::Local::now().timestamp_millis());
            state
                .async_tasks
                .insert(task_id.clone(), AsyncTaskState::Running);

            // Add to pane state for command execution UI
            let pane_state = state
                .pane_states
                .entry(PaneId::Operations)
                .or_insert_with(PaneState::default);
            pane_state
                .form_values
                .insert("executing_command".to_string(), command.clone());
            pane_state
                .form_values
                .insert("command_output".to_string(), String::new());

            state.add_notification(&format!("Executing: {}", command), NotificationKind::Info);

            effects.push(SideEffect::RunCommand { task_id, command });
        }
        OperationsMessage::CommandOutput(output) => {
            // Append output to command result buffer
            let pane_state = state
                .pane_states
                .entry(PaneId::Operations)
                .or_insert_with(PaneState::default);

            let current = pane_state
                .form_values
                .get("command_output")
                .cloned()
                .unwrap_or_default();
            let new_output = if current.is_empty() {
                output
            } else {
                format!("{}\n{}", current, output)
            };
            pane_state
                .form_values
                .insert("command_output".to_string(), new_output);
        }
        OperationsMessage::CommandCompleted(result) => {
            match result {
                Ok(cmd_result) => {
                    // Create workflow record for completed command
                    let workflow = crate::data::Workflow {
                        id: format!("cmd_{}", chrono::Local::now().timestamp_millis()),
                        name: format!("Command: {}", cmd_result.command),
                        status: if cmd_result.exit_code == 0 {
                            crate::data::WorkflowStatus::Succeeded
                        } else {
                            crate::data::WorkflowStatus::Failed
                        },
                        created_by: "user".to_string(),
                        created_at: std::time::SystemTime::now(),
                        completed_at: Some(std::time::SystemTime::now()),
                    };

                    state.add_workflow(workflow);

                    // Prepare notification data before borrowing pane_state
                    let exit_code = cmd_result.exit_code;
                    let duration_ms = cmd_result.duration_ms;
                    let status = if exit_code == 0 {
                        "succeeded"
                    } else {
                        "failed"
                    };
                    let notif_kind = if exit_code == 0 {
                        NotificationKind::Success
                    } else {
                        NotificationKind::Warning
                    };

                    // Now update pane state
                    {
                        let pane_state = state
                            .pane_states
                            .entry(PaneId::Operations)
                            .or_insert_with(PaneState::default);
                        pane_state
                            .form_values
                            .insert("last_exit_code".to_string(), exit_code.to_string());
                        pane_state
                            .form_values
                            .insert("last_duration".to_string(), format!("{}ms", duration_ms));
                    }

                    // Add notification after pane state is updated and borrow dropped
                    state.add_notification(
                        &format!("Command {} (exit code: {})", status, exit_code),
                        notif_kind,
                    );

                    // Clear executing flag
                    {
                        let pane_state = state
                            .pane_states
                            .entry(PaneId::Operations)
                            .or_insert_with(PaneState::default);
                        pane_state.form_values.remove("executing_command");
                    }
                }
                Err(err) => {
                    state.add_notification(
                        &format!("Command failed: {}", err),
                        NotificationKind::Error,
                    );

                    let pane_state = state
                        .pane_states
                        .entry(PaneId::Operations)
                        .or_insert_with(PaneState::default);
                    pane_state
                        .form_values
                        .insert("command_output".to_string(), format!("Error: {}", err));
                }
            }
        }
        OperationsMessage::CancelCommand => {
            let should_cancel = {
                let pane_state = state
                    .pane_states
                    .entry(PaneId::Operations)
                    .or_insert_with(PaneState::default);
                pane_state.form_values.get("executing_command").is_some()
            };

            if should_cancel {
                let task_ids: Vec<_> = state
                    .async_tasks
                    .keys()
                    .filter(|id| id.starts_with("cmd_"))
                    .cloned()
                    .collect();

                for task_id in task_ids {
                    state.async_tasks.remove(&task_id);
                    effects.push(SideEffect::CancelCommand { task_id });
                }

                state.add_notification("Command cancelled", NotificationKind::Info);

                let pane_state = state
                    .pane_states
                    .entry(PaneId::Operations)
                    .or_insert_with(PaneState::default);
                pane_state.form_values.remove("executing_command");
            }
        }
        _ => {
            // Other operations handled elsewhere
        }
    }
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
    FetchDatasets { task_id: String },
    FetchOperations { task_id: String },
    FetchPipeline { task_id: String },
    FetchCommands { task_id: String },
    SaveSettings(AppSettings),
    // Grid management (Phase 3)
    LoadGrid { task_id: String, file_path: String },
    SendMessage(Box<Message>),
    // Command execution (Phase 4)
    RunCommand { task_id: String, command: String },
    CancelCommand { task_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Workflow;
    use crate::{create_fixture_datasets, QueryError};

    #[test]
    fn test_fetch_datasets_message() {
        let state = AppState::new();
        assert!(!state.datasets_loading);

        // Send FetchDatasets message
        let msg = Message::Datasets(DatasetsMessage::FetchDatasets);
        let (new_state, effects) = update(state, msg);

        // Verify loading flag is set
        assert!(new_state.datasets_loading);

        // Verify side effect is created
        assert!(!effects.is_empty());
        match &effects[0] {
            SideEffect::FetchDatasets { task_id } => {
                assert_eq!(task_id, "fetch_datasets");
            }
            _ => panic!("Expected FetchDatasets side effect"),
        }
    }

    #[test]
    fn test_datasets_loaded_success() {
        let mut state = AppState::new();
        state.datasets_loading = true;

        // Create fixture datasets
        let datasets = create_fixture_datasets();

        // Send DatasetsLoaded message with Ok result
        let msg = Message::Datasets(DatasetsMessage::DatasetsLoaded(Ok(datasets.clone())));
        let (new_state, _effects) = update(state, msg);

        // Verify results are cached
        assert!(new_state.datasets.is_some());
        match &new_state.datasets {
            Some(Ok(loaded)) => {
                assert_eq!(loaded.len(), datasets.len());
            }
            _ => panic!("Expected Ok(Vec<DatasetEntry>)"),
        }

        // Verify loading flag is cleared
        assert!(!new_state.datasets_loading);

        // Verify notification was added
        assert!(!new_state.notifications.is_empty());
    }

    #[test]
    fn test_datasets_loaded_error() {
        let mut state = AppState::new();
        state.datasets_loading = true;

        // Send DatasetsLoaded message with Err result
        let error = QueryError::ConnectionFailed("Network error".to_string());
        let msg = Message::Datasets(DatasetsMessage::DatasetsLoaded(Err(error.clone())));
        let (new_state, _effects) = update(state, msg);

        // Verify error is cached
        assert!(new_state.datasets.is_some());
        assert!(matches!(
            &new_state.datasets,
            Some(Err(QueryError::ConnectionFailed(_)))
        ));

        // Verify loading flag is cleared
        assert!(!new_state.datasets_loading);

        // Verify error notification was added
        assert!(!new_state.notifications.is_empty());
        let last_notif = new_state.notifications.last().unwrap();
        assert!(matches!(last_notif.kind, NotificationKind::Error));
    }

    #[test]
    fn test_fetch_and_load_flow() {
        let state = AppState::new();

        // Step 1: Trigger fetch
        let msg1 = Message::Datasets(DatasetsMessage::FetchDatasets);
        let (state1, effects1) = update(state, msg1);

        assert!(state1.datasets_loading);
        assert!(!effects1.is_empty());

        // Step 2: Complete fetch with success
        let datasets = create_fixture_datasets();
        let msg2 = Message::Datasets(DatasetsMessage::DatasetsLoaded(Ok(datasets)));
        let (state2, _effects2) = update(state1, msg2);

        // Verify final state
        assert!(!state2.datasets_loading);
        assert!(state2.datasets.is_some());
        assert!(matches!(&state2.datasets, Some(Ok(_))));
    }

    // Dashboard async tests
    #[test]
    fn test_fetch_metrics_message() {
        let state = AppState::new();
        assert!(!state.metrics_loading);

        let msg = Message::Dashboard(DashboardMessage::FetchMetrics);
        let (new_state, effects) = update(state, msg);

        assert!(new_state.metrics_loading);
        assert!(!effects.is_empty());
        match &effects[0] {
            SideEffect::FetchMetrics { task_id } => {
                assert_eq!(task_id, "fetch_metrics");
            }
            _ => panic!("Expected FetchMetrics side effect"),
        }
    }

    #[test]
    fn test_metrics_loaded_success() {
        let mut state = AppState::new();
        state.metrics_loading = true;

        let metrics = crate::data::SystemMetrics {
            deliverability_score: 85.5,
            lole_hours_per_year: 9.2,
            eue_mwh_per_year: 15.3,
        };

        let msg = Message::Dashboard(DashboardMessage::MetricsLoaded(Ok(metrics.clone())));
        let (new_state, _effects) = update(state, msg);

        assert!(!new_state.metrics_loading);
        assert!(new_state.metrics.is_some());
        match &new_state.metrics {
            Some(Ok(loaded)) => {
                assert_eq!(loaded.deliverability_score, 85.5);
            }
            _ => panic!("Expected Ok(SystemMetrics)"),
        }
        assert!(!new_state.notifications.is_empty());
    }

    // Operations async tests
    #[test]
    fn test_fetch_operations_message() {
        let state = AppState::new();
        assert!(!state.workflows_loading);

        let msg = Message::Operations(OperationsMessage::FetchOperations);
        let (new_state, effects) = update(state, msg);

        assert!(new_state.workflows_loading);
        assert!(!effects.is_empty());
        match &effects[0] {
            SideEffect::FetchOperations { task_id } => {
                assert_eq!(task_id, "fetch_operations");
            }
            _ => panic!("Expected FetchOperations side effect"),
        }
    }

    #[test]
    fn test_operations_loaded_success() {
        let mut state = AppState::new();
        state.workflows_loading = true;

        let workflows = vec![];
        let msg = Message::Operations(OperationsMessage::OperationsLoaded(Ok(workflows)));
        let (new_state, _effects) = update(state, msg);

        assert!(!new_state.workflows_loading);
        assert!(new_state.workflows.is_some());
        assert!(matches!(&new_state.workflows, Some(Ok(_))));
    }

    // Pipeline async tests
    #[test]
    fn test_fetch_pipeline_message() {
        let state = AppState::new();
        assert!(!state.pipeline_loading);

        let msg = Message::Pipeline(PipelineMessage::FetchPipeline);
        let (new_state, effects) = update(state, msg);

        assert!(new_state.pipeline_loading);
        assert!(!effects.is_empty());
        match &effects[0] {
            SideEffect::FetchPipeline { task_id } => {
                assert_eq!(task_id, "fetch_pipeline");
            }
            _ => panic!("Expected FetchPipeline side effect"),
        }
    }

    #[test]
    fn test_pipeline_loaded_success() {
        let mut state = AppState::new();
        state.pipeline_loading = true;

        let config = r#"{"name":"test"}"#.to_string();
        let msg = Message::Pipeline(PipelineMessage::PipelineLoaded(Ok(config)));
        let (new_state, _effects) = update(state, msg);

        assert!(!new_state.pipeline_loading);
        assert!(new_state.pipeline_config.is_some());
        assert!(matches!(&new_state.pipeline_config, Some(Ok(_))));
    }

    // Commands async tests
    #[test]
    fn test_fetch_commands_message() {
        let state = AppState::new();
        assert!(!state.commands_loading);

        let msg = Message::Commands(CommandsMessage::FetchCommands);
        let (new_state, effects) = update(state, msg);

        assert!(new_state.commands_loading);
        assert!(!effects.is_empty());
        match &effects[0] {
            SideEffect::FetchCommands { task_id } => {
                assert_eq!(task_id, "fetch_commands");
            }
            _ => panic!("Expected FetchCommands side effect"),
        }
    }

    #[test]
    fn test_commands_loaded_success() {
        let mut state = AppState::new();
        state.commands_loading = true;

        let commands = vec!["cmd1".to_string(), "cmd2".to_string()];
        let msg = Message::Commands(CommandsMessage::CommandsLoaded(Ok(commands.clone())));
        let (new_state, _effects) = update(state, msg);

        assert!(!new_state.commands_loading);
        assert!(new_state.commands.is_some());
        match &new_state.commands {
            Some(Ok(loaded)) => {
                assert_eq!(loaded.len(), 2);
            }
            _ => panic!("Expected Ok(Vec<String>)"),
        }
    }

    // Concurrent async operations test
    #[test]
    fn test_concurrent_pane_fetches() {
        let state = AppState::new();

        // Trigger all pane fetches
        let msg1 = Message::Dashboard(DashboardMessage::FetchMetrics);
        let (state1, effects1) = update(state, msg1);
        assert!(state1.metrics_loading);

        let msg2 = Message::Datasets(DatasetsMessage::FetchDatasets);
        let (state2, effects2) = update(state1, msg2);
        assert!(state2.datasets_loading);
        assert!(state2.metrics_loading);

        let msg3 = Message::Operations(OperationsMessage::FetchOperations);
        let (state3, effects3) = update(state2, msg3);
        assert!(state3.workflows_loading);
        assert!(state3.datasets_loading);
        assert!(state3.metrics_loading);

        // All three fetches should have been spawned
        assert_eq!(effects1.len(), 1);
        assert_eq!(effects2.len(), 1);
        assert_eq!(effects3.len(), 1);
    }

    // Grid management tests (Phase 3)
    #[test]
    fn test_load_grid_message() {
        let state = AppState::new();
        let file_path = "/test_data/matpower/ieee14.arrow".to_string();

        let msg = Message::Datasets(DatasetsMessage::LoadGrid(file_path.clone()));
        let (new_state, effects) = update(state, msg);

        // Should create a load task
        assert!(!effects.is_empty());
        match &effects[0] {
            SideEffect::LoadGrid {
                task_id,
                file_path: path,
            } => {
                assert!(!task_id.is_empty());
                assert_eq!(path, &file_path);
            }
            _ => panic!("Expected LoadGrid side effect"),
        }

        // Task should be tracked
        assert!(new_state.async_tasks.len() > 0);
    }

    #[test]
    fn test_switch_grid_message() {
        let state = AppState::new();
        let grid_id = "test-grid-123".to_string();

        // Mock grid service with loaded grid
        let _ = state.list_grids();

        let msg = Message::Datasets(DatasetsMessage::SwitchGrid(grid_id.clone()));
        let (new_state, effects) = update(state, msg);

        // Should set current grid
        assert_eq!(new_state.current_grid_id.as_ref(), Some(&grid_id));

        // Should trigger metrics fetch
        assert!(!effects.is_empty());
    }

    #[test]
    fn test_grid_loaded_success_message() {
        let state = AppState::new();
        let grid_id = "ieee14".to_string();

        let msg = Message::Datasets(DatasetsMessage::GridLoaded(grid_id.clone()));
        let (_new_state, effects) = update(state, msg);

        // Should trigger dataset fetch and metrics fetch
        assert!(effects.len() >= 1);
    }

    #[test]
    fn test_unload_grid_message() {
        let state = AppState::new();
        let grid_id = "test-grid".to_string();

        // Send unload message (grid doesn't exist, so it will fail)
        let msg = Message::Datasets(DatasetsMessage::UnloadGrid(grid_id.clone()));
        let (new_state, _effects) = update(state, msg);

        // Should show error notification since grid wasn't loaded
        assert!(!new_state.notifications.is_empty());
        let notification = &new_state.notifications[0];
        assert!(notification.message.to_lowercase().contains("no grid"));
    }

    #[test]
    fn test_refresh_grids_message() {
        let state = AppState::new();

        let msg = Message::Datasets(DatasetsMessage::RefreshGrids);
        let (_new_state, effects) = update(state, msg);

        // Should trigger dataset fetch
        assert!(!effects.is_empty());
        match &effects[0] {
            SideEffect::FetchDatasets { task_id } => {
                assert_eq!(task_id, "fetch_grids");
            }
            _ => panic!("Expected FetchDatasets side effect"),
        }
    }

    #[test]
    fn test_grid_load_failed_message() {
        let state = AppState::new();
        let error_msg = "File not found".to_string();

        let msg = Message::Datasets(DatasetsMessage::GridLoadFailed(error_msg.clone()));
        let (new_state, _effects) = update(state, msg);

        // Should show error notification
        assert!(!new_state.notifications.is_empty());
        let notification = &new_state.notifications[0];
        let msg_lower = notification.message.to_lowercase();
        assert!(msg_lower.contains("failed") || msg_lower.contains("error"));
        assert!(msg_lower.contains("grid"));
    }

    #[test]
    fn test_load_grid_with_multiple_tasks() {
        let state = AppState::new();

        // Send multiple load grid messages
        let msg1 = Message::Datasets(DatasetsMessage::LoadGrid("grid1.arrow".to_string()));
        let (state1, _effects1) = update(state, msg1);

        let msg2 = Message::Datasets(DatasetsMessage::LoadGrid("grid2.arrow".to_string()));
        let (state2, _effects2) = update(state1, msg2);

        // Both tasks should be tracked
        assert!(state2.async_tasks.len() >= 2);
    }

    // Workflow tracking tests (Phase 3)
    #[test]
    fn test_add_workflow() {
        use crate::data::WorkflowStatus;

        let mut state = AppState::new();
        assert_eq!(state.executed_workflows.len(), 0);

        let workflow = Workflow {
            id: "wf1".to_string(),
            name: "Test Workflow".to_string(),
            status: WorkflowStatus::Succeeded,
            created_by: "test".to_string(),
            created_at: std::time::SystemTime::now(),
            completed_at: Some(std::time::SystemTime::now()),
        };

        state.add_workflow(workflow);
        assert_eq!(state.executed_workflows.len(), 1);
        assert!(state.workflows.is_some());
    }

    #[test]
    fn test_get_workflows() {
        use crate::data::WorkflowStatus;

        let mut state = AppState::new();

        for i in 0..3 {
            let workflow = Workflow {
                id: format!("wf{}", i),
                name: format!("Workflow {}", i),
                status: WorkflowStatus::Succeeded,
                created_by: "test".to_string(),
                created_at: std::time::SystemTime::now(),
                completed_at: Some(std::time::SystemTime::now()),
            };
            state.add_workflow(workflow);
        }

        let workflows = state.get_workflows();
        assert_eq!(workflows.len(), 3);
    }

    #[test]
    fn test_workflow_memory_cleanup() {
        use crate::data::WorkflowStatus;

        let mut state = AppState::new();

        // Add more than 100 workflows
        for i in 0..105 {
            let workflow = Workflow {
                id: format!("wf{}", i),
                name: format!("Workflow {}", i),
                status: WorkflowStatus::Succeeded,
                created_by: "test".to_string(),
                created_at: std::time::SystemTime::now(),
                completed_at: Some(std::time::SystemTime::now()),
            };
            state.add_workflow(workflow);
        }

        // Should keep only last 100
        assert_eq!(state.executed_workflows.len(), 100);

        // Oldest should be removed
        assert_eq!(state.executed_workflows[0].id, "wf5");
        assert_eq!(state.executed_workflows[99].id, "wf104");
    }

    #[test]
    fn test_clear_workflows() {
        use crate::data::WorkflowStatus;

        let mut state = AppState::new();

        let workflow = Workflow {
            id: "wf1".to_string(),
            name: "Test".to_string(),
            status: WorkflowStatus::Succeeded,
            created_by: "test".to_string(),
            created_at: std::time::SystemTime::now(),
            completed_at: None,
        };

        state.add_workflow(workflow);
        assert_eq!(state.executed_workflows.len(), 1);

        state.clear_workflows();
        assert_eq!(state.executed_workflows.len(), 0);
    }

    // Command execution tests (Phase 4, Task 2)
    #[test]
    fn test_execute_command_creates_task() {
        let state = AppState::new();
        let msg = Message::Operations(OperationsMessage::ExecuteCommand("echo test".to_string()));
        let (new_state, effects) = update(state, msg);

        // Should create async task
        assert_eq!(new_state.async_tasks.len(), 1);

        // Task ID should start with cmd_
        let task_id = new_state.async_tasks.keys().next().unwrap();
        assert!(task_id.starts_with("cmd_"));

        // Should generate RunCommand side effect
        assert!(!effects.is_empty());
        assert!(matches!(&effects[0], SideEffect::RunCommand { .. }));

        // Should add notification
        assert!(!new_state.notifications.is_empty());
        let notif = &new_state.notifications[0];
        assert!(notif.message.contains("Executing"));
    }

    #[test]
    fn test_execute_command_initializes_pane_state() {
        let state = AppState::new();
        let cmd = "gat-cli datasets list".to_string();
        let msg = Message::Operations(OperationsMessage::ExecuteCommand(cmd.clone()));
        let (new_state, _effects) = update(state, msg);

        // Verify pane state was initialized
        let pane_state = new_state
            .pane_states
            .get(&PaneId::Operations)
            .expect("Operations pane state should exist");

        // Should store executing_command
        assert_eq!(pane_state.form_values.get("executing_command"), Some(&cmd));

        // Should initialize empty command_output
        assert_eq!(
            pane_state.form_values.get("command_output"),
            Some(&String::new())
        );
    }

    #[test]
    fn test_command_output_appends_to_buffer() {
        let mut state = AppState::new();

        // Initialize pane state with some output
        let pane_state = state
            .pane_states
            .entry(PaneId::Operations)
            .or_insert_with(PaneState::default);
        pane_state
            .form_values
            .insert("command_output".to_string(), "line 1\nline 2".to_string());

        // Send CommandOutput message
        let msg = Message::Operations(OperationsMessage::CommandOutput("line 3".to_string()));
        let (new_state, _effects) = update(state, msg);

        // Verify output was appended with newline
        let pane_state = new_state.pane_states.get(&PaneId::Operations).unwrap();
        let output = pane_state.form_values.get("command_output").unwrap();
        assert_eq!(output, "line 1\nline 2\nline 3");
    }

    #[test]
    fn test_command_output_first_line() {
        let state = AppState::new();

        // Send CommandOutput to empty buffer
        let msg = Message::Operations(OperationsMessage::CommandOutput("first line".to_string()));
        let (new_state, _effects) = update(state, msg);

        // Verify first output doesn't add extra newline
        let pane_state = new_state.pane_states.get(&PaneId::Operations).unwrap();
        let output = pane_state.form_values.get("command_output").unwrap();
        assert_eq!(output, "first line");
    }

    #[test]
    fn test_command_completed_success() {
        use crate::data::WorkflowStatus;

        let state = AppState::new();
        let result = CommandResult {
            command: "echo success".to_string(),
            exit_code: 0,
            stdout: "success output".to_string(),
            stderr: String::new(),
            duration_ms: 150,
            timed_out: false,
        };

        let msg = Message::Operations(OperationsMessage::CommandCompleted(Ok(result)));
        let (new_state, _effects) = update(state, msg);

        // Should create workflow record
        assert_eq!(new_state.executed_workflows.len(), 1);
        let workflow = &new_state.executed_workflows[0];
        assert_eq!(
            workflow.status,
            WorkflowStatus::Succeeded,
            "Successful command should have Succeeded status"
        );
        assert!(workflow.name.contains("echo success"));

        // Should store exit code and duration in pane state
        let pane_state = new_state.pane_states.get(&PaneId::Operations).unwrap();
        assert_eq!(
            pane_state.form_values.get("last_exit_code"),
            Some(&"0".to_string())
        );
        assert_eq!(
            pane_state.form_values.get("last_duration"),
            Some(&"150ms".to_string())
        );

        // Should add success notification
        let notif = new_state.notifications.last().unwrap();
        assert!(matches!(notif.kind, NotificationKind::Success));
        assert!(notif.message.contains("succeeded"));

        // Should clear executing_command flag
        assert!(!pane_state.form_values.contains_key("executing_command"));
    }

    #[test]
    fn test_command_completed_failure() {
        use crate::data::WorkflowStatus;

        let state = AppState::new();
        let result = CommandResult {
            command: "false".to_string(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "error message".to_string(),
            duration_ms: 50,
            timed_out: false,
        };

        let msg = Message::Operations(OperationsMessage::CommandCompleted(Ok(result)));
        let (new_state, _effects) = update(state, msg);

        // Should create workflow record with Failed status
        let workflow = &new_state.executed_workflows[0];
        assert_eq!(
            workflow.status,
            WorkflowStatus::Failed,
            "Failed command should have Failed status"
        );

        // Should add warning notification (non-zero exit code)
        let notif = new_state.notifications.last().unwrap();
        assert!(matches!(notif.kind, NotificationKind::Warning));
        assert!(notif.message.contains("failed"));
        assert!(notif.message.contains("exit code: 1"));
    }

    #[test]
    fn test_command_completed_error() {
        let state = AppState::new();
        let error_msg = "Command execution failed: timeout".to_string();

        let msg = Message::Operations(OperationsMessage::CommandCompleted(Err(error_msg.clone())));
        let (new_state, _effects) = update(state, msg);

        // Should not create workflow (error case)
        assert_eq!(new_state.executed_workflows.len(), 0);

        // Should add error notification
        let notif = new_state.notifications.last().unwrap();
        assert!(matches!(notif.kind, NotificationKind::Error));
        assert!(notif.message.contains("failed"));

        // Should store error in command_output
        let pane_state = new_state.pane_states.get(&PaneId::Operations).unwrap();
        let output = pane_state.form_values.get("command_output").unwrap();
        assert!(output.contains(&error_msg));
    }

    #[test]
    fn test_cancel_command_removes_tasks() {
        let mut state = AppState::new();

        // Create a running command task
        let task_id = "cmd_1234567890".to_string();
        state
            .async_tasks
            .insert(task_id.clone(), AsyncTaskState::Running);

        // Mark command as executing in pane state
        let pane_state = state
            .pane_states
            .entry(PaneId::Operations)
            .or_insert_with(PaneState::default);
        pane_state
            .form_values
            .insert("executing_command".to_string(), "echo test".to_string());

        // Send CancelCommand
        let msg = Message::Operations(OperationsMessage::CancelCommand);
        let (new_state, effects) = update(state, msg);

        // Task should be removed
        assert!(!new_state.async_tasks.contains_key(&task_id));

        // Should generate CancelCommand side effect
        assert!(effects
            .iter()
            .any(|e| matches!(e, SideEffect::CancelCommand { .. })));

        // Should clear executing_command flag
        let pane_state = new_state.pane_states.get(&PaneId::Operations).unwrap();
        assert!(!pane_state.form_values.contains_key("executing_command"));

        // Should add notification
        let notif = new_state.notifications.last().unwrap();
        assert!(notif.message.contains("cancelled"));
    }

    #[test]
    fn test_cancel_command_idempotent() {
        let state = AppState::new();

        // No executing command, send CancelCommand
        let msg = Message::Operations(OperationsMessage::CancelCommand);
        let (new_state, effects) = update(state, msg);

        // Should not create effects or notifications
        assert!(effects.is_empty());
        assert!(new_state.notifications.is_empty());
    }

    #[test]
    fn test_cancel_command_multiple_tasks() {
        let mut state = AppState::new();

        // Create multiple command tasks
        for i in 0..3 {
            let task_id = format!("cmd_{}", i);
            state.async_tasks.insert(task_id, AsyncTaskState::Running);
        }

        // Mark one command as executing
        let pane_state = state
            .pane_states
            .entry(PaneId::Operations)
            .or_insert_with(PaneState::default);
        pane_state
            .form_values
            .insert("executing_command".to_string(), "test".to_string());

        // Send CancelCommand
        let msg = Message::Operations(OperationsMessage::CancelCommand);
        let (new_state, effects) = update(state, msg);

        // All cmd_* tasks should be removed
        for i in 0..3 {
            let task_id = format!("cmd_{}", i);
            assert!(!new_state.async_tasks.contains_key(&task_id));
        }

        // Should have 3 CancelCommand effects
        let cancel_effects: Vec<_> = effects
            .iter()
            .filter(|e| matches!(e, SideEffect::CancelCommand { .. }))
            .collect();
        assert_eq!(cancel_effects.len(), 3);
    }

    #[test]
    fn test_command_workflow_records_execution_time() {
        let state = AppState::new();
        let result = CommandResult {
            command: "time consuming".to_string(),
            exit_code: 0,
            stdout: "done".to_string(),
            stderr: String::new(),
            duration_ms: 5000,
            timed_out: false,
        };

        let msg = Message::Operations(OperationsMessage::CommandCompleted(Ok(result)));
        let (new_state, _effects) = update(state, msg);

        // Verify duration is stored
        let pane_state = new_state.pane_states.get(&PaneId::Operations).unwrap();
        assert_eq!(
            pane_state.form_values.get("last_duration"),
            Some(&"5000ms".to_string())
        );

        // Verify workflow has completion time
        let workflow = &new_state.executed_workflows[0];
        assert!(workflow.completed_at.is_some());
    }
}
