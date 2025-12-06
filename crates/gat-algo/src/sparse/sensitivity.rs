//! Sparse PTDF and LODF matrices for contingency analysis.
//!
//! ## Power Transfer Distribution Factors (PTDF)
//!
//! PTDF[ℓ,n] = sensitivity of flow on branch ℓ to injection at bus n:
//! ```text
//! ΔP_ℓ = PTDF[ℓ,n] × ΔP_injection_n
//! ```
//!
//! ## Line Outage Distribution Factors (LODF)
//!
//! LODF[ℓ,m] = fraction of branch m's flow that shifts to branch ℓ when m trips:
//! ```text
//! P_ℓ_post = P_ℓ_pre + LODF[ℓ,m] × P_m_pre
//! ```
//!
//! These enable fast N-k contingency screening without re-solving power flow.

use super::susceptance::{SparseSusceptance, SusceptanceError};
use gat_core::{BranchId, BusId, Edge, Network};
use sprs::CsMat;
use std::collections::HashMap;
use thiserror::Error;

/// Errors from sensitivity matrix operations
#[derive(Debug, Error)]
pub enum SensitivityError {
    #[error("Susceptance matrix error: {0}")]
    Susceptance(#[from] SusceptanceError),

    #[error("Network must have at least 2 buses")]
    TooFewBuses,

    #[error("Network must have at least 1 branch")]
    NoBranches,

    #[error("Matrix inversion failed: {0}")]
    InversionFailed(String),

    #[error("Branch {0} not found")]
    BranchNotFound(i64),
}

/// PTDF matrix: sensitivity of branch flows to bus injections.
///
/// Dense storage is used because PTDF matrices are typically dense
/// (every branch is affected by injection at every bus to some degree).
#[derive(Debug, Clone)]
pub struct PtdfMatrix {
    /// Row index → branch ID
    pub branch_ids: Vec<BranchId>,
    /// Column index → bus ID
    pub bus_ids: Vec<BusId>,
    /// PTDF values: ptdf[branch_idx][bus_idx]
    pub values: Vec<Vec<f64>>,
    /// Lookup: branch_id → row index
    branch_to_idx: HashMap<BranchId, usize>,
    /// Lookup: bus_id → column index
    bus_to_idx: HashMap<BusId, usize>,
}

impl PtdfMatrix {
    /// Get PTDF for branch ℓ with respect to injection at bus n.
    pub fn get(&self, branch_id: BranchId, bus_id: BusId) -> Option<f64> {
        let branch_idx = self.branch_to_idx.get(&branch_id)?;
        let bus_idx = self.bus_to_idx.get(&bus_id)?;
        Some(self.values[*branch_idx][*bus_idx])
    }

    /// Get PTDF by indices
    pub fn get_by_idx(&self, branch_idx: usize, bus_idx: usize) -> f64 {
        self.values
            .get(branch_idx)
            .and_then(|row| row.get(bus_idx))
            .copied()
            .unwrap_or(0.0)
    }

    /// Number of branches (rows).
    pub fn num_branches(&self) -> usize {
        self.branch_ids.len()
    }

    /// Number of buses (columns).
    pub fn num_buses(&self) -> usize {
        self.bus_ids.len()
    }

    /// Get branch index from ID
    pub fn branch_index(&self, id: BranchId) -> Option<usize> {
        self.branch_to_idx.get(&id).copied()
    }

    /// Get bus index from ID
    pub fn bus_index(&self, id: BusId) -> Option<usize> {
        self.bus_to_idx.get(&id).copied()
    }
}

/// LODF matrix: flow redistribution factors for branch outages.
#[derive(Debug, Clone)]
pub struct LodfMatrix {
    /// Row/column index → branch ID
    pub branch_ids: Vec<BranchId>,
    /// LODF values: lodf[ℓ_idx][m_idx]
    pub values: Vec<Vec<f64>>,
    /// Lookup: branch_id → index
    branch_to_idx: HashMap<BranchId, usize>,
}

impl LodfMatrix {
    /// Get LODF for branch ℓ when branch m is outaged.
    pub fn get(&self, branch_l: BranchId, branch_m: BranchId) -> Option<f64> {
        let l_idx = self.branch_to_idx.get(&branch_l)?;
        let m_idx = self.branch_to_idx.get(&branch_m)?;
        Some(self.values[*l_idx][*m_idx])
    }

