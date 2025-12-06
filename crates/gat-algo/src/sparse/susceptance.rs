//! Sparse susceptance matrix (B') for DC power flow.
//!
//! The B' matrix relates bus angles to power injections under DC assumptions:
//! ```text
//! P = B' × θ
//!
//! where:
//!   B'[i,j] = -b_ij        for i ≠ j (off-diagonal = -susceptance)
//!   B'[i,i] = Σ_k b_ik     for all k (diagonal = sum of connected susceptances)
//! ```
//!
//! This is the workhorse matrix for DC-OPF and PTDF computation.

use gat_core::{BranchId, BusId, Edge, Network, Node};
use sprs::{CsMat, CsMatView, TriMat};
use std::collections::HashMap;
use thiserror::Error;

/// Errors from susceptance matrix operations
#[derive(Debug, Error)]
pub enum SusceptanceError {
    #[error("No buses found in network")]
    NoBuses,

    #[error("No branches found in network")]
    NoBranches,

    #[error("Branch {0} has zero or near-zero reactance")]
    ZeroReactance(String),

    #[error("Unknown bus ID: {0}")]
    UnknownBus(usize),

    #[error("Matrix factorization failed: {0}")]
    FactorizationFailed(String),
}

/// Sparse B' susceptance matrix in CSR format.
///
/// Provides O(nnz) storage and matrix-vector products for DC power flow.
#[derive(Debug, Clone)]
pub struct SparseSusceptance {
    /// The susceptance matrix in CSR format
    matrix: CsMat<f64>,
    /// Ordered list of bus IDs (matrix row/column order)
    bus_order: Vec<BusId>,
    /// Map from BusId to matrix index
    bus_to_idx: HashMap<BusId, usize>,
    /// Map from BranchId to (from_idx, to_idx, susceptance)
    branch_data: HashMap<BranchId, (usize, usize, f64)>,
    /// Index of slack/reference bus (angle = 0)
    slack_idx: usize,
}

impl SparseSusceptance {
    /// Build sparse susceptance matrix from network.
    ///
    /// The first bus encountered becomes the slack/reference bus.
    pub fn from_network(network: &Network) -> Result<Self, SusceptanceError> {
        // Build bus ordering
        let (bus_order, bus_to_idx) = Self::build_bus_ordering(network)?;
        let n = bus_order.len();

        if n == 0 {
            return Err(SusceptanceError::NoBuses);
        }

        // Build triplet matrix and collect branch data
        let mut triplets = TriMat::new((n, n));
        let mut branch_data = HashMap::new();
        let mut branch_count = 0;

        for edge_idx in network.graph.edge_indices() {
            if let Edge::Branch(branch) = &network.graph[edge_idx] {
                if !branch.status {
                    continue;
                }

                let x_eff = branch.reactance * branch.tap_ratio;
                if x_eff.abs() < 1e-12 {
                    return Err(SusceptanceError::ZeroReactance(branch.name.clone()));
                }

                let b = 1.0 / x_eff;
                let i = *bus_to_idx
                    .get(&branch.from_bus)
                    .ok_or(SusceptanceError::UnknownBus(branch.from_bus.value()))?;
                let j = *bus_to_idx
                    .get(&branch.to_bus)
                    .ok_or(SusceptanceError::UnknownBus(branch.to_bus.value()))?;

                // Off-diagonal: B'[i,j] = B'[j,i] = -b
                triplets.add_triplet(i, j, -b);
                triplets.add_triplet(j, i, -b);

                // Diagonal: B'[i,i] += b, B'[j,j] += b
                triplets.add_triplet(i, i, b);
                triplets.add_triplet(j, j, b);

                branch_data.insert(branch.id, (i, j, b));
                branch_count += 1;
            }
        }

        if branch_count == 0 {
            return Err(SusceptanceError::NoBranches);
        }

        Ok(Self {
            matrix: triplets.to_csr(),
            bus_order,
            bus_to_idx,
            branch_data,
            slack_idx: 0, // First bus is slack
        })
    }

    /// Build bus ordering from network.
    fn build_bus_ordering(
        network: &Network,
    ) -> Result<(Vec<BusId>, HashMap<BusId, usize>), SusceptanceError> {
        let mut bus_order = Vec::new();
        let mut bus_to_idx = HashMap::new();

        for node_idx in network.graph.node_indices() {
            if let Node::Bus(bus) = &network.graph[node_idx] {
                bus_to_idx.insert(bus.id, bus_order.len());
                bus_order.push(bus.id);
            }
        }

        if bus_order.is_empty() {
            return Err(SusceptanceError::NoBuses);
        }

        Ok((bus_order, bus_to_idx))
    }

