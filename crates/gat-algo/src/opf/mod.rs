//! Optimal Power Flow solvers
//!
//! This module provides OPF solvers with multiple solution methods:
//! - Economic dispatch (merit-order, no network)
//! - DC-OPF (linearized power flow)
//! - SOCP relaxation (convex AC approximation)
//! - AC-OPF (full nonlinear, future)

mod types;
mod economic;
mod dc_opf;

pub use types::{ConstraintInfo, ConstraintType, OpfMethod, OpfSolution};

use crate::OpfError;
use gat_core::Network;

/// Unified OPF solver supporting multiple solution methods
pub struct OpfSolver {
    method: OpfMethod,
    max_iterations: usize,
    tolerance: f64,
}

impl OpfSolver {
    /// Create new OPF solver with default settings (SOCP method)
    pub fn new() -> Self {
        Self {
            method: OpfMethod::default(),
            max_iterations: 100,
            tolerance: 1e-6,
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

    /// Get the configured method
    pub fn method(&self) -> OpfMethod {
        self.method
    }

    /// Solve OPF for the given network
    pub fn solve(&self, network: &Network) -> Result<OpfSolution, OpfError> {
        match self.method {
            OpfMethod::EconomicDispatch => economic::solve(network, self.max_iterations, self.tolerance),
            OpfMethod::DcOpf => dc_opf::solve(network, self.max_iterations, self.tolerance),
            OpfMethod::SocpRelaxation => Err(OpfError::NotImplemented("SOCP not yet implemented".into())),
            OpfMethod::AcOpf => Err(OpfError::NotImplemented("AC-OPF not yet implemented".into())),
        }
    }
}

impl Default for OpfSolver {
    fn default() -> Self {
        Self::new()
    }
}
