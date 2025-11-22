/// Service integration layer for GAT CLI functionality
///
/// This module provides abstractions for executing GAT operations and retrieving data.
/// It maps pane actions to appropriate GAT CLI commands and handles async execution.

use std::collections::HashMap;

/// GAT service command builder
pub struct GatService {
    pub cli_path: String,
    pub default_timeout: u64,
}

impl GatService {
    pub fn new(cli_path: impl Into<String>, timeout_secs: u64) -> Self {
        Self {
            cli_path: cli_path.into(),
            default_timeout: timeout_secs,
        }
    }

    /// List available datasets from catalog
    pub fn list_datasets(&self, limit: usize) -> String {
        format!("{} datasets list --limit {}", self.cli_path, limit)
    }

    /// Download dataset for preview
    pub fn preview_dataset(&self, dataset_id: &str) -> String {
        format!("{} dataset download --id {} --format preview", self.cli_path, dataset_id)
    }

    /// Upload user dataset
    pub fn upload_dataset(&self, file_path: &str) -> String {
        format!("{} dataset upload --file {}", self.cli_path, file_path)
    }

    /// Validate scenario specification
    pub fn validate_scenarios(&self, spec_path: &str) -> String {
        format!("{} scenarios validate --spec {}", self.cli_path, spec_path)
    }

    /// Materialize scenarios from template
    pub fn materialize_scenarios(&self, template_path: &str, output: &str) -> String {
        format!(
            "{} scenarios materialize --template {} --output {}",
            self.cli_path, template_path, output
        )
    }

    /// Run power flow analysis
    pub fn batch_power_flow(&self, manifest: &str, max_jobs: usize) -> String {
        format!(
            "{} batch pf --manifest {} --max-jobs {}",
            self.cli_path, manifest, max_jobs
        )
    }

    /// Run optimal power flow
    pub fn batch_optimal_power_flow(&self, manifest: &str, max_jobs: usize) -> String {
        format!(
            "{} batch opf --manifest {} --max-jobs {}",
            self.cli_path, manifest, max_jobs
        )
    }

    /// Extract GNN features from grid
    pub fn featurize_gnn(&self, grid_file: &str, group_by: &str, output: &str) -> String {
        format!(
            "{} featurize gnn --grid-file {} --group-by {} --output {}",
            self.cli_path, grid_file, group_by, output
        )
    }

    /// Extract KPI features
    pub fn featurize_kpi(&self, batch_root: &str, output: &str) -> String {
        format!(
            "{} featurize kpi --batch-root {} --output {}",
            self.cli_path, batch_root, output
        )
    }

    /// Calculate congestion rents
    pub fn alloc_rents(&self, opf_file: &str, output: &str) -> String {
        format!(
            "{} alloc rents --opf-file {} --output {}",
            self.cli_path, opf_file, output
        )
    }

    /// Calculate KPI contributions
    pub fn alloc_kpi(&self, opf_file: &str, kpi_file: &str, output: &str) -> String {
        format!(
            "{} alloc kpi --opf-file {} --kpi-file {} --output {}",
            self.cli_path, opf_file, kpi_file, output
        )
    }

    /// Compute deliverability score
    pub fn analytics_deliverability(&self, grid_file: &str, flows: &str, output: &str) -> String {
        format!(
            "{} analytics ds --grid-file {} --flows {} --output {}",
            self.cli_path, grid_file, flows, output
        )
    }

    /// Compute reliability metrics
    pub fn analytics_reliability(&self, manifest: &str, flows: &str, output: &str) -> String {
        format!(
            "{} analytics reliability --manifest {} --flows {} --output {}",
            self.cli_path, manifest, flows, output
        )
    }

    /// Estimate ELCC
    pub fn analytics_elcc(&self, profiles: &str, reliability: &str, output: &str) -> String {
        format!(
            "{} analytics elcc --profiles {} --reliability {} --output {}",
            self.cli_path, profiles, reliability, output
        )
    }

    /// Get dataset metadata
    pub fn get_dataset_info(&self, dataset_id: &str) -> String {
        format!("{} dataset info --id {}", self.cli_path, dataset_id)
    }

    /// List available runs
    pub fn list_runs(&self, limit: usize) -> String {
        format!("{} runs list --limit {}", self.cli_path, limit)
    }

    /// Get run status
    pub fn get_run_status(&self, run_id: &str) -> String {
        format!("{} runs status --id {}", self.cli_path, run_id)
    }

    /// Cancel running job
    pub fn cancel_run(&self, run_id: &str) -> String {
        format!("{} runs cancel --id {}", self.cli_path, run_id)
    }
}

/// Pipeline service for workflow operations
pub struct PipelineService {
    gat: GatService,
}

impl PipelineService {
    pub fn new(gat: GatService) -> Self {
        Self { gat }
    }

    /// Validate complete pipeline
    pub fn validate_pipeline(&self, config: &HashMap<String, String>) -> String {
        let mut cmd = self.gat.cli_path.clone();
        cmd.push_str(" validate");

        for (key, val) in config {
            cmd.push_str(&format!(" --{} {}", key, val));
        }

        cmd
    }

