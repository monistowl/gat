use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// OPF solution method
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

// ============================================================================
// WARM-START INFRASTRUCTURE
// ============================================================================
//
// These types enable cascaded solving where solutions from simpler problems
// (DC-OPF, SOCP) warm-start more complex problems (SOCP, AC-OPF).
//
// The "convexity cascade" approach:
// DC-OPF (LP, fast) → SOCP (convex cone) → AC-OPF (NLP)
//
// Each stage provides progressively better initial points, avoiding
// cold-start convergence issues in the nonlinear solver.

/// Warm-start data for SOCP from DC-OPF solution.
///
/// DC-OPF provides bus angles and generator real power outputs,
/// which can initialize SOCP's angle and Pg variables.
///
/// # Fields
/// - `bus_angles`: Voltage angles in radians (from DC power flow)
/// - `generator_p`: Real power dispatch in MW
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcWarmStart {
    /// Bus voltage angles in radians (θ)
    pub bus_angles: HashMap<String, f64>,
    /// Generator real power output in MW
    pub generator_p: HashMap<String, f64>,
}

impl From<&OpfSolution> for DcWarmStart {
    fn from(sol: &OpfSolution) -> Self {
        Self {
            bus_angles: sol.bus_voltage_ang.clone(),
            generator_p: sol.generator_p.clone(),
        }
    }
}

/// Warm-start data for AC-OPF from SOCP solution.
///
/// SOCP provides a full AC-feasible (or near-feasible) solution
/// including voltage magnitudes, angles, and reactive power.
/// This is excellent initialization for IPOPT or L-BFGS.
///
/// # Fields
/// - `bus_voltage_mag`: Voltage magnitudes in p.u.
/// - `bus_voltage_angle`: Voltage angles in degrees
/// - `generator_p`: Real power dispatch in MW
/// - `generator_q`: Reactive power dispatch in MVAr
/// - `branch_p_flow`: Real power flow in MW
/// - `branch_q_flow`: Reactive power flow in MVAr
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocpWarmStart {
    /// Bus voltage magnitudes in per-unit
    pub bus_voltage_mag: HashMap<String, f64>,
    /// Bus voltage angles in degrees
    pub bus_voltage_angle: HashMap<String, f64>,
    /// Generator real power output in MW
    pub generator_p: HashMap<String, f64>,
    /// Generator reactive power output in MVAr
    pub generator_q: HashMap<String, f64>,
    /// Branch real power flow in MW
    pub branch_p_flow: HashMap<String, f64>,
    /// Branch reactive power flow in MVAr
    pub branch_q_flow: HashMap<String, f64>,
}

impl From<&OpfSolution> for SocpWarmStart {
    fn from(sol: &OpfSolution) -> Self {
        Self {
            bus_voltage_mag: sol.bus_voltage_mag.clone(),
            bus_voltage_angle: sol.bus_voltage_ang.clone(),
            generator_p: sol.generator_p.clone(),
            generator_q: sol.generator_q.clone(),
            branch_p_flow: sol.branch_p_flow.clone(),
            branch_q_flow: sol.branch_q_flow.clone(),
        }
    }
}

impl SocpWarmStart {
    /// Convert warm-start data to a flat vector for NLP solvers.
    ///
    /// The vector layout matches the standard AC-OPF variable ordering:
    /// [Vm(buses), Va(buses), Pg(gens), Qg(gens)]
    ///
    /// # Arguments
    /// * `bus_order` - Ordered list of bus names for consistent indexing
    /// * `gen_order` - Ordered list of generator names for consistent indexing
    pub fn to_vec(&self, bus_order: &[String], gen_order: &[String]) -> Vec<f64> {
        let n_bus = bus_order.len();
        let n_gen = gen_order.len();
        let mut x = vec![0.0; 2 * n_bus + 2 * n_gen];

        // Voltage magnitudes (default to 1.0 if missing)
        for (i, name) in bus_order.iter().enumerate() {
            x[i] = self.bus_voltage_mag.get(name).copied().unwrap_or(1.0);
        }

        // Voltage angles in radians (convert from degrees)
        for (i, name) in bus_order.iter().enumerate() {
            let angle_deg = self.bus_voltage_angle.get(name).copied().unwrap_or(0.0);
            x[n_bus + i] = angle_deg.to_radians();
        }

        // Generator real power (default to midpoint of typical range)
        for (i, name) in gen_order.iter().enumerate() {
            x[2 * n_bus + i] = self.generator_p.get(name).copied().unwrap_or(0.0);
        }

        // Generator reactive power
        for (i, name) in gen_order.iter().enumerate() {
            x[2 * n_bus + n_gen + i] = self.generator_q.get(name).copied().unwrap_or(0.0);
        }

        x
    }
}

/// Result from cascaded OPF solving.
///
/// Contains solutions from each stage of the cascade, allowing
/// analysis of how the solution evolves through refinement stages.
#[derive(Debug, Clone, Default)]
pub struct CascadedResult {
    /// DC-OPF solution (if computed)
    pub dc_solution: Option<OpfSolution>,
    /// SOCP solution (if computed)
    pub socp_solution: Option<OpfSolution>,
    /// AC-OPF solution (if computed)
    pub ac_solution: Option<OpfSolution>,
    /// Final solution (best available)
    pub final_solution: OpfSolution,
    /// Total solve time across all stages in milliseconds
    pub total_time_ms: u128,
}

impl From<OpfSolution> for CascadedResult {
    fn from(sol: OpfSolution) -> Self {
        let method = sol.method_used;
        let time = sol.solve_time_ms;
        let mut result = CascadedResult {
            final_solution: sol.clone(),
            total_time_ms: time,
            ..Default::default()
        };

        match method {
            OpfMethod::DcOpf => result.dc_solution = Some(sol),
            OpfMethod::SocpRelaxation => result.socp_solution = Some(sol),
            OpfMethod::AcOpf => result.ac_solution = Some(sol),
            _ => {}
        }

        result
    }
}
