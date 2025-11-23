/// Real data integration using gat-core for power system grids
///
/// GatCoreQueryBuilder implements the QueryBuilder trait to provide
/// actual power system data from loaded networks, replacing MockQueryBuilder.
use super::query_builder::{QueryBuilder, QueryError};
use super::{graph_stats_to_system_metrics, network_to_dataset_entry, GridService};
use crate::data::{DatasetEntry, SystemMetrics, Workflow};
use async_trait::async_trait;
use gat_core::graph_utils;

/// Real data query builder using gat-core for power system analysis
///
/// GatCoreQueryBuilder provides actual data from loaded power system grids
/// stored in GridService. It queries networks for:
/// - Grid information (datasets)
/// - Network analytics (metrics via GraphStats)
/// - Workflow information (operations tracking)
/// - Pipeline configuration
/// - Available commands
///
/// Errors returned as QueryError variants for consistency with trait.
pub struct GatCoreQueryBuilder {
    grid_service: GridService,
    current_grid_id: Option<String>,
}

impl GatCoreQueryBuilder {
    /// Create a new GatCoreQueryBuilder with a grid service
    pub fn new(grid_service: GridService) -> Self {
        Self {
            grid_service,
            current_grid_id: None,
        }
    }

    /// Create a new GatCoreQueryBuilder and set the current grid
    pub fn with_grid(grid_service: GridService, grid_id: String) -> Self {
        Self {
            grid_service,
            current_grid_id: Some(grid_id),
        }
    }

    /// Set the current active grid by ID
    pub fn set_current_grid(&mut self, grid_id: String) {
        self.current_grid_id = Some(grid_id);
    }

    /// Get the current active grid ID
    pub fn current_grid(&self) -> Option<&str> {
        self.current_grid_id.as_deref()
    }

    /// Clear the current grid selection
    pub fn clear_current_grid(&mut self) {
        self.current_grid_id = None;
    }

    /// Helper to ensure a grid is loaded
    fn get_current_grid_id(&self) -> Result<String, QueryError> {
        self.current_grid_id
            .clone()
            .ok_or_else(|| QueryError::NotFound("No grid loaded".to_string()))
    }
}

#[async_trait]
impl QueryBuilder for GatCoreQueryBuilder {
    /// Fetch all available datasets (grids)
    ///
    /// Returns list of all loaded grids with their properties.
    /// If no grids are loaded, returns empty list.
    async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError> {
        let grid_ids = self.grid_service.list_grids();

        if grid_ids.is_empty() {
            // No grids loaded - can return empty list or error
            // For now, return empty list to allow UI to show "no data"
            return Ok(Vec::new());
        }

        let mut datasets = Vec::new();

        for grid_id in grid_ids {
            match self.grid_service.get_grid(&grid_id) {
                Ok(network) => {
                    let dataset = network_to_dataset_entry(&grid_id, &network);
                    datasets.push(dataset);
                }
                Err(e) => {
                    // Log error but continue with other grids
                    tracing::warn!("Failed to load grid {}: {:?}", grid_id, e);
                }
            }
        }

        Ok(datasets)
    }

    /// Fetch a specific dataset (grid) by ID
    async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError> {
        self.grid_service
            .get_grid(id)
            .map(|network| network_to_dataset_entry(id, &network))
            .map_err(|_| QueryError::NotFound(format!("Grid {} not found", id)))
    }

    /// Fetch workflows (operations executed on grids)
    ///
    /// Currently returns empty list. In future, could track:
    /// - Power flow analyses
    /// - Scenario evaluations
    /// - Analytics runs
    async fn get_workflows(&self) -> Result<Vec<Workflow>, QueryError> {
        // TODO: In Phase 3, track workflow history from executed analyses
        Ok(vec![])
    }

    /// Fetch system metrics for the current grid
    ///
    /// Calculates network statistics from the currently active grid.
    /// Returns error if no grid is currently loaded.
    async fn get_metrics(&self) -> Result<SystemMetrics, QueryError> {
        let grid_id = self.get_current_grid_id()?;

        let network = self
            .grid_service
            .get_grid(&grid_id)
            .map_err(|e| QueryError::ConnectionFailed(format!("{:?}", e)))?;

        // Calculate graph statistics
        let stats = graph_utils::graph_stats(&network)
            .map_err(|e| QueryError::ParseError(format!("Failed to calculate stats: {:?}", e)))?;

        Ok(graph_stats_to_system_metrics(&stats))
    }

