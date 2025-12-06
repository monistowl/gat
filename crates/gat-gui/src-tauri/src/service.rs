//! Application service layer bridging Tauri to gat-ui-common.
//!
//! This module provides `AppService`, which wraps `gat_ui_common::UiService` and manages
//! the tokio runtime needed for async operations.

use std::sync::Arc;

use gat_ui_common::{GatConfig, UiService, Workspace};
use parking_lot::RwLock;
use tokio::runtime::{Handle, Runtime};

/// Application service managing the UI backend and async runtime.
///
/// This is the primary state object managed by Tauri. It owns the tokio runtime
/// and the `UiService` from gat-ui-common.
pub struct AppService {
    /// Tokio runtime for async operations.
    #[allow(dead_code)] // Used by block_on/spawn when commands migrate to async
    runtime: Runtime,

    /// The underlying UI service from gat-ui-common.
    #[allow(dead_code)] // Full UiService access for future async migration
    service: UiService,
}

impl AppService {
    /// Create a new application service.
    ///
    /// Initializes the tokio runtime and loads configuration.
    pub fn new() -> Result<Self, String> {
        // Create a multi-threaded tokio runtime
        let runtime = Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

        // Initialize UiService on the runtime
        let service = UiService::new()
            .map_err(|e| format!("Failed to initialize service: {e}"))?;

        Ok(Self { runtime, service })
    }

    /// Get a handle to the tokio runtime for spawning tasks.
    #[allow(dead_code)] // For future async command migration
    pub fn handle(&self) -> Handle {
        self.runtime.handle().clone()
    }

    /// Get the underlying UI service.
    #[allow(dead_code)] // Full UiService access for future async migration
    pub fn service(&self) -> &UiService {
        &self.service
    }

    /// Get access to the workspace.
    pub fn workspace(&self) -> &Arc<RwLock<Workspace>> {
        self.service.workspace()
    }

    /// Get access to the configuration.
    pub fn config(&self) -> &Arc<RwLock<GatConfig>> {
        self.service.config()
    }

    /// Run an async operation on the runtime, blocking until completion.
    ///
    /// Use this for short operations. For long-running tasks, prefer spawning
    /// and returning a job handle.
    #[allow(dead_code)] // For future async command migration
    pub fn block_on<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        self.runtime.block_on(future)
    }

    /// Spawn an async task on the runtime without blocking.
    #[allow(dead_code)] // For future async command migration
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }
}

impl Default for AppService {
    fn default() -> Self {
        Self::new().expect("Failed to create AppService")
    }
}
