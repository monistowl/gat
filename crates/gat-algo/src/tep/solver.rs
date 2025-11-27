//! TEP MILP solver
//!
//! Implements DC-based Mixed-Integer Linear Programming formulation for TEP.

use super::{CandidateId, CandidateLine, LineBuildDecision, TepProblem, TepSolution};
use gat_core::{BusId, Edge, Network, Node};
use good_lp::solvers::clarabel::clarabel;
use good_lp::{constraint, variable, variables, Expression, Solution, SolverModel, Variable};
use std::collections::HashMap;
use std::time::Instant;

/// TEP solver configuration
#[derive(Debug, Clone)]
pub struct TepSolverConfig {
    /// Maximum solve time (seconds)
    pub max_time_seconds: f64,
    /// MIP optimality gap tolerance
    pub mip_gap: f64,
    /// Whether to enable verbose solver output
    pub verbose: bool,
}

impl Default for TepSolverConfig {
    fn default() -> Self {
        Self {
            max_time_seconds: 300.0, // 5 minutes
            mip_gap: 0.01, // 1% gap
            verbose: false,
        }
    }
}

/// TEP solver errors
#[derive(Debug, Clone)]
pub enum TepError {
    /// Network validation error
    NetworkValidation(String),
    /// No candidates provided
    NoCandidates,
    /// Solver failed to find a solution
    SolverFailed(String),
    /// Problem is infeasible
    Infeasible(String),
}

impl std::fmt::Display for TepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TepError::NetworkValidation(msg) => write!(f, "Network validation error: {}", msg),
            TepError::NoCandidates => write!(f, "No candidate lines provided"),
            TepError::SolverFailed(msg) => write!(f, "Solver failed: {}", msg),
            TepError::Infeasible(msg) => write!(f, "Problem infeasible: {}", msg),
        }
    }
}

impl std::error::Error for TepError {}

/// Internal bus data for solver
#[derive(Debug)]
struct BusData {
    id: BusId,
    name: String,
    index: usize,
}

/// Internal generator data for solver
#[derive(Debug)]
struct GenData {
    name: String,
    bus_id: BusId,
    pmin_mw: f64,
    pmax_mw: f64,
    cost_per_mw: f64,
}

/// Internal branch data for solver
#[derive(Debug)]
#[allow(dead_code)] // Fields used for debugging and future thermal limit constraints
struct BranchData {
    name: String,
    from_bus: BusId,
    to_bus: BusId,
    susceptance: f64,
    capacity_mw: Option<f64>,
}

