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
//! By default, AC-OPF uses the pure-Rust L-BFGS solver. To require native
//! IPOPT (and fail if unavailable), use `require_native(true)`:
//!
//! ```ignore
//! let solver = OpfSolver::new()
//!     .with_method(OpfMethod::AcOpf)
//!     .require_native(true);  // Fails if IPOPT not installed
//! ```

pub mod ac_nlp;
pub mod dispatch;
mod dc_opf;
mod economic;
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
    /// If true, fail when native solver requested but not available.
    /// If false (default), silently fall back to pure-Rust solver.
    require_native: bool,
}

impl OpfSolver {
    /// Create new OPF solver with default settings (SOCP method)
    pub fn new() -> Self {
        Self {
            method: OpfMethod::default(),
            max_iterations: 100,
            tolerance: 1e-6,
            require_native: false,
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
            OpfMethod::DcOpf => dc_opf::solve(network, self.max_iterations, self.tolerance),
            OpfMethod::SocpRelaxation => socp::solve(network, self.max_iterations, self.tolerance),
            OpfMethod::AcOpf => {
                // Check if native IPOPT is required but not available
                if self.require_native {
                    #[cfg(not(feature = "native-dispatch"))]
                    {
                        return Err(OpfError::NotImplemented(
                            "Native IPOPT requested but 'native-dispatch' feature not enabled. \
                             Either install IPOPT (`cargo xtask solver build ipopt --install`) \
                             or use the pure-Rust solver by removing require_native(true).".to_string()
                        ));
                    }

                    #[cfg(feature = "native-dispatch")]
                    {
                        // Check if IPOPT is actually installed
                        if !is_native_solver_available("ipopt") {
                            return Err(OpfError::NotImplemented(
                                "Native IPOPT requested but not installed. \
                                 Install with: cargo xtask solver build ipopt --install".to_string()
                            ));
                        }
                        // TODO: Actually dispatch to native IPOPT via IPC
                        // For now, fall through to L-BFGS with a warning
                        eprintln!("Warning: Native IPOPT requested but IPC dispatch not yet implemented, using L-BFGS");
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

/// Check if a native solver is available (installed and enabled).
#[cfg(feature = "native-dispatch")]
fn is_native_solver_available(name: &str) -> bool {
    // Check the solvers state file
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };

    let state_path = home.join(".gat").join("config").join("solvers.toml");
    if !state_path.exists() {
        return false;
    }

    // Parse the state file
    let contents = match std::fs::read_to_string(&state_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Check if solver is in the installed table
    contents.contains(&format!("[installed.{}]", name))
}
