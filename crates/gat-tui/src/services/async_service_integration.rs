/// Integration of Async Event Handling with TUI Service Layer
///
/// This module connects the AsyncEvent dispatcher with the TuiServiceLayer,
/// allowing background services to fetch data and dispatch results back to
/// the UI through a clean, decoupled interface.
use crate::services::{AsyncEvent, EventResult, TuiServiceLayer};
use std::sync::Arc;

/// Service that integrates TuiServiceLayer with async event handling
pub struct AsyncServiceIntegration {
    service_layer: Arc<TuiServiceLayer>,
}

impl AsyncServiceIntegration {
    /// Create new async integration with a service layer
    pub fn new(service_layer: Arc<TuiServiceLayer>) -> Self {
        Self { service_layer }
    }

    /// Handle an async event and return the result
    pub async fn handle_event(&self, event: &AsyncEvent) -> EventResult {
        match event {
            AsyncEvent::FetchDatasets => self.fetch_datasets().await,
            AsyncEvent::FetchDataset(id) => self.fetch_dataset(id).await,
            AsyncEvent::FetchWorkflows => self.fetch_workflows().await,
            AsyncEvent::FetchMetrics => self.fetch_metrics().await,
            AsyncEvent::FetchPipelineConfig => self.fetch_pipeline_config().await,
            AsyncEvent::FetchCommands => self.fetch_commands().await,

            AsyncEvent::RunAnalytics(analytics_type, options) => {
                self.run_analytics(analytics_type, options).await
            }

            AsyncEvent::RunScenarioValidation(spec) => self.validate_scenario(spec).await,

            AsyncEvent::RunScenarioMaterialize(template, output) => {
                self.materialize_scenario(template, output).await
            }

            AsyncEvent::RunBatchPowerFlow(manifest, max_jobs) => {
                self.batch_power_flow(manifest, *max_jobs).await
            }

            AsyncEvent::RunBatchOPF(manifest, max_jobs, solver) => {
                self.batch_opf(manifest, *max_jobs, solver).await
            }

            AsyncEvent::RunGeoJoin(left, right, output) => self.geo_join(left, right, output).await,

            AsyncEvent::ExecuteCommand(cmd) => self.execute_command(cmd).await,

            AsyncEvent::Shutdown => EventResult::Success("Shutting down".to_string()),
        }
    }

    // ============================================================================
    // Dataset Operations
    // ============================================================================

    async fn fetch_datasets(&self) -> EventResult {
        match self.service_layer.get_datasets().await {
            Ok(datasets) => EventResult::Success(format!("Fetched {} datasets", datasets.len())),
            Err(e) => EventResult::Error(format!("Failed to fetch datasets: {}", e)),
        }
    }

    async fn fetch_dataset(&self, id: &str) -> EventResult {
        match self.service_layer.get_dataset(id).await {
            Ok(dataset) => EventResult::Success(format!("Fetched dataset: {}", dataset.name)),
            Err(e) => EventResult::Error(format!("Failed to fetch dataset: {}", e)),
        }
    }

    // ============================================================================
    // Workflow Operations
    // ============================================================================

    async fn fetch_workflows(&self) -> EventResult {
        match self.service_layer.get_workflows().await {
            Ok(workflows) => EventResult::Success(format!("Fetched {} workflows", workflows.len())),
            Err(e) => EventResult::Error(format!("Failed to fetch workflows: {}", e)),
        }
    }

    // ============================================================================
    // Metrics Operations
    // ============================================================================

    async fn fetch_metrics(&self) -> EventResult {
        match self.service_layer.get_metrics().await {
            Ok(metrics) => EventResult::Success(format!(
                "Fetched metrics: DS={:.1}%, LOLE={:.1} h/yr, EUE={:.1} MWh/yr",
                metrics.deliverability_score, metrics.lole_hours_per_year, metrics.eue_mwh_per_year
            )),
            Err(e) => EventResult::Error(format!("Failed to fetch metrics: {}", e)),
        }
    }

    // ============================================================================
    // Pipeline Operations
    // ============================================================================

    async fn fetch_pipeline_config(&self) -> EventResult {
        match self.service_layer.get_pipeline_config().await {
            Ok(config) => match self.service_layer.parse_pipeline_config(&config) {
                Ok(_) => EventResult::Success("Pipeline config fetched and validated".to_string()),
                Err(e) => EventResult::Error(format!("Failed to parse config: {}", e)),
            },
            Err(e) => EventResult::Error(format!("Failed to fetch pipeline config: {}", e)),
        }
    }

    // ============================================================================
    // Command Operations
    // ============================================================================

    async fn fetch_commands(&self) -> EventResult {
        match self.service_layer.get_commands().await {
            Ok(commands) => {
                EventResult::Success(format!("Fetched {} available commands", commands.len()))
            }
            Err(e) => EventResult::Error(format!("Failed to fetch commands: {}", e)),
        }
    }

    async fn execute_command(&self, cmd: &str) -> EventResult {
        match self.service_layer.validate_command(cmd) {
            Ok(validated) => EventResult::Success(format!(
                "Command validated: {:?}",
                validated.subcommand.unwrap_or_default()
            )),
            Err(e) => {
                let suggestion = self.service_layer.suggest_command_fix(cmd);
                if let Some(suggestion) = suggestion {
                    EventResult::Error(format!("Invalid command. {}", suggestion))
                } else {
                    EventResult::Error(format!("Invalid command: {}", e))
                }
            }
        }
    }

