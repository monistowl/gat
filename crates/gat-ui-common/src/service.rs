//! Main UI service entry point.
//!
//! The [`UiService`] provides a unified interface for UI applications,
//! coordinating workspace management, analysis execution, and configuration.

use std::path::Path;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::analysis::AnalysisService;
use crate::config::GatConfig;
use crate::error::Result;
use crate::events::{JobEvent, WorkspaceEvent};
use crate::jobs::JobTracker;
use crate::workspace::Workspace;

/// Main service coordinating all UI functionality.
///
/// This is the primary entry point for UI applications. It owns and coordinates:
/// - [`Workspace`]: Network state and analysis cache
/// - [`AnalysisService`]: Async analysis execution
/// - [`JobTracker`]: Background job management
/// - [`GatConfig`]: Application configuration
///
/// # Example
///
/// ```ignore
/// let service = UiService::new()?;
///
/// // Load a network
/// service.load_network("case14.m")?;
///
/// // Run analysis
/// let handle = service.analysis().run(AnalysisRequest::AcPowerFlow(Default::default()))?;
///
/// // Wait for result
/// let result = handle.result.await?;
/// ```
pub struct UiService {
    /// Shared workspace state.
    workspace: Arc<RwLock<Workspace>>,

    /// Job tracker for background tasks.
    jobs: Arc<JobTracker>,

    /// Analysis execution service.
    analysis: AnalysisService,

    /// Application configuration.
    config: Arc<RwLock<GatConfig>>,
}

impl UiService {
    /// Create a new UI service with default configuration.
    ///
    /// Loads configuration from `~/.gat/config.toml` if it exists,
    /// otherwise uses defaults.
    pub fn new() -> Result<Self> {
        let config = GatConfig::load_or_migrate()?;
        Self::with_config(config)
    }

    /// Create a new UI service with the provided configuration.
    pub fn with_config(config: GatConfig) -> Result<Self> {
        let workspace = Arc::new(RwLock::new(Workspace::new()));
        let jobs = JobTracker::shared();
        let config = Arc::new(RwLock::new(config));

        let analysis = AnalysisService::new(
            Arc::clone(&workspace),
            Arc::clone(&jobs),
            Arc::clone(&config),
        );

        Ok(Self {
            workspace,
            jobs,
            analysis,
            config,
        })
    }

    /// Get the workspace (read-only access).
    pub fn workspace(&self) -> &Arc<RwLock<Workspace>> {
        &self.workspace
    }

    /// Get the analysis service.
    pub fn analysis(&self) -> &AnalysisService {
        &self.analysis
    }

    /// Get the job tracker.
    pub fn jobs(&self) -> &Arc<JobTracker> {
        &self.jobs
    }

    /// Get the configuration (read-only access).
    pub fn config(&self) -> &Arc<RwLock<GatConfig>> {
        &self.config
    }

    // ─────────────────────────────────────────────────────────────────────
    // Convenience methods that delegate to components
    // ─────────────────────────────────────────────────────────────────────

    /// Load a network from a file.
    ///
    /// This clears any cached analysis results and updates recent files.
    pub fn load_network(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        // Load the network
        {
            let mut ws = self.workspace.write();
            ws.load(&path)?;
        }

        // Update recent files
        {
            let mut cfg = self.config.write();
            cfg.add_recent_file(path);
        }

        Ok(())
    }

    /// Unload the current network.
    pub fn unload_network(&self) {
        let mut ws = self.workspace.write();
        ws.unload();
    }

    /// Check if a network is loaded.
    pub fn has_network(&self) -> bool {
        self.workspace.read().network().is_some()
    }

    /// Get the number of active jobs.
    pub fn active_job_count(&self) -> usize {
        self.jobs.active_count()
    }

    /// Subscribe to workspace events.
    pub fn subscribe_workspace(&self) -> tokio::sync::broadcast::Receiver<WorkspaceEvent> {
        self.workspace.read().subscribe()
    }

    /// Subscribe to job events.
    pub fn subscribe_jobs(&self) -> tokio::sync::broadcast::Receiver<JobEvent> {
        self.jobs.subscribe()
    }

    /// Save the current configuration.
    pub fn save_config(&self) -> Result<()> {
        let cfg = self.config.read();
        cfg.save()
    }

    /// Get recent files from config.
    pub fn recent_files(&self) -> Vec<std::path::PathBuf> {
        self.config.read().core.recent_files.clone()
    }
}

impl Default for UiService {
    fn default() -> Self {
        Self::new().expect("failed to create default UiService")
    }
}

/// Builder for configuring a [`UiService`].
pub struct UiServiceBuilder {
    config: Option<GatConfig>,
    config_path: Option<std::path::PathBuf>,
}

impl UiServiceBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: None,
            config_path: None,
        }
    }

    /// Use a specific configuration.
    pub fn config(mut self, config: GatConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Load configuration from a specific path.
    pub fn config_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.config_path = Some(path.into());
        self
    }

    /// Build the UI service.
    pub fn build(self) -> Result<UiService> {
        let config = if let Some(cfg) = self.config {
            cfg
        } else if let Some(path) = self.config_path {
            GatConfig::load_from(&path)?
        } else {
            GatConfig::load_or_migrate()?
        };

        UiService::with_config(config)
    }
}

impl Default for UiServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = UiService::with_config(GatConfig::default()).unwrap();
        assert!(!service.has_network());
        assert_eq!(service.active_job_count(), 0);
    }

    #[test]
    fn test_builder() {
        let mut config = GatConfig::default();
        config.core.pf_max_iter = 50;

        let service = UiServiceBuilder::new().config(config).build().unwrap();

        assert_eq!(service.config.read().core.pf_max_iter, 50);
    }
}
