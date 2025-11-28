//! Native solver dispatch via subprocess IPC.
//!
//! This module handles conversion between gat-core types and the Arrow IPC
//! protocol used by native solver binaries (gat-clp, gat-cbc, gat-ipopt).
//!
//! # Architecture
//!
//! ```text
//! Network (gat-core)          ProblemBatch (IPC)           Native Solver
//!        │                          │                           │
//!        │  network_to_problem()    │    stdin (Arrow IPC)      │
//!        ├─────────────────────────>├──────────────────────────>│
//!        │                          │                           │
//!        │                          │   stdout (Arrow IPC)      │
//!        │<─────────────────────────├<──────────────────────────┤
//!        │  solution_to_opf()       │                           │
//! OpfSolution                 SolutionBatch
//! ```

use crate::opf::{OpfMethod, OpfSolution};
use crate::OpfError;
use gat_core::{Edge, Network, Node};
use gat_solver_common::problem::{ProblemBatch, ProblemType};
use gat_solver_common::solution::{SolutionBatch, SolutionStatus};
use gat_solver_common::{SolverId, SolverProcess};
use std::collections::HashMap;

/// Convert a Network to a ProblemBatch for IPC transmission.
///
/// This extracts bus, generator, and branch data from the Network graph
/// and serializes it into the flat array format expected by solver plugins.
pub fn network_to_problem(
    network: &Network,
    problem_type: ProblemType,
    timeout_seconds: u64,
) -> ProblemBatch {
    let mut problem = ProblemBatch::new(problem_type);
    problem.timeout_seconds = timeout_seconds;

    // Build index mappings for buses
    let mut bus_id_to_idx: HashMap<gat_core::BusId, usize> = HashMap::new();

    // Extract buses
    // First bus is treated as slack (type 3), others as PQ (type 1)
    let mut first_bus = true;
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            let idx = problem.bus_id.len();
            bus_id_to_idx.insert(bus.id, idx);

            problem.bus_id.push(bus.id.value() as i64);
            problem.bus_name.push(bus.name.clone());
            problem.bus_v_min.push(bus.vmin_pu.unwrap_or(0.9));
            problem.bus_v_max.push(bus.vmax_pu.unwrap_or(1.1));
            problem.bus_v_mag.push(bus.voltage_pu);
            problem.bus_v_ang.push(bus.angle_rad);
            problem.bus_p_load.push(0.0); // Will be aggregated from loads
            problem.bus_q_load.push(0.0);
            // First bus is slack, rest are PQ
            problem.bus_type.push(if first_bus { 3 } else { 1 });
            first_bus = false;
        }
    }

    // Aggregate loads into bus data
    for node_idx in network.graph.node_indices() {
        if let Node::Load(load) = &network.graph[node_idx] {
            if let Some(&bus_idx) = bus_id_to_idx.get(&load.bus) {
                problem.bus_p_load[bus_idx] += load.active_power_mw;
                problem.bus_q_load[bus_idx] += load.reactive_power_mvar;
            }
        }
    }

    // Extract generators
    for node_idx in network.graph.node_indices() {
        if let Node::Gen(gen) = &network.graph[node_idx] {
            if !gen.status {
                continue; // Skip offline generators
            }

            problem.gen_id.push(gen.id.value() as i64);
            problem.gen_bus_id.push(gen.bus.value() as i64);
            problem.gen_p_min.push(gen.pmin_mw);
            problem.gen_p_max.push(gen.pmax_mw);
            problem.gen_q_min.push(gen.qmin_mvar);
            problem.gen_q_max.push(gen.qmax_mvar);

            // Extract polynomial cost coefficients
            let (c0, c1, c2) = match &gen.cost_model {
                gat_core::CostModel::NoCost => (0.0, 0.0, 0.0),
                gat_core::CostModel::Polynomial(coeffs) => {
                    let c0 = coeffs.first().copied().unwrap_or(0.0);
                    let c1 = coeffs.get(1).copied().unwrap_or(0.0);
                    let c2 = coeffs.get(2).copied().unwrap_or(0.0);
                    (c0, c1, c2)
                }
                gat_core::CostModel::PiecewiseLinear(segments) => {
                    // Approximate with marginal cost at midpoint
                    if !segments.is_empty() {
                        let mid = (gen.pmin_mw + gen.pmax_mw) / 2.0;
                        let mc = gen.cost_model.marginal_cost(mid);
                        (0.0, mc, 0.0)
                    } else {
                        (0.0, 0.0, 0.0)
                    }
                }
            };

            problem.gen_cost_c0.push(c0);
            problem.gen_cost_c1.push(c1);
            problem.gen_cost_c2.push(c2);
            problem
                .gen_v_setpoint
                .push(gen.voltage_setpoint_pu.unwrap_or(1.0));
            problem.gen_status.push(1);
        }
    }

    // Extract branches
    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            if !branch.status {
                continue; // Skip offline branches
            }

            problem.branch_id.push(branch.id.value() as i64);
            problem.branch_from.push(branch.from_bus.value() as i64);
            problem.branch_to.push(branch.to_bus.value() as i64);
            problem.branch_r.push(branch.resistance);
            problem.branch_x.push(branch.reactance);
            problem.branch_b.push(branch.charging_b_pu);
            problem.branch_rate.push(branch.rating_a_mva.unwrap_or(0.0));
            problem.branch_tap.push(branch.tap_ratio);
            problem.branch_shift.push(branch.phase_shift_rad);
            problem.branch_status.push(1);
        }
    }

    problem
}

