//! Optimal Power Flow solvers
//!
//! This module provides OPF solvers with multiple solution methods:
//! - Economic dispatch (merit-order, no network)
//! - DC-OPF (linearized power flow)
//! - SOCP relaxation (convex AC approximation)
//! - AC-OPF (full nonlinear)
//!
//! # Solver Dispatch
//!
//! The [`dispatch`] module handles solver selection between pure-Rust
//! and native solvers based on availability and user configuration.
//!
//! # Native Solver Mode
//!
//! By default, DC-OPF uses Clarabel and AC-OPF uses L-BFGS. To use native
//! solvers (CLP for LP, IPOPT for NLP), enable the `native-dispatch` feature
//! and use `prefer_native(true)`:
//!
//! ```ignore
//! let solver = OpfSolver::new()
//!     .with_method(OpfMethod::DcOpf)
//!     .prefer_native(true);  // Use CLP if available
//! ```

pub mod ac_nlp;
mod dc_opf;
pub mod dispatch;
mod economic;
#[cfg(feature = "native-dispatch")]
pub mod native_dispatch;
mod socp;
mod types;
pub mod traits;
pub mod registry;
mod dispatcher;

pub use dispatch::{DispatchConfig, ProblemClass, SolverBackend, SolverDispatcher};
pub use types::{
    CascadedResult, ConstraintInfo, ConstraintType, DcWarmStart, OpfMethod, OpfSolution,
    SocpWarmStart,
};
pub use traits::{OpfBackend, OpfFormulation, OpfProblem, SolverConfig, WarmStartKind};
pub use registry::SolverRegistry;
pub use dispatcher::OpfDispatcher;

use crate::OpfError;
use gat_core::Network;

/// Unified OPF solver supporting multiple solution methods
pub struct OpfSolver {
    method: OpfMethod,
    max_iterations: usize,
    tolerance: f64,
    timeout_seconds: u64,
    /// If true, fail when native solver requested but not available.
    /// If false (default), silently fall back to pure-Rust solver.
    require_native: bool,
    /// If true, prefer native solvers when available.
    prefer_native: bool,
    /// If true, use enhanced SOCP with OBBT bound tightening and QC envelopes.
    use_enhanced_socp: bool,
}

impl OpfSolver {
    /// Create new OPF solver with default settings (SOCP method)
    pub fn new() -> Self {
        Self {
            method: OpfMethod::default(),
            max_iterations: 100,
            tolerance: 1e-6,
            timeout_seconds: 300, // 5 minutes default
            require_native: false,
            prefer_native: false,
            use_enhanced_socp: false,
        }
    }

    /// Set solution method
    pub fn with_method(mut self, method: OpfMethod) -> Self {
        self.method = method;
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max_iter: usize) -> Self {
        self.max_iterations = max_iter;
        self
    }

    /// Set convergence tolerance
    pub fn with_tolerance(mut self, tol: f64) -> Self {
        self.tolerance = tol;
        self
    }

    /// Set solver timeout in seconds
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Prefer native solvers when available.
    ///
    /// When `prefer_native(true)` is set:
    /// - For `DcOpf`: uses CLP if installed, falls back to Clarabel
    /// - For `AcOpf`: uses IPOPT if installed, falls back to L-BFGS
    /// - For other methods: no effect
    ///
    /// This allows using optimized native solvers without failing if they're
    /// not available.
    pub fn prefer_native(mut self, prefer: bool) -> Self {
        self.prefer_native = prefer;
        self
    }

    /// Require native solver (fail if unavailable instead of falling back).
    ///
    /// When `require_native(true)` is set:
    /// - For `AcOpf`: requires IPOPT to be installed, fails otherwise
    /// - For other methods: no effect (they don't have native backends)
    ///
    /// This prevents silent fallback to the pure-Rust L-BFGS solver when
    /// the user explicitly wants IPOPT's superior convergence.
    pub fn require_native(mut self, require: bool) -> Self {
        self.require_native = require;
        self
    }

    /// Enable enhanced SOCP with OBBT bound tightening and QC envelopes.
    ///
    /// When enabled for `SocpRelaxation`:
    /// - Applies Optimization-Based Bound Tightening (OBBT) to tighten variable bounds
    /// - Adds Quadratic Convex (QC) envelope constraints for cos(θ) terms
    /// - Results in tighter relaxation at the cost of additional computation
    ///
    /// Has no effect on other methods.
    pub fn enhanced_socp(mut self, enhanced: bool) -> Self {
        self.use_enhanced_socp = enhanced;
        self
    }