    /// Estimate post-contingency flow on branch ℓ after branch m trips.
    /// `flow_l_post = flow_l_pre + LODF[ℓ,m] × flow_m_pre`
    pub fn estimate_post_outage_flow(
        &self,
        branch_l: BranchId,
        branch_m: BranchId,
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

    /// Get branch index from ID
    pub fn branch_index(&self, id: BranchId) -> Option<usize> {
        self.branch_to_idx.get(&id).copied()
    }
}

/// Sparse PTDF computation using factored B' matrix.
///
/// This is the main entry point for computing sensitivity factors.
pub struct SparsePtdf;

impl SparsePtdf {
    /// Compute PTDF matrix from network.
    ///
    /// Algorithm:
    /// 1. Build sparse B' susceptance matrix
    /// 2. Compute X = (B'_reduced)⁻¹ via LU factorization
    /// 3. For each branch ℓ from bus i to j:
    ///    PTDF[ℓ,n] = (X[i,n] - X[j,n]) / x_ℓ
    pub fn compute_ptdf(network: &Network) -> Result<PtdfMatrix, SensitivityError> {
        // Build susceptance matrix
        let b_prime = SparseSusceptance::from_network(network)?;
        let n_bus = b_prime.n_bus();

        if n_bus < 2 {
            return Err(SensitivityError::TooFewBuses);
        }

        // Collect branch data
        let mut branches: Vec<(BranchId, BusId, BusId, f64)> = Vec::new();
        for edge in network.graph.edge_references() {
            if let Edge::Branch(branch) = edge.weight() {
                if branch.status {
                    let x = (branch.reactance * branch.tap_ratio).abs().max(1e-6);
                    branches.push((branch.id, branch.from_bus, branch.to_bus, x));
                }
            }
        }

        if branches.is_empty() {
            return Err(SensitivityError::NoBranches);
        }

        // Sort by branch ID for consistent ordering
        branches.sort_by_key(|b| b.0.value());

        let branch_ids: Vec<BranchId> = branches.iter().map(|b| b.0).collect();
        let bus_ids: Vec<BusId> = b_prime.bus_order().to_vec();

        let branch_to_idx: HashMap<BranchId, usize> = branch_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();
        let bus_to_idx: HashMap<BusId, usize> =
            bus_ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();

        // Get reduced matrix (slack removed)
        let (reduced_matrix, _reduced_order) = b_prime.reduced_matrix();
        let slack_idx = b_prime.slack_idx();

        // Compute X = (B'_reduced)⁻¹
        let x_inv = Self::compute_b_inverse(&reduced_matrix, n_bus, slack_idx)?;

        // Compute PTDF
        let n_branches = branches.len();
        let mut ptdf = vec![vec![0.0; n_bus]; n_branches];

        for (branch_idx, &(_, from_bus, to_bus, x)) in branches.iter().enumerate() {
            let i = bus_to_idx[&from_bus];
            let j = bus_to_idx[&to_bus];

            for bus_idx in 0..n_bus {
                ptdf[branch_idx][bus_idx] = (x_inv[i][bus_idx] - x_inv[j][bus_idx]) / x;
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

    /// Compute LODF matrix from PTDF.
    ///
    /// LODF[ℓ,m] = PTDF_transfer[ℓ, i→j] / (1 - PTDF_transfer[m, i→j])
    ///
    /// where (i,j) are terminal buses of branch m.
    pub fn compute_lodf(
        network: &Network,
        ptdf: &PtdfMatrix,
    ) -> Result<LodfMatrix, SensitivityError> {
        // Get branch terminal buses mapping
        let mut branch_terminals: HashMap<BranchId, (BusId, BusId)> = HashMap::new();
        for edge in network.graph.edge_references() {
            if let Edge::Branch(branch) = edge.weight() {
                if branch.status {
                    branch_terminals.insert(branch.id, (branch.from_bus, branch.to_bus));
                }
            }
        }

        let n = ptdf.num_branches();
        let mut lodf = vec![vec![0.0; n]; n];

        for (l_idx, &branch_l) in ptdf.branch_ids.iter().enumerate() {
            for (m_idx, &branch_m) in ptdf.branch_ids.iter().enumerate() {
                if l_idx == m_idx {
                    // Diagonal: branch's LODF with itself is -1
                    lodf[l_idx][m_idx] = -1.0;
                    continue;
                }

                // Get terminal buses of branch m
                let (from_m, to_m) = match branch_terminals.get(&branch_m) {
                    Some(&buses) => buses,
                    None => continue,
                };

                // Transfer PTDF: sensitivity to transfer from from_m to to_m
                let ptdf_m_from = ptdf.get(branch_m, from_m).unwrap_or(0.0);
                let ptdf_m_to = ptdf.get(branch_m, to_m).unwrap_or(0.0);
                let ptdf_m_transfer = ptdf_m_from - ptdf_m_to;

                let ptdf_l_from = ptdf.get(branch_l, from_m).unwrap_or(0.0);
                let ptdf_l_to = ptdf.get(branch_l, to_m).unwrap_or(0.0);
                let ptdf_l_transfer = ptdf_l_from - ptdf_l_to;

                // LODF[ℓ,m] = PTDF_transfer[ℓ, i→j] / (1 - PTDF_transfer[m, i→j])
                let denom = 1.0 - ptdf_m_transfer;
                if denom.abs() < 1e-10 {
                    // Island or radial branch - LODF is undefined/infinite
                    lodf[l_idx][m_idx] = f64::INFINITY;
                } else {
                    lodf[l_idx][m_idx] = ptdf_l_transfer / denom;
                }
            }
        }

        Ok(LodfMatrix {
            branch_ids: ptdf.branch_ids.clone(),
            values: lodf,
            branch_to_idx: ptdf.branch_to_idx.clone(),
        })
    }

    /// Compute (B'_reduced)⁻¹ extended with zeros for slack bus.
    fn compute_b_inverse(
        reduced: &CsMat<f64>,
        full_size: usize,
        slack_idx: usize,
    ) -> Result<Vec<Vec<f64>>, SensitivityError> {
        let m = reduced.rows();
        if m == 0 {
            return Err(SensitivityError::InversionFailed("Empty matrix".into()));
        }

        // Convert sparse to dense for LU decomposition
        // (For very large networks, we'd use sparse direct solvers like UMFPACK)
        let mut dense = vec![vec![0.0; m]; m];
        for (val, (i, j)) in reduced.iter() {
            dense[i][j] = *val;
        }

        // LU decomposition with partial pivoting
        let inv_reduced = Self::lu_inverse(&dense)?;

        // Extend back to full size (slack row/col stays zero)
        let mut x = vec![vec![0.0; full_size]; full_size];

        // Map reduced indices back to full indices
        let mut reduced_to_full: Vec<usize> = Vec::with_capacity(m);
        for i in 0..full_size {
            if i != slack_idx {
                reduced_to_full.push(i);
            }
        }

        for (ri, &fi) in reduced_to_full.iter().enumerate() {
            for (rj, &fj) in reduced_to_full.iter().enumerate() {
                x[fi][fj] = inv_reduced[ri][rj];
            }
        }

        Ok(x)
    }

    /// LU decomposition-based matrix inverse with partial pivoting.
    fn lu_inverse(a: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, SensitivityError> {
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
                return Err(SensitivityError::InversionFailed(
                    "Matrix is singular".into(),
                ));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, Bus, Node};

    fn create_3bus_network() -> Network {
        let mut network = Network::new();

        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Default::default()
        }));
        let b2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Default::default()
        }));
        let b3 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(3),
            name: "Bus3".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            ..Default::default()
        }));

        // Triangle topology
        network.graph.add_edge(
            b1,
            b2,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Line1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                reactance: 0.1,
                ..Default::default()
            }),
        );
        network.graph.add_edge(
            b2,
            b3,
            Edge::Branch(Branch {
                id: BranchId::new(2),
                name: "Line2-3".to_string(),
                from_bus: BusId::new(2),
                to_bus: BusId::new(3),
                reactance: 0.1,
                ..Default::default()
            }),
        );
        network.graph.add_edge(
            b1,
            b3,
            Edge::Branch(Branch {
                id: BranchId::new(3),
                name: "Line1-3".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(3),
                reactance: 0.2,
                ..Default::default()
            }),
        );

        network
    }

    #[test]
    fn test_ptdf_dimensions() {
        let network = create_3bus_network();
        let ptdf = SparsePtdf::compute_ptdf(&network).unwrap();

        assert_eq!(ptdf.num_branches(), 3);
        assert_eq!(ptdf.num_buses(), 3);
    }

    #[test]
    fn test_ptdf_slack_bus_zero() {
        // PTDF values for injection at slack bus should be ~0
        let network = create_3bus_network();
        let ptdf = SparsePtdf::compute_ptdf(&network).unwrap();

        // Slack is first bus (BusId::new(1))
        let slack_bus = BusId::new(1);
        for &branch_id in &ptdf.branch_ids {
            let val = ptdf.get(branch_id, slack_bus).unwrap();
            assert!(
                val.abs() < 1e-10,
                "PTDF for slack should be ~0, got {}",
                val
            );
        }
    }

    #[test]
    fn test_lodf_diagonal() {
        let network = create_3bus_network();
        let ptdf = SparsePtdf::compute_ptdf(&network).unwrap();
        let lodf = SparsePtdf::compute_lodf(&network, &ptdf).unwrap();

        // Diagonal should be -1
        for &branch_id in &lodf.branch_ids {
            let val = lodf.get(branch_id, branch_id).unwrap();
            assert!((val + 1.0).abs() < 1e-10, "LODF diagonal should be -1");
        }
    }

    #[test]
    fn test_lodf_flow_estimate() {
        let network = create_3bus_network();
        let ptdf = SparsePtdf::compute_ptdf(&network).unwrap();
        let lodf = SparsePtdf::compute_lodf(&network, &ptdf).unwrap();

        let branch_1 = BranchId::new(1);
        let branch_2 = BranchId::new(2);

        // Pre-outage flows
        let flow_1 = 100.0;
        let flow_2 = 50.0;

        // Estimate flow on branch 2 after branch 1 trips
        let new_flow = lodf
            .estimate_post_outage_flow(branch_2, branch_1, flow_2, flow_1)
            .unwrap();

        assert!(new_flow.is_finite());
    }
}
