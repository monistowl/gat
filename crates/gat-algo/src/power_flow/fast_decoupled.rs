//! Fast-Decoupled Power Flow (FDPF) Solver
//!
//! Implements the Stott-Alsac fast-decoupled load flow method which decouples
//! the P-θ and Q-V subproblems for faster convergence on well-conditioned networks.
//!
//! ## Algorithm Overview
//!
//! Instead of solving the full Jacobian system, FDPF uses:
//! - B' matrix for P-θ subproblem: ΔP/V = B' × Δθ
//! - B'' matrix for Q-V subproblem: ΔQ/V = B'' × ΔV/V
//!
//! The matrices are constant (don't need to be rebuilt each iteration) which
//! gives approximately 5x speedup over full Newton-Raphson.
//!
//! ## References
//!
//! - Stott & Alsac (1974): "Fast Decoupled Load Flow"
//!   IEEE Trans. PAS, 93(3), 859-869
//!   DOI: [10.1109/TPAS.1974.293985](https://doi.org/10.1109/TPAS.1974.293985)

use std::collections::HashMap;
use gat_core::{BusId, Edge, Network, Node};

/// Build the B' (B-prime) matrix for the P-θ subproblem.
///
/// B'_ij = -1/x_ij for off-diagonal elements (connected buses)
/// B'_ii = Σ(1/x_ik) for diagonal elements (sum of connected susceptances)
///
/// This is the standard XB formulation where B' ignores resistance and
/// uses only reactance (susceptance = 1/x).
pub fn build_b_prime_matrix(network: &Network) -> Vec<Vec<f64>> {
    // Collect buses and create index mapping
    let mut bus_ids: Vec<BusId> = network
        .graph
        .node_weights()
        .filter_map(|n| match n {
            Node::Bus(bus) => Some(bus.id),
            _ => None,
        })
        .collect();
    bus_ids.sort_by_key(|b| b.value());

    let n = bus_ids.len();
    let id_to_idx: HashMap<BusId, usize> = bus_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    let mut b_prime = vec![vec![0.0; n]; n];

    // Process each branch
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            if !branch.status {
                continue;
            }

            let Some(&i) = id_to_idx.get(&branch.from_bus) else { continue };
            let Some(&j) = id_to_idx.get(&branch.to_bus) else { continue };

            // Susceptance = 1/x (ignoring resistance for B')
            let x = branch.reactance.abs().max(1e-6);
            let b = 1.0 / x;

            // Off-diagonal: -b
            b_prime[i][j] -= b;
            b_prime[j][i] -= b;

            // Diagonal: +b
            b_prime[i][i] += b;
            b_prime[j][j] += b;
        }
    }

    b_prime
}

/// Build the B'' (B-double-prime) matrix for the Q-V subproblem.
///
/// B''_ij = -1/(x_ij × tap) for off-diagonal elements (connected buses)
/// B''_ii includes:
/// - Transformer tap ratio effects: 1/(x×tap²) from side, 1/x to side
/// - Line charging susceptance: branch.charging_b / 2
/// - Shunt elements: Node::Shunt with bs_pu field
///
/// For transformers, tap = branch.tap_ratio if > 0, else 1.0
pub fn build_b_double_prime_matrix(network: &Network) -> Vec<Vec<f64>> {
    // Collect buses and create index mapping
    let mut bus_ids: Vec<BusId> = network
        .graph
        .node_weights()
        .filter_map(|n| match n {
            Node::Bus(bus) => Some(bus.id),
            _ => None,
        })
        .collect();
    bus_ids.sort_by_key(|b| b.value());

    let n = bus_ids.len();
    let id_to_idx: HashMap<BusId, usize> = bus_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    let mut b_double_prime = vec![vec![0.0; n]; n];

    // Process each branch
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            if !branch.status {
                continue;
            }

            let Some(&i) = id_to_idx.get(&branch.from_bus) else { continue };
            let Some(&j) = id_to_idx.get(&branch.to_bus) else { continue };

            // Susceptance b = 1/x
            let x = branch.reactance.abs().max(1e-6);
            let b = 1.0 / x;

            // Tap ratio (use 1.0 if not set)
            let tap = if branch.tap_ratio > 0.0 { branch.tap_ratio } else { 1.0 };

            // Off-diagonal: -b/tap
            let b_off = b / tap;
            b_double_prime[i][j] -= b_off;
            b_double_prime[j][i] -= b_off;

            // Diagonal: from-side adds b/(tap²), to-side adds b
            b_double_prime[i][i] += b / (tap * tap);
            b_double_prime[j][j] += b;

            // Add line charging susceptance (split equally between buses)
            let half_charging = branch.charging_b.value() / 2.0;
            b_double_prime[i][i] += half_charging;
            b_double_prime[j][j] += half_charging;
        }
    }

    // Add shunt susceptances from Node::Shunt elements
    for node in network.graph.node_weights() {
        if let Node::Shunt(shunt) = node {
            if let Some(&idx) = id_to_idx.get(&shunt.bus) {
                b_double_prime[idx][idx] += shunt.bs_pu;
            }
        }
    }

    b_double_prime
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus, Edge, Network, Node};

    fn build_3bus_network() -> Network {
        let mut network = Network::new();
        // Bus 0 (slack), Bus 1 (PV), Bus 2 (PQ)
        let b0 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(0),
            name: "Bus0".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));
        let b2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Bus::default()
        }));

        // Branch 0-1: x=0.1, tap=1.0
        network.graph.add_edge(b0, b1, Edge::Branch(Branch {
            id: BranchId::new(0),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
            ..Branch::default()
        }));
        // Branch 1-2: x=0.2, tap=1.0
        network.graph.add_edge(b1, b2, Edge::Branch(Branch {
            id: BranchId::new(1),
            from_bus: BusId::new(1),
            to_bus: BusId::new(2),
            resistance: 0.02,
            reactance: 0.2,
            ..Branch::default()
        }));
        // Branch 0-2: x=0.15, tap=1.0
        network.graph.add_edge(b0, b2, Edge::Branch(Branch {
            id: BranchId::new(2),
            from_bus: BusId::new(0),
            to_bus: BusId::new(2),
            resistance: 0.015,
            reactance: 0.15,
            ..Branch::default()
        }));

        network
    }

    #[test]
    fn test_b_prime_matrix_construction() {
        let network = build_3bus_network();
        let b_prime = build_b_prime_matrix(&network);

        // B' diagonal should be sum of branch susceptances
        // Bus 0: connected to bus 1 (1/0.1=10) and bus 2 (1/0.15=6.67)
        assert!((b_prime[0][0] - 16.67).abs() < 0.1);
        // Off-diagonal should be negative susceptance
        assert!((b_prime[0][1] - (-10.0)).abs() < 0.1);
    }

    #[test]
    fn test_b_double_prime_matrix_construction() {
        let network = build_3bus_network();
        let b_double_prime = build_b_double_prime_matrix(&network);

        // B'' should have similar structure to B' for networks without transformers
        assert!(b_double_prime[0][0] > 0.0);
        assert!(b_double_prime[0][1] < 0.0);
    }
}
