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

    // TODO: Formulate LP
    // TODO: Solve and extract results

    Err(OpfError::NotImplemented("DC-OPF data extraction done, LP formulation next".into()))
}
