/// Grid management service for loading and caching power system networks
///
/// This module provides GridService which manages lifecycle of loaded
/// power system grids (networks). It uses gat-io to load grids from files
/// and caches them in memory for fast access and analysis.
use gat_core::Network;
use gat_io::importers;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Error type for grid service operations
#[derive(Debug, Clone)]
pub enum GridError {
    /// Grid file not found
    NotFound(String),
    /// Failed to load grid file
    LoadFailed(String),
    /// Grid not loaded
    GridNotLoaded(String),
    /// Grid already loaded with this ID
    AlreadyExists(String),
    /// Invalid grid file format
    InvalidFormat(String),
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GridError::NotFound(msg) => write!(f, "Grid not found: {}", msg),
            GridError::LoadFailed(msg) => write!(f, "Failed to load grid: {}", msg),
            GridError::GridNotLoaded(id) => write!(f, "Grid not loaded: {}", id),
            GridError::AlreadyExists(id) => write!(f, "Grid already exists: {}", id),
            GridError::InvalidFormat(msg) => write!(f, "Invalid grid format: {}", msg),
        }
    }
}

impl std::error::Error for GridError {}

/// Manages loaded power system grids
///
/// GridService handles:
/// - Loading grids from files (Arrow, Matpower, PSSE formats via gat-io)
/// - Caching loaded grids in memory by ID
/// - Retrieving loaded grids for analysis
/// - Listing available loaded grids
///
/// Uses Arc<RwLock> for thread-safe concurrent access to networks.
/// Note: Networks are stored in Arc for efficient sharing without cloning.
pub struct GridService {
    // Grid cache: ID -> Arc-wrapped Network
    networks: Arc<RwLock<HashMap<String, Arc<Network>>>>,
}

impl GridService {
    /// Create a new GridService with empty cache
    pub fn new() -> Self {
        Self {
            networks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load a grid from an Arrow file and cache it
    ///
    /// Returns a unique grid ID for referencing the loaded grid.
    /// If a grid with the same content is already loaded, this loads it again
    /// with a new ID.
    pub fn load_grid_from_arrow(&self, file_path: &str) -> Result<String, GridError> {
        // Verify file exists
        if !Path::new(file_path).exists() {
            return Err(GridError::NotFound(file_path.to_string()));
        }

        // Load grid using gat-io
        let network = importers::load_grid_from_arrow(file_path)
            .map_err(|e| GridError::LoadFailed(format!("{:?}", e)))?;

        // Generate unique ID for this grid
        let grid_id = Uuid::new_v4().to_string();

        // Cache the network wrapped in Arc
        {
            let mut networks = self.networks.write();
            networks.insert(grid_id.clone(), Arc::new(network));
        }

        Ok(grid_id)
    }

    /// Load a grid from a Matpower .m file and cache it
    ///
    /// Matpower files must be converted to Arrow format first via gat-io.
    pub fn load_grid_from_matpower(
        &self,
        file_path: &str,
        output_arrow: &str,
    ) -> Result<String, GridError> {
        // Verify file exists
        if !Path::new(file_path).exists() {
            return Err(GridError::NotFound(file_path.to_string()));
        }

        // Import from Matpower to Arrow (if not already done)
        if !Path::new(output_arrow).exists() {
            importers::import_matpower_case(file_path, output_arrow)
                .map_err(|e| GridError::LoadFailed(format!("Matpower import failed: {:?}", e)))?;
        }

        // Load the Arrow file
        self.load_grid_from_arrow(output_arrow)
    }

    /// Get an Arc reference to a loaded grid by ID
    ///
    /// Returns Arc<Network> for efficient sharing without cloning.
    pub fn get_grid(&self, grid_id: &str) -> Result<Arc<Network>, GridError> {
        let networks = self.networks.read();
        networks
            .get(grid_id)
            .map(Arc::clone)
            .ok_or_else(|| GridError::GridNotLoaded(grid_id.to_string()))
    }

    /// List IDs of all loaded grids
    pub fn list_grids(&self) -> Vec<String> {
        let networks = self.networks.read();
        networks.keys().cloned().collect()
    }

    /// Unload a grid by ID, freeing memory
    pub fn unload_grid(&self, grid_id: &str) -> Result<(), GridError> {
        let mut networks = self.networks.write();
        networks
            .remove(grid_id)
            .ok_or_else(|| GridError::GridNotLoaded(grid_id.to_string()))?;
        Ok(())
    }

    /// Get the number of loaded grids
    pub fn grid_count(&self) -> usize {
        let networks = self.networks.read();
        networks.len()
    }

    /// Clear all loaded grids from cache
    pub fn clear_all(&self) {
        let mut networks = self.networks.write();
        networks.clear();
    }
}

impl Default for GridService {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GridService {
    fn clone(&self) -> Self {
        Self {
            networks: Arc::clone(&self.networks),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_service_creation() {
        let service = GridService::new();
        assert_eq!(service.grid_count(), 0);
        assert!(service.list_grids().is_empty());
    }

    #[test]
    fn test_list_grids_empty() {
        let service = GridService::new();
        let grids = service.list_grids();
        assert_eq!(grids.len(), 0);
    }

    #[test]
    fn test_unload_nonexistent_grid() {
        let service = GridService::new();
        let result = service.unload_grid("nonexistent-id");
        assert!(result.is_err());
        match result {
            Err(GridError::GridNotLoaded(id)) => {
                assert_eq!(id, "nonexistent-id");
            }
            _ => panic!("Expected GridNotLoaded error"),
        }
    }

    #[test]
    fn test_get_nonexistent_grid() {
        let service = GridService::new();
        let result = service.get_grid("nonexistent-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_all() {
        let service = GridService::new();
        service.clear_all();
        assert_eq!(service.grid_count(), 0);
    }

    #[test]
    fn test_grid_service_clone() {
        let service1 = GridService::new();
        let service2 = service1.clone();

        // Both should reference same underlying cache
        assert_eq!(service1.grid_count(), 0);
        assert_eq!(service2.grid_count(), 0);
    }

    #[test]
    fn test_file_not_found() {
        let service = GridService::new();
        let result = service.load_grid_from_arrow("/nonexistent/path.arrow");
        assert!(result.is_err());
        match result {
            Err(GridError::NotFound(path)) => {
                assert!(path.contains("nonexistent"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_default_trait() {
        let service = GridService::default();
        assert_eq!(service.grid_count(), 0);
    }
}