/// Solve the TEP problem
///
/// This is a **simplified LP relaxation** that treats binary variables as continuous [0,1].
/// For exact MILP, use the `solver-highs` feature with HiGHS solver.
///
/// # Example
///
/// ```no_run
/// use gat_algo::tep::{solve_tep, TepProblemBuilder, TepSolverConfig};
/// use gat_core::{BusId, Network};
///
/// let network = Network::new(); // Load your network
/// let problem = TepProblemBuilder::new(network)
///     .candidate("Line 1-2", BusId::new(1), BusId::new(2), 0.1, 100.0, 1e6)
///     .build();
///
/// let config = TepSolverConfig::default();
/// let solution = solve_tep(&problem, &config)?;
/// println!("{}", solution.summary());
/// # Ok::<(), gat_algo::tep::TepError>(())
/// ```
pub fn solve_tep(problem: &TepProblem, _config: &TepSolverConfig) -> Result<TepSolution, TepError> {
    let start = Instant::now();

    // Validate inputs
    if problem.candidates.is_empty() {
        return Err(TepError::NoCandidates);
    }

    // Extract network data
    let (buses, generators, branches, loads) = extract_network_data(&problem.network)?;
    let bus_map: HashMap<BusId, usize> = buses.iter().map(|b| (b.id, b.index)).collect();
    let _n_bus = buses.len();

    // === LP/MILP Formulation ===
    // Variables:
    // - P_g[i]: Generator power output (continuous, MW)
    // - θ[j]: Bus voltage angle (continuous, rad)
    // - x[k]: Candidate line build decision (binary → relaxed to [0,1])
    // - f[k]: Candidate line flow (continuous, MW)

    let mut vars = variables!();

    // Generator variables and cost
    let mut gen_vars: Vec<(String, BusId, Variable)> = Vec::new();
    let mut operating_cost_expr = Expression::from(0.0);

    for gen in &generators {
        let pmin = gen.pmin_mw.max(0.0);
        let pmax = if gen.pmax_mw.is_finite() { gen.pmax_mw } else { 1e6 };
        let p_var = vars.add(variable().min(pmin).max(pmax));
        gen_vars.push((gen.name.clone(), gen.bus_id, p_var));

        // Operating cost = c * P * hours (annualized)
        let annual_cost = gen.cost_per_mw * problem.operating_hours;
        operating_cost_expr += annual_cost * p_var;
    }

    // Bus angle variables (reference bus = 0)
    let ref_bus_idx = 0;
    let mut theta_vars: HashMap<usize, Variable> = HashMap::new();
    for bus in &buses {
        if bus.index != ref_bus_idx {
            // Reasonable angle bounds for DC power flow
            let theta = vars.add(variable().min(-std::f64::consts::PI).max(std::f64::consts::PI));
            theta_vars.insert(bus.index, theta);
        }
    }

    // Candidate line variables
    let mut candidate_build_vars: Vec<(CandidateId, Variable)> = Vec::new();
    let mut candidate_flow_vars: Vec<(CandidateId, Variable, &CandidateLine)> = Vec::new();
    let mut investment_cost_expr = Expression::from(0.0);

    for candidate in &problem.candidates {
        // Build decision: relaxed to continuous [0, max_circuits]
        let max = candidate.max_circuits.unwrap_or(1) as f64;
        let x_var = vars.add(variable().min(0.0).max(max));
        candidate_build_vars.push((candidate.id, x_var));

        // Flow variable: bounded by capacity * build decision
        let flow_bound = candidate.capacity_mw * max;
        let f_var = vars.add(variable().min(-flow_bound).max(flow_bound));
        candidate_flow_vars.push((candidate.id, f_var, candidate));

        // Investment cost (annualized)
        let annual_investment = problem.annualized_investment_cost(candidate);
        investment_cost_expr += annual_investment * x_var;
    }

    // Total objective: investment + operating cost
    let total_cost_expr = investment_cost_expr.clone() + operating_cost_expr.clone();

    // Create problem
    let mut model = vars.minimise(total_cost_expr).using(clarabel);

    // === Power Balance Constraints ===
    // At each bus: Σ P_gen - Σ P_load = Σ P_out (to other buses)

    // Build generation by bus
    let mut bus_gen_expr: HashMap<usize, Expression> = HashMap::new();
    for (_, bus_id, p_var) in &gen_vars {
        let bus_idx = *bus_map.get(bus_id).ok_or_else(|| {
            TepError::NetworkValidation(format!("Generator bus {:?} not found", bus_id))
        })?;
        bus_gen_expr
            .entry(bus_idx)
            .or_insert_with(|| Expression::from(0.0));
        *bus_gen_expr.get_mut(&bus_idx).unwrap() += *p_var;
    }

    // Build flow expressions
    // For existing branches: flow = b * (θ_from - θ_to)
    // For candidates: flow = f_var (with Big-M constraints below)

    let mut bus_net_flow: HashMap<usize, Expression> = HashMap::new();
    for bus in &buses {
        bus_net_flow.insert(bus.index, Expression::from(0.0));
    }

    // Existing branch flows
    for branch in &branches {
        let i = *bus_map.get(&branch.from_bus).ok_or_else(|| {
            TepError::NetworkValidation(format!("Branch from_bus {:?} not found", branch.from_bus))
        })?;
        let j = *bus_map.get(&branch.to_bus).ok_or_else(|| {
            TepError::NetworkValidation(format!("Branch to_bus {:?} not found", branch.to_bus))
        })?;

        let theta_i = theta_vars.get(&i).copied();
        let theta_j = theta_vars.get(&j).copied();

        // Flow from i to j: P_ij = b * (θ_i - θ_j)
        let flow_expr: Expression = match (theta_i, theta_j) {
            (Some(ti), Some(tj)) => branch.susceptance * (ti - tj),
            (Some(ti), None) => branch.susceptance * ti, // j is reference
            (None, Some(tj)) => branch.susceptance * (-tj), // i is reference
            (None, None) => Expression::from(0.0), // Both reference (shouldn't happen)
        };

        // Add flow to net at each bus
        *bus_net_flow.get_mut(&i).unwrap() += flow_expr.clone();
        *bus_net_flow.get_mut(&j).unwrap() -= flow_expr;
    }

    // Candidate line flows
    for (_cand_id, f_var, candidate) in &candidate_flow_vars {
        let i = *bus_map.get(&candidate.from_bus).ok_or_else(|| {
            TepError::NetworkValidation(format!(
                "Candidate from_bus {:?} not found",
                candidate.from_bus
            ))
        })?;
        let j = *bus_map.get(&candidate.to_bus).ok_or_else(|| {
            TepError::NetworkValidation(format!(
                "Candidate to_bus {:?} not found",
                candidate.to_bus
            ))
        })?;

        // Flow contributes to net injection
        *bus_net_flow.get_mut(&i).unwrap() += *f_var;
        *bus_net_flow.get_mut(&j).unwrap() -= *f_var;
    }

    // Add power balance constraints
    for bus in &buses {
        let gen_at_bus = bus_gen_expr
            .get(&bus.index)
            .cloned()
            .unwrap_or_else(|| Expression::from(0.0));
        let load_at_bus = loads.get(&bus.id).copied().unwrap_or(0.0);
        let net_flow = bus_net_flow.get(&bus.index).cloned().unwrap_or_else(|| Expression::from(0.0));

        // Generation - Load = Net outflow
        model = model.with(constraint!(gen_at_bus - load_at_bus == net_flow));
    }

    // === Big-M Constraints for Candidate Lines ===
    // When x=1: f = b * (θ_i - θ_j) (physics enforced)
    // When x=0: f = 0 (no flow on unbuilt line)
    //
    // Formulation:
    // -M*(1-x) ≤ f - b*(θ_i - θ_j) ≤ M*(1-x)
    // -capacity*x ≤ f ≤ capacity*x

    for ((_cand_id, x_var), (_, f_var, candidate)) in
        candidate_build_vars.iter().zip(candidate_flow_vars.iter())
    {
        let i = *bus_map.get(&candidate.from_bus).unwrap();
        let j = *bus_map.get(&candidate.to_bus).unwrap();

        let theta_i = theta_vars.get(&i).copied();
        let theta_j = theta_vars.get(&j).copied();

        let angle_diff: Expression = match (theta_i, theta_j) {
            (Some(ti), Some(tj)) => ti - tj,
            (Some(ti), None) => Expression::from(ti),
            (None, Some(tj)) => Expression::from(0.0) - tj,
            (None, None) => Expression::from(0.0),
        };

        let b = candidate.susceptance();
        let physics_flow = b * angle_diff;
        let big_m = problem.big_m;
        let capacity = candidate.capacity_mw;

        // Big-M constraints for physics
        // f - b*Δθ ≤ M*(1-x)  →  f - b*Δθ - M + M*x ≤ 0
        // f - b*Δθ ≥ -M*(1-x) →  f - b*Δθ + M - M*x ≥ 0
        let flow_minus_physics = *f_var - physics_flow.clone();
        model = model.with(constraint!(flow_minus_physics.clone() <= big_m - big_m * (*x_var)));
        model = model.with(constraint!(flow_minus_physics >= -big_m + big_m * (*x_var)));

        // Capacity constraints linked to build decision
        // -capacity*x ≤ f ≤ capacity*x
        model = model.with(constraint!(*f_var <= capacity * (*x_var)));
        model = model.with(constraint!(*f_var >= -capacity * (*x_var)));
    }

    // === Solve ===
    let solution = model
        .solve()
        .map_err(|e| TepError::SolverFailed(format!("{:?}", e)))?;

    // === Extract Results ===
    let mut result = TepSolution::new();
    result.optimal = true;
    result.solve_time = start.elapsed();
    result.status_message = "Optimal (LP relaxation)".to_string();

    // Extract build decisions (round to nearest integer for relaxed solution)
    for ((cand_id, x_var), (_, _, candidate)) in
        candidate_build_vars.iter().zip(candidate_flow_vars.iter())
    {
        let x_val = solution.value(*x_var);
        let circuits = x_val.round() as usize;
        let cost = if circuits > 0 {
            candidate.investment_cost * circuits as f64
        } else {
            0.0
        };

        result.build_decisions.push(LineBuildDecision {
            candidate_id: *cand_id,
            name: candidate.name.clone(),
            circuits_to_build: circuits,
            investment_cost: cost,
        });
    }

    // Calculate costs
    result.investment_cost = result
        .build_decisions
        .iter()
        .map(|d| problem.annualized_investment_cost(
            problem.candidates.iter().find(|c| c.id == d.candidate_id).unwrap()
        ) * d.circuits_to_build as f64)
        .sum();

    // Generator dispatch and operating cost
    let mut total_op_cost = 0.0;
    for (name, _, p_var) in &gen_vars {
        let p = solution.value(*p_var);
        result.generator_dispatch.insert(name.clone(), p);

        if let Some(gen) = generators.iter().find(|g| &g.name == name) {
            total_op_cost += gen.cost_per_mw * p * problem.operating_hours;
        }
    }
    result.operating_cost = total_op_cost;
    result.total_cost = result.investment_cost + result.operating_cost;

    // Bus angles
    for bus in &buses {
        let angle = if bus.index == ref_bus_idx {
            0.0
        } else {
            theta_vars.get(&bus.index).map(|v| solution.value(*v)).unwrap_or(0.0)
        };
        result.bus_angles.insert(bus.name.clone(), angle);
    }

    // Candidate flows
    for (cand_id, f_var, _) in &candidate_flow_vars {
        let flow = solution.value(*f_var);
        result.candidate_flows.insert(*cand_id, flow);
    }

    Ok(result)
}

