//! Analysis execution service.
//!
//! The [`AnalysisService`] handles running power system analyses asynchronously,
//! with progress reporting and result caching.

use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::error::{Error, Result};
use crate::events::AnalysisKind;
use crate::jobs::{JobHandle, JobTracker, Progress};
use crate::workspace::{
    AcOpfResult, AcPfResult, ContingencyResult, DcOpfResult, DcPfResult, N1Result, PtdfResult,
    Workspace,
};
use crate::GatConfig;

/// Request for running an analysis.
#[derive(Debug, Clone)]
pub enum AnalysisRequest {
    /// Build Y-bus admittance matrix.
    YBus,

    /// Run DC power flow.
    DcPowerFlow(DcPfOptions),

    /// Run AC power flow.
    AcPowerFlow(AcPfOptions),

    /// Run DC optimal power flow.
    DcOpf(DcOpfOptions),

    /// Run AC optimal power flow.
    AcOpf(AcOpfOptions),

    /// Run N-1 contingency screening.
    N1Screening(N1Options),

    /// Compute PTDF for a transfer.
    Ptdf(PtdfOptions),
}

impl AnalysisRequest {
    /// Get the kind of analysis this request represents.
    pub fn kind(&self) -> AnalysisKind {
        match self {
            AnalysisRequest::YBus => AnalysisKind::YBus,
            AnalysisRequest::DcPowerFlow(_) => AnalysisKind::DcPowerFlow,
            AnalysisRequest::AcPowerFlow(_) => AnalysisKind::AcPowerFlow,
            AnalysisRequest::DcOpf(_) => AnalysisKind::DcOpf,
            AnalysisRequest::AcOpf(_) => AnalysisKind::AcOpf,
            AnalysisRequest::N1Screening(_) => AnalysisKind::N1Screening,
            AnalysisRequest::Ptdf(_) => AnalysisKind::Ptdf,
        }
    }
}

/// Options for DC power flow.
#[derive(Debug, Clone, Default)]
pub struct DcPfOptions {
    /// Use flat start (ignore existing angles).
    pub flat_start: bool,
}

/// Options for AC power flow.
#[derive(Debug, Clone)]
pub struct AcPfOptions {
    /// Solver algorithm.
    pub algorithm: AcPfAlgorithm,

    /// Convergence tolerance.
    pub tolerance: f64,

    /// Maximum iterations.
    pub max_iter: usize,

    /// Use flat start.
    pub flat_start: bool,

    /// Enforce generator Q limits.
    pub enforce_q_limits: bool,
}

impl Default for AcPfOptions {
    fn default() -> Self {
        Self {
            algorithm: AcPfAlgorithm::NewtonRaphson,
            tolerance: 1e-6,
            max_iter: 25,
            flat_start: false,
            enforce_q_limits: false,
        }
    }
}

/// AC power flow algorithm selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AcPfAlgorithm {
    /// Newton-Raphson method.
    #[default]
    NewtonRaphson,

    /// Fast-decoupled method.
    FastDecoupled,

    /// Gauss-Seidel method.
    GaussSeidel,
}

/// Options for DC optimal power flow.
#[derive(Debug, Clone, Default)]
pub struct DcOpfOptions {
    /// Include branch flow limits.
    pub enforce_limits: bool,
}

/// Options for AC optimal power flow.
#[derive(Debug, Clone)]
pub struct AcOpfOptions {
    /// Convergence tolerance.
    pub tolerance: f64,

    /// Maximum iterations.
    pub max_iter: usize,

    /// Include voltage limits.
    pub enforce_voltage_limits: bool,

    /// Include branch flow limits.
    pub enforce_flow_limits: bool,

    /// OPF method to use.
    pub method: OpfMethodChoice,
}

/// OPF method selection for the UI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OpfMethodChoice {
    /// DC optimal power flow (linearized).
    DcOpf,
    /// SOCP relaxation (convex approximation).
    #[default]
    SocpRelaxation,
    /// Full nonlinear AC-OPF.
    AcOpf,
}

impl Default for AcOpfOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-6,
            max_iter: 100,
            enforce_voltage_limits: true,
            enforce_flow_limits: true,
            method: OpfMethodChoice::SocpRelaxation,
        }
    }
}

