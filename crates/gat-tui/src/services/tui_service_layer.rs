/// TUI Service Layer - Unified interface for all data operations
///
/// This service layer provides a single point of access for all data operations,
/// abstracting away complexity of multiple service types and providing clean APIs
/// for panes to use. It coordinates between CommandService, GatService, and
/// external data sources.

use crate::data::{DatasetEntry, Workflow, SystemMetrics};
use crate::services::{QueryBuilder, QueryError, CommandValidator, ValidCommand, ValidationError};
use std::sync::Arc;

/// Enumeration of available analytics types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsType {
    Reliability,
    DeliverabilityScore,
    ELCC,
    PowerFlow,
}

impl AnalyticsType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reliability => "reliability",
            Self::DeliverabilityScore => "deliverability_score",
            Self::ELCC => "elcc",
            Self::PowerFlow => "power_flow",
        }
    }
}

/// Unified TUI service layer coordinating all data access
pub struct TuiServiceLayer {
    query_builder: Arc<dyn QueryBuilder>,
    command_validator: CommandValidator,
}

impl TuiServiceLayer {
    /// Create new TUI service layer
    pub fn new(query_builder: Arc<dyn QueryBuilder>) -> Self {
        Self {
            query_builder,
            command_validator: CommandValidator::new(),
        }
    }

    // ============================================================================
    // Dataset Operations
    // ============================================================================

    /// Get all available datasets from catalog
    pub async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError> {
        self.query_builder.get_datasets().await
    }

    /// Get a specific dataset by ID
    pub async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError> {
        self.query_builder.get_dataset(id).await
    }

    /// Search datasets by name or metadata
    pub fn search_datasets(&self, datasets: &[DatasetEntry], query: &str) -> Vec<DatasetEntry> {
        datasets
            .iter()
            .filter(|d| {
                d.id.contains(query)
                    || d.name.to_lowercase().contains(&query.to_lowercase())
                    || d.description.to_lowercase().contains(&query.to_lowercase())
            })
            .cloned()
            .collect()
    }

    // ============================================================================
    // Workflow Operations
    // ============================================================================

    /// Get all executed workflows
    pub async fn get_workflows(&self) -> Result<Vec<Workflow>, QueryError> {
        self.query_builder.get_workflows().await
    }

    /// Filter workflows by status
    pub fn filter_workflows_by_status(
        &self,
        workflows: &[Workflow],
        status: crate::data::WorkflowStatus,
    ) -> Vec<Workflow> {
        workflows
            .iter()
            .filter(|w| w.status == status)
            .cloned()
            .collect()
    }

    // ============================================================================
    // Metrics Operations
    // ============================================================================

    /// Get system metrics (reliability, etc.)
    pub async fn get_metrics(&self) -> Result<SystemMetrics, QueryError> {
        self.query_builder.get_metrics().await
    }

    /// Calculate reliability metrics from workflow history
    pub fn calculate_reliability_metrics(&self, workflows: &[Workflow]) -> SystemMetrics {
        let total = workflows.len() as f64;
        if total == 0.0 {
            return SystemMetrics {
                deliverability_score: 0.0,
                lole_hours_per_year: 0.0,
                eue_mwh_per_year: 0.0,
            };
        }

        let successful = workflows
            .iter()
            .filter(|w| w.status == crate::data::WorkflowStatus::Succeeded)
            .count() as f64;

        SystemMetrics {
            deliverability_score: (successful / total) * 100.0,
            lole_hours_per_year: (1.0 - successful / total) * 8760.0,
            eue_mwh_per_year: (1.0 - successful / total) * 1000.0,
        }
    }

    // ============================================================================
    // Pipeline Operations
    // ============================================================================

    /// Get pipeline configuration
    pub async fn get_pipeline_config(&self) -> Result<String, QueryError> {
        self.query_builder.get_pipeline_config().await
    }

    /// Parse pipeline configuration JSON
    pub fn parse_pipeline_config(&self, config: &str) -> Result<serde_json::Value, QueryError> {
        serde_json::from_str(config)
            .map_err(|e| QueryError::ParseError(format!("Invalid pipeline config: {}", e)))
    }

    // ============================================================================
    // Command Operations
    // ============================================================================

    /// Get available commands from catalog
    pub async fn get_commands(&self) -> Result<Vec<String>, QueryError> {
        self.query_builder.get_commands().await
    }

    /// Validate a command string
    pub fn validate_command(&self, command: &str) -> Result<ValidCommand, ValidationError> {
        self.command_validator.validate(command)
    }

    /// Get command suggestions for typos
    pub fn suggest_command_fix(&self, invalid_cmd: &str) -> Option<String> {
        self.command_validator.suggest_fix(invalid_cmd)
    }

