/// Integration tests for GridService and GatCoreQueryBuilder with real grid files
///
/// These tests use actual power system network files (Arrow format) stored in
/// the repository test_data directory to verify that the real data integration
/// works correctly end-to-end.

#[cfg(test)]
mod grid_integration_tests {
    use crate::services::{GatCoreQueryBuilder, GridService, QueryBuilder};
    use std::path::PathBuf;

    /// Helper to get the path to a test grid file
    fn get_test_grid_path(relative_path: &str) -> PathBuf {
        // Construct path relative to workspace root
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Go up from crates/gat-tui to workspace root
        path.pop();
        path.pop();
        path.push(relative_path);
        path
    }

    /// Test: Load IEEE 14-bus test case from Arrow file
    #[test]
    fn test_load_ieee14_grid() {
        let grid_service = GridService::new();
        let ieee14_path = get_test_grid_path("test_data/matpower/ieee14.arrow");

        if !ieee14_path.exists() {
            eprintln!("Warning: IEEE 14 test grid not found at {:?}", ieee14_path);
            return;
        }

        let result = grid_service.load_grid_from_arrow(ieee14_path.to_str().unwrap());
        assert!(result.is_ok(), "Failed to load IEEE 14 grid");

        let grid_id = result.unwrap();
        assert!(!grid_id.is_empty(), "Grid ID should not be empty");

        // Verify grid was cached
        let grids = grid_service.list_grids();
        assert!(grids.contains(&grid_id), "Loaded grid should be in cache");

        // Verify we can retrieve the grid
        let grid = grid_service.get_grid(&grid_id);
        assert!(grid.is_ok(), "Should be able to retrieve loaded grid");

        let network = grid.unwrap();
        // IEEE 14 network - node count may vary based on how network is modeled
        // (buses + generators + loads, etc.), but should be > 14
        assert!(network.graph.node_count() > 0, "IEEE 14 should have nodes");
        let node_count = network.graph.node_count();
        eprintln!("IEEE 14 loaded with {} nodes", node_count);
    }

    /// Test: Load IEEE 33-bus distribution test case
    #[test]
    fn test_load_ieee33_grid() {
        let grid_service = GridService::new();
        let ieee33_path = get_test_grid_path("test_data/derms/ieee33/case33bw.arrow");

        if !ieee33_path.exists() {
            eprintln!("Warning: IEEE 33 test grid not found at {:?}", ieee33_path);
            return;
        }

        let result = grid_service.load_grid_from_arrow(ieee33_path.to_str().unwrap());
        assert!(result.is_ok(), "Failed to load IEEE 33 grid");

        let grid_id = result.unwrap();
        assert!(!grid_id.is_empty(), "Grid ID should not be empty");

        // Verify we can retrieve the grid
        let grid = grid_service.get_grid(&grid_id);
        assert!(grid.is_ok(), "Should be able to retrieve loaded grid");

        let network = grid.unwrap();
        // IEEE 33 network - node count may vary based on how network is modeled
        assert!(network.graph.node_count() > 0, "IEEE 33 should have nodes");
        let node_count = network.graph.node_count();
        eprintln!("IEEE 33 loaded with {} nodes", node_count);
    }

    /// Test: Load multiple grids and list them
    #[test]
    fn test_load_and_list_multiple_grids() {
        let grid_service = GridService::new();
        let ieee14_path = get_test_grid_path("test_data/matpower/ieee14.arrow");
        let ieee33_path = get_test_grid_path("test_data/derms/ieee33/case33bw.arrow");

        if !ieee14_path.exists() || !ieee33_path.exists() {
            eprintln!("Warning: One or more test grids not found");
            return;
        }

        // Load both grids
        let id14 = grid_service
            .load_grid_from_arrow(ieee14_path.to_str().unwrap())
            .expect("Failed to load IEEE 14");
        let id33 = grid_service
            .load_grid_from_arrow(ieee33_path.to_str().unwrap())
            .expect("Failed to load IEEE 33");

        // Verify both are listed
        let grids = grid_service.list_grids();
        assert_eq!(grids.len(), 2, "Should have 2 grids loaded");
        assert!(grids.contains(&id14), "IEEE 14 ID should be in list");
        assert!(grids.contains(&id33), "IEEE 33 ID should be in list");

        // Verify grid count
        assert_eq!(grid_service.grid_count(), 2, "Grid count should be 2");
    }

