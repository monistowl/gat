//! Problem representation for solver IPC.
//!
//! Defines the data structures sent from gat to solver plugins.

use serde::{Deserialize, Serialize};

/// Type of optimization problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemType {
    /// AC Optimal Power Flow (nonlinear)
    AcOpf,
    /// DC Optimal Power Flow (linear approximation)
    DcOpf,
    /// Linear Program
    Lp,
    /// Second-Order Cone Program
    Socp,
    /// Mixed-Integer Program
    Mip,
    /// Mixed-Integer Nonlinear Program
    Minlp,
}

impl std::fmt::Display for ProblemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProblemType::AcOpf => write!(f, "AC-OPF"),
            ProblemType::DcOpf => write!(f, "DC-OPF"),
            ProblemType::Lp => write!(f, "LP"),
            ProblemType::Socp => write!(f, "SOCP"),
            ProblemType::Mip => write!(f, "MIP"),
            ProblemType::Minlp => write!(f, "MINLP"),
        }
    }
}

/// Problem batch for IPC transmission.
///
/// This structure holds all the data needed to solve an OPF problem,
/// serialized as Arrow arrays for efficient IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemBatch {
    /// Type of problem being solved.
    pub problem_type: ProblemType,

    /// Protocol version for compatibility checking.
    pub protocol_version: i32,

    /// System base MVA.
    pub base_mva: f64,

    /// Convergence tolerance.
    pub tolerance: f64,

    /// Maximum solver iterations.
    pub max_iterations: i32,

    /// Timeout in seconds (0 = no timeout).
    pub timeout_seconds: u64,

    // === Bus data ===
    /// Bus IDs (1-indexed).
    pub bus_id: Vec<i64>,
    /// Bus names (optional).
    pub bus_name: Vec<String>,
    /// Minimum voltage magnitude (p.u.).
    pub bus_v_min: Vec<f64>,
    /// Maximum voltage magnitude (p.u.).
    pub bus_v_max: Vec<f64>,
    /// Active power load (MW).
    pub bus_p_load: Vec<f64>,
    /// Reactive power load (MVAr).
    pub bus_q_load: Vec<f64>,
    /// Bus type: 1=PQ, 2=PV, 3=Slack.
    pub bus_type: Vec<i32>,
    /// Initial voltage magnitude (p.u.).
    pub bus_v_mag: Vec<f64>,
    /// Initial voltage angle (radians).
    pub bus_v_ang: Vec<f64>,

    // === Generator data ===
    /// Generator IDs.
    pub gen_id: Vec<i64>,
    /// Bus ID where generator is connected.
    pub gen_bus_id: Vec<i64>,
    /// Minimum active power output (MW).
    pub gen_p_min: Vec<f64>,
    /// Maximum active power output (MW).
    pub gen_p_max: Vec<f64>,
    /// Minimum reactive power output (MVAr).
    pub gen_q_min: Vec<f64>,
    /// Maximum reactive power output (MVAr).
    pub gen_q_max: Vec<f64>,
    /// Cost coefficient c0 ($/hr).
    pub gen_cost_c0: Vec<f64>,
    /// Cost coefficient c1 ($/MWh).
    pub gen_cost_c1: Vec<f64>,
    /// Cost coefficient c2 ($/MW^2h).
    pub gen_cost_c2: Vec<f64>,
    /// Voltage setpoint (p.u.).
    pub gen_v_setpoint: Vec<f64>,
    /// Generator status (1=on, 0=off).
    pub gen_status: Vec<i32>,

    // === Branch data ===
    /// Branch IDs.
    pub branch_id: Vec<i64>,
    /// From bus ID.
    pub branch_from: Vec<i64>,
    /// To bus ID.
    pub branch_to: Vec<i64>,
    /// Resistance (p.u.).
    pub branch_r: Vec<f64>,
    /// Reactance (p.u.).
    pub branch_x: Vec<f64>,
    /// Total line charging susceptance (p.u.).
    pub branch_b: Vec<f64>,
    /// MVA rating (0 = unlimited).
    pub branch_rate: Vec<f64>,
    /// Transformer tap ratio (1.0 for lines).
    pub branch_tap: Vec<f64>,
    /// Transformer phase shift (radians).
    pub branch_shift: Vec<f64>,
    /// Branch status (1=on, 0=off).
    pub branch_status: Vec<i32>,
}

impl Default for ProblemBatch {
    fn default() -> Self {
        Self::new(ProblemType::AcOpf)
    }
}

impl ProblemBatch {
    /// Create an empty problem batch with default settings.
    pub fn new(problem_type: ProblemType) -> Self {
        Self {
            problem_type,
            protocol_version: crate::PROTOCOL_VERSION,
            base_mva: 100.0,
            tolerance: 1e-6,
            max_iterations: 100,
            timeout_seconds: 0,
            bus_id: Vec::new(),
            bus_name: Vec::new(),
            bus_v_min: Vec::new(),
            bus_v_max: Vec::new(),
            bus_p_load: Vec::new(),
            bus_q_load: Vec::new(),
            bus_type: Vec::new(),
            bus_v_mag: Vec::new(),
            bus_v_ang: Vec::new(),
            gen_id: Vec::new(),
            gen_bus_id: Vec::new(),
            gen_p_min: Vec::new(),
            gen_p_max: Vec::new(),
            gen_q_min: Vec::new(),
            gen_q_max: Vec::new(),
            gen_cost_c0: Vec::new(),
            gen_cost_c1: Vec::new(),
            gen_cost_c2: Vec::new(),
            gen_v_setpoint: Vec::new(),
            gen_status: Vec::new(),
            branch_id: Vec::new(),
            branch_from: Vec::new(),
            branch_to: Vec::new(),
            branch_r: Vec::new(),
            branch_x: Vec::new(),
            branch_b: Vec::new(),
            branch_rate: Vec::new(),
            branch_tap: Vec::new(),
            branch_shift: Vec::new(),
            branch_status: Vec::new(),
        }
    }

    /// Number of buses in the problem.
    pub fn num_buses(&self) -> usize {
        self.bus_id.len()
    }

    /// Number of generators in the problem.
    pub fn num_generators(&self) -> usize {
        self.gen_id.len()
    }

    /// Number of branches in the problem.
    pub fn num_branches(&self) -> usize {
        self.branch_id.len()
    }
}
