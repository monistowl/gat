//! Solution representation for solver IPC.
//!
//! Defines the data structures returned from solver plugins to gat.

use serde::{Deserialize, Serialize};

/// Status of the solver solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolutionStatus {
    /// Optimal solution found.
    Optimal,
    /// Problem is infeasible.
    Infeasible,
    /// Problem is unbounded.
    Unbounded,
    /// Solver timed out.
    Timeout,
    /// Solver hit iteration limit.
    IterationLimit,
    /// Numerical difficulties.
    NumericalError,
    /// Generic error occurred.
    Error,
    /// Solution status unknown.
    Unknown,
}

impl SolutionStatus {
    /// Check if this status represents a successful solve.
    pub fn is_success(&self) -> bool {
        matches!(self, SolutionStatus::Optimal)
    }

    /// Check if this status represents a failure.
    pub fn is_failure(&self) -> bool {
        !self.is_success() && !matches!(self, SolutionStatus::Unknown)
    }
}

impl std::fmt::Display for SolutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolutionStatus::Optimal => write!(f, "optimal"),
            SolutionStatus::Infeasible => write!(f, "infeasible"),
            SolutionStatus::Unbounded => write!(f, "unbounded"),
            SolutionStatus::Timeout => write!(f, "timeout"),
            SolutionStatus::IterationLimit => write!(f, "iteration_limit"),
            SolutionStatus::NumericalError => write!(f, "numerical_error"),
            SolutionStatus::Error => write!(f, "error"),
            SolutionStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Solution batch from solver IPC.
///
/// This structure holds the solution data returned by the solver,
/// serialized as Arrow arrays for efficient IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionBatch {
    /// Solution status.
    pub status: SolutionStatus,

    /// Objective value (total cost $/hr for OPF).
    pub objective: f64,

    /// Number of iterations performed.
    pub iterations: i32,

    /// Solve time in milliseconds.
    pub solve_time_ms: i64,

    /// Error message (if status is error/infeasible).
    pub error_message: Option<String>,

    // === Bus results ===
    /// Bus IDs (matching input order).
    pub bus_id: Vec<i64>,
    /// Voltage magnitude (p.u.).
    pub bus_v_mag: Vec<f64>,
    /// Voltage angle (degrees).
    pub bus_v_ang: Vec<f64>,
    /// Locational marginal price ($/MWh).
    pub bus_lmp: Vec<f64>,

    // === Generator results ===
    /// Generator IDs (matching input order).
    pub gen_id: Vec<i64>,
    /// Active power output (MW).
    pub gen_p: Vec<f64>,
    /// Reactive power output (MVAr).
    pub gen_q: Vec<f64>,

    // === Branch results (optional) ===
    /// Branch IDs (matching input order).
    pub branch_id: Vec<i64>,
    /// Active power flow at from bus (MW).
    pub branch_p_from: Vec<f64>,
    /// Reactive power flow at from bus (MVAr).
    pub branch_q_from: Vec<f64>,
    /// Active power flow at to bus (MW).
    pub branch_p_to: Vec<f64>,
    /// Reactive power flow at to bus (MVAr).
    pub branch_q_to: Vec<f64>,
}

impl SolutionBatch {
    /// Create an empty solution with error status.
    pub fn error(message: &str) -> Self {
        Self {
            status: SolutionStatus::Error,
            objective: f64::NAN,
            iterations: 0,
            solve_time_ms: 0,
            error_message: Some(message.to_string()),
            bus_id: Vec::new(),
            bus_v_mag: Vec::new(),
            bus_v_ang: Vec::new(),
            bus_lmp: Vec::new(),
            gen_id: Vec::new(),
            gen_p: Vec::new(),
            gen_q: Vec::new(),
            branch_id: Vec::new(),
            branch_p_from: Vec::new(),
            branch_q_from: Vec::new(),
            branch_p_to: Vec::new(),
            branch_q_to: Vec::new(),
        }
    }

    /// Create an infeasible solution.
    pub fn infeasible(message: &str) -> Self {
        Self {
            status: SolutionStatus::Infeasible,
            error_message: Some(message.to_string()),
            ..Self::error(message)
        }
    }

    /// Create a timeout solution.
    pub fn timeout(seconds: u64) -> Self {
        Self {
            status: SolutionStatus::Timeout,
            error_message: Some(format!("Solver timed out after {} seconds", seconds)),
            ..Self::error("")
        }
    }

    /// Check if solution is optimal.
    pub fn is_optimal(&self) -> bool {
        self.status.is_success()
    }

    /// Number of buses in the solution.
    pub fn num_buses(&self) -> usize {
        self.bus_id.len()
    }

    /// Number of generators in the solution.
    pub fn num_generators(&self) -> usize {
        self.gen_id.len()
    }

    /// Number of branches in the solution.
    pub fn num_branches(&self) -> usize {
        self.branch_id.len()
    }
}

impl Default for SolutionBatch {
    fn default() -> Self {
        Self::error("No solution")
    }
}