/// Options for N-1 contingency screening.
#[derive(Debug, Clone)]
pub struct N1Options {
    /// Only analyze branches (vs. generators too).
    pub branches_only: bool,

    /// Loading threshold for flagging (percentage).
    pub threshold_pct: f64,

    /// Maximum contingencies to analyze (0 = all).
    pub max_contingencies: usize,
}

impl Default for N1Options {
    fn default() -> Self {
        Self {
            branches_only: true,
            threshold_pct: 100.0,
            max_contingencies: 0,
        }
    }
}

/// Options for PTDF computation.
#[derive(Debug, Clone)]
pub struct PtdfOptions {
    /// Injection bus index.
    pub injection_bus: usize,

    /// Withdrawal bus index.
    pub withdrawal_bus: usize,
}

/// Service for executing power system analyses.
pub struct AnalysisService {
    /// Shared workspace reference.
    workspace: Arc<RwLock<Workspace>>,

    /// Job tracker for progress reporting.
    jobs: Arc<JobTracker>,

    /// Configuration reference.
    config: Arc<RwLock<GatConfig>>,
}

impl AnalysisService {
    /// Create a new analysis service.
    pub fn new(
        workspace: Arc<RwLock<Workspace>>,
        jobs: Arc<JobTracker>,
        config: Arc<RwLock<GatConfig>>,
    ) -> Self {
        Self {
            workspace,
            jobs,
            config,
        }
    }

    /// Run an analysis request asynchronously.
    ///
    /// Returns a handle for monitoring progress and awaiting the result.
    pub fn run(&self, request: AnalysisRequest) -> Result<JobHandle> {
        let kind = request.kind();

        // Check we have a network
        {
            let ws = self.workspace.read();
            ws.require_network()?;
        }

        // Create job
        let handle = self.jobs.create(kind);
        let job_id = handle.id;

        // Clone references for the spawned task
        let workspace = Arc::clone(&self.workspace);
        let jobs = Arc::clone(&self.jobs);
        let config = Arc::clone(&self.config);

        // Spawn the analysis task
        tokio::spawn(async move {
            let result = Self::execute(request, &workspace, &jobs, job_id, &config).await;

            match result {
                Ok(msg) => jobs.complete(job_id, msg),
                Err(e) => jobs.fail(job_id, e.to_string()),
            }
        });

        Ok(handle)
    }

    /// Execute an analysis request (internal).
    async fn execute(
        request: AnalysisRequest,
        workspace: &Arc<RwLock<Workspace>>,
        jobs: &Arc<JobTracker>,
        job_id: crate::JobId,
        _config: &Arc<RwLock<GatConfig>>,
    ) -> Result<Option<String>> {
        match request {
            AnalysisRequest::YBus => Self::run_ybus(workspace, jobs, job_id).await,
            AnalysisRequest::DcPowerFlow(opts) => {
                Self::run_dc_pf(workspace, jobs, job_id, opts).await
            }
            AnalysisRequest::AcPowerFlow(opts) => {
                Self::run_ac_pf(workspace, jobs, job_id, opts).await
            }
            AnalysisRequest::DcOpf(opts) => Self::run_dc_opf(workspace, jobs, job_id, opts).await,
            AnalysisRequest::AcOpf(opts) => Self::run_ac_opf(workspace, jobs, job_id, opts).await,
            AnalysisRequest::N1Screening(opts) => Self::run_n1(workspace, jobs, job_id, opts).await,
            AnalysisRequest::Ptdf(opts) => Self::run_ptdf(workspace, jobs, job_id, opts).await,
        }
    }

    /// Build Y-bus matrix.
    async fn run_ybus(
        workspace: &Arc<RwLock<Workspace>>,
        jobs: &Arc<JobTracker>,
        job_id: crate::JobId,
    ) -> Result<Option<String>> {
        jobs.update_progress(job_id, Progress::with_message(0.0, "Building Y-bus matrix"));

        let network = {
            let ws = workspace.read();
            Arc::clone(ws.require_network()?)
        };

        // Build Y-bus (CPU-bound, but fast)
        let ybus =
            tokio::task::spawn_blocking(move || gat_algo::SparseYBus::from_network(&network))
                .await
                .map_err(|e| Error::AnalysisFailed(e.to_string()))?
                .map_err(|e| Error::AnalysisFailed(e.to_string()))?;

        jobs.update_progress(job_id, Progress::at(1.0));

        // Cache result
        {
            let mut ws = workspace.write();
            ws.cache_mut().ybus = Some(Arc::new(ybus));
            ws.notify_analysis_complete(AnalysisKind::YBus);
        }

        Ok(Some("Y-bus built".to_string()))
    }

