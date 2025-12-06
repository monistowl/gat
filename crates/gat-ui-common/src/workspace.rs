//! Workspace state management.
//!
//! The [`Workspace`] holds the currently loaded network and cached analysis results.
//! When the network changes, all cached results are invalidated.

use std::path::PathBuf;
use std::sync::Arc;

use gat_core::Network;
use tokio::sync::broadcast;

use crate::error::{Error, Result};
use crate::events::{AnalysisKind, WorkspaceEvent};

/// Cache for analysis results.
///
/// Results are stored as `Arc` for cheap cloning and sharing between UI components.
/// All caches are invalidated when the network changes.
#[derive(Debug, Default)]
pub struct AnalysisCache {
    /// Sparse Y-bus admittance matrix.
    pub ybus: Option<Arc<gat_algo::sparse::SparseYBus>>,

    /// DC power flow solution.
    pub dc_pf: Option<Arc<DcPfResult>>,

    /// AC power flow solution.
    pub ac_pf: Option<Arc<AcPfResult>>,

    /// DC optimal power flow solution.
    pub dc_opf: Option<Arc<DcOpfResult>>,

    /// AC optimal power flow solution.
    pub ac_opf: Option<Arc<AcOpfResult>>,

    /// N-1 contingency screening results.
    pub n1: Option<Arc<N1Result>>,

    /// PTDF matrix (keyed by injection/withdrawal bus pair).
    pub ptdf: Option<Arc<PtdfResult>>,
}

impl AnalysisCache {
    /// Clear all cached results.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Check if a specific analysis is cached.
    pub fn has(&self, kind: AnalysisKind) -> bool {
        match kind {
            AnalysisKind::YBus => self.ybus.is_some(),
            AnalysisKind::DcPowerFlow => self.dc_pf.is_some(),
            AnalysisKind::AcPowerFlow => self.ac_pf.is_some(),
            AnalysisKind::DcOpf => self.dc_opf.is_some(),
            AnalysisKind::AcOpf => self.ac_opf.is_some(),
            AnalysisKind::N1Screening => self.n1.is_some(),
            AnalysisKind::Ptdf => self.ptdf.is_some(),
            AnalysisKind::Lodf => false, // TODO: add LODF cache
        }
    }
}

/// DC power flow result wrapper.
#[derive(Debug, Clone)]
pub struct DcPfResult {
    /// Bus voltage angles in radians.
    pub angles: Vec<f64>,
    /// Branch real power flows in per-unit.
    pub branch_flows: Vec<f64>,
}

/// AC power flow result wrapper.
#[derive(Debug, Clone)]
pub struct AcPfResult {
    /// Bus voltage magnitudes in per-unit.
    pub voltages: Vec<f64>,
    /// Bus voltage angles in radians.
    pub angles: Vec<f64>,
    /// Branch real power flows (from side) in per-unit.
    pub p_from: Vec<f64>,
    /// Branch reactive power flows (from side) in per-unit.
    pub q_from: Vec<f64>,
    /// Number of iterations to converge.
    pub iterations: usize,
    /// Final mismatch.
    pub mismatch: f64,
}

/// DC-OPF result wrapper.
#[derive(Debug, Clone)]
pub struct DcOpfResult {
    /// Generator real power dispatch in per-unit.
    pub pg: Vec<f64>,
    /// Bus voltage angles in radians.
    pub angles: Vec<f64>,
    /// Branch real power flows in per-unit.
    pub branch_flows: Vec<f64>,
    /// Locational marginal prices ($/MWh).
    pub lmps: Vec<f64>,
    /// Total generation cost ($/hr).
    pub total_cost: f64,
}

/// AC-OPF result wrapper.
#[derive(Debug, Clone)]
pub struct AcOpfResult {
    /// Generator real power dispatch in per-unit.
    pub pg: Vec<f64>,
    /// Generator reactive power dispatch in per-unit.
    pub qg: Vec<f64>,
    /// Bus voltage magnitudes in per-unit.
    pub voltages: Vec<f64>,
    /// Bus voltage angles in radians.
    pub angles: Vec<f64>,
    /// Locational marginal prices ($/MWh).
    pub lmps: Vec<f64>,
    /// Total generation cost ($/hr).
    pub total_cost: f64,
    /// Solver iterations.
    pub iterations: usize,
}

/// N-1 contingency screening result.
#[derive(Debug, Clone)]
pub struct N1Result {
    /// List of contingencies analyzed.
    pub contingencies: Vec<ContingencyResult>,
    /// Number of secure contingencies.
    pub n_secure: usize,
    /// Number of contingencies with violations.
    pub n_violations: usize,
}

/// Result for a single contingency.
#[derive(Debug, Clone)]
pub struct ContingencyResult {
    /// Name of the outaged element.
    pub outage_name: String,
    /// Whether the system is secure under this contingency.
    pub secure: bool,
    /// Worst loading percentage (>100% means violation).
    pub worst_loading_pct: f64,
    /// Name of the branch with worst loading.
    pub worst_branch: Option<String>,
}

