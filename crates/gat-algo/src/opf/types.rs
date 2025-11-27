use std::collections::HashMap;
use std::fmt;

use serde::Serialize;

/// OPF solution method
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum OpfMethod {
    /// Merit-order economic dispatch (no network constraints)
    EconomicDispatch,
    /// DC optimal power flow (LP with B-matrix)
    DcOpf,
    /// Second-order cone relaxation of AC-OPF
    #[default]
    SocpRelaxation,
    /// Full nonlinear AC-OPF (interior point) - not yet implemented
    AcOpf,
}

impl fmt::Display for OpfMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpfMethod::EconomicDispatch => write!(f, "economic"),
            OpfMethod::DcOpf => write!(f, "dc"),
            OpfMethod::SocpRelaxation => write!(f, "socp"),
            OpfMethod::AcOpf => write!(f, "ac"),
        }
    }
}

impl std::str::FromStr for OpfMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "economic" | "fast" => Ok(OpfMethod::EconomicDispatch),
            "dc" | "balanced" => Ok(OpfMethod::DcOpf),
            "socp" | "accurate" => Ok(OpfMethod::SocpRelaxation),
            "ac" => Ok(OpfMethod::AcOpf),
            _ => Err(format!("Unknown OPF method: {}", s)),
        }
    }
}

/// Type of constraint for reporting
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ConstraintType {
    GeneratorPMax,
    GeneratorPMin,
    GeneratorQMax,
    GeneratorQMin,
    BranchFlowLimit,
    VoltageMax,
    VoltageMin,
    PowerBalance,
}

/// Information about a binding or violated constraint
#[derive(Debug, Clone, Serialize)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub value: f64,
    pub limit: f64,
    pub shadow_price: f64,
}

/// OPF solution output
#[derive(Debug, Clone, Serialize)]
pub struct OpfSolution {
    // === Status ===
    pub converged: bool,
    pub method_used: OpfMethod,
    pub iterations: usize,
    pub solve_time_ms: u128,

    // === Objective ===
    pub objective_value: f64,

    // === Primal Variables ===
    pub generator_p: HashMap<String, f64>,
    pub generator_q: HashMap<String, f64>,
    pub bus_voltage_mag: HashMap<String, f64>,
    pub bus_voltage_ang: HashMap<String, f64>,
    pub branch_p_flow: HashMap<String, f64>,
    pub branch_q_flow: HashMap<String, f64>,

    // === Dual Variables ===
    pub bus_lmp: HashMap<String, f64>,

    // === Constraint Info ===
    pub binding_constraints: Vec<ConstraintInfo>,
    pub total_losses_mw: f64,
}

impl Default for OpfSolution {
    fn default() -> Self {
        Self {
            converged: false,
            method_used: OpfMethod::default(),
            iterations: 0,
            solve_time_ms: 0,
            objective_value: 0.0,
            generator_p: HashMap::new(),
            generator_q: HashMap::new(),
            bus_voltage_mag: HashMap::new(),
            bus_voltage_ang: HashMap::new(),
            branch_p_flow: HashMap::new(),
            branch_q_flow: HashMap::new(),
            bus_lmp: HashMap::new(),
            binding_constraints: Vec::new(),
            total_losses_mw: 0.0,
        }
    }
}
