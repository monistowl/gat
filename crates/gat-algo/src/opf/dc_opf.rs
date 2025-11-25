//! DC Optimal Power Flow with B-matrix formulation
//!
//! Linearized OPF using DC power flow approximation:
//! - Ignores reactive power
//! - Assumes flat voltage magnitudes (|V| = 1.0 p.u.)
//! - Linearizes branch flows: P_ij = (θ_i - θ_j) / x_ij

use crate::opf::{ConstraintInfo, ConstraintType, OpfMethod, OpfSolution};
use crate::OpfError;
use gat_core::{Branch, BusId, Edge, Gen, Load, Network, Node};
use good_lp::{
    constraint, variable, variables, Expression, ProblemVariables, Solution, SolverModel, Variable,
};
use good_lp::solvers::clarabel::clarabel;
use sprs::{CsMat, TriMat};
use std::collections::HashMap;
use std::time::Instant;

/// Internal representation of a bus for DC-OPF
#[derive(Debug, Clone)]
struct BusData {
    id: BusId,
    name: String,
    index: usize,  // Matrix index
}

/// Internal representation of a generator for DC-OPF
#[derive(Debug, Clone)]
struct GenData {
    name: String,
    bus_id: BusId,
    pmin_mw: f64,
    pmax_mw: f64,
    cost_coeffs: Vec<f64>,  // [c0, c1, c2, ...] for polynomial
}

/// Internal representation of a branch for DC-OPF
#[derive(Debug, Clone)]
struct BranchData {
    name: String,
    from_bus: BusId,
    to_bus: BusId,
    susceptance: f64,  // b = 1/x (per unit)
}

/// Extract network data into solver-friendly format
fn extract_network_data(
    network: &Network,
) -> Result<(Vec<BusData>, Vec<GenData>, Vec<BranchData>, HashMap<BusId, f64>), OpfError> {
    let mut buses = Vec::new();
    let mut generators = Vec::new();
    let mut loads: HashMap<BusId, f64> = HashMap::new();

    // First pass: extract buses and assign indices
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
            Node::Gen(gen) => {
                let cost_coeffs = match &gen.cost_model {
                    gat_core::CostModel::NoCost => vec![0.0, 0.0],
                    gat_core::CostModel::Polynomial(c) => c.clone(),
                    gat_core::CostModel::PiecewiseLinear(_) => {
                        // Approximate with marginal cost at midpoint
                        let mid = (gen.pmin_mw + gen.pmax_mw) / 2.0;
                        vec![0.0, gen.cost_model.marginal_cost(mid)]
                    }
                };
                generators.push(GenData {
                    name: gen.name.clone(),
                    bus_id: gen.bus,
                    pmin_mw: gen.pmin_mw,
                    pmax_mw: gen.pmax_mw,
                    cost_coeffs,
                });
            }
            Node::Load(load) => {
                *loads.entry(load.bus).or_insert(0.0) += load.active_power_mw;
            }
        }
    }

    if buses.is_empty() {
        return Err(OpfError::DataValidation("No buses in network".into()));
    }

    if generators.is_empty() {
        return Err(OpfError::DataValidation("No generators in network".into()));
    }

    // Extract branches
    let mut branches = Vec::new();
    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            if branch.reactance.abs() < 1e-12 {
                return Err(OpfError::DataValidation(format!(
                    "Branch {} has zero reactance",
                    branch.name
                )));
            }
            branches.push(BranchData {
                name: branch.name.clone(),
                from_bus: branch.from_bus,
                to_bus: branch.to_bus,
                susceptance: 1.0 / branch.reactance,
            });
        }
    }

    Ok((buses, generators, branches, loads))
}

/// Build bus ID to index mapping
fn build_bus_index_map(buses: &[BusData]) -> HashMap<BusId, usize> {
    buses.iter().map(|b| (b.id, b.index)).collect()
}

/// Build the B' susceptance matrix (sparse)
///
/// B'[i,j] = -b_ij for i ≠ j (off-diagonal = -susceptance of branch i-j)
/// B'[i,i] = Σ b_ik for all k (diagonal = sum of susceptances of all branches at bus i)
fn build_b_prime_matrix(
    n_bus: usize,
    branches: &[BranchData],
    bus_map: &HashMap<BusId, usize>,
) -> CsMat<f64> {
    let mut triplets = TriMat::new((n_bus, n_bus));

    for branch in branches {
        let i = *bus_map.get(&branch.from_bus).expect("from_bus in map");
        let j = *bus_map.get(&branch.to_bus).expect("to_bus in map");
        let b = branch.susceptance;

        // Off-diagonal: B'[i,j] = B'[j,i] = -b
        triplets.add_triplet(i, j, -b);
        triplets.add_triplet(j, i, -b);

        // Diagonal: B'[i,i] += b, B'[j,j] += b
        triplets.add_triplet(i, i, b);
        triplets.add_triplet(j, j, b);
    }

    triplets.to_csr()
}