    // ============================================================================
    // Analytics Operations
    // ============================================================================

    /// Execute an analytics command and return results
    pub fn build_analytics_command(
        &self,
        analytics_type: AnalyticsType,
        dataset_id: &str,
        options: &[(&str, &str)],
    ) -> String {
        let mut cmd = format!("gat-cli analytics {} --dataset-id {}", analytics_type.as_str(), dataset_id);
        for (key, value) in options {
            cmd.push_str(&format!(" --{} {}", key, value));
        }
        cmd
    }

    /// Build reliability analysis command
    pub fn build_reliability_command(&self, manifest: &str, output: &str) -> String {
        format!(
            "gat-cli analytics reliability --manifest {} --output {}",
            manifest, output
        )
    }

    /// Build deliverability score command
    pub fn build_ds_command(&self, manifest: &str, output: &str) -> String {
        format!(
            "gat-cli analytics ds --manifest {} --output {}",
            manifest, output
        )
    }

    /// Build ELCC analysis command
    pub fn build_elcc_command(&self, dataset: &str, scenarios: usize, output: &str) -> String {
        format!(
            "gat-cli analytics elcc --dataset {} --scenarios {} --output {}",
            dataset, scenarios, output
        )
    }

    // ============================================================================
    // Scenario Operations
    // ============================================================================

    /// Build scenario validation command
    pub fn build_scenario_validate_command(&self, spec_path: &str) -> String {
        format!("gat-cli scenarios validate --spec {}", spec_path)
    }

    /// Build scenario materialization command
    pub fn build_scenario_materialize_command(&self, template: &str, output: &str) -> String {
        format!("gat-cli scenarios materialize --template {} --output {}", template, output)
    }

    /// Build scenario expansion command
    pub fn build_scenario_expand_command(&self, template: &str, vars: &[(String, String)], output: &str) -> String {
        let mut cmd = format!("gat-cli scenarios expand --template {} --output {}", template, output);
        for (key, value) in vars {
            cmd.push_str(&format!(" --var {}={}", key, value));
        }
        cmd
    }

    // ============================================================================
    // Batch Operations
    // ============================================================================

    /// Build batch power flow command
    pub fn build_batch_pf_command(&self, manifest: &str, max_jobs: usize) -> String {
        format!("gat-cli batch pf --manifest {} --max-jobs {}", manifest, max_jobs)
    }

    /// Build batch OPF command
    pub fn build_batch_opf_command(&self, manifest: &str, max_jobs: usize, solver: &str) -> String {
        format!(
            "gat-cli batch opf --manifest {} --max-jobs {} --solver {}",
            manifest, max_jobs, solver
        )
    }

    // ============================================================================
    // Geographic/Spatial Operations
    // ============================================================================

    /// Build geographic join command
    pub fn build_geo_join_command(&self, left_file: &str, right_file: &str, output: &str) -> String {
        format!(
            "gat-cli geo join --left {} --right {} --output {}",
            left_file, right_file, output
        )
    }

    /// Build geographic query command
    pub fn build_geo_query_command(&self, file: &str, bounds: (f64, f64, f64, f64), output: &str) -> String {
        let (min_lat, min_lon, max_lat, max_lon) = bounds;
        format!(
            "gat-cli geo query --file {} --bounds {},{},{},{} --output {}",
            file, min_lat, min_lon, max_lat, max_lon, output
        )
    }

    // ============================================================================
    // Allocation Operations
    // ============================================================================

    /// Build allocation analysis command
    pub fn build_allocation_command(&self, manifest: &str, scope: &str, output: &str) -> String {
        format!(
            "gat-cli allocation analyze --manifest {} --scope {} --output {}",
            manifest, scope, output
        )
    }

    /// Build rents decomposition command
    pub fn build_rents_command(&self, results_file: &str, output: &str) -> String {
        format!(
            "gat-cli allocation rents --results {} --output {}",
            results_file, output
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::MockQueryBuilder;

    #[tokio::test]
    async fn test_service_layer_creation() {
        let qb = Arc::new(MockQueryBuilder);
        let _service = TuiServiceLayer::new(qb);
        // Verify service layer can be created without panicking
        assert!(true);
    }

    #[tokio::test]
    async fn test_get_datasets() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let datasets = service.get_datasets().await.unwrap();
        assert_eq!(datasets.len(), 3);
    }

    #[tokio::test]
    async fn test_get_dataset() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let dataset = service.get_dataset("opsd-2024").await.unwrap();
        assert_eq!(dataset.name, "OPSD Snapshot");
    }

    #[test]
    fn test_search_datasets() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let datasets = crate::create_fixture_datasets();