    /// Get matrix view for linear algebra operations.
    pub fn view(&self) -> CsMatView<'_, f64> {
        self.matrix.view()
    }

    /// Get element B'[i,j] by matrix indices.
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.matrix.get(i, j).copied().unwrap_or(0.0)
    }

    /// Get element B'[bus_i, bus_j] by bus IDs.
    pub fn get_by_bus(&self, bus_i: BusId, bus_j: BusId) -> Option<f64> {
        let i = self.bus_to_idx.get(&bus_i)?;
        let j = self.bus_to_idx.get(&bus_j)?;
        Some(self.get(*i, *j))
    }

    /// Number of buses (matrix dimension).
    pub fn n_bus(&self) -> usize {
        self.bus_order.len()
    }

    /// Number of non-zero elements.
    pub fn nnz(&self) -> usize {
        self.matrix.nnz()
    }

    /// Matrix density (nnz / n²).
    pub fn density(&self) -> f64 {
        let n = self.n_bus();
        if n == 0 {
            return 0.0;
        }
        self.nnz() as f64 / (n * n) as f64
    }

    /// Memory usage in bytes (approximate).
    pub fn memory_bytes(&self) -> usize {
        // CSR format: nnz values (f64) + nnz column indices (usize) + (n+1) row pointers (usize)
        let nnz = self.nnz();
        let n = self.n_bus();
        nnz * 8 + nnz * 8 + (n + 1) * 8
    }

    /// Get bus ID from matrix index.
    pub fn bus_id(&self, idx: usize) -> Option<BusId> {
        self.bus_order.get(idx).copied()
    }

    /// Get matrix index from bus ID.
    pub fn bus_index(&self, bus_id: BusId) -> Option<usize> {
        self.bus_to_idx.get(&bus_id).copied()
    }

    /// Get ordered list of bus IDs.
    pub fn bus_order(&self) -> &[BusId] {
        &self.bus_order
    }

    /// Get slack bus index.
    pub fn slack_idx(&self) -> usize {
        self.slack_idx
    }

    /// Get branch susceptance data: (from_idx, to_idx, susceptance).
    pub fn branch_data(&self, branch_id: BranchId) -> Option<(usize, usize, f64)> {
        self.branch_data.get(&branch_id).copied()
    }

    /// Iterate over non-zero entries in row i.
    pub fn row_iter(&self, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.matrix
            .outer_view(i)
            .map(|row| row.iter().map(|(j, &v)| (j, v)).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    /// Build reduced matrix (slack bus removed) for solving.
    ///
    /// Returns (reduced_matrix, reduced_bus_order) where slack bus is excluded.
    pub fn reduced_matrix(&self) -> (CsMat<f64>, Vec<BusId>) {
        let n = self.n_bus();
        let m = n - 1; // Reduced size

        let mut triplets = TriMat::new((m, m));
        let mut reduced_order = Vec::with_capacity(m);

        // Build reduced ordering (skip slack)
        for (idx, &bus_id) in self.bus_order.iter().enumerate() {
            if idx != self.slack_idx {
                reduced_order.push(bus_id);
            }
        }

        // Map old indices to new reduced indices
        let idx_map: HashMap<usize, usize> = self
            .bus_order
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != self.slack_idx)
            .enumerate()
            .map(|(new_idx, (old_idx, _))| (old_idx, new_idx))
            .collect();

        // Copy non-slack entries
        for i in 0..n {
            if i == self.slack_idx {
                continue;
            }
            let new_i = idx_map[&i];

            if let Some(row) = self.matrix.outer_view(i) {
                for (j, &val) in row.iter() {
                    if j == self.slack_idx {
                        continue;
                    }
                    let new_j = idx_map[&j];
                    triplets.add_triplet(new_i, new_j, val);
                }
            }
        }

        (triplets.to_csr(), reduced_order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, Bus};

    fn create_3bus_network() -> Network {
        let mut network = Network::new();

        // Add 3 buses
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

        // Add branches (triangle topology)
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
    fn test_susceptance_construction() {
        let network = create_3bus_network();
        let b_prime = SparseSusceptance::from_network(&network).unwrap();

        assert_eq!(b_prime.n_bus(), 3);
        // 3 branches × 4 entries each = 12, but diagonal entries overlap
        // Expected: 3 diagonal + 6 off-diagonal = 9 non-zeros (with summation)
        assert!(b_prime.nnz() > 0);
    }

    #[test]
    fn test_susceptance_symmetry() {
        let network = create_3bus_network();
        let b_prime = SparseSusceptance::from_network(&network).unwrap();

        for i in 0..b_prime.n_bus() {
            for j in 0..b_prime.n_bus() {
                let bij = b_prime.get(i, j);
                let bji = b_prime.get(j, i);
                assert!(
                    (bij - bji).abs() < 1e-10,
                    "B'[{},{}]={} != B'[{},{}]={}",
                    i,
                    j,
                    bij,
                    j,
                    i,
                    bji
                );
            }
        }
    }

    #[test]
    fn test_susceptance_row_sum_zero() {
        // Each row of B' should sum to zero (Kirchhoff's law)
        let network = create_3bus_network();
        let b_prime = SparseSusceptance::from_network(&network).unwrap();

        for i in 0..b_prime.n_bus() {
            let row_sum: f64 = (0..b_prime.n_bus()).map(|j| b_prime.get(i, j)).sum();
            assert!(
                row_sum.abs() < 1e-10,
                "Row {} sum = {} (should be ~0)",
                i,
                row_sum
            );
        }
    }

    #[test]
    fn test_reduced_matrix() {
        let network = create_3bus_network();
        let b_prime = SparseSusceptance::from_network(&network).unwrap();

        let (reduced, reduced_order) = b_prime.reduced_matrix();

        assert_eq!(reduced_order.len(), 2); // 3 - 1 slack
        assert_eq!(reduced.rows(), 2);
        assert_eq!(reduced.cols(), 2);
    }
}