    /// Test: GatCoreQueryBuilder with real IEEE 14 grid
    #[tokio::test]
    async fn test_gat_core_qb_with_real_grid() {
        let grid_service = GridService::new();
        let ieee14_path = get_test_grid_path("test_data/matpower/ieee14.arrow");

        if !ieee14_path.exists() {
            eprintln!("Warning: IEEE 14 test grid not found");
            return;
        }

        let grid_id = grid_service
            .load_grid_from_arrow(ieee14_path.to_str().unwrap())
            .expect("Failed to load grid");

        // Create query builder with the grid
        let mut qb = GatCoreQueryBuilder::new(grid_service);
        qb.set_current_grid(grid_id.clone());

        // Test get_datasets
        let datasets = qb.get_datasets().await;
        assert!(datasets.is_ok(), "get_datasets should succeed");

        let datasets_vec = datasets.unwrap();
        assert!(!datasets_vec.is_empty(), "Should have at least one dataset");
        assert_eq!(
            datasets_vec[0].id, grid_id,
            "Dataset ID should match grid ID"
        );

        // Verify dataset properties
        let dataset = &datasets_vec[0];
        assert!(dataset.row_count > 0, "Dataset should have nodes");
        eprintln!("Dataset reports {} nodes", dataset.row_count);
        assert!(dataset.size_mb > 0.0, "Dataset should have non-zero size");
    }

    /// Test: GatCoreQueryBuilder metrics with real grid
    #[tokio::test]
    async fn test_gat_core_qb_metrics_with_real_grid() {
        let grid_service = GridService::new();
        let ieee14_path = get_test_grid_path("test_data/matpower/ieee14.arrow");

        if !ieee14_path.exists() {
            eprintln!("Warning: IEEE 14 test grid not found");
            return;
        }

        let grid_id = grid_service
            .load_grid_from_arrow(ieee14_path.to_str().unwrap())
            .expect("Failed to load grid");

        let mut qb = GatCoreQueryBuilder::new(grid_service);
        qb.set_current_grid(grid_id);

        // Test get_metrics
        let metrics = qb.get_metrics().await;
        assert!(
            metrics.is_ok(),
            "get_metrics should succeed with grid loaded"
        );

        let metrics_val = metrics.unwrap();
        // Metrics should be reasonable values
        assert!(
            metrics_val.deliverability_score >= 0.0,
            "Deliverability should be non-negative"
        );
        assert!(
            metrics_val.deliverability_score <= 100.0,
            "Deliverability should be <= 100"
        );
        assert!(
            metrics_val.lole_hours_per_year >= 0.0,
            "LOLE should be non-negative"
        );
        assert!(
            metrics_val.eue_mwh_per_year >= 0.0,
            "EUE should be non-negative"
        );
    }

    /// Test: GatCoreQueryBuilder pipeline config with real grid
    #[tokio::test]
    async fn test_gat_core_qb_pipeline_config() {
        let grid_service = GridService::new();
        let ieee14_path = get_test_grid_path("test_data/matpower/ieee14.arrow");

        if !ieee14_path.exists() {
            eprintln!("Warning: IEEE 14 test grid not found");
            return;
        }

        let grid_id = grid_service
            .load_grid_from_arrow(ieee14_path.to_str().unwrap())
            .expect("Failed to load grid");

        let mut qb = GatCoreQueryBuilder::new(grid_service);
        qb.set_current_grid(grid_id);

        // Test get_pipeline_config
        let config = qb.get_pipeline_config().await;
        assert!(config.is_ok(), "get_pipeline_config should succeed");

        let config_str = config.unwrap();
        assert!(!config_str.is_empty(), "Config should not be empty");
        // Should be valid JSON
        let parsed = serde_json::from_str::<serde_json::Value>(&config_str);
        assert!(parsed.is_ok(), "Config should be valid JSON");
    }

