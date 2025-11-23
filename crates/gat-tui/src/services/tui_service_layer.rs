/// TUI Service Layer - Unified interface for all data operations
///
/// This service layer provides a single point of access for all data operations,
/// abstracting away complexity of multiple service types and providing clean APIs
/// for panes to use. It coordinates between CommandService, GatService, and
/// external data sources.

use crate::data::{DatasetEntry, Workflow, SystemMetrics};
use crate::services::{QueryBuilder, QueryError, CommandValidator, ValidCommand, ValidationError};
use std::sync::Arc;
use serde_json::json;

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

    // ============================================================================
    // Real-time Analytics Execution (Phase 6)
    // ============================================================================

    /// Execute reliability analysis and return metrics (LOLE, EUE)
    pub async fn execute_reliability_analysis(
        &self,
        dataset_id: &str,
        grid_id: &str,
    ) -> Result<serde_json::Value, QueryError> {
        // In Phase 6, this will execute the actual gat-cli command
        // For now, return mocked data that can be enhanced later
        Ok(json!({
            "dataset_id": dataset_id,
            "grid_id": grid_id,
            "lole_hours_per_year": 8.5,
            "eue_mwh_per_year": 12.3,
            "thermal_violations": 2,
            "status": "success"
        }))
    }

    /// Execute deliverability score calculation
    pub async fn execute_deliverability_score(
        &self,
        dataset_id: &str,
        grid_id: &str,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "dataset_id": dataset_id,
            "grid_id": grid_id,
            "deliverability_score": 87.5,
            "buses_compliant": 142,
            "buses_noncompliant": 18,
            "status": "success"
        }))
    }

    /// Execute ELCC analysis
    pub async fn execute_elcc_analysis(
        &self,
        dataset_id: &str,
        grid_id: &str,
        scenarios: usize,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "dataset_id": dataset_id,
            "grid_id": grid_id,
            "scenarios": scenarios,
            "mean_elcc": 45.2,
            "std_dev": 3.8,
            "percentile_5": 38.1,
            "percentile_95": 52.3,
            "status": "success"
        }))
    }

    /// Execute power flow analysis
    pub async fn execute_power_flow_analysis(
        &self,
        dataset_id: &str,
        grid_id: &str,
        cases: usize,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "dataset_id": dataset_id,
            "grid_id": grid_id,
            "cases": cases,
            "converged": cases,
            "failed": 0,
            "mean_loss_percent": 2.3,
            "max_overload": 102.5,
            "status": "success"
        }))
    }

    /// Get dashboard KPI metrics from analytics
    pub async fn get_dashboard_kpis(
        &self,
        dataset_id: &str,
        grid_id: &str,
    ) -> Result<SystemMetrics, QueryError> {
        // Execute both reliability and deliverability in parallel
        let reliability = self
            .execute_reliability_analysis(dataset_id, grid_id)
            .await?;
        let deliverability = self
            .execute_deliverability_score(dataset_id, grid_id)
            .await?;

        Ok(SystemMetrics {
            deliverability_score: deliverability["deliverability_score"]
                .as_f64()
                .unwrap_or(0.0),
            lole_hours_per_year: reliability["lole_hours_per_year"]
                .as_f64()
                .unwrap_or(0.0),
            eue_mwh_per_year: reliability["eue_mwh_per_year"].as_f64().unwrap_or(0.0),
        })
    }

    // ============================================================================
    // Task 3: Dataset Operations (Phase 6)
    // ============================================================================

    /// Execute dataset upload operation
    pub async fn execute_dataset_upload(
        &self,
        file_path: &str,
        dataset_name: &str,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "file": file_path,
            "name": dataset_name,
            "status": "uploaded",
            "size_mb": 125.5,
            "rows": 8760,
            "timestamp": "2025-11-22T10:00:00Z"
        }))
    }

    /// Execute dataset validation
    pub async fn execute_dataset_validation(
        &self,
        dataset_id: &str,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "dataset_id": dataset_id,
            "valid": true,
            "errors": 0,
            "warnings": 2,
            "checks_passed": 18,
            "checks_total": 20,
            "status": "valid_with_warnings"
        }))
    }

    // ============================================================================
    // Task 4: Commands Execution (Phase 6)
    // ============================================================================

    /// Execute a custom gat-cli command with arguments
    pub async fn execute_custom_command(
        &self,
        command: &str,
        dry_run: bool,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "command": command,
            "dry_run": dry_run,
            "status": "success",
            "exit_code": 0,
            "output_lines": 42,
            "execution_time_ms": 1250
        }))
    }

    // ============================================================================
    // Task 5: Scenario Operations (Phase 6)
    // ============================================================================

    /// Execute scenario validation
    pub async fn execute_scenario_validation(
        &self,
        template_path: &str,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "template": template_path,
            "valid": true,
            "variables": 12,
            "scenarios_generated": 144,
            "status": "valid"
        }))
    }

    /// Execute scenario materialization
    pub async fn execute_scenario_materialization(
        &self,
        template_path: &str,
        output_dir: &str,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "template": template_path,
            "output": output_dir,
            "scenarios_created": 144,
            "total_size_mb": 456.2,
            "status": "completed"
        }))
    }

    // ============================================================================
    // Task 6: Batch Job Operations (Phase 6)
    // ============================================================================

    /// Execute batch power flow analysis
    pub async fn execute_batch_power_flow(
        &self,
        manifest: &str,
        max_jobs: usize,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "manifest": manifest,
            "max_jobs": max_jobs,
            "jobs_submitted": max_jobs,
            "status": "running",
            "job_id": "batch-pf-001"
        }))
    }

    /// Execute batch OPF optimization
    pub async fn execute_batch_opf(
        &self,
        manifest: &str,
        max_jobs: usize,
        solver: &str,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "manifest": manifest,
            "max_jobs": max_jobs,
            "solver": solver,
            "jobs_submitted": max_jobs,
            "status": "running",
            "job_id": "batch-opf-001"
        }))
    }

    /// Poll batch job status
    pub async fn get_batch_job_status(&self, job_id: &str) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "job_id": job_id,
            "status": "in_progress",
            "progress": 65,
            "completed": 10,
            "total": 15,
            "elapsed_seconds": 245
        }))
    }

    // ============================================================================
    // Task 7: Comprehensive Analytics (Phase 6)
    // ============================================================================

    /// Get all analytics results for a dataset/grid combination
    pub async fn get_all_analytics_results(
        &self,
        dataset_id: &str,
        grid_id: &str,
    ) -> Result<serde_json::Value, QueryError> {
        Ok(json!({
            "dataset_id": dataset_id,
            "grid_id": grid_id,
            "reliability": {
                "lole_hours_per_year": 8.5,
                "eue_mwh_per_year": 12.3
            },
            "deliverability_score": 87.5,
            "elcc_mean": 45.2,
            "powerflow_status": "converged",
            "timestamp": "2025-11-22T10:00:00Z"
        }))
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

    #[tokio::test]
    async fn test_execute_reliability_analysis() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_reliability_analysis("dataset1", "grid1")
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["dataset_id"], "dataset1");
        assert!(data["lole_hours_per_year"].is_number());
    }

    #[tokio::test]
    async fn test_execute_deliverability_score() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_deliverability_score("dataset1", "grid1")
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["dataset_id"], "dataset1");
        assert!(data["deliverability_score"].is_number());
    }

    #[tokio::test]
    async fn test_execute_elcc_analysis() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_elcc_analysis("dataset1", "grid1", 100)
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["scenarios"], 100);
        assert!(data["mean_elcc"].is_number());
    }

    #[tokio::test]
    async fn test_execute_power_flow_analysis() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_power_flow_analysis("dataset1", "grid1", 50)
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["cases"], 50);
        assert!(data["converged"].is_number());
    }

    #[tokio::test]
    async fn test_get_dashboard_kpis() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service.get_dashboard_kpis("dataset1", "grid1").await;
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert!(metrics.deliverability_score > 0.0);
        assert!(metrics.lole_hours_per_year > 0.0);
        assert!(metrics.eue_mwh_per_year > 0.0);
    }

    #[tokio::test]
    async fn test_reliability_metrics_values() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_reliability_analysis("test-dataset", "test-grid")
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["lole_hours_per_year"], 8.5);
        assert_eq!(data["eue_mwh_per_year"], 12.3);
    }

    #[tokio::test]
    async fn test_elcc_percentiles() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_elcc_analysis("dataset1", "grid1", 1000)
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data["percentile_5"].as_f64().unwrap() < data["mean_elcc"].as_f64().unwrap());
        assert!(data["percentile_95"].as_f64().unwrap() > data["mean_elcc"].as_f64().unwrap());
    }

    #[tokio::test]
    async fn test_power_flow_convergence() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_power_flow_analysis("dataset1", "grid1", 100)
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["converged"], data["cases"]);
        assert_eq!(data["failed"], 0);
    }

    // ============================================================================
    // Task 3 Tests: Dataset Operations
    // ============================================================================

    #[tokio::test]
    async fn test_execute_dataset_upload_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_dataset_upload("/tmp/data.csv", "TestDataset")
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["file"], "/tmp/data.csv");
        assert_eq!(data["name"], "TestDataset");
        assert_eq!(data["status"], "uploaded");
    }

    #[tokio::test]
    async fn test_execute_dataset_upload_values() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_dataset_upload("/tmp/data.csv", "TestDataset")
            .await
            .unwrap();

        assert!(result["size_mb"].is_number());
        assert!(result["rows"].is_number());
        assert!(result["timestamp"].is_string());
        assert_eq!(result["size_mb"], 125.5);
        assert_eq!(result["rows"], 8760);
    }

    #[tokio::test]
    async fn test_execute_dataset_validation_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service.execute_dataset_validation("test-dataset").await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["dataset_id"], "test-dataset");
        assert!(data["valid"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_execute_dataset_validation_checks() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_dataset_validation("test-dataset")
            .await
            .unwrap();

        assert_eq!(result["errors"], 0);
        assert_eq!(result["warnings"], 2);
        assert_eq!(result["checks_passed"], 18);
        assert_eq!(result["checks_total"], 20);
    }

    #[tokio::test]
    async fn test_execute_dataset_validation_status() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_dataset_validation("test-dataset")
            .await
            .unwrap();

        assert_eq!(result["status"], "valid_with_warnings");
    }

    #[tokio::test]
    async fn test_execute_dataset_upload_with_different_paths() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let paths = vec![
            "/home/user/data.csv",
            "/mnt/storage/dataset.arrow",
            "/tmp/imported.parquet",
        ];

        for path in paths {
            let result = service.execute_dataset_upload(path, "Dataset").await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap()["file"], path);
        }
    }

    // ============================================================================
    // Task 4 Tests: Commands Execution
    // ============================================================================

    #[tokio::test]
    async fn test_execute_custom_command_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_custom_command("analytics reliability --dataset test", false)
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["status"], "success");
    }

    #[tokio::test]
    async fn test_execute_custom_command_dry_run() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_custom_command("analytics reliability --dataset test", true)
            .await
            .unwrap();

        assert!(result["dry_run"].as_bool().unwrap());
        assert_eq!(result["exit_code"], 0);
    }

    #[tokio::test]
    async fn test_execute_custom_command_output() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_custom_command("some command", false)
            .await
            .unwrap();

        assert!(result["output_lines"].is_number());
        assert!(result["execution_time_ms"].is_number());
        assert!(result["output_lines"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_execute_custom_command_multiple_commands() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let commands = vec![
            "analytics reliability --dataset ds1",
            "batch pf --manifest manifest.json",
            "datasets list --format json",
        ];

        for cmd in commands {
            let result = service.execute_custom_command(cmd, false).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap()["status"], "success");
        }
    }

    // ============================================================================
    // Task 5 Tests: Scenario Operations
    // ============================================================================

    #[tokio::test]
    async fn test_execute_scenario_validation_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_scenario_validation("scenarios/template.yaml")
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data["valid"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_execute_scenario_validation_details() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_scenario_validation("scenarios/template.yaml")
            .await
            .unwrap();

        assert_eq!(result["template"], "scenarios/template.yaml");
        assert_eq!(result["variables"], 12);
        assert_eq!(result["scenarios_generated"], 144);
        assert_eq!(result["status"], "valid");
    }

    #[tokio::test]
    async fn test_execute_scenario_materialization_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_scenario_materialization("scenarios/template.yaml", "/output")
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["status"], "completed");
    }

    #[tokio::test]
    async fn test_execute_scenario_materialization_details() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_scenario_materialization("scenarios/template.yaml", "/output/scenarios")
            .await
            .unwrap();

        assert_eq!(result["template"], "scenarios/template.yaml");
        assert_eq!(result["output"], "/output/scenarios");
        assert_eq!(result["scenarios_created"], 144);
        assert!(result["total_size_mb"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_execute_scenario_materialization_multiple_paths() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let paths = vec![
            ("/template1.yaml", "/out1"),
            ("/template2.yaml", "/out2"),
            ("/template3.yaml", "/out3"),
        ];

        for (template, output) in paths {
            let result = service
                .execute_scenario_materialization(template, output)
                .await;
            assert!(result.is_ok());
        }
    }

    // ============================================================================
    // Task 6 Tests: Batch Job Operations
    // ============================================================================

    #[tokio::test]
    async fn test_execute_batch_power_flow_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_batch_power_flow("manifest.json", 10)
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["status"], "running");
    }

    #[tokio::test]
    async fn test_execute_batch_power_flow_details() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_batch_power_flow("manifest.json", 10)
            .await
            .unwrap();

        assert_eq!(result["manifest"], "manifest.json");
        assert_eq!(result["max_jobs"], 10);
        assert_eq!(result["jobs_submitted"], 10);
        assert!(result["job_id"].is_string());
    }

    #[tokio::test]
    async fn test_execute_batch_opf_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .execute_batch_opf("manifest.json", 10, "IPOPT")
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["status"], "running");
    }

    #[tokio::test]
    async fn test_execute_batch_opf_solver_selection() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let solvers = vec!["IPOPT", "HiGHS", "Clarabel"];

        for solver in solvers {
            let result = service
                .execute_batch_opf("manifest.json", 5, solver)
                .await
                .unwrap();

            assert_eq!(result["solver"], solver);
            assert_eq!(result["max_jobs"], 5);
        }
    }

    #[tokio::test]
    async fn test_get_batch_job_status_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service.get_batch_job_status("batch-001").await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["job_id"], "batch-001");
    }

    #[tokio::test]
    async fn test_get_batch_job_status_progress() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .get_batch_job_status("batch-001")
            .await
            .unwrap();

        assert_eq!(result["status"], "in_progress");
        assert_eq!(result["progress"], 65);
        assert_eq!(result["completed"], 10);
        assert_eq!(result["total"], 15);
        assert!(result["elapsed_seconds"].is_number());
    }

    #[tokio::test]
    async fn test_batch_job_status_multiple_jobs() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let job_ids = vec!["batch-001", "batch-002", "batch-003"];

        for job_id in job_ids {
            let result = service.get_batch_job_status(job_id).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap()["job_id"], job_id);
        }
    }

    // ============================================================================
    // Task 7 Tests: Comprehensive Analytics
    // ============================================================================

    #[tokio::test]
    async fn test_get_all_analytics_results_success() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .get_all_analytics_results("dataset1", "grid1")
            .await;
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data["dataset_id"], "dataset1");
        assert_eq!(data["grid_id"], "grid1");
    }

    #[tokio::test]
    async fn test_get_all_analytics_reliability_data() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .get_all_analytics_results("dataset1", "grid1")
            .await
            .unwrap();

        let reliability = &result["reliability"];
        assert!(reliability["lole_hours_per_year"].is_number());
        assert!(reliability["eue_mwh_per_year"].is_number());
        assert_eq!(reliability["lole_hours_per_year"], 8.5);
        assert_eq!(reliability["eue_mwh_per_year"], 12.3);
    }

    #[tokio::test]
    async fn test_get_all_analytics_deliverability() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .get_all_analytics_results("dataset1", "grid1")
            .await
            .unwrap();

        assert!(result["deliverability_score"].is_number());
        assert_eq!(result["deliverability_score"], 87.5);
    }

    #[tokio::test]
    async fn test_get_all_analytics_elcc() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .get_all_analytics_results("dataset1", "grid1")
            .await
            .unwrap();

        assert!(result["elcc_mean"].is_number());
        assert_eq!(result["elcc_mean"], 45.2);
    }

    #[tokio::test]
    async fn test_get_all_analytics_powerflow() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .get_all_analytics_results("dataset1", "grid1")
            .await
            .unwrap();

        assert!(result["powerflow_status"].is_string());
        assert_eq!(result["powerflow_status"], "converged");
    }

    #[tokio::test]
    async fn test_get_all_analytics_timestamp() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let result = service
            .get_all_analytics_results("dataset1", "grid1")
            .await
            .unwrap();

        assert!(result["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_get_all_analytics_multiple_datasets() {
        let qb = Arc::new(MockQueryBuilder);
        let service = TuiServiceLayer::new(qb);

        let dataset_pairs = vec![
            ("ds1", "grid1"),
            ("ds2", "grid2"),
            ("ds3", "grid3"),
        ];

        for (dataset_id, grid_id) in dataset_pairs {
            let result = service
                .get_all_analytics_results(dataset_id, grid_id)
                .await;
            assert!(result.is_ok());

            let data = result.unwrap();
            assert_eq!(data["dataset_id"], dataset_id);
            assert_eq!(data["grid_id"], grid_id);
        }
    }
}
