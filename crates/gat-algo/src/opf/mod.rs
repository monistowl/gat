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

pub use dispatch::{DispatchConfig, ProblemClass, SolverBackend, SolverDispatcher};
pub use types::{ConstraintInfo, ConstraintType, OpfMethod, OpfSolution};

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
            OpfMethod::SocpRelaxation => socp::solve(network, self.max_iterations, self.tolerance),
            OpfMethod::AcOpf => {
                // Check if native IPOPT is required but not available
                if self.require_native {
                    #[cfg(not(feature = "native-dispatch"))]
                    {
                        return Err(OpfError::NotImplemented(
                            "Native IPOPT requested but 'native-dispatch' feature not enabled. \
                             Either install IPOPT (`cargo xtask solver build ipopt --install`) \
                             or use the pure-Rust solver by removing require_native(true)."
                                .to_string(),
                        ));
                    }

                    #[cfg(feature = "native-dispatch")]
                    {
                        if native_dispatch::is_ipopt_available() {
                            return native_dispatch::solve_ac_opf_native(
                                network,
                                self.timeout_seconds,
                            );
                        }
                        // Fall through to L-BFGS if IPOPT not installed
                    }
                }

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