    // ============================================================================
    // Analytics Operations
    // ============================================================================

    async fn run_analytics(
        &self,
        analytics_type: &str,
        options: &[(String, String)],
    ) -> EventResult {
        let cmd = self.service_layer.build_analytics_command(
            match analytics_type {
                "reliability" => crate::services::AnalyticsType::Reliability,
                "ds" => crate::services::AnalyticsType::DeliverabilityScore,
                "elcc" => crate::services::AnalyticsType::ELCC,
                "powerflow" => crate::services::AnalyticsType::PowerFlow,
                _ => {
                    return EventResult::Error(format!(
                        "Unknown analytics type: {}",
                        analytics_type
                    ))
                }
            },
            "dataset1",
            &options
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<Vec<_>>(),
        );
        EventResult::Success(format!("Analytics command: {}", cmd))
    }

    // ============================================================================
    // Scenario Operations
    // ============================================================================

    async fn validate_scenario(&self, spec_path: &str) -> EventResult {
        let cmd = self
            .service_layer
            .build_scenario_validate_command(spec_path);
        EventResult::Success(format!("Scenario validation: {}", cmd))
    }

    async fn materialize_scenario(&self, template: &str, output: &str) -> EventResult {
        let cmd = self
            .service_layer
            .build_scenario_materialize_command(template, output);
        EventResult::Success(format!("Scenario materialization: {}", cmd))
    }

    // ============================================================================
    // Batch Operations
    // ============================================================================

    async fn batch_power_flow(&self, manifest: &str, max_jobs: usize) -> EventResult {
        let cmd = self
            .service_layer
            .build_batch_pf_command(manifest, max_jobs);
        EventResult::Success(format!("Batch power flow: {}", cmd))
    }

    async fn batch_opf(&self, manifest: &str, max_jobs: usize, solver: &str) -> EventResult {
        let cmd = self
            .service_layer
            .build_batch_opf_command(manifest, max_jobs, solver);
        EventResult::Success(format!("Batch OPF: {}", cmd))
    }

    // ============================================================================
    // Geographic Operations
    // ============================================================================

    async fn geo_join(&self, left: &str, right: &str, output: &str) -> EventResult {
        let cmd = self
            .service_layer
            .build_geo_join_command(left, right, output);
        EventResult::Success(format!("Geo join: {}", cmd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::MockQueryBuilder;

    #[tokio::test]
    async fn test_async_integration_creation() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let _integration = AsyncServiceIntegration::new(service);
        // Just verify creation without panicking
    }

    #[tokio::test]
    async fn test_handle_fetch_datasets() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration.handle_event(&AsyncEvent::FetchDatasets).await;
        match result {
            EventResult::Success(msg) => assert!(msg.contains("Fetched")),
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_handle_fetch_workflows() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration.handle_event(&AsyncEvent::FetchWorkflows).await;
        match result {
            EventResult::Success(msg) => assert!(msg.contains("workflows")),
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_handle_fetch_metrics() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration.handle_event(&AsyncEvent::FetchMetrics).await;
        match result {
            EventResult::Success(msg) => assert!(msg.contains("DS=")),
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_handle_fetch_pipeline_config() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration
            .handle_event(&AsyncEvent::FetchPipelineConfig)
            .await;
        assert!(matches!(result, EventResult::Success(_)));
    }

    #[tokio::test]
    async fn test_handle_fetch_commands() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration.handle_event(&AsyncEvent::FetchCommands).await;
        match result {
            EventResult::Success(msg) => assert!(msg.contains("commands")),
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_handle_execute_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration
            .handle_event(&AsyncEvent::ExecuteCommand(
                "gat-cli datasets list".to_string(),
            ))
            .await;
        assert!(matches!(result, EventResult::Success(_)));
    }

    #[tokio::test]
    async fn test_handle_invalid_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration
            .handle_event(&AsyncEvent::ExecuteCommand("invalid_command".to_string()))
            .await;
        match result {
            EventResult::Error(msg) => assert!(msg.contains("Invalid")),
            _ => panic!("Expected error"),
        }
    }

    #[tokio::test]
    async fn test_handle_batch_power_flow() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration
            .handle_event(&AsyncEvent::RunBatchPowerFlow(
                "manifest.json".to_string(),
                10,
            ))
            .await;
        match result {
            EventResult::Success(msg) => assert!(msg.contains("Batch power flow")),
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_handle_geo_join() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration
            .handle_event(&AsyncEvent::RunGeoJoin(
                "left.geo".to_string(),
                "right.geo".to_string(),
                "out.geo".to_string(),
            ))
            .await;
        match result {
            EventResult::Success(msg) => assert!(msg.contains("geo join")),
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_handle_shutdown() {
        let qb = Arc::new(MockQueryBuilder);
        let service = Arc::new(TuiServiceLayer::new(qb));
        let integration = AsyncServiceIntegration::new(service);

        let result = integration.handle_event(&AsyncEvent::Shutdown).await;
        assert!(matches!(result, EventResult::Success(_)));
    }
}