    /// Fetch pipeline configuration
    ///
    /// Returns JSON describing the pipeline configuration.
    /// In future, could return actual pipeline from loaded grid metadata.
    async fn get_pipeline_config(&self) -> Result<String, QueryError> {
        let grid_id = self.get_current_grid_id()?;

        let network = self
            .grid_service
            .get_grid(&grid_id)
            .map_err(|e| QueryError::ConnectionFailed(format!("{:?}", e)))?;

        let stats = graph_utils::graph_stats(&network)
            .map_err(|e| QueryError::ParseError(format!("Failed to get stats: {:?}", e)))?;

        // Build pipeline config from network properties
        let config = serde_json::json!({
            "name": format!("Analysis Pipeline for Grid {}", grid_id),
            "stages": [
                {
                    "name": "Data Load",
                    "type": "source",
                    "status": "complete"
                },
                {
                    "name": "Network Analysis",
                    "type": "transform",
                    "properties": {
                        "nodes": stats.node_count,
                        "edges": stats.edge_count,
                        "density": stats.density,
                        "components": stats.connected_components
                    }
                },
                {
                    "name": "Results Export",
                    "type": "sink",
                    "status": "ready"
                }
            ]
        });

        Ok(config.to_string())
    }

    /// Fetch available commands
    ///
    /// Returns list of GAT commands that can be executed.
    /// Includes options for analysis, optimization, and visualization.
    async fn get_commands(&self) -> Result<Vec<String>, QueryError> {
        Ok(vec![
            "grid info".to_string(),
            "grid stats".to_string(),
            "grid topology".to_string(),
            "graph analyze".to_string(),
            "find-islands".to_string(),
            "export graph".to_string(),
            "export json".to_string(),
            "metrics calculate".to_string(),
            "scenario create".to_string(),
            "powerflow run".to_string(),
            "powerflow analyze".to_string(),
            "reliability metrics".to_string(),
            "allocation analysis".to_string(),
            "batch analyze".to_string(),
        ])
    }
}

impl Clone for GatCoreQueryBuilder {
    fn clone(&self) -> Self {
        Self {
            grid_service: self.grid_service.clone(),
            current_grid_id: self.current_grid_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gat_core_query_builder_creation() {
        let grid_service = GridService::new();
        let qb = GatCoreQueryBuilder::new(grid_service);
        assert!(qb.current_grid().is_none());
    }

    #[test]
    fn test_gat_core_with_grid() {
        let grid_service = GridService::new();
        let grid_id = "test-grid-123".to_string();
        let qb = GatCoreQueryBuilder::with_grid(grid_service, grid_id.clone());
        assert_eq!(qb.current_grid(), Some("test-grid-123"));
    }

    #[test]
    fn test_set_current_grid() {
        let grid_service = GridService::new();
        let mut qb = GatCoreQueryBuilder::new(grid_service);
        assert!(qb.current_grid().is_none());

        qb.set_current_grid("new-grid".to_string());
        assert_eq!(qb.current_grid(), Some("new-grid"));
    }

    #[test]
    fn test_clear_current_grid() {
        let grid_service = GridService::new();
        let mut qb = GatCoreQueryBuilder::with_grid(grid_service, "test-grid".to_string());
        assert!(qb.current_grid().is_some());

        qb.clear_current_grid();
        assert!(qb.current_grid().is_none());
    }

    #[test]
    fn test_get_current_grid_id_when_none() {
        let grid_service = GridService::new();
        let qb = GatCoreQueryBuilder::new(grid_service);
        let result = qb.get_current_grid_id();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_current_grid_id_when_set() {
        let grid_service = GridService::new();
        let qb = GatCoreQueryBuilder::with_grid(grid_service, "test-123".to_string());
        let result = qb.get_current_grid_id();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-123");
    }

    #[test]
    fn test_gat_core_query_builder_clone() {
        let grid_service = GridService::new();
        let qb1 = GatCoreQueryBuilder::with_grid(grid_service, "test-grid".to_string());
        let qb2 = qb1.clone();
        assert_eq!(qb2.current_grid(), qb1.current_grid());
    }

    #[tokio::test]
    async fn test_get_datasets_empty() {
        let grid_service = GridService::new();
        let qb = GatCoreQueryBuilder::new(grid_service);
        let datasets = qb.get_datasets().await.unwrap();
        assert_eq!(datasets.len(), 0);
    }

    #[tokio::test]
    async fn test_get_commands() {
        let grid_service = GridService::new();
        let qb = GatCoreQueryBuilder::new(grid_service);
        let commands = qb.get_commands().await.unwrap();
        assert!(!commands.is_empty());
        assert!(commands.contains(&"grid info".to_string()));
        assert!(commands.contains(&"powerflow run".to_string()));
    }

    #[tokio::test]
    async fn test_get_metrics_no_grid() {
        let grid_service = GridService::new();
        let qb = GatCoreQueryBuilder::new(grid_service);
        let result = qb.get_metrics().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_pipeline_config_no_grid() {
        let grid_service = GridService::new();
        let qb = GatCoreQueryBuilder::new(grid_service);
        let result = qb.get_pipeline_config().await;
        assert!(result.is_err());
    }
}
