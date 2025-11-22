/// Pane integration with GAT services
///
/// This module handles message routing from panes to appropriate GAT services,
/// executes commands, and updates state with results.

use crate::models::AppState;
use crate::message::{DatasetsMessage, PipelineMessage, OperationsMessage, CommandsMessage};
use crate::services::{GatService, DatasetsService, PipelineService, OperationsService};

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
    pub fn handle_datasets_message(&self, state: &mut AppState, msg: DatasetsMessage) -> Option<String> {
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
        }
    }

    /// Handle Pipeline pane messages
    pub fn handle_pipeline_message(&self, state: &mut AppState, msg: PipelineMessage) -> Option<String> {
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
    pub fn handle_operations_message(&self, state: &mut AppState, msg: OperationsMessage) -> Option<String> {
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
                        let cmd = self.gat_service.analytics_reliability("manifest.json", "flows.parquet", "reliability.json");
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
        }
    }

    /// Handle Commands pane messages
    pub fn handle_commands_message(&self, _state: &mut AppState, msg: CommandsMessage) -> Option<String> {
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
}
