use crate::message::{CommandsMessage, DatasetsMessage, OperationsMessage, PipelineMessage};
/// Pane integration with GAT services
///
/// This module handles message routing from panes to appropriate GAT services,
/// executes commands, and updates state with results.
use crate::models::AppState;
use crate::services::{DatasetsService, GatService, OperationsService, PipelineService};

/// Integration coordinator for handling pane messages with GAT services
pub struct PaneIntegrator {
    gat_service: GatService,
    datasets_service: DatasetsService,
    pipeline_service: PipelineService,
    operations_service: OperationsService,
}

impl PaneIntegrator {
    pub fn new(cli_path: String, timeout: u64) -> Self {
        let gat = GatService::new(cli_path, timeout);
        Self {
            datasets_service: DatasetsService::new(gat.clone()),
            pipeline_service: PipelineService::new(gat.clone()),
            operations_service: OperationsService::new(gat.clone()),
            gat_service: gat,
        }
    }

    /// Handle Datasets pane messages
    pub fn handle_datasets_message(
        &self,
        state: &mut AppState,
        msg: DatasetsMessage,
    ) -> Option<String> {
        match msg {
            DatasetsMessage::RefreshList => {
                // Generate command to list datasets
                let cmd = self.gat_service.list_datasets(50);
                Some(cmd)
            }
            DatasetsMessage::UploadDataset(file_path) => {
                // Generate upload command
                let cmd = self.gat_service.upload_dataset(&file_path);
                Some(cmd)
            }
            DatasetsMessage::SearchDatasets(query) => {
                // Filter datasets by search query
                let cmd = self.datasets_service.search_datasets(&query);
                Some(cmd)
            }
            DatasetsMessage::DeleteDataset(_idx) => {
                // Show confirmation dialog
                state.show_confirmation(
                    "Are you sure you want to delete this dataset?".to_string(),
                    "Delete".to_string(),
                    "Cancel".to_string(),
                );
                None
            }
            DatasetsMessage::SelectDataset(_idx) => None, // Local selection only
            DatasetsMessage::FetchDatasets => {
                // Fetch datasets from service - handled in update.rs
                None
            }
            DatasetsMessage::DatasetsLoaded(_result) => {
                // Results handled in update.rs
                None
            }

            // Grid management (Phase 3) - handled in update.rs
            DatasetsMessage::LoadGrid(_) => None,
            DatasetsMessage::UnloadGrid(_) => None,
            DatasetsMessage::SwitchGrid(_) => None,
            DatasetsMessage::RefreshGrids => None,
            DatasetsMessage::GridLoaded(_) => None,
            DatasetsMessage::GridLoadFailed(_) => None,
        }
    }

    /// Handle Pipeline pane messages
    pub fn handle_pipeline_message(
        &self,
        state: &mut AppState,
        msg: PipelineMessage,
    ) -> Option<String> {
        match msg {
            PipelineMessage::RunPipeline => {
                // Validate pipeline first
                if let Some(pane_state) = state.pane_states.get(&crate::models::PaneId::Pipeline) {
                    let config = pane_state.form_values.clone();
                    let cmd = self.pipeline_service.validate_pipeline(&config);
                    Some(cmd)
                } else {
                    None
                }
            }
            PipelineMessage::AddTransform(_) => {
                // Local UI update only
                None
            }
            PipelineMessage::RemoveTransform(_) => None,
            PipelineMessage::SelectNode(_) => None,
            PipelineMessage::UpdateConfig(_) => None,
            PipelineMessage::FetchPipeline | PipelineMessage::PipelineLoaded(_) => {
                // Handled in update.rs
                None
            }
        }
    }