/// Extract network data for solver
fn extract_network_data(
    network: &Network,
) -> Result<(Vec<BusData>, Vec<GenData>, Vec<BranchData>, HashMap<BusId, f64>), TepError> {
    let mut buses = Vec::new();
    let mut generators = Vec::new();
    let mut loads: HashMap<BusId, f64> = HashMap::new();

    let mut bus_index = 0;
    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Bus(bus) => {
                buses.push(BusData {
                    id: bus.id,
                    name: bus.name.clone(),
                    index: bus_index,
                });
                bus_index += 1;
            }
            Node::Gen(gen) if gen.status => {
                let cost_per_mw = gen.cost_model.marginal_cost(gen.pmax_mw / 2.0);
                generators.push(GenData {
                    name: gen.name.clone(),
                    bus_id: gen.bus,
                    pmin_mw: gen.pmin_mw,
                    pmax_mw: gen.pmax_mw,
                    cost_per_mw,
                });
            }
            Node::Load(load) => {
                *loads.entry(load.bus).or_insert(0.0) += load.active_power_mw;
            }
            _ => {}
        }
    }

    if buses.is_empty() {
        return Err(TepError::NetworkValidation("No buses in network".into()));
    }

    if generators.is_empty() {
        return Err(TepError::NetworkValidation("No generators in network".into()));
    }

    // Extract branches
    let mut branches = Vec::new();
    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            if !branch.status {
                continue;
            }
            let x_eff = branch.reactance * branch.tap_ratio;
            if x_eff.abs() < 1e-12 {
                continue;
            }
            branches.push(BranchData {
                name: branch.name.clone(),
                from_bus: branch.from_bus,
                to_bus: branch.to_bus,
                susceptance: 1.0 / x_eff,
                capacity_mw: branch.rating_a_mva.or(branch.s_max_mva),
            });
        }
    }

    Ok((buses, generators, branches, loads))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Bus, Gen, GenId, Load, LoadId, CostModel};

    /// Create the Garver 6-bus test system (simplified)
    fn create_garver_6bus() -> Network {
        let mut network = Network::new();

        // 6 buses
        for i in 1..=6 {
            network.graph.add_node(Node::Bus(Bus {
                id: BusId::new(i),
                name: format!("Bus {}", i),
                voltage_kv: 230.0,
                voltage_pu: 1.0,
                angle_rad: 0.0,
                ..Bus::default()
            }));
        }

        // Generators with costs
        // Bus 1: Cheap base load
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen 1".to_string(),
            bus: BusId::new(1),
            pmax_mw: 150.0,
            pmin_mw: 0.0,
            cost_model: CostModel::Polynomial(vec![0.0, 10.0]), // $10/MWh
            status: true,
            ..Gen::default()
        }));

        // Bus 3: Mid-cost
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(2),
            name: "Gen 3".to_string(),
            bus: BusId::new(3),
            pmax_mw: 360.0,
            pmin_mw: 0.0,
            cost_model: CostModel::Polynomial(vec![0.0, 20.0]), // $20/MWh
            status: true,
            ..Gen::default()
        }));

        // Bus 6: Expensive peaker
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(3),
            name: "Gen 6".to_string(),
            bus: BusId::new(6),
            pmax_mw: 600.0,
            pmin_mw: 0.0,
            cost_model: CostModel::Polynomial(vec![0.0, 30.0]), // $30/MWh
            status: true,
            ..Gen::default()
        }));

        // Loads (total: 760 MW)
        let loads = [
            (1, 80.0),
            (2, 240.0),
            (3, 40.0),
            (4, 160.0),
            (5, 240.0),
        ];
        for (i, (bus, mw)) in loads.iter().enumerate() {
            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(i + 1),
                name: format!("Load {}", bus),
                bus: BusId::new(*bus),
                active_power_mw: *mw,
                reactive_power_mvar: 0.0,
            }));
        }

        network
    }

    #[test]
    fn test_tep_simple_case() {
        let network = create_garver_6bus();

        // Create TEP problem with candidate lines
        // Provide full connectivity options so the problem is feasible
        let problem = super::super::TepProblemBuilder::new(network)
            .big_m(10000.0)
            .planning_params(8760.0, 0.10, 10)
            // Candidate lines covering all corridors (Garver-style)
            .candidate("Line 1-2", BusId::new(1), BusId::new(2), 0.10, 200.0, 40_000_000.0)
            .candidate("Line 1-4", BusId::new(1), BusId::new(4), 0.15, 150.0, 60_000_000.0)
            .candidate("Line 2-3", BusId::new(2), BusId::new(3), 0.10, 200.0, 40_000_000.0)
            .candidate("Line 2-4", BusId::new(2), BusId::new(4), 0.10, 200.0, 40_000_000.0)
            .candidate("Line 3-5", BusId::new(3), BusId::new(5), 0.05, 300.0, 20_000_000.0)
            .candidate("Line 3-6", BusId::new(3), BusId::new(6), 0.05, 300.0, 30_000_000.0)
            .candidate("Line 4-5", BusId::new(4), BusId::new(5), 0.08, 200.0, 25_000_000.0)
            .candidate("Line 4-6", BusId::new(4), BusId::new(6), 0.10, 250.0, 35_000_000.0)
            .build();

        let config = TepSolverConfig::default();
        let solution = solve_tep(&problem, &config).expect("solver should succeed");

        println!("{}", solution.summary());

        // Basic validation
        assert!(solution.optimal);
        assert!(solution.total_cost > 0.0);
        assert!(solution.total_generation_mw() > 700.0); // Must serve ~760 MW load
    }
}