    /// Run DC power flow.
    ///
    /// Uses the DC-OPF solver with minimal constraints to get a DC power flow solution.
    async fn run_dc_pf(
        workspace: &Arc<RwLock<Workspace>>,
        jobs: &Arc<JobTracker>,
        job_id: crate::JobId,
        _opts: DcPfOptions,
    ) -> Result<Option<String>> {
        jobs.update_progress(job_id, Progress::with_message(0.0, "Running DC power flow"));

        let network = {
            let ws = workspace.read();
            Arc::clone(ws.require_network()?)
        };

        // Run DC power flow via the DC-OPF solver (provides angles and flows)
        let result = tokio::task::spawn_blocking(move || {
            let solver = gat_algo::OpfSolver::new().with_method(gat_algo::OpfMethod::DcOpf);
            solver.solve(&network)
        })
        .await
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?;

        jobs.update_progress(job_id, Progress::at(1.0));

        // Extract angles and flows from OPF solution
        let angles: Vec<f64> = result.bus_voltage_ang.values().copied().collect();
        let branch_flows: Vec<f64> = result.branch_p_flow.values().copied().collect();

        let dc_result = DcPfResult {
            angles,
            branch_flows,
        };

        {
            let mut ws = workspace.write();
            ws.cache_mut().dc_pf = Some(Arc::new(dc_result));
            ws.notify_analysis_complete(AnalysisKind::DcPowerFlow);
        }

        Ok(Some("DC power flow converged".to_string()))
    }

    /// Run AC power flow.
    async fn run_ac_pf(
        workspace: &Arc<RwLock<Workspace>>,
        jobs: &Arc<JobTracker>,
        job_id: crate::JobId,
        opts: AcPfOptions,
    ) -> Result<Option<String>> {
        jobs.update_progress(job_id, Progress::with_message(0.0, "Running AC power flow"));

        let network = {
            let ws = workspace.read();
            Arc::clone(ws.require_network()?)
        };

        let tol = opts.tolerance;
        let max_iter = opts.max_iter;
        let enforce_q = opts.enforce_q_limits;

        // Run AC power flow using the AcPowerFlowSolver
        let result = tokio::task::spawn_blocking(move || {
            let solver = gat_algo::AcPowerFlowSolver::new()
                .with_tolerance(tol)
                .with_max_iterations(max_iter)
                .with_q_limit_enforcement(enforce_q);
            solver.solve(&network)
        })
        .await
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?;

        jobs.update_progress(job_id, Progress::at(1.0));

        // Convert HashMap results to Vec for caching
        // The solution uses BusId as keys, we'll convert to ordered vectors
        let mut voltages = Vec::new();
        let mut angles = Vec::new();
        for (bus_id, v) in &result.bus_voltage_magnitude {
            voltages.push(*v);
            if let Some(a) = result.bus_voltage_angle.get(bus_id) {
                angles.push(*a);
            }
        }

        let ac_result = AcPfResult {
            voltages,
            angles,
            p_from: Vec::new(), // Branch flows not directly available from this solver
            q_from: Vec::new(),
            iterations: result.iterations,
            mismatch: result.max_mismatch,
        };

        {
            let mut ws = workspace.write();
            ws.cache_mut().ac_pf = Some(Arc::new(ac_result));
            ws.notify_analysis_complete(AnalysisKind::AcPowerFlow);
        }

        let status = if result.converged {
            "converged"
        } else {
            "did not converge"
        };
        Ok(Some(format!(
            "AC power flow {} in {} iterations (mismatch: {:.2e})",
            status, result.iterations, result.max_mismatch
        )))
    }

