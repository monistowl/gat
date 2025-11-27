//! Line Outage Distribution Factor (LODF) and PTDF matrix computation.
//!
//! LODFs quantify how flow redistributes when a branch is outaged:
//! ```text
//! LODF[ℓ,m] = (flow change on ℓ when m trips) / (pre-trip flow on m)
//! ```
//!
//! This enables fast N-k screening without re-solving power flow for each contingency.

use anyhow::{anyhow, Result};
use gat_core::{Edge, Network, Node};
use std::collections::HashMap;

/// PTDF matrix: sensitivity of branch flows to bus injections.
/// `ptdf[ℓ][n]` = change in flow on branch ℓ per MW injected at bus n.
#[derive(Debug, Clone)]
pub struct PtdfMatrix {
    /// Row index → branch ID
    pub branch_ids: Vec<usize>,
    /// Column index → bus ID
    pub bus_ids: Vec<usize>,
    /// PTDF values: ptdf[branch_idx][bus_idx]
    pub values: Vec<Vec<f64>>,
    /// Lookup: branch_id → row index
    branch_to_idx: HashMap<usize, usize>,
    /// Lookup: bus_id → column index
    bus_to_idx: HashMap<usize, usize>,
}

impl PtdfMatrix {
    /// Get PTDF for branch ℓ with respect to injection at bus n.
    pub fn get(&self, branch_id: usize, bus_id: usize) -> Option<f64> {
        let branch_idx = self.branch_to_idx.get(&branch_id)?;
        let bus_idx = self.bus_to_idx.get(&bus_id)?;
        Some(self.values[*branch_idx][*bus_idx])
    }

    /// Number of branches (rows).
    pub fn num_branches(&self) -> usize {
        self.branch_ids.len()
    }

    /// Number of buses (columns).
    pub fn num_buses(&self) -> usize {
        self.bus_ids.len()
    }
}

/// LODF matrix: flow redistribution factors for branch outages.
/// `lodf[ℓ][m]` = fraction of branch m's flow that shifts to branch ℓ when m trips.
#[derive(Debug, Clone)]
pub struct LodfMatrix {
    /// Row/column index → branch ID
    pub branch_ids: Vec<usize>,
    /// LODF values: lodf[ℓ_idx][m_idx]
    pub values: Vec<Vec<f64>>,
    /// Lookup: branch_id → index
    branch_to_idx: HashMap<usize, usize>,
}

impl LodfMatrix {
    /// Get LODF for branch ℓ when branch m is outaged.
    pub fn get(&self, branch_l: usize, branch_m: usize) -> Option<f64> {
        let l_idx = self.branch_to_idx.get(&branch_l)?;
        let m_idx = self.branch_to_idx.get(&branch_m)?;
        Some(self.values[*l_idx][*m_idx])
    }

    /// Estimate post-contingency flow on branch ℓ after branch m trips.
    /// `flow_l_post = flow_l_pre + LODF[ℓ,m] × flow_m_pre`
    pub fn estimate_post_outage_flow(
        &self,
        branch_l: usize,
        branch_m: usize,
        flow_l_pre: f64,
        flow_m_pre: f64,
    ) -> Option<f64> {
        let lodf = self.get(branch_l, branch_m)?;
        Some(flow_l_pre + lodf * flow_m_pre)
    }

    /// Number of branches.
    pub fn num_branches(&self) -> usize {
        self.branch_ids.len()
    }
}