    /// Get the configured method
    pub fn method(&self) -> OpfMethod {
        self.method
    }

    /// Check if native solver is required
    pub fn requires_native(&self) -> bool {
        self.require_native
    }

    /// Solve OPF for the given network
    pub fn solve(&self, network: &Network) -> Result<OpfSolution, OpfError> {
        match self.method {
            OpfMethod::EconomicDispatch => {
                economic::solve(network, self.max_iterations, self.tolerance)
            }
            OpfMethod::DcOpf => {
                // Try native CLP if preferred and available
                #[cfg(feature = "native-dispatch")]
                if self.prefer_native && native_dispatch::is_clp_available() {
                    return native_dispatch::solve_dc_opf_native(network, self.timeout_seconds);
                }

                // Fall back to pure-Rust Clarabel solver
                dc_opf::solve(network, self.max_iterations, self.tolerance)
            }
            OpfMethod::SocpRelaxation => {
                if self.use_enhanced_socp {
                    let config = socp::SocpSolverConfig {
                        max_iter: self.max_iterations as u32,
                        tol_feas: self.tolerance,
                        tol_gap: self.tolerance,
                        equilibrate: true,
                        verbose: false,
                    };
                    socp::solve_enhanced(network, &config, true, true)
                } else {
                    socp::solve(network, self.max_iterations, self.tolerance)
                }
            }
            OpfMethod::AcOpf => {
                // Try direct IPOPT if solver-ipopt feature is enabled and preferred
                #[cfg(feature = "solver-ipopt")]
                if self.prefer_native || self.require_native {
                    let problem = ac_nlp::AcOpfProblem::from_network(network)?;

                    // Strategy: Try flat-start first, fall back to DC warm-start if needed.
                    //
                    // Flat start works well for small networks (14-57 buses) because
                    // the optimization landscape is relatively smooth. For larger
                    // networks (73+ buses), the flat start may be too far from
                    // feasibility, causing IPOPT to converge to infeasibility or
                    // exceed iteration limits.
                    //
                    // DC warm-start provides generator dispatch and angles that are
                    // approximately correct, giving IPOPT a better starting point.
                    // However, DC ignores voltages and reactive power, so the initial
                    // point may still violate some AC constraints - that's why we
                    // don't enable IPOPT's barrier warm-start options.

                    // First attempt: flat start
                    let flat_result = ac_nlp::solve_with_ipopt(
                        &problem,
                        Some(self.max_iterations),
                        Some(self.tolerance),
                    );

                    match flat_result {
                        Ok(solution) => return Ok(solution),
                        Err(ref flat_error) => {
                            // Check if this is a convergence failure that might benefit
                            // from a better initial point
                            let error_msg = format!("{:?}", flat_error);
                            let is_convergence_failure = error_msg
                                .contains("MaximumIterationsExceeded")
                                || error_msg.contains("InfeasibleProblemDetected")
                                || error_msg.contains("RestorationFailed");

                            if !is_convergence_failure {
                                // Not a convergence issue, propagate the error
                                return flat_result;
                            }

                            // === Fallback 1: DC warm-start ===
                            // DC-OPF provides angles (θ) and generator dispatch (Pg) that
                            // satisfy ∑P = 0 at each bus exactly. This gives IPOPT a
                            // starting point with consistent power flow physics.
                            //
                            // Note: DC ignores voltage magnitudes and reactive power,
                            // so we use flat values (V=1.0, Qg=0) for those variables.
                            if let Ok(dc_solution) =
                                dc_opf::solve(network, self.max_iterations, self.tolerance)
                            {
                                let dc_warm: DcWarmStart = (&dc_solution).into();

                                // Configure IPOPT for DC warm-start - more iterations since
                                // DC solution may be far from AC-feasible for reactive power
                                let mut dc_config = ac_nlp::IpoptConfig::warm_start_from_dc();
                                dc_config.max_iter = 500;
                                dc_config.print_level = 0; // Quiet for fallback

                                if let Ok(solution) =
                                    ac_nlp::solve_with_dc_warm_start(&problem, &dc_warm, &dc_config)
                                {
                                    return Ok(solution);
                                }
                                // DC warm-start failed, try SOCP next
                            }

                            // === Fallback 2: SOCP warm-start ===
                            // SOCP provides V, θ, Pg, Qg but the relaxation allows
                            // constraint slack that may accumulate in larger networks.
                            // Still worth trying as it may work for some cases.
                            let socp_solution =
                                socp::solve(network, self.max_iterations, self.tolerance)?;

                            let socp_warm: SocpWarmStart = (&socp_solution).into();

                            // Use more iterations for SOCP fallback
                            let mut ipopt_config = ac_nlp::IpoptConfig::warm_start_from_socp();
                            ipopt_config.max_iter = 1000; // More iterations for challenging cases
                            ipopt_config.print_level = 0; // Quiet for fallback

                            // Retry with SOCP warm-start
                            match ac_nlp::solve_with_socp_warm_start(
                                &problem,
                                &socp_warm,
                                &ipopt_config,
                            ) {
                                Ok(solution) => return Ok(solution),
                                Err(_) => {
                                    // Both DC and SOCP warm-starts failed, return flat error
                                    return flat_result;
                                }
                            }
                        }
                    }
                }

                // Try native-dispatch IPOPT if preferred and available
                #[cfg(feature = "native-dispatch")]
                if self.prefer_native && native_dispatch::is_ipopt_available() {
                    return native_dispatch::solve_ac_opf_native(network, self.timeout_seconds);
                }

                // Check if native IPOPT is required but not available
                if self.require_native {
                    #[cfg(not(any(feature = "solver-ipopt", feature = "native-dispatch")))]
                    {
                        return Err(OpfError::NotImplemented(
                            "Native IPOPT requested but neither 'solver-ipopt' nor 'native-dispatch' \
                             feature is enabled. Build with: --features solver-ipopt"
                                .to_string(),
                        ));
                    }

                    #[cfg(all(feature = "native-dispatch", not(feature = "solver-ipopt")))]
                    {
                        if !native_dispatch::is_ipopt_available() {
                            return Err(OpfError::NotImplemented(
                                "Native IPOPT requested but not installed. \
                                 Build with: cargo build -p gat-ipopt --features ipopt-sys --release"
                                    .to_string(),
                            ));
                        }
                        // IPOPT is available, dispatch to it
                        return native_dispatch::solve_ac_opf_native(network, self.timeout_seconds);
                    }
                }

                // Fall back to pure-Rust L-BFGS solver
                let problem = ac_nlp::AcOpfProblem::from_network(network)?;
                ac_nlp::solve_ac_opf(&problem, self.max_iterations, self.tolerance)
            }
        }
    }
}