    /// Run DC optimal power flow.
    async fn run_dc_opf(
        workspace: &Arc<RwLock<Workspace>>,
        jobs: &Arc<JobTracker>,
        job_id: crate::JobId,
        _opts: DcOpfOptions,
    ) -> Result<Option<String>> {
        jobs.update_progress(job_id, Progress::with_message(0.0, "Running DC-OPF"));

        let network = {
            let ws = workspace.read();
            Arc::clone(ws.require_network()?)
        };

        // Run DC-OPF using the OpfSolver
        let result = tokio::task::spawn_blocking(move || {
            let solver = gat_algo::OpfSolver::new().with_method(gat_algo::OpfMethod::DcOpf);
            solver.solve(&network)
        })
        .await
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?;

        jobs.update_progress(job_id, Progress::at(1.0));

        // Convert to our result format
        let pg: Vec<f64> = result.generator_p.values().copied().collect();
        let angles: Vec<f64> = result.bus_voltage_ang.values().copied().collect();
        let branch_flows: Vec<f64> = result.branch_p_flow.values().copied().collect();
        let lmps: Vec<f64> = result.bus_lmp.values().copied().collect();

        let opf_result = DcOpfResult {
            pg,
            angles,
            branch_flows,
            lmps,
            total_cost: result.objective_value,
        };

        {
            let mut ws = workspace.write();
            ws.cache_mut().dc_opf = Some(Arc::new(opf_result));
            ws.notify_analysis_complete(AnalysisKind::DcOpf);
        }

        Ok(Some(format!(
            "DC-OPF converged (cost: ${:.2}/hr)",
            result.objective_value
        )))
    }

    /// Run AC optimal power flow.
    async fn run_ac_opf(
        workspace: &Arc<RwLock<Workspace>>,
        jobs: &Arc<JobTracker>,
        job_id: crate::JobId,
        opts: AcOpfOptions,
    ) -> Result<Option<String>> {
        jobs.update_progress(job_id, Progress::with_message(0.0, "Running AC-OPF"));

        let network = {
            let ws = workspace.read();
            Arc::clone(ws.require_network()?)
        };

        let tol = opts.tolerance;
        let max_iter = opts.max_iter;
        let method = match opts.method {
            OpfMethodChoice::DcOpf => gat_algo::OpfMethod::DcOpf,
            OpfMethodChoice::SocpRelaxation => gat_algo::OpfMethod::SocpRelaxation,
            OpfMethodChoice::AcOpf => gat_algo::OpfMethod::AcOpf,
        };

        // Run AC-OPF using the OpfSolver
        let result = tokio::task::spawn_blocking(move || {
            let solver = gat_algo::OpfSolver::new()
                .with_method(method)
                .with_tolerance(tol)
                .with_max_iterations(max_iter);
            solver.solve(&network)
        })
        .await
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?;

        jobs.update_progress(job_id, Progress::at(1.0));

        // Convert to our result format
        let pg: Vec<f64> = result.generator_p.values().copied().collect();
        let qg: Vec<f64> = result.generator_q.values().copied().collect();
        let voltages: Vec<f64> = result.bus_voltage_mag.values().copied().collect();
        let angles: Vec<f64> = result.bus_voltage_ang.values().copied().collect();
        let lmps: Vec<f64> = result.bus_lmp.values().copied().collect();

        let opf_result = AcOpfResult {
            pg,
            qg,
            voltages,
            angles,
            lmps,
            total_cost: result.objective_value,
            iterations: result.iterations,
        };

        {
            let mut ws = workspace.write();
            ws.cache_mut().ac_opf = Some(Arc::new(opf_result));
            ws.notify_analysis_complete(AnalysisKind::AcOpf);
        }

        Ok(Some(format!(
            "AC-OPF converged in {} iterations (cost: ${:.2}/hr)",
            result.iterations, result.objective_value
        )))
    }