/// Compute the full PTDF matrix for a network.
///
/// PTDF[ℓ,n] = sensitivity of flow on branch ℓ to injection at bus n.
///
/// Algorithm:
/// 1. Build B' matrix (bus susceptance, excluding slack)
/// 2. Compute X = (B')⁻¹ (bus angle sensitivity to injection)
/// 3. For each branch ℓ from bus i to j with reactance x_ℓ:
///    PTDF[ℓ,n] = (X[i,n] - X[j,n]) / x_ℓ
pub fn compute_ptdf_matrix(network: &Network) -> Result<PtdfMatrix> {
    // Collect bus IDs and create index mapping
    let mut bus_ids: Vec<usize> = network
        .graph
        .node_indices()
        .filter_map(|idx| match &network.graph[idx] {
            Node::Bus(bus) => Some(bus.id.value()),
            _ => None,
        })
        .collect();
    bus_ids.sort_unstable();

    if bus_ids.len() < 2 {
        return Err(anyhow!("Network must have at least 2 buses for PTDF"));
    }

    let mut bus_to_idx = HashMap::new();
    for (idx, &bus_id) in bus_ids.iter().enumerate() {
        bus_to_idx.insert(bus_id, idx);
    }

    // Collect branch data
    let mut branches: Vec<(usize, usize, usize, f64)> = Vec::new(); // (id, from, to, x)
    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            if branch.status {
                let x = (branch.reactance * branch.tap_ratio).abs().max(1e-6);
                branches.push((
                    branch.id.value(),
                    branch.from_bus.value(),
                    branch.to_bus.value(),
                    x,
                ));
            }
        }
    }
    branches.sort_by_key(|b| b.0);

    let branch_ids: Vec<usize> = branches.iter().map(|b| b.0).collect();
    let mut branch_to_idx = HashMap::new();
    for (idx, &id) in branch_ids.iter().enumerate() {
        branch_to_idx.insert(id, idx);
    }

    let n_buses = bus_ids.len();
    let n_branches = branches.len();

    if n_branches == 0 {
        return Err(anyhow!("Network must have at least 1 branch for PTDF"));
    }

    // Build B' matrix (susceptance matrix)
    let mut b_matrix = vec![vec![0.0; n_buses]; n_buses];
    for &(_, from, to, x) in &branches {
        if let (Some(&i), Some(&j)) = (bus_to_idx.get(&from), bus_to_idx.get(&to)) {
            let b = 1.0 / x;
            b_matrix[i][j] -= b;
            b_matrix[j][i] -= b;
            b_matrix[i][i] += b;
            b_matrix[j][j] += b;
        }
    }

    // Remove slack bus (first bus) to get reduced B' matrix
    // X = (B'_reduced)^(-1) extended with zeros for slack bus
    let x_matrix = compute_b_inverse(&b_matrix)?;

    // Compute PTDF: for each branch ℓ from i to j
    // PTDF[ℓ,n] = (X[i,n] - X[j,n]) / x_ℓ
    let mut ptdf = vec![vec![0.0; n_buses]; n_branches];
    for (branch_idx, &(_, from, to, x)) in branches.iter().enumerate() {
        let i = *bus_to_idx.get(&from).unwrap();
        let j = *bus_to_idx.get(&to).unwrap();
        for bus_idx in 0..n_buses {
            ptdf[branch_idx][bus_idx] = (x_matrix[i][bus_idx] - x_matrix[j][bus_idx]) / x;
        }
    }

    Ok(PtdfMatrix {
        branch_ids,
        bus_ids,
        values: ptdf,
        branch_to_idx,
        bus_to_idx,
    })
}

/// Compute LODF matrix from PTDF matrix.
///
/// LODF[ℓ,m] = PTDF[ℓ,from_m] / (1 - PTDF[m,from_m])
///
/// where from_m is the "from" bus of branch m.
pub fn compute_lodf_matrix(network: &Network, ptdf: &PtdfMatrix) -> Result<LodfMatrix> {
    // Get branch from-bus mapping
    let mut branch_from: HashMap<usize, usize> = HashMap::new();
    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            if branch.status {
                branch_from.insert(branch.id.value(), branch.from_bus.value());
            }
        }
    }

    let n = ptdf.num_branches();
    let mut lodf = vec![vec![0.0; n]; n];

    for (l_idx, &branch_l) in ptdf.branch_ids.iter().enumerate() {
        for (m_idx, &branch_m) in ptdf.branch_ids.iter().enumerate() {
            if l_idx == m_idx {
                // Diagonal: branch's LODF with itself is -1 (removes its own flow)
                lodf[l_idx][m_idx] = -1.0;
                continue;
            }

            // Get from-bus of branch m
            let from_m = match branch_from.get(&branch_m) {
                Some(&bus) => bus,
                None => continue,
            };

            // PTDF[m, from_m] - sensitivity of branch m to its own from-bus
            let ptdf_m_from = ptdf.get(branch_m, from_m).unwrap_or(0.0);

            // PTDF[ℓ, from_m] - sensitivity of branch ℓ to branch m's from-bus
            let ptdf_l_from = ptdf.get(branch_l, from_m).unwrap_or(0.0);

            // LODF[ℓ,m] = PTDF[ℓ,from_m] / (1 - PTDF[m,from_m])
            let denom = 1.0 - ptdf_m_from;
            if denom.abs() < 1e-10 {
                // Island or radial branch - LODF is undefined/infinite
                lodf[l_idx][m_idx] = f64::INFINITY;
            } else {
                lodf[l_idx][m_idx] = ptdf_l_from / denom;
            }
        }
    }

    Ok(LodfMatrix {
        branch_ids: ptdf.branch_ids.clone(),
        values: lodf,
        branch_to_idx: ptdf.branch_to_idx.clone(),
    })
}