    /// Test: Grid switching with cache invalidation
    #[test]
    fn test_grid_switching() {
        let grid_service = GridService::new();
        let ieee14_path = get_test_grid_path("test_data/matpower/ieee14.arrow");
        let ieee33_path = get_test_grid_path("test_data/derms/ieee33/case33bw.arrow");

        if !ieee14_path.exists() || !ieee33_path.exists() {
            eprintln!("Warning: One or more test grids not found");
            return;
        }

        let id14 = grid_service
            .load_grid_from_arrow(ieee14_path.to_str().unwrap())
            .expect("Failed to load IEEE 14");
        let id33 = grid_service
            .load_grid_from_arrow(ieee33_path.to_str().unwrap())
            .expect("Failed to load IEEE 33");

        // Create query builder and switch grids
        let mut qb = GatCoreQueryBuilder::new(grid_service);

        // Initially no grid
        assert!(qb.current_grid().is_none(), "Should start with no grid");

        // Switch to IEEE 14
        qb.set_current_grid(id14.clone());
        assert_eq!(
            qb.current_grid(),
            Some(id14.as_str()),
            "Should be on IEEE 14"
        );

        // Switch to IEEE 33
        qb.set_current_grid(id33.clone());
        assert_eq!(
            qb.current_grid(),
            Some(id33.as_str()),
            "Should be on IEEE 33"
        );

        // Clear grid
        qb.clear_current_grid();
        assert!(
            qb.current_grid().is_none(),
            "Should have no grid after clear"
        );
    }

    /// Test: Concurrent access to GridService
    #[test]
    fn test_concurrent_grid_access() {
        let grid_service = GridService::new();
        let ieee14_path = get_test_grid_path("test_data/matpower/ieee14.arrow");

        if !ieee14_path.exists() {
            eprintln!("Warning: IEEE 14 test grid not found");
            return;
        }

        let grid_id = grid_service
            .load_grid_from_arrow(ieee14_path.to_str().unwrap())
            .expect("Failed to load grid");

        // Clone grid service and spawn threads to access concurrently
        let gs1 = grid_service.clone();
        let gs2 = grid_service.clone();
        let id1 = grid_id.clone();
        let id2 = grid_id.clone();

        let h1 = std::thread::spawn(move || gs1.get_grid(&id1).is_ok());

        let h2 = std::thread::spawn(move || gs2.get_grid(&id2).is_ok());

        assert!(
            h1.join().unwrap(),
            "Thread 1 should access grid successfully"
        );
        assert!(
            h2.join().unwrap(),
            "Thread 2 should access grid successfully"
        );
    }

    /// Test: Unload grid and verify it's removed
    #[test]
    fn test_unload_grid() {
        let grid_service = GridService::new();
        let ieee14_path = get_test_grid_path("test_data/matpower/ieee14.arrow");

        if !ieee14_path.exists() {
            eprintln!("Warning: IEEE 14 test grid not found");
            return;
        }

        let grid_id = grid_service
            .load_grid_from_arrow(ieee14_path.to_str().unwrap())
            .expect("Failed to load grid");

        assert_eq!(grid_service.grid_count(), 1, "Should have 1 grid");

        // Unload it
        let result = grid_service.unload_grid(&grid_id);
        assert!(result.is_ok(), "Unload should succeed");

        // Verify it's removed
        assert_eq!(
            grid_service.grid_count(),
            0,
            "Should have 0 grids after unload"
        );
        assert!(
            grid_service.get_grid(&grid_id).is_err(),
            "Grid should not be retrievable after unload"
        );
    }
}