impl Default for OpfSolver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CASCADED SOLVER
// ============================================================================
//
// The cascaded solver implements the "convexity cascade" approach:
// DC-OPF (LP) → SOCP (convex cone) → AC-OPF (NLP)
//
// Each stage warm-starts the next, providing progressively better bounds
// and avoiding cold-start convergence issues.

/// Solve OPF using the cascaded approach with automatic warm-starting.
///
/// The cascade proceeds through convexity levels:
/// 1. DC-OPF (LP): Fast, globally optimal, provides angles and Pg
/// 2. SOCP (convex cone): Better approximation, provides Vm, Va, Pg, Qg
/// 3. AC-OPF (NLP): Full nonlinear solution
///
/// The solver stops at the specified target method.
///
/// # Arguments
/// * `network` - The power network to solve
/// * `target` - Stop at this method level (DC, SOCP, or AC)
/// * `config` - Solver configuration options
///
/// # Returns
/// A `CascadedResult` containing solutions from each computed stage.
///
/// # Example
/// ```ignore
/// // Solve up to SOCP level with DC warm-start
/// let result = solve_cascaded(&network, OpfMethod::SocpRelaxation, &config)?;
/// println!("SOCP objective: {}", result.final_solution.objective_value);
///
/// // Full cascade to AC-OPF
/// let result = solve_cascaded(&network, OpfMethod::AcOpf, &config)?;
/// println!("AC objective: {}", result.final_solution.objective_value);
/// ```
pub fn solve_cascaded(
    network: &Network,
    target: OpfMethod,
    config: &CascadedConfig,
) -> Result<CascadedResult, OpfError> {
    use std::time::Instant;
    let start = Instant::now();

    let mut result = CascadedResult::default();

    // Stage 1: DC-OPF (always run as warm-start for higher stages)
    let dc_solver = OpfSolver::new()
        .with_method(OpfMethod::DcOpf)
        .with_max_iterations(config.max_iterations)
        .with_tolerance(config.tolerance);

    let dc_solution = if config.use_loss_factors {
        dc_opf::solve_with_losses(network, 3, config.max_iterations, config.tolerance)?
    } else {
        dc_solver.solve(network)?
    };

    result.dc_solution = Some(dc_solution.clone());

    if target == OpfMethod::DcOpf || target == OpfMethod::EconomicDispatch {
        result.final_solution = dc_solution;
        result.total_time_ms = start.elapsed().as_millis();
        return Ok(result);
    }

    // Stage 2: SOCP with DC warm-start
    let _dc_warm: DcWarmStart = (&dc_solution).into();

    // Use enhanced SOCP if configured (OBBT + QC envelopes for tighter relaxation)
    let socp_solver = OpfSolver::new()
        .with_method(OpfMethod::SocpRelaxation)
        .with_max_iterations(config.max_iterations)
        .with_tolerance(config.tolerance)
        .enhanced_socp(config.use_enhanced_socp);

    let socp_solution = socp_solver.solve(network)?;
    result.socp_solution = Some(socp_solution.clone());

    if target == OpfMethod::SocpRelaxation {
        result.final_solution = socp_solution;
        result.total_time_ms = start.elapsed().as_millis();
        return Ok(result);
    }

    // Stage 3: AC-OPF with SOCP warm-start
    let socp_warm: SocpWarmStart = (&socp_solution).into();

    // Use IPOPT with warm-start if available and requested
    #[cfg(feature = "solver-ipopt")]
    let ac_solution = if config.prefer_native {
        // Build AC problem and solve with SOCP warm-start
        let problem = ac_nlp::AcOpfProblem::from_network(network)?;
        let ipopt_config = ac_nlp::IpoptConfig {
            max_iter: config.max_iterations as i32,
            tol: config.tolerance,
            warm_start: true,
            print_level: 5, // Debug: show IPOPT iteration progress
            ..Default::default()
        };
        ac_nlp::solve_with_socp_warm_start(&problem, &socp_warm, &ipopt_config)?
    } else {
        // Fallback to L-BFGS solver
        let ac_solver = OpfSolver::new()
            .with_method(OpfMethod::AcOpf)
            .with_max_iterations(config.max_iterations)
            .with_tolerance(config.tolerance);
        ac_solver.solve(network)?
    };

    // Without IPOPT, use L-BFGS (warm-start via initial point)
    #[cfg(not(feature = "solver-ipopt"))]
    let ac_solution = {
        let problem = ac_nlp::AcOpfProblem::from_network(network)?;
        let bus_order: Vec<String> = problem.buses.iter().map(|b| b.name.clone()).collect();
        let gen_order: Vec<String> = problem.generators.iter().map(|g| g.name.clone()).collect();
        let initial_point = socp_warm.to_vec(&bus_order, &gen_order);
        ac_nlp::solve_ac_opf_warm_start(
            &problem,
            initial_point,
            config.max_iterations,
            config.tolerance,
        )?
    };

    result.ac_solution = Some(ac_solution.clone());
    result.final_solution = ac_solution;
    result.total_time_ms = start.elapsed().as_millis();

    Ok(result)
}

/// Configuration for cascaded OPF solving.
#[derive(Debug, Clone)]
pub struct CascadedConfig {
    /// Maximum iterations per solver stage
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Whether to use loss factors in DC-OPF stage
    pub use_loss_factors: bool,
    /// Whether to prefer native solvers (IPOPT) for AC stage
    pub prefer_native: bool,
    /// Whether to use enhanced SOCP with OBBT and QC envelopes
    pub use_enhanced_socp: bool,
    /// Timeout per stage in seconds
    pub timeout_seconds: u64,
}

impl Default for CascadedConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
            use_loss_factors: true,
            prefer_native: true,
            use_enhanced_socp: false, // Disabled by default (adds overhead)
            timeout_seconds: 300,
        }
    }
}