    /// Handle Operations pane messages
    pub fn handle_operations_message(
        &self,
        state: &mut AppState,
        msg: OperationsMessage,
    ) -> Option<String> {
        match msg {
            OperationsMessage::Execute => {
                // Determine which operation based on active tab
                let pane_state = state.pane_states.get(&crate::models::PaneId::Operations);
                match pane_state.and_then(|ps| Some(ps.selected_tab)) {
                    Some(0) => {
                        // Batch operations
                        let cmd = self.operations_service.list_batch_jobs(20);
                        Some(cmd)
                    }
                    Some(1) => {
                        // Allocation operations - show dialog for scenario selection
                        state.show_info(
                            "Allocation Analysis".to_string(),
                            "Select a scenario to analyze allocations".to_string(),
                            None,
                        );
                        None
                    }
                    Some(2) => {
                        // Reliability operations
                        let cmd = self.gat_service.analytics_reliability(
                            "manifest.json",
                            "flows.parquet",
                            "reliability.json",
                        );
                        Some(cmd)
                    }
                    _ => None,
                }
            }
            OperationsMessage::ConfigChange(key, value) => {
                // Store config value in pane state
                let pane_state = state
                    .pane_states
                    .entry(crate::models::PaneId::Operations)
                    .or_insert_with(crate::models::PaneState::default);
                pane_state.form_values.insert(key, value);
                None
            }
            OperationsMessage::CancelRun => {
                // Cancel any running operation
                None
            }
            OperationsMessage::SelectTab(_) => None,
            OperationsMessage::FetchOperations | OperationsMessage::OperationsLoaded(_) => {
                // Handled in update.rs
                None
            }
            // Command execution (Phase 4) - handled in update.rs
            OperationsMessage::ExecuteCommand(_) => None,
            OperationsMessage::CommandOutput(_) => None,
            OperationsMessage::CommandCompleted(_) => None,
            OperationsMessage::CancelCommand => None,
        }
    }

    /// Handle Commands pane messages
    pub fn handle_commands_message(
        &self,
        _state: &mut AppState,
        msg: CommandsMessage,
    ) -> Option<String> {
        match msg {
            CommandsMessage::ExecuteCommand(cmd) => Some(cmd),
            CommandsMessage::SearchCommands(_) => None, // Local filtering
            CommandsMessage::SelectCommand(_) => None,
            CommandsMessage::ClearHistory => None,
            CommandsMessage::CancelExecution => None,
            CommandsMessage::FetchCommands | CommandsMessage::CommandsLoaded(_) => {
                // Handled in update.rs
                None
            }
        }
    }
}