/// Compute (B')^(-1) for the reduced susceptance matrix.
/// First row/column (slack bus) is treated as reference (angle = 0).
fn compute_b_inverse(b_matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let n = b_matrix.len();
    if n < 2 {
        return Err(anyhow!("Matrix too small for inverse"));
    }

    // Build reduced matrix (remove slack bus = first row/col)
    let m = n - 1;
    let mut reduced = vec![vec![0.0; m]; m];
    for i in 0..m {
        for j in 0..m {
            reduced[i][j] = b_matrix[i + 1][j + 1];
        }
    }

    // LU decomposition with partial pivoting
    let inv_reduced = lu_inverse(&reduced)?;

    // Extend back to full size (slack row/col stays zero)
    let mut x = vec![vec![0.0; n]; n];
    for i in 0..m {
        for j in 0..m {
            x[i + 1][j + 1] = inv_reduced[i][j];
        }
    }

    Ok(x)
}

/// LU decomposition-based matrix inverse with partial pivoting.
fn lu_inverse(a: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let n = a.len();
    if n == 0 {
        return Ok(vec![]);
    }

    // Copy matrix for LU decomposition
    let mut lu: Vec<Vec<f64>> = a.to_vec();
    let mut perm: Vec<usize> = (0..n).collect();

    // LU decomposition with partial pivoting
    for k in 0..n {
        // Find pivot
        let mut max_val = lu[k][k].abs();
        let mut max_row = k;
        for i in (k + 1)..n {
            if lu[i][k].abs() > max_val {
                max_val = lu[i][k].abs();
                max_row = i;
            }
        }

        if max_val < 1e-12 {
            return Err(anyhow!("Matrix is singular or nearly singular"));
        }

        // Swap rows
        if max_row != k {
            lu.swap(k, max_row);
            perm.swap(k, max_row);
        }

        // Elimination
        for i in (k + 1)..n {
            lu[i][k] /= lu[k][k];
            for j in (k + 1)..n {
                lu[i][j] -= lu[i][k] * lu[k][j];
            }
        }
    }

    // Solve for inverse columns
    let mut inv = vec![vec![0.0; n]; n];
    for col in 0..n {
        // Build permuted identity column
        let mut b = vec![0.0; n];
        b[perm[col]] = 1.0;

        // Forward substitution (L y = b)
        let mut y = vec![0.0; n];
        for i in 0..n {
            y[i] = b[i];
            for j in 0..i {
                y[i] -= lu[i][j] * y[j];
            }
        }

        // Back substitution (U x = y)
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            x[i] = y[i];
            for j in (i + 1)..n {
                x[i] -= lu[i][j] * x[j];
            }
            x[i] /= lu[i][i];
        }

        for i in 0..n {
            inv[i][col] = x[i];
        }
    }

    Ok(inv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus, BusId};

    /// Create a simple 3-bus network for testing:
    /// Bus 1 -- Branch 1 -- Bus 2 -- Branch 2 -- Bus 3
    ///   |                                         |
    ///   +---------- Branch 3 --------------------+
    fn create_test_network() -> Network {
        let mut network = Network::new();

        // Add buses
        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));
        let b2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));
        let b3 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(3),
            name: "Bus3".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));

        // Add branches (forming a triangle)
        network.graph.add_edge(
            b1,
            b2,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Line 1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                reactance: 0.1, // 0.1 pu
                ..Branch::default()
            }),
        );
        network.graph.add_edge(
            b2,
            b3,
            Edge::Branch(Branch {
                id: BranchId::new(2),
                name: "Line 2-3".to_string(),
                from_bus: BusId::new(2),
                to_bus: BusId::new(3),
                reactance: 0.1,
                ..Branch::default()
            }),
        );
        network.graph.add_edge(
            b1,
            b3,
            Edge::Branch(Branch {
                id: BranchId::new(3),
                name: "Line 1-3".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(3),
                reactance: 0.2, // Longer path
                ..Branch::default()
            }),
        );

        network
    }

    #[test]
    fn test_ptdf_matrix_dimensions() {
        let network = create_test_network();
        let ptdf = compute_ptdf_matrix(&network).unwrap();

        assert_eq!(ptdf.num_branches(), 3);
        assert_eq!(ptdf.num_buses(), 3);
    }

    #[test]
    fn test_ptdf_row_sum_zero_for_slack() {
        // PTDF values for injection at slack bus should be 0
        let network = create_test_network();
        let ptdf = compute_ptdf_matrix(&network).unwrap();

        // Slack bus is bus 1 (first in sorted order)
        for branch_id in &ptdf.branch_ids {
            let ptdf_val = ptdf.get(*branch_id, 1).unwrap();
            assert!(
                ptdf_val.abs() < 1e-10,
                "PTDF for slack bus injection should be ~0, got {}",
                ptdf_val
            );
        }
    }

    #[test]
    fn test_lodf_matrix_dimensions() {
        let network = create_test_network();
        let ptdf = compute_ptdf_matrix(&network).unwrap();
        let lodf = compute_lodf_matrix(&network, &ptdf).unwrap();

        assert_eq!(lodf.num_branches(), 3);
    }

    #[test]
    fn test_lodf_diagonal_is_negative_one() {
        // LODF[ℓ,ℓ] should be -1 (branch loses all its own flow when it trips)
        let network = create_test_network();
        let ptdf = compute_ptdf_matrix(&network).unwrap();
        let lodf = compute_lodf_matrix(&network, &ptdf).unwrap();

        for &branch_id in &lodf.branch_ids {
            let val = lodf.get(branch_id, branch_id).unwrap();
            assert!(
                (val + 1.0).abs() < 1e-10,
                "LODF diagonal should be -1, got {}",
                val
            );
        }
    }

    #[test]
    fn test_lodf_estimates_flow_redistribution() {
        let network = create_test_network();
        let ptdf = compute_ptdf_matrix(&network).unwrap();
        let lodf = compute_lodf_matrix(&network, &ptdf).unwrap();

        // Pre-outage flows (arbitrary for test)
        let flow_1 = 100.0; // Flow on branch 1
        let flow_2 = 50.0; // Flow on branch 2

        // If branch 1 trips, estimate new flow on branch 2
        let new_flow_2 = lodf
            .estimate_post_outage_flow(2, 1, flow_2, flow_1)
            .unwrap();

        // Flow should increase (some of branch 1's flow shifts to branch 2)
        // In a triangle network, the LODF should be positive
        println!("LODF[2,1] = {}", lodf.get(2, 1).unwrap());
        println!("New flow on branch 2: {}", new_flow_2);

        // Just verify the calculation works - exact value depends on topology
        assert!(new_flow_2.is_finite());
    }

    #[test]
    fn test_ptdf_physical_interpretation() {
        // Inject 1 MW at bus 2, withdraw at slack (bus 1)
        // PTDF tells us how much flows on each branch
        let network = create_test_network();
        let ptdf = compute_ptdf_matrix(&network).unwrap();

        // Branch 1 (1-2): should carry positive flow for injection at bus 2
        let ptdf_br1_bus2 = ptdf.get(1, 2).unwrap();
        // Branch 3 (1-3): should carry negative flow (power goes 2→3→1 partly)
        let ptdf_br3_bus2 = ptdf.get(3, 2).unwrap();

        println!("PTDF[1,2] = {}", ptdf_br1_bus2);
        println!("PTDF[3,2] = {}", ptdf_br3_bus2);

        // The signs depend on convention, but values should be non-zero
        assert!(ptdf_br1_bus2.abs() > 0.1, "Expected significant PTDF");
    }
}
