//! Application state for Tauri.
//!
//! Contains both the legacy batch run tracking (for backward compatibility)
//! and the new `AppService` from gat-ui-common integration.

use gat_batch::BatchJobRecord;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::service::AppService;

/// Legacy batch run tracking (for backward compatibility during migration).
#[derive(Debug, Clone)]
#[allow(dead_code)] // Legacy struct for batch tracking migration
pub struct BatchRun {
    pub run_id: String,
    pub status: String, // "running" | "completed" | "failed"
    pub completed: usize,
    pub total: usize,
    pub results: Option<Vec<BatchJobRecord>>,
    pub error: Option<String>,
}

/// Application state managed by Tauri.
///
/// This struct is the single source of truth for all state in the application.
/// It includes:
/// - `service`: The new gat-ui-common integration (workspace, config, jobs)
/// - `batch_runs`: Legacy batch tracking (will be migrated to service.jobs())
pub struct AppState {
    /// New unified service layer from gat-ui-common.
    pub service: AppService,

    /// Legacy batch run tracking (for backward compatibility).
    pub batch_runs: Arc<Mutex<HashMap<String, BatchRun>>>,
}

impl AppState {
    /// Create a new application state with initialized service.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            service: AppService::new()?,
            batch_runs: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new().expect("Failed to initialize AppState")
    }
}