/// Helper for cloning GatService (since it's in a trait object context)
impl Clone for GatService {
    fn clone(&self) -> Self {
        GatService::new(self.cli_path.clone(), self.default_timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrator_creation() {
        let integrator = PaneIntegrator::new("gat-cli".to_string(), 300);
        // Verify the integrator was created successfully
        assert!(!integrator.gat_service.cli_path.is_empty());
    }

    #[test]
    fn test_datasets_refresh() {
        let integrator = PaneIntegrator::new("gat-cli".to_string(), 300);
        let mut state = AppState::new();
        let cmd = integrator.handle_datasets_message(&mut state, DatasetsMessage::RefreshList);
        assert!(cmd.is_some());
        assert!(cmd.unwrap().contains("datasets list"));
    }

    #[test]
    fn test_datasets_upload() {
        let integrator = PaneIntegrator::new("gat-cli".to_string(), 300);
        let mut state = AppState::new();
        let cmd = integrator.handle_datasets_message(
            &mut state,
            DatasetsMessage::UploadDataset("data.csv".to_string()),
        );
        assert!(cmd.is_some());
        assert!(cmd.unwrap().contains("dataset upload"));
    }

    #[test]
    fn test_datasets_delete_shows_confirmation() {
        let integrator = PaneIntegrator::new("gat-cli".to_string(), 300);
        let mut state = AppState::new();
        let cmd = integrator.handle_datasets_message(&mut state, DatasetsMessage::DeleteDataset(0));
        assert!(cmd.is_none()); // No command, shows modal instead
        assert!(state.is_modal_open()); // Modal should be open
    }

    #[test]
    fn test_pipeline_run() {
        let integrator = PaneIntegrator::new("gat-cli".to_string(), 300);
        let mut state = AppState::new();
        let cmd = integrator.handle_pipeline_message(&mut state, PipelineMessage::RunPipeline);
        // Should generate a validation command
        assert!(cmd.is_some());
    }

    #[test]
    fn test_operations_execute() {
        let integrator = PaneIntegrator::new("gat-cli".to_string(), 300);
        let mut state = AppState::new();
        let cmd = integrator.handle_operations_message(&mut state, OperationsMessage::Execute);
        // Tab 0 (Batch) is selected by default
        assert!(cmd.is_some());
        assert!(cmd.unwrap().contains("batch list"));
    }

    #[test]
    fn test_operations_config() {
        let integrator = PaneIntegrator::new("gat-cli".to_string(), 300);
        let mut state = AppState::new();
        let cmd = integrator.handle_operations_message(
            &mut state,
            OperationsMessage::ConfigChange("manifest".to_string(), "data.json".to_string()),
        );
        assert!(cmd.is_none()); // Config change is local only
    }

    #[test]
    fn test_commands_execute() {
        let integrator = PaneIntegrator::new("gat-cli".to_string(), 300);
        let mut state = AppState::new();
        let cmd = integrator.handle_commands_message(
            &mut state,
            CommandsMessage::ExecuteCommand("gat-cli --version".to_string()),
        );
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap(), "gat-cli --version");
    }

    #[test]
    fn test_gat_service_clone() {
        let svc1 = GatService::new("gat-cli", 300);
        let svc2 = svc1.clone();
        // Verify clone has same settings
        assert_eq!(svc1.cli_path, svc2.cli_path);
        assert_eq!(svc1.default_timeout, svc2.default_timeout);
    }

    // Phase 4: Full Integration Tests for Command Execution Pipeline

    #[test]
    fn test_command_execution_full_pipeline() {
        use crate::message::{Message, OperationsMessage};
        use crate::update::update;

        let state = AppState::new();
        let msg = Message::Operations(OperationsMessage::ExecuteCommand("echo test".to_string()));
        let (new_state, effects) = update(state, msg);

        // Verify state was updated
        assert!(new_state.command_queue.len() > 0 || !effects.is_empty());
    }

    #[test]
    fn test_command_validation_flow() {
        use crate::services::CommandValidator;

        let validator = CommandValidator::new();

        // Valid command should pass
        let result = validator.validate("gat-cli datasets list");
        assert!(result.is_ok());

        let cmd = result.unwrap();
        assert_eq!(cmd.subcommand, Some("datasets".to_string()));
    }

    #[test]
    fn test_command_export_flow() {
        use crate::models::ExecutedCommand;
        use std::time::UNIX_EPOCH;

        let mut state = AppState::new();

        // Add commands to history
        let cmd1 = ExecutedCommand {
            id: "cmd_1".to_string(),
            command: "echo test1".to_string(),
            exit_code: 0,
            stdout: "test1".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            timed_out: false,
            executed_at: UNIX_EPOCH,
        };

        let cmd2 = ExecutedCommand {
            id: "cmd_2".to_string(),
            command: "echo test2".to_string(),
            exit_code: 0,
            stdout: "test2".to_string(),
            stderr: String::new(),
            duration_ms: 150,
            timed_out: false,
            executed_at: UNIX_EPOCH,
        };

        state.add_command_to_history(cmd1);
        state.add_command_to_history(cmd2);

        // Test JSON export
        let json_result = state.export_commands_json();
        assert!(json_result.is_ok());
        let json = json_result.unwrap();
        assert!(json.contains("echo test1"));
        assert!(json.contains("echo test2"));

        // Test CSV export
        let csv_result = state.export_commands_csv();
        assert!(csv_result.is_ok());
        let csv = csv_result.unwrap();
        assert!(csv.contains("echo test1"));
        assert!(csv.contains("cmd_2"));

        // Test stats calculation
        let stats = state.get_command_stats();
        assert_eq!(stats.total_commands, 2);
        assert_eq!(stats.successful_count, 2);
        assert_eq!(stats.success_rate, 100.0);
    }

    #[test]
    fn test_command_history_lru_cleanup() {
        use crate::models::ExecutedCommand;
        use std::time::UNIX_EPOCH;

        let mut state = AppState::new();

        // Add more than 500 commands to test LRU cleanup
        for i in 0..510 {
            let cmd = ExecutedCommand {
                id: format!("cmd_{}", i),
                command: format!("echo {}", i),
                exit_code: 0,
                stdout: format!("output{}", i),
                stderr: String::new(),
                duration_ms: 100 + i as u64,
                timed_out: false,
                executed_at: UNIX_EPOCH,
            };
            state.add_command_to_history(cmd);
        }

        // Should keep only last 500
        assert_eq!(state.command_history_count(), 500);

        // First command should be removed (cmd_0 through cmd_9 should be gone)
        let history = state.get_command_history();
        assert!(!history.iter().any(|c| c.id == "cmd_0"));
        assert!(!history.iter().any(|c| c.id == "cmd_9"));

        // Commands from cmd_10 onward should be present
        assert!(history.iter().any(|c| c.id == "cmd_10"));

        // Last command should be present (cmd_509)
        assert!(history.iter().any(|c| c.id == "cmd_509"));
    }

    #[test]
    fn test_command_search_and_filter() {
        use crate::models::ExecutedCommand;
        use std::time::UNIX_EPOCH;

        let mut state = AppState::new();

        // Add various commands
        for (i, &(cmd_str, exit_code)) in [
            ("echo success", 0),
            ("false", 1),
            ("echo another", 0),
            ("failing command", 1),
        ]
        .iter()
        .enumerate()
        {
            let cmd = ExecutedCommand {
                id: format!("cmd_{}", i),
                command: cmd_str.to_string(),
                exit_code,
                stdout: "output".to_string(),
                stderr: if exit_code != 0 {
                    "error".to_string()
                } else {
                    String::new()
                },
                duration_ms: 100,
                timed_out: false,
                executed_at: UNIX_EPOCH,
            };
            state.add_command_to_history(cmd);
        }

        // Test search
        let results = state.search_command_history("echo");
        assert_eq!(results.len(), 2);

        // Test successful commands
        let successful = state.get_successful_commands();
        assert_eq!(successful.len(), 2);

        // Test failed commands
        let failed = state.get_failed_commands();
        assert_eq!(failed.len(), 2);
    }

    #[test]
    fn test_command_history_persistence_json_roundtrip() {
        use crate::models::ExecutedCommand;
        use std::time::UNIX_EPOCH;

        let mut state = AppState::new();

        let cmd = ExecutedCommand {
            id: "cmd_test".to_string(),
            command: "echo persist".to_string(),
            exit_code: 0,
            stdout: "persisted".to_string(),
            stderr: String::new(),
            duration_ms: 500,
            timed_out: false,
            executed_at: UNIX_EPOCH,
        };

        state.add_command_to_history(cmd.clone());

        // Export to JSON
        let json = state.export_commands_json().unwrap();

        // Deserialize from JSON
        let deserialized: Vec<ExecutedCommand> = serde_json::from_str(&json).unwrap();

        // Verify the data persisted correctly
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].id, "cmd_test");
        assert_eq!(deserialized[0].command, "echo persist");
        assert_eq!(deserialized[0].exit_code, 0);
    }

    #[test]
    fn test_command_execution_state_lifecycle() {
        use crate::models::{PaneId, PaneState};

        let mut state = AppState::new();

        // Initialize pane state
        let pane_state = state
            .pane_states
            .entry(PaneId::Operations)
            .or_insert_with(PaneState::default);

        // Set command input
        pane_state
            .form_values
            .insert("executing_command".to_string(), "echo test".to_string());
        pane_state
            .form_values
            .insert("command_output".to_string(), String::new());

        // Simulate output accumulation
        pane_state
            .form_values
            .insert("command_output".to_string(), "output line 1\n".to_string());

        // Verify final state
        let pane_state = state.pane_states.get(&PaneId::Operations).unwrap();
        assert_eq!(
            pane_state.form_values.get("executing_command"),
            Some(&"echo test".to_string())
        );
        assert!(pane_state
            .form_values
            .get("command_output")
            .unwrap()
            .contains("output line 1"));
    }

    #[test]
    fn test_integration_command_validator_with_export() {
        use crate::models::ExecutedCommand;
        use crate::services::CommandValidator;
        use std::time::UNIX_EPOCH;

        let validator = CommandValidator::new();
        let mut state = AppState::new();

        // Validate multiple commands
        let commands = vec![
            "gat-cli datasets list",
            "gat-cli opf analysis",
            "gat-cli derms envelope",
        ];

        for cmd in commands {
            let validation = validator.validate(cmd);
            assert!(validation.is_ok());

            // Create executed command record
            let executed = ExecutedCommand {
                id: format!("validated_{}", state.command_history_count()),
                command: cmd.to_string(),
                exit_code: 0,
                stdout: "success".to_string(),
                stderr: String::new(),
                duration_ms: 100,
                timed_out: false,
                executed_at: UNIX_EPOCH,
            };
            state.add_command_to_history(executed);
        }

        // Verify all commands recorded and exportable
        assert_eq!(state.command_history_count(), 3);
        let export = state.export_commands_csv();
        assert!(export.is_ok());
        assert!(export.unwrap().contains("datasets list"));
    }

    #[test]
    fn test_command_stats_comprehensive() {
        use crate::models::ExecutedCommand;
        use std::time::UNIX_EPOCH;

        let mut state = AppState::new();

        // Create a variety of command outcomes
        let scenarios = vec![
            ("success_1", 0, 100, false),
            ("success_2", 0, 200, false),
            ("failure_1", 1, 150, false),
            ("timeout_1", -1, 5000, true),
            ("success_3", 0, 300, false),
        ];

        for (cmd, exit_code, duration, timed_out) in scenarios {
            let executed = ExecutedCommand {
                id: cmd.to_string(),
                command: format!("cmd {}", cmd),
                exit_code,
                stdout: "output".to_string(),
                stderr: if exit_code != 0 {
                    "error".to_string()
                } else {
                    String::new()
                },
                duration_ms: duration,
                timed_out,
                executed_at: UNIX_EPOCH,
            };
            state.add_command_to_history(executed);
        }

        let stats = state.get_command_stats();
        assert_eq!(stats.total_commands, 5);
        assert_eq!(stats.successful_count, 3);
        assert_eq!(stats.failed_count, 1);
        assert_eq!(stats.timed_out_count, 1);
        assert!((stats.success_rate - 60.0).abs() < 0.1); // 60.0 with floating point tolerance
        assert_eq!(stats.fastest_duration_ms, 100);
        assert_eq!(stats.slowest_duration_ms, 5000);
    }

    #[test]
    fn test_recent_commands_ordering() {
        use crate::models::ExecutedCommand;
        use std::time::UNIX_EPOCH;

        let mut state = AppState::new();

        // Add commands in order
        for i in 0..5 {
            let cmd = ExecutedCommand {
                id: format!("cmd_{}", i),
                command: format!("echo {}", i),
                exit_code: 0,
                stdout: format!("output {}", i),
                stderr: String::new(),
                duration_ms: 100 + i as u64,
                timed_out: false,
                executed_at: UNIX_EPOCH,
            };
            state.add_command_to_history(cmd);
        }

        // Get recent 3 (should be in reverse order)
        let recent = state.get_recent_commands(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, "cmd_4"); // Most recent first
        assert_eq!(recent[1].id, "cmd_3");
        assert_eq!(recent[2].id, "cmd_2");
    }

    #[test]
    fn test_clear_command_history() {
        use crate::models::ExecutedCommand;
        use std::time::UNIX_EPOCH;

        let mut state = AppState::new();

        // Add some commands
        for i in 0..5 {
            let cmd = ExecutedCommand {
                id: format!("cmd_{}", i),
                command: format!("echo {}", i),
                exit_code: 0,
                stdout: "output".to_string(),
                stderr: String::new(),
                duration_ms: 100,
                timed_out: false,
                executed_at: UNIX_EPOCH,
            };
            state.add_command_to_history(cmd);
        }

        assert_eq!(state.command_history_count(), 5);

        // Clear history
        state.clear_command_history();
        assert_eq!(state.command_history_count(), 0);
        assert_eq!(state.command_success_rate(), 0.0);
    }
}