/// PTDF computation result.
#[derive(Debug, Clone)]
pub struct PtdfResult {
    /// Injection bus ID.
    pub injection_bus: usize,
    /// Withdrawal bus ID.
    pub withdrawal_bus: usize,
    /// PTDF values per branch.
    pub ptdf_values: Vec<f64>,
}

/// The workspace containing network state and cached analysis.
pub struct Workspace {
    /// Currently loaded network.
    network: Option<Arc<Network>>,

    /// Path the network was loaded from.
    source_path: Option<PathBuf>,

    /// Cached analysis results.
    cache: AnalysisCache,

    /// Event broadcaster for state changes.
    events_tx: broadcast::Sender<WorkspaceEvent>,
}

impl Workspace {
    /// Create a new empty workspace.
    pub fn new() -> Self {
        let (events_tx, _) = broadcast::channel(64);
        Self {
            network: None,
            source_path: None,
            cache: AnalysisCache::default(),
            events_tx,
        }
    }

    /// Get the currently loaded network, if any.
    pub fn network(&self) -> Option<&Arc<Network>> {
        self.network.as_ref()
    }

    /// Get the network, or error if none loaded.
    pub fn require_network(&self) -> Result<&Arc<Network>> {
        self.network.as_ref().ok_or(Error::NoNetworkLoaded)
    }

    /// Get the source file path.
    pub fn source_path(&self) -> Option<&PathBuf> {
        self.source_path.as_ref()
    }

    /// Get the analysis cache.
    pub fn cache(&self) -> &AnalysisCache {
        &self.cache
    }

    /// Get mutable access to the analysis cache.
    pub fn cache_mut(&mut self) -> &mut AnalysisCache {
        &mut self.cache
    }

    /// Load a network from a file.
    ///
    /// Supports MATPOWER, PSS/E, CIM, pandapower, and PowerModels formats.
    /// Format is auto-detected from file extension.
    pub fn load(&mut self, path: impl Into<PathBuf>) -> Result<()> {
        use gat_io::importers::Format;

        let path = path.into();

        // Auto-detect format and parse
        let format = Format::detect(&path)
            .map(|(f, _confidence)| f)
            .ok_or_else(|| Error::LoadFailed {
                path: path.clone(),
                source: anyhow::anyhow!("unsupported file format"),
            })?;

        let path_str = path.to_string_lossy();
        let import_result = format.parse(&path_str).map_err(|e| Error::LoadFailed {
            path: path.clone(),
            source: e,
        })?;

        let network = import_result.network;

        // Count elements using graph iteration
        let mut n_bus = 0;
        let mut n_branch = 0;
        let mut n_gen = 0;
        for node in network.graph.node_weights() {
            match node {
                gat_core::Node::Bus(_) => n_bus += 1,
                gat_core::Node::Gen(_) => n_gen += 1,
                _ => {}
            }
        }
        for edge in network.graph.edge_weights() {
            if matches!(edge, gat_core::Edge::Branch(_)) {
                n_branch += 1;
            }
        }

        // Clear old state
        self.cache.clear();

        // Set new state
        self.network = Some(Arc::new(network));
        self.source_path = Some(path.clone());

        // Notify subscribers
        let _ = self.events_tx.send(WorkspaceEvent::NetworkLoaded {
            path,
            n_bus,
            n_branch,
            n_gen,
        });

        Ok(())
    }

    /// Unload the current network.
    pub fn unload(&mut self) {
        if self.network.is_some() {
            self.network = None;
            self.source_path = None;
            self.cache.clear();
            let _ = self.events_tx.send(WorkspaceEvent::NetworkUnloaded);
        }
    }

    /// Invalidate all cached results (e.g., after network modification).
    pub fn invalidate_cache(&mut self) {
        self.cache.clear();
        let _ = self.events_tx.send(WorkspaceEvent::CacheInvalidated);
    }

    /// Subscribe to workspace events.
    pub fn subscribe(&self) -> broadcast::Receiver<WorkspaceEvent> {
        self.events_tx.subscribe()
    }

    /// Notify that an analysis completed (for cache updates).
    pub fn notify_analysis_complete(&self, kind: AnalysisKind) {
        let _ = self
            .events_tx
            .send(WorkspaceEvent::AnalysisComplete { kind });
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_starts_empty() {
        let ws = Workspace::new();
        assert!(ws.network().is_none());
        assert!(ws.source_path().is_none());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = AnalysisCache::default();
        cache.dc_pf = Some(Arc::new(DcPfResult {
            angles: vec![0.0],
            branch_flows: vec![0.0],
        }));
        assert!(cache.has(AnalysisKind::DcPowerFlow));

        cache.clear();
        assert!(!cache.has(AnalysisKind::DcPowerFlow));
    }
}
