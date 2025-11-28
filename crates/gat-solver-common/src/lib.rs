//! Common types and IPC protocol for GAT solver plugins.
//!
//! This crate defines the Arrow IPC schema for communication between the main
//! `gat` binary and external solver plugins (e.g., `gat-ipopt`, `gat-cbc`).
//!
//! # Architecture
//!
//! ```text
//! gat (main) ──stdin──> gat-ipopt (subprocess)
//!            <─stdout──
//!            <─stderr── (logs/errors)
//! ```
//!
//! Communication uses Arrow IPC format for structured data transfer.

pub mod error;
pub mod ipc;
pub mod problem;
pub mod solution;
pub mod subprocess;

pub use error::{ExitCode, SolverError};
pub use problem::{ProblemBatch, ProblemType};
pub use solution::{SolutionBatch, SolutionStatus};
pub use subprocess::SolverProcess;

/// Protocol version for IPC compatibility checking.
/// Increment when making breaking changes to the schema.
pub const PROTOCOL_VERSION: i32 = 1;

/// Available native solvers that can be installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SolverId {
    /// IPOPT - Interior Point Optimizer for NLP
    Ipopt,
    /// CBC - COIN-OR Branch and Cut for MIP
    Cbc,
    /// HiGHS - High-performance LP/MIP solver
    Highs,
    /// Bonmin - Basic Open-source Nonlinear Mixed INteger programming
    Bonmin,
    /// Couenne - Convex Over and Under ENvelopes for Nonlinear Estimation
    Couenne,
    /// SYMPHONY - COIN-OR parallel MIP solver
    Symphony,
}

impl SolverId {
    /// Get the binary name for this solver.
    pub fn binary_name(&self) -> &'static str {
        match self {
            SolverId::Ipopt => "gat-ipopt",
            SolverId::Cbc => "gat-cbc",
            SolverId::Highs => "gat-highs",
            SolverId::Bonmin => "gat-bonmin",
            SolverId::Couenne => "gat-couenne",
            SolverId::Symphony => "gat-symphony",
        }
    }

    /// Get the display name for this solver.
    pub fn display_name(&self) -> &'static str {
        match self {
            SolverId::Ipopt => "IPOPT",
            SolverId::Cbc => "CBC",
            SolverId::Highs => "HiGHS",
            SolverId::Bonmin => "Bonmin",
            SolverId::Couenne => "Couenne",
            SolverId::Symphony => "SYMPHONY",
        }
    }

    /// Get a description of what this solver does.
    pub fn description(&self) -> &'static str {
        match self {
            SolverId::Ipopt => "NLP interior-point optimizer",
            SolverId::Cbc => "MIP branch-and-cut",
            SolverId::Highs => "LP/MIP high-performance",
            SolverId::Bonmin => "MINLP branch-and-bound",
            SolverId::Couenne => "Global optimization",
            SolverId::Symphony => "Parallel MIP",
        }
    }

    /// Get all available solver IDs.
    pub fn all() -> &'static [SolverId] {
        &[
            SolverId::Ipopt,
            SolverId::Cbc,
            SolverId::Highs,
            SolverId::Bonmin,
            SolverId::Couenne,
            SolverId::Symphony,
        ]
    }
}

impl std::fmt::Display for SolverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for SolverId {
    type Err = SolverError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ipopt" => Ok(SolverId::Ipopt),
            "cbc" => Ok(SolverId::Cbc),
            "highs" => Ok(SolverId::Highs),
            "bonmin" => Ok(SolverId::Bonmin),
            "couenne" => Ok(SolverId::Couenne),
            "symphony" => Ok(SolverId::Symphony),
            _ => Err(SolverError::UnknownSolver(s.to_string())),
        }
    }
}

/// Pure-Rust solvers that are always available (no native code).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PureRustSolver {
    /// Clarabel - Conic solver (SOCP, SDP)
    Clarabel,
    /// L-BFGS - Limited-memory BFGS for NLP
    Lbfgs,
}

impl PureRustSolver {
    pub fn display_name(&self) -> &'static str {
        match self {
            PureRustSolver::Clarabel => "Clarabel",
            PureRustSolver::Lbfgs => "L-BFGS",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PureRustSolver::Clarabel => "Conic (SOCP, SDP)",
            PureRustSolver::Lbfgs => "NLP penalty method",
        }
    }
}

/// The resolved solver choice for a problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverChoice {
    /// Use a native solver plugin (subprocess).
    Native(SolverId),
    /// Use a pure-Rust solver (in-process).
    PureRust(PureRustSolver),
}

impl std::fmt::Display for SolverChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverChoice::Native(id) => write!(f, "{}", id),
            SolverChoice::PureRust(solver) => write!(f, "{}", solver.display_name()),
        }
    }
}