    /// Execute pipeline from source to output
    pub fn execute_pipeline(&self, source_file: &str, _output_dir: &str) -> String {
        // This would be a more complex operation spanning multiple commands
        format!(
            "{} batch pf --manifest {} 2>&1",
            self.gat.cli_path, source_file
        )
    }
}

/// Datasets service for data management
pub struct DatasetsService {
    gat: GatService,
}

impl DatasetsService {
    pub fn new(gat: GatService) -> Self {
        Self { gat }
    }

    /// Search datasets by name or format
    pub fn search_datasets(&self, query: &str) -> String {
        format!(
            "{} datasets list --filter '{}' --format json",
            self.gat.cli_path, query
        )
    }

    /// Get dataset metadata and statistics
    pub fn dataset_statistics(&self, dataset_id: &str) -> String {
        format!(
            "{} dataset info --id {} --include stats",
            self.gat.cli_path, dataset_id
        )
    }

    /// Stream dataset preview
    pub fn preview_dataset(&self, dataset_id: &str, rows: usize) -> String {
        format!(
            "{} dataset preview --id {} --rows {}",
            self.gat.cli_path, dataset_id, rows
        )
    }
}

/// Operations service for batch, allocation, and reliability workflows
pub struct OperationsService {
    gat: GatService,
}

impl OperationsService {
    pub fn new(gat: GatService) -> Self {
        Self { gat }
    }

    /// Get current allocation results
    pub fn get_allocations(&self, scenario_id: &str) -> String {
        format!(
            "{} alloc list --scenario {}",
            self.gat.cli_path, scenario_id
        )
    }

    /// Get reliability metrics for scenario
    pub fn get_reliability_metrics(&self, scenario_id: &str) -> String {
        format!(
            "{} analytics reliability --scenario {}",
            self.gat.cli_path, scenario_id
        )
    }

    /// List batch jobs
    pub fn list_batch_jobs(&self, limit: usize) -> String {
        format!("{} batch list --limit {}", self.gat.cli_path, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gat_service_creation() {
        let svc = GatService::new("gat-cli", 300);
        assert_eq!(svc.default_timeout, 300);
    }

    #[test]
    fn test_list_datasets_command() {
        let svc = GatService::new("gat-cli", 300);
        let cmd = svc.list_datasets(10);
        assert!(cmd.contains("datasets list"));
        assert!(cmd.contains("--limit 10"));
    }

    #[test]
    fn test_batch_power_flow_command() {
        let svc = GatService::new("gat-cli", 300);
        let cmd = svc.batch_power_flow("manifest.json", 4);
        assert!(cmd.contains("batch pf"));
        assert!(cmd.contains("manifest.json"));
        assert!(cmd.contains("--max-jobs 4"));
    }

    #[test]
    fn test_analytics_commands() {
        let svc = GatService::new("gat-cli", 300);
        let cmd1 = svc.analytics_deliverability("grid.arrow", "flows.parquet", "output.parquet");
        assert!(cmd1.contains("analytics ds"));

        let cmd2 = svc.analytics_reliability("manifest.json", "flows.parquet", "output.json");
        assert!(cmd2.contains("analytics reliability"));

        let cmd3 = svc.analytics_elcc("profiles.parquet", "reliability.json", "output.parquet");
        assert!(cmd3.contains("analytics elcc"));
    }

    #[test]
    fn test_allocation_commands() {
        let svc = GatService::new("gat-cli", 300);
        let cmd1 = svc.alloc_rents("opf.parquet", "rents.parquet");
        assert!(cmd1.contains("alloc rents"));

        let cmd2 = svc.alloc_kpi("opf.parquet", "kpi.parquet", "contrib.parquet");
        assert!(cmd2.contains("alloc kpi"));
    }

    #[test]
    fn test_featurize_commands() {
        let svc = GatService::new("gat-cli", 300);
        let cmd1 = svc.featurize_gnn("grid.arrow", "zone", "features.parquet");
        assert!(cmd1.contains("featurize gnn"));

        let cmd2 = svc.featurize_kpi("batch_root", "features.parquet");
        assert!(cmd2.contains("featurize kpi"));
    }

    #[test]
    fn test_pipeline_service() {
        let gat = GatService::new("gat-cli", 300);
        let svc = PipelineService::new(gat);
        let mut config = HashMap::new();
        config.insert("source".to_string(), "input.json".to_string());
        let cmd = svc.validate_pipeline(&config);
        assert!(cmd.contains("validate"));
        assert!(cmd.contains("--source input.json"));
    }

    #[test]
    fn test_datasets_service() {
        let gat = GatService::new("gat-cli", 300);
        let svc = DatasetsService::new(gat);
        let cmd = svc.search_datasets("OPSD");
        assert!(cmd.contains("datasets list"));
        assert!(cmd.contains("OPSD"));
    }

    #[test]
    fn test_operations_service() {
        let gat = GatService::new("gat-cli", 300);
        let svc = OperationsService::new(gat);
        let cmd = svc.list_batch_jobs(20);
        assert!(cmd.contains("batch list"));
        assert!(cmd.contains("--limit 20"));
    }
}