/// Convert a SolutionBatch from IPC back to an OpfSolution.
///
/// This maps the flat array format from the solver back to the
/// HashMap-based OpfSolution with named entries.
pub fn solution_to_opf(
    solution: &SolutionBatch,
    network: &Network,
    method: OpfMethod,
) -> OpfSolution {
    // Build name lookups from network
    let mut bus_names: HashMap<i64, String> = HashMap::new();
    let mut gen_names: HashMap<i64, String> = HashMap::new();
    let mut branch_names: HashMap<i64, String> = HashMap::new();

    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Bus(bus) => {
                bus_names.insert(bus.id.value() as i64, bus.name.clone());
            }
            Node::Gen(gen) => {
                gen_names.insert(gen.id.value() as i64, gen.name.clone());
            }
            _ => {}
        }
    }

    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            branch_names.insert(branch.id.value() as i64, branch.name.clone());
        }
    }

    let converged = matches!(solution.status, SolutionStatus::Optimal);

    let mut result = OpfSolution {
        converged,
        method_used: method,
        iterations: solution.iterations as usize,
        solve_time_ms: solution.solve_time_ms as u128,
        objective_value: solution.objective,
        ..Default::default()
    };

    // Map bus results
    for (i, &bus_id) in solution.bus_id.iter().enumerate() {
        if let Some(name) = bus_names.get(&bus_id) {
            if i < solution.bus_v_mag.len() {
                result
                    .bus_voltage_mag
                    .insert(name.clone(), solution.bus_v_mag[i]);
            }
            if i < solution.bus_v_ang.len() {
                result
                    .bus_voltage_ang
                    .insert(name.clone(), solution.bus_v_ang[i]);
            }
            if i < solution.bus_lmp.len() {
                result.bus_lmp.insert(name.clone(), solution.bus_lmp[i]);
            }
        }
    }

    // Map generator results
    for (i, &gen_id) in solution.gen_id.iter().enumerate() {
        if let Some(name) = gen_names.get(&gen_id) {
            if i < solution.gen_p.len() {
                result.generator_p.insert(name.clone(), solution.gen_p[i]);
            }
            if i < solution.gen_q.len() {
                result.generator_q.insert(name.clone(), solution.gen_q[i]);
            }
        }
    }

    // Map branch results
    for (i, &branch_id) in solution.branch_id.iter().enumerate() {
        if let Some(name) = branch_names.get(&branch_id) {
            if i < solution.branch_p_from.len() {
                result
                    .branch_p_flow
                    .insert(name.clone(), solution.branch_p_from[i]);
            }
            if i < solution.branch_q_from.len() {
                result
                    .branch_q_flow
                    .insert(name.clone(), solution.branch_q_from[i]);
            }
        }
    }

    result
}

/// Solve DC-OPF using the native CLP solver via subprocess.
///
/// This function:
/// 1. Converts the Network to a ProblemBatch
/// 2. Spawns the gat-clp solver binary
/// 3. Sends the problem via Arrow IPC stdin
/// 4. Receives the solution via Arrow IPC stdout
/// 5. Converts back to OpfSolution
pub fn solve_dc_opf_native(
    network: &Network,
    timeout_seconds: u64,
) -> Result<OpfSolution, OpfError> {
    // Find the solver binary
    let binary_path = SolverProcess::find_binary(SolverId::Clp).map_err(|e| {
        OpfError::NotImplemented(format!(
            "Native CLP solver not found: {}. \
             Build with: cargo build -p gat-clp",
            e
        ))
    })?;

    // Convert network to IPC format
    let problem = network_to_problem(network, ProblemType::DcOpf, timeout_seconds);

    // Create and run solver process
    let solver = SolverProcess::new(SolverId::Clp, binary_path, timeout_seconds);
    let solution = solver
        .solve_blocking(&problem)
        .map_err(|e| OpfError::NumericalIssue(format!("Native CLP solver failed: {}", e)))?;

    // Check solution status
    match solution.status {
        SolutionStatus::Optimal => {}
        SolutionStatus::Infeasible => {
            return Err(OpfError::Infeasible("Problem is infeasible".to_string()));
        }
        SolutionStatus::Unbounded => {
            return Err(OpfError::Unbounded);
        }
        SolutionStatus::Timeout => {
            return Err(OpfError::SolverTimeout(std::time::Duration::from_secs(
                timeout_seconds,
            )));
        }
        SolutionStatus::IterationLimit => {
            return Err(OpfError::NumericalIssue(
                "Iteration limit reached".to_string(),
            ));
        }
        _ => {
            if let Some(msg) = &solution.error_message {
                return Err(OpfError::NumericalIssue(msg.clone()));
            }
        }
    }

    // Convert back to OpfSolution
    Ok(solution_to_opf(&solution, network, OpfMethod::DcOpf))
}

/// Check if the native CLP solver is available.
pub fn is_clp_available() -> bool {
    SolverProcess::find_binary(SolverId::Clp).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Bus, BusId, CostModel, Gen, GenId};

    fn create_test_network() -> Network {
        let mut network = Network::new();

        // Add a bus
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            ..Bus::default()
        }));

        // Add a generator
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen1".to_string(),
            bus: BusId::new(1),
            pmin_mw: 0.0,
            pmax_mw: 100.0,
            cost_model: CostModel::Polynomial(vec![0.0, 10.0]),
            status: true,
            ..Gen::default()
        }));

        network
    }

    #[test]
    fn test_network_to_problem() {
        let network = create_test_network();
        let problem = network_to_problem(&network, ProblemType::DcOpf, 60);

        assert_eq!(problem.bus_id.len(), 1);
        assert_eq!(problem.gen_id.len(), 1);
        assert_eq!(problem.bus_id[0], 1);
        assert_eq!(problem.gen_p_max[0], 100.0);
        assert_eq!(problem.gen_cost_c1[0], 10.0);
    }
}