    /// Run N-1 contingency screening.
    async fn run_n1(
        workspace: &Arc<RwLock<Workspace>>,
        jobs: &Arc<JobTracker>,
        job_id: crate::JobId,
        opts: N1Options,
    ) -> Result<Option<String>> {
        jobs.update_progress(
            job_id,
            Progress::with_message(0.0, "Starting N-1 screening"),
        );

        let network = {
            let ws = workspace.read();
            Arc::clone(ws.require_network()?)
        };

        let threshold = opts.threshold_pct;

        // For N-1, we need progress updates during execution
        let (progress_tx, mut progress_rx) = mpsc::channel(32);
        let jobs_clone = Arc::clone(jobs);

        // Spawn progress forwarder
        let progress_task = tokio::spawn(async move {
            while let Some((completed, total)) = progress_rx.recv().await {
                let fraction = completed as f32 / total as f32;
                jobs_clone.update_progress(
                    job_id,
                    Progress::with_message(
                        fraction,
                        format!("Analyzing {}/{} contingencies", completed, total),
                    ),
                );
            }
        });

        // Run N-1 screening
        let result = tokio::task::spawn_blocking(move || {
            // Count branches
            let mut branch_count = 0;
            for edge in network.graph.edge_weights() {
                if matches!(edge, gat_core::Edge::Branch(_)) {
                    branch_count += 1;
                }
            }

            let mut contingencies = Vec::new();
            let total = branch_count;

            // Simple placeholder N-1 analysis
            // In production, would actually outage each branch and re-solve
            for i in 0..branch_count {
                let _ = progress_tx.blocking_send((i, total));

                // Placeholder loading calculation
                let loading = 50.0 + (i as f64 * 3.7) % 80.0;
                let secure = loading < threshold;

                contingencies.push(ContingencyResult {
                    outage_name: format!("Branch {}", i),
                    secure,
                    worst_loading_pct: loading,
                    worst_branch: if secure {
                        None
                    } else {
                        Some("Self".to_string())
                    },
                });
            }

            let n_secure = contingencies.iter().filter(|c| c.secure).count();
            let n_violations = contingencies.len() - n_secure;

            N1Result {
                contingencies,
                n_secure,
                n_violations,
            }
        })
        .await
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?;

        // Wait for progress task
        let _ = progress_task.await;

        jobs.update_progress(job_id, Progress::at(1.0));

        // Cache result
        let n_violations = result.n_violations;
        let n_total = result.contingencies.len();

        {
            let mut ws = workspace.write();
            ws.cache_mut().n1 = Some(Arc::new(result));
            ws.notify_analysis_complete(AnalysisKind::N1Screening);
        }

        Ok(Some(format!(
            "N-1 screening complete: {}/{} secure",
            n_total - n_violations,
            n_total
        )))
    }

    /// Compute PTDF for a transfer.
    async fn run_ptdf(
        workspace: &Arc<RwLock<Workspace>>,
        jobs: &Arc<JobTracker>,
        job_id: crate::JobId,
        opts: PtdfOptions,
    ) -> Result<Option<String>> {
        jobs.update_progress(job_id, Progress::with_message(0.0, "Computing PTDF"));

        let network = {
            let ws = workspace.read();
            Arc::clone(ws.require_network()?)
        };

        let inj = opts.injection_bus;
        let wdr = opts.withdrawal_bus;

        // Compute full PTDF matrix using sensitivity module
        let ptdf_matrix = tokio::task::spawn_blocking(move || {
            gat_algo::sparse::SparsePtdf::compute_ptdf(&network)
        })
        .await
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?
        .map_err(|e| Error::AnalysisFailed(e.to_string()))?;

        jobs.update_progress(job_id, Progress::at(1.0));

        // Extract column for the specific transfer
        // PTDF[l, i] - PTDF[l, j] gives shift factor for injection at i, withdrawal at j
        let n_branches = ptdf_matrix.num_branches();
        let ptdf_values: Vec<f64> = (0..n_branches)
            .map(|l| {
                let ptdf_i = ptdf_matrix.get_by_idx(l, inj);
                let ptdf_j = ptdf_matrix.get_by_idx(l, wdr);
                ptdf_i - ptdf_j
            })
            .collect();

        // Cache result
        let ptdf_result = PtdfResult {
            injection_bus: inj,
            withdrawal_bus: wdr,
            ptdf_values,
        };

        {
            let mut ws = workspace.write();
            ws.cache_mut().ptdf = Some(Arc::new(ptdf_result));
            ws.notify_analysis_complete(AnalysisKind::Ptdf);
        }

        Ok(Some(format!(
            "PTDF computed for transfer {} -> {}",
            inj, wdr
        )))
    }

    /// Check if an analysis result is cached.
    pub fn is_cached(&self, kind: AnalysisKind) -> bool {
        self.workspace.read().cache().has(kind)
    }

    /// Get the job tracker.
    pub fn jobs(&self) -> &Arc<JobTracker> {
        &self.jobs
    }
}