        let results = service.search_datasets(&datasets, "OPSD");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "opsd-2024");
    }

    #[test]
    fn test_search_datasets_no_match() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let datasets = crate::create_fixture_datasets();

        let results = service.search_datasets(&datasets, "nonexistent");
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let metrics = service.get_metrics().await.unwrap();
        assert!(metrics.deliverability_score > 0.0);
    }

    #[test]
    fn test_validate_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let result = service.validate_command("gat-cli datasets list");
        assert!(result.is_ok());
    }

    #[test]
    fn test_suggest_command_fix() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let suggestion = service.suggest_command_fix("gat-cli datsets list");
        assert!(suggestion.is_some());
    }

    #[test]
    fn test_build_reliability_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_reliability_command("manifest.json", "out.json");
        assert!(cmd.contains("analytics reliability"));
        assert!(cmd.contains("manifest.json"));
    }

    #[test]
    fn test_build_ds_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_ds_command("manifest.json", "out.json");
        assert!(cmd.contains("analytics ds"));
    }

    #[test]
    fn test_build_elcc_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_elcc_command("dataset.arrow", 100, "out.json");
        assert!(cmd.contains("analytics elcc"));
        assert!(cmd.contains("100"));
    }

    #[test]
    fn test_build_batch_pf_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_batch_pf_command("manifest.json", 10);
        assert!(cmd.contains("batch pf"));
        assert!(cmd.contains("10"));
    }

    #[test]
    fn test_build_batch_opf_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_batch_opf_command("manifest.json", 10, "IPOPT");
        assert!(cmd.contains("batch opf"));
        assert!(cmd.contains("IPOPT"));
    }

    #[test]
    fn test_build_geo_join_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_geo_join_command("left.geo", "right.geo", "out.geo");
        assert!(cmd.contains("geo join"));
    }

    #[test]
    fn test_build_geo_query_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let bounds = (40.0, -120.0, 42.0, -118.0);
        let cmd = service.build_geo_query_command("map.geo", bounds, "out.geo");
        assert!(cmd.contains("geo query"));
        assert!(cmd.contains("40,"));
    }

    #[test]
    fn test_build_scenario_validate_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_scenario_validate_command("spec.yaml");
        assert!(cmd.contains("scenarios validate"));
    }

    #[test]
    fn test_build_scenario_materialize_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_scenario_materialize_command("template.yaml", "out/");
        assert!(cmd.contains("scenarios materialize"));
    }

    #[test]
    fn test_build_scenario_expand_with_vars() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let vars = vec![
            ("YEAR".to_string(), "2024".to_string()),
            ("REGION".to_string(), "CA".to_string()),
        ];
        let cmd = service.build_scenario_expand_command("template.yaml", &vars, "out/");
        assert!(cmd.contains("scenarios expand"));
        assert!(cmd.contains("YEAR=2024"));
        assert!(cmd.contains("REGION=CA"));
    }

    #[test]
    fn test_build_allocation_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_allocation_command("manifest.json", "nodal", "out.json");
        assert!(cmd.contains("allocation analyze"));
    }

    #[test]
    fn test_build_rents_command() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let cmd = service.build_rents_command("results.json", "out.json");
        assert!(cmd.contains("allocation rents"));
    }

    #[test]
    fn test_build_analytics_command_with_options() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);
        let options = vec![("metric", "lole"), ("year", "2024")];
        let cmd = service.build_analytics_command(AnalyticsType::Reliability, "dataset1", &options);
        assert!(cmd.contains("analytics reliability"));
        assert!(cmd.contains("dataset1"));
        assert!(cmd.contains("--metric lole"));
        assert!(cmd.contains("--year 2024"));
    }

    #[tokio::test]
    async fn test_calculate_reliability_metrics() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let workflows = vec![
            crate::data::Workflow {
                id: "w1".to_string(),
                name: "Success".to_string(),
                status: crate::data::WorkflowStatus::Succeeded,
                created_by: "user".to_string(),
                created_at: std::time::SystemTime::now(),
                completed_at: Some(std::time::SystemTime::now()),
            },
            crate::data::Workflow {
                id: "w2".to_string(),
                name: "Failed".to_string(),
                status: crate::data::WorkflowStatus::Failed,
                created_by: "user".to_string(),
                created_at: std::time::SystemTime::now(),
                completed_at: Some(std::time::SystemTime::now()),
            },
        ];

        let metrics = service.calculate_reliability_metrics(&workflows);
        assert_eq!(metrics.deliverability_score, 50.0);
    }

    #[tokio::test]
    async fn test_parse_pipeline_config() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let config = r#"{"name": "test", "stages": []}"#;
        let result = service.parse_pipeline_config(config);
        assert!(result.is_ok());
    }
}