/// Solve DC-OPF for the given network
pub fn solve(
    network: &Network,
    _max_iterations: usize,
    _tolerance: f64,
) -> Result<OpfSolution, OpfError> {
    let start = Instant::now();

    // Extract network data
    let (buses, generators, branches, loads) = extract_network_data(network)?;
    let bus_map = build_bus_index_map(&buses);
    let n_bus = buses.len();

    // Build B' susceptance matrix
    let b_prime = build_b_prime_matrix(n_bus, &branches, &bus_map);

    // === LP Formulation ===
    // Variables: P_g[i] for each generator, θ[j] for each bus (except reference)
    // Objective: minimize Σ c1*P_g
    // Constraints:
    //   - Power balance at each bus: Σ P_g - Σ P_d = Σ B'[i,j] * θ[j]
    //   - Generator limits: P_g_min ≤ P_g ≤ P_g_max
    //   - Reference bus angle: θ_0 = 0 (not a variable)

    let mut vars = variables!();

    // Generator power variables
    let mut gen_vars: Vec<(String, BusId, Variable)> = Vec::new();
    let mut cost_terms: Vec<Expression> = Vec::new();

    for gen in &generators {
        let pmin = gen.pmin_mw.max(0.0);
        let pmax = if gen.pmax_mw.is_finite() { gen.pmax_mw } else { 1e6 };
        let p_var = vars.add(variable().min(pmin).max(pmax));
        gen_vars.push((gen.name.clone(), gen.bus_id, p_var));

        // Linear cost approximation: c1 * P (ignore c0 constant, c2 quadratic for LP)
        let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
        cost_terms.push(c1 * p_var);
    }

    // Build cost expression
    let cost_expr = cost_terms.into_iter().fold(Expression::from(0.0), |acc, term| acc + term);

    // Bus angle variables (reference bus = 0, not a variable)
    let ref_bus_idx = 0;  // First bus is reference
    let mut theta_vars: HashMap<usize, Variable> = HashMap::new();
    for bus in &buses {
        if bus.index != ref_bus_idx {
            // Angles can be large in per-unit MW formulation, use wide bounds
            let theta = vars.add(variable().min(-1e6).max(1e6));
            theta_vars.insert(bus.index, theta);
        }
    }

    // Build power balance constraint for each bus:
    // Σ P_g(bus) - P_load(bus) = Σ_j B'[bus,j] * (θ_bus - θ_j)
    let problem = vars.minimise(cost_expr).using(clarabel);

    // Collect net injection per bus from generators
    let mut bus_gen_expr: HashMap<usize, Expression> = HashMap::new();
    for (_, bus_id, p_var) in &gen_vars {
        let bus_idx = *bus_map.get(bus_id).expect("gen bus in map");
        bus_gen_expr
            .entry(bus_idx)
            .or_insert_with(|| Expression::from(0.0));
        *bus_gen_expr.get_mut(&bus_idx).unwrap() += *p_var;
    }

    // Add power balance constraints
    let mut problem = problem;
    for bus in &buses {
        let i = bus.index;

        // LHS: net generation - load
        let gen_at_bus = bus_gen_expr
            .get(&i)
            .cloned()
            .unwrap_or_else(|| Expression::from(0.0));
        let load_at_bus = loads.get(&bus.id).copied().unwrap_or(0.0);
        let net_injection = gen_at_bus - load_at_bus;

        // RHS: Σ_j B'[i,j] * θ[j]
        let mut flow_expr = Expression::from(0.0);

        // Get row i of B' matrix
        let row = b_prime.outer_view(i);
        if let Some(row_view) = row {
            for (j, &b_ij) in row_view.iter() {
                if let Some(&theta_j) = theta_vars.get(&j) {
                    flow_expr += b_ij * theta_j;
                }
                // If j is reference bus (not in theta_vars), θ_j = 0, no contribution
            }
        }

        // Constraint: net_injection = flow_expr
        problem = problem.with(constraint!(net_injection - flow_expr == 0.0));
    }

    // Solve
    let solution = problem.solve().map_err(|e| {
        OpfError::NumericalIssue(format!("LP solver failed: {:?}", e))
    })?;

    // === Extract Results ===
    let mut result = OpfSolution {
        converged: true,
        method_used: OpfMethod::DcOpf,
        iterations: 1,
        solve_time_ms: start.elapsed().as_millis(),
        objective_value: 0.0,
        ..Default::default()
    };

    // Generator outputs and objective
    let mut total_cost = 0.0;
    for (name, _bus_id, p_var) in &gen_vars {
        let p = solution.value(*p_var);
        result.generator_p.insert(name.clone(), p);

        // Find generator cost coeffs
        if let Some(gen) = generators.iter().find(|g| &g.name == name) {
            let c0 = gen.cost_coeffs.get(0).copied().unwrap_or(0.0);
            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
            total_cost += c0 + c1 * p + c2 * p * p;
        }
    }
    result.objective_value = total_cost;

    // Bus angles
    for bus in &buses {
        let theta = if bus.index == ref_bus_idx {
            0.0
        } else {
            theta_vars.get(&bus.index).map(|v| solution.value(*v)).unwrap_or(0.0)
        };
        result.bus_voltage_ang.insert(bus.name.clone(), theta);
        result.bus_voltage_mag.insert(bus.name.clone(), 1.0);  // DC assumption
    }

    // Branch flows: P_ij = b_ij * (θ_i - θ_j)
    for branch in &branches {
        let i = *bus_map.get(&branch.from_bus).expect("from_bus");
        let j = *bus_map.get(&branch.to_bus).expect("to_bus");

        let theta_i = if i == ref_bus_idx {
            0.0
        } else {
            theta_vars.get(&i).map(|v| solution.value(*v)).unwrap_or(0.0)
        };
        let theta_j = if j == ref_bus_idx {
            0.0
        } else {
            theta_vars.get(&j).map(|v| solution.value(*v)).unwrap_or(0.0)
        };

        let flow = branch.susceptance * (theta_i - theta_j);
        result.branch_p_flow.insert(branch.name.clone(), flow);
    }

    // Estimate losses (simplified: use 1% of load for DC-OPF)
    let total_load: f64 = loads.values().sum();
    result.total_losses_mw = total_load * 0.01;

    Ok(result)
}
