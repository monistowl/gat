//! Merit-order economic dispatch
//!
//! Dispatches generators in order of marginal cost to minimize total cost.
//! Does not model network constraints, losses, or reactive power.

use crate::{
    opf::{OpfMethod, OpfSolution},
    OpfError,
};
use gat_core::{Gen, Network, Node};
use std::time::Instant;

/// Solve using merit-order economic dispatch
pub fn solve(
    network: &Network,
    _max_iterations: usize,
    _tolerance: f64,
) -> Result<OpfSolution, OpfError> {
    let start = Instant::now();

    // Collect generators and loads
    let mut generators: Vec<Gen> = Vec::new();
    let mut total_load = 0.0;

    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Gen(gen) => {
                generators.push(gen.clone());
            }
            Node::Load(load) => {
                total_load += load.active_power_mw;
            }
            Node::Bus(_) => {}
            Node::Shunt(_) => {}
        }
    }

    if generators.is_empty() {
        return Err(OpfError::DataValidation(
            "No generators in network".to_string(),
        ));
    }

    // Estimate losses at 1% of load for DC approximation
    let loss_estimate = total_load * 0.01;
    let required_generation = total_load + loss_estimate;

    // Check total capacity
    let total_pmax: f64 = generators.iter().map(|g| g.pmax_mw).sum();
    let total_pmin: f64 = generators.iter().map(|g| g.pmin_mw).sum();

    if required_generation > total_pmax {
        return Err(OpfError::Infeasible(format!(
            "Generator capacity insufficient: need {:.2} MW, max {:.2} MW",
            required_generation, total_pmax
        )));
    }

    if required_generation < total_pmin {
        return Err(OpfError::Infeasible(format!(
            "Load too low for minimum generation: need {:.2} MW, min {:.2} MW",
            required_generation, total_pmin
        )));
    }

    // Economic dispatch using merit order
    let dispatch = economic_dispatch(&generators, required_generation)?;

    // Compute objective value using actual cost functions
    let objective_value: f64 = generators
        .iter()
        .zip(dispatch.iter())
        .map(|(gen, &p)| gen.cost_model.evaluate(p))
        .sum();

    // Build solution
    let mut solution = OpfSolution {
        converged: true,
        method_used: OpfMethod::EconomicDispatch,
        iterations: 1,
        solve_time_ms: start.elapsed().as_millis(),
        objective_value,
        total_losses_mw: loss_estimate,
        ..Default::default()
    };

    // Record generator outputs
    for (gen, &output) in generators.iter().zip(dispatch.iter()) {
        solution.generator_p.insert(gen.name.clone(), output);
    }

    // Set voltages to nominal (1.0 pu) - no voltage model in economic dispatch
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            solution.bus_voltage_mag.insert(bus.name.clone(), 1.0);
            solution.bus_voltage_ang.insert(bus.name.clone(), 0.0);
        }
    }

    Ok(solution)
}

/// Economic dispatch using merit order
fn economic_dispatch(generators: &[Gen], required_generation: f64) -> Result<Vec<f64>, OpfError> {
    let n = generators.len();
    let mut dispatch = vec![0.0; n];

    // Start with minimum generation for all units
    for (i, gen) in generators.iter().enumerate() {
        dispatch[i] = gen.pmin_mw;
    }

    // Calculate how much more we need beyond minimum
    let total_pmin: f64 = generators.iter().map(|g| g.pmin_mw).sum();
    let mut remaining = required_generation - total_pmin;

    if remaining < 0.0 {
        return Ok(dispatch);
    }

    // Create merit order: sort by marginal cost at Pmin
    let mut merit_order: Vec<usize> = (0..n).collect();
    merit_order.sort_by(|&a, &b| {
        let mc_a = generators[a]
            .cost_model
            .marginal_cost(generators[a].pmin_mw);
        let mc_b = generators[b]
            .cost_model
            .marginal_cost(generators[b].pmin_mw);
        mc_a.partial_cmp(&mc_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Dispatch in merit order
    for &idx in &merit_order {
        if remaining <= 1e-6 {
            break;
        }

        let gen = &generators[idx];
        let current = dispatch[idx];
        let headroom = (gen.pmax_mw - current).max(0.0);
        let increment = remaining.min(headroom);

        dispatch[idx] = current + increment;
        remaining -= increment;
    }

    if remaining > 1e-3 {
        return Err(OpfError::Infeasible(format!(
            "Cannot meet load: {:.3} MW unserved after dispatch",
            remaining
        )));
    }

    Ok(dispatch)
}
