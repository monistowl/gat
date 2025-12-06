//! Sparse Y-bus (admittance) matrix for AC power flow.
//!
//! The Y-bus matrix is the complex admittance matrix used in AC power flow:
//! ```text
//! I = Y × V
//!
//! where Y[i,j] = G[i,j] + jB[i,j] (conductance + j×susceptance)
//! ```
//!
//! This module provides CSR storage for the real (G) and imaginary (B) parts
//! separately, enabling efficient sparse operations.

use gat_core::{BusId, Edge, Network, Node};
use num_complex::Complex64;
use sprs::{CsMat, CsMatView, TriMat};
use std::collections::HashMap;
use thiserror::Error;

/// Errors from Y-bus matrix operations
#[derive(Debug, Error)]
pub enum YBusError {
    #[error("No buses found in network")]
    NoBuses,

    #[error("Branch {0} has zero impedance")]
    ZeroImpedance(String),

    #[error("Unknown bus ID: {0}")]
    UnknownBus(usize),
}

/// Sparse Y-bus matrix in CSR format.
///
/// Stores G (conductance) and B (susceptance) matrices separately for
/// efficient access to real and imaginary parts.
#[derive(Debug, Clone)]
pub struct SparseYBus {
    /// Number of buses
    n_bus: usize,
    /// Real part (conductance G) in CSR format
    g_matrix: CsMat<f64>,
    /// Imaginary part (susceptance B) in CSR format
    b_matrix: CsMat<f64>,
    /// Bus ID to index mapping
    bus_map: HashMap<BusId, usize>,
    /// Index to Bus ID mapping
    idx_to_bus: Vec<BusId>,
}

impl SparseYBus {
    /// Build sparse Y-bus from network.
    pub fn from_network(network: &Network) -> Result<Self, YBusError> {
        // Index buses
        let mut bus_map: HashMap<BusId, usize> = HashMap::new();
        let mut idx_to_bus: Vec<BusId> = Vec::new();
        let mut bus_idx = 0;

        for node_idx in network.graph.node_indices() {
            if let Node::Bus(bus) = &network.graph[node_idx] {
                bus_map.insert(bus.id, bus_idx);
                idx_to_bus.push(bus.id);
                bus_idx += 1;
            }
        }

        let n_bus = bus_map.len();
        if n_bus == 0 {
            return Err(YBusError::NoBuses);
        }

        // Build triplet matrices (COO format, then convert to CSR)
        let mut g_triplet = TriMat::new((n_bus, n_bus));
        let mut b_triplet = TriMat::new((n_bus, n_bus));

        // Process branches
        for edge_idx in network.graph.edge_indices() {
            if let Edge::Branch(branch) = &network.graph[edge_idx] {
                if !branch.status {
                    continue;
                }

                let from_idx = *bus_map
                    .get(&branch.from_bus)
                    .ok_or(YBusError::UnknownBus(branch.from_bus.value()))?;
                let to_idx = *bus_map
                    .get(&branch.to_bus)
                    .ok_or(YBusError::UnknownBus(branch.to_bus.value()))?;

                // Series admittance y = 1/(r + jx)
                let z = Complex64::new(branch.resistance, branch.reactance);
                if z.norm() < 1e-12 {
                    return Err(YBusError::ZeroImpedance(branch.name.clone()));
                }
                let y_series = z.inv();

                let tau = branch.tap_ratio;
                let phi = branch.phase_shift.value();
                let tau2 = tau * tau;
                let shift = Complex64::from_polar(1.0, -phi);

                let y_shunt_half = Complex64::new(0.0, branch.charging_b.value() / 2.0);

                // Diagonal entries
                let y_ii = y_series / tau2 + y_shunt_half;
                let y_jj = y_series + y_shunt_half;

                // Off-diagonal entries
                let y_ij = -y_series / tau * shift.conj();
                let y_ji = -y_series / tau * shift;

                // Accumulate into triplets
                g_triplet.add_triplet(from_idx, from_idx, y_ii.re);
                b_triplet.add_triplet(from_idx, from_idx, y_ii.im);
                g_triplet.add_triplet(to_idx, to_idx, y_jj.re);
                b_triplet.add_triplet(to_idx, to_idx, y_jj.im);
                g_triplet.add_triplet(from_idx, to_idx, y_ij.re);
                b_triplet.add_triplet(from_idx, to_idx, y_ij.im);
                g_triplet.add_triplet(to_idx, from_idx, y_ji.re);
                b_triplet.add_triplet(to_idx, from_idx, y_ji.im);
            }
        }

        // Add shunt admittances
        for node_idx in network.graph.node_indices() {
            if let Node::Shunt(shunt) = &network.graph[node_idx] {
                if let Some(&bus_idx) = bus_map.get(&shunt.bus) {
                    // Shunt admittance: Y_sh = G_sh + jB_sh
                    g_triplet.add_triplet(bus_idx, bus_idx, shunt.gs_pu);
                    b_triplet.add_triplet(bus_idx, bus_idx, shunt.bs_pu);
                }
            }
        }

        Ok(Self {
            n_bus,
            g_matrix: g_triplet.to_csr(),
            b_matrix: b_triplet.to_csr(),
            bus_map,
            idx_to_bus,
        })
    }

    /// Number of buses
    pub fn n_bus(&self) -> usize {
        self.n_bus
    }

    /// Get G[i,j] (conductance)
    pub fn g(&self, i: usize, j: usize) -> f64 {
        self.g_matrix.get(i, j).copied().unwrap_or(0.0)
    }

    /// Get B[i,j] (susceptance)
    pub fn b(&self, i: usize, j: usize) -> f64 {
        self.b_matrix.get(i, j).copied().unwrap_or(0.0)
    }

    /// Get complex Y[i,j] = G[i,j] + jB[i,j]
    pub fn y(&self, i: usize, j: usize) -> Complex64 {
        Complex64::new(self.g(i, j), self.b(i, j))
    }

    /// Get bus index from ID
    pub fn bus_index(&self, id: BusId) -> Option<usize> {
        self.bus_map.get(&id).copied()
    }

    /// Get bus ID from index
    pub fn bus_id(&self, idx: usize) -> Option<BusId> {
        self.idx_to_bus.get(idx).copied()
    }

    /// Get G matrix view
    pub fn g_view(&self) -> CsMatView<'_, f64> {
        self.g_matrix.view()
    }

    /// Get B matrix view
    pub fn b_view(&self) -> CsMatView<'_, f64> {
        self.b_matrix.view()
    }

    /// Number of non-zeros in G matrix
    pub fn g_nnz(&self) -> usize {
        self.g_matrix.nnz()
    }

    /// Number of non-zeros in B matrix
    pub fn b_nnz(&self) -> usize {
        self.b_matrix.nnz()
    }

    /// Total non-zeros (G + B)
    pub fn nnz(&self) -> usize {
        self.g_nnz() + self.b_nnz()
    }

    /// Iterate over non-zero entries in row i of G matrix (zero-allocation).
    ///
    /// Directly accesses CSR arrays to avoid temporary CsVecView allocation.
    pub fn g_row_iter(&self, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        let indptr = self.g_matrix.indptr();
        let start = indptr.index(i);
        let end = indptr.index(i + 1);
        let indices = &self.g_matrix.indices()[start..end];
        let data = &self.g_matrix.data()[start..end];
        indices.iter().zip(data.iter()).map(|(&j, &v)| (j, v))
    }

    /// Iterate over non-zero entries in row i of B matrix (zero-allocation).
    ///
    /// Directly accesses CSR arrays to avoid temporary CsVecView allocation.
    pub fn b_row_iter(&self, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        let indptr = self.b_matrix.indptr();
        let start = indptr.index(i);
        let end = indptr.index(i + 1);
        let indices = &self.b_matrix.indices()[start..end];
        let data = &self.b_matrix.data()[start..end];
        indices.iter().zip(data.iter()).map(|(&j, &v)| (j, v))
    }

    /// Memory usage in bytes (approximate)
    pub fn memory_bytes(&self) -> usize {
        // CSR: nnz values + nnz col indices + (n+1) row ptrs, times 2 for G and B
        let g_mem = self.g_nnz() * 16 + (self.n_bus + 1) * 8;
        let b_mem = self.b_nnz() * 16 + (self.n_bus + 1) * 8;
        g_mem + b_mem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus};

    fn create_3bus_network() -> Network {
        let mut network = Network::new();

        // Add 3 buses
        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            voltage_pu: gat_core::PerUnit(1.0),
            angle_rad: gat_core::Radians(0.0),
            vmin_pu: Some(gat_core::PerUnit(0.95)),
            vmax_pu: Some(gat_core::PerUnit(1.05)),
            ..Default::default()
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            voltage_pu: gat_core::PerUnit(1.0),
            angle_rad: gat_core::Radians(0.0),
            vmin_pu: Some(gat_core::PerUnit(0.95)),
            vmax_pu: Some(gat_core::PerUnit(1.05)),
            ..Default::default()
        }));
        let bus3_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(3),
            name: "Bus3".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            voltage_pu: gat_core::PerUnit(1.0),
            angle_rad: gat_core::Radians(0.0),
            vmin_pu: Some(gat_core::PerUnit(0.95)),
            vmax_pu: Some(gat_core::PerUnit(1.05)),
            ..Default::default()
        }));

        // Add branches (forming a triangle topology)
        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Branch1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                charging_b: gat_core::PerUnit(0.02),
                s_max: Some(gat_core::MegavoltAmperes(100.0)),
                rating_a: Some(gat_core::MegavoltAmperes(100.0)),
                ..Branch::default()
            }),
        );
        network.graph.add_edge(
            bus2_idx,
            bus3_idx,
            Edge::Branch(Branch {
                id: BranchId::new(2),
                name: "Branch2-3".to_string(),
                from_bus: BusId::new(2),
                to_bus: BusId::new(3),
                resistance: 0.01,
                reactance: 0.1,
                charging_b: gat_core::PerUnit(0.02),
                s_max: Some(gat_core::MegavoltAmperes(100.0)),
                rating_a: Some(gat_core::MegavoltAmperes(100.0)),
                ..Branch::default()
            }),
        );
        network.graph.add_edge(
            bus1_idx,
            bus3_idx,
            Edge::Branch(Branch {
                id: BranchId::new(3),
                name: "Branch1-3".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(3),
                resistance: 0.01,
                reactance: 0.1,
                charging_b: gat_core::PerUnit(0.02),
                s_max: Some(gat_core::MegavoltAmperes(100.0)),
                rating_a: Some(gat_core::MegavoltAmperes(100.0)),
                ..Branch::default()
            }),
        );

        network
    }

    #[test]
    fn test_ybus_construction() {
        let network = create_3bus_network();
        let ybus = SparseYBus::from_network(&network).unwrap();
        assert_eq!(ybus.n_bus(), 3);
        // Diagonal should be non-zero
        assert!(ybus.g(0, 0).abs() > 0.0 || ybus.b(0, 0).abs() > 0.0);
    }

    #[test]
    fn test_ybus_symmetry_no_phase_shift() {
        let network = create_3bus_network();
        let ybus = SparseYBus::from_network(&network).unwrap();
        // Without phase shifters, Y-bus should be symmetric
        for i in 0..ybus.n_bus() {
            for j in 0..ybus.n_bus() {
                let g_diff = (ybus.g(i, j) - ybus.g(j, i)).abs();
                let b_diff = (ybus.b(i, j) - ybus.b(j, i)).abs();
                assert!(g_diff < 1e-10, "G asymmetry at [{},{}]", i, j);
                assert!(b_diff < 1e-10, "B asymmetry at [{},{}]", i, j);
            }
        }
    }

    #[test]
    fn test_ybus_bus_mapping() {
        let network = create_3bus_network();
        let ybus = SparseYBus::from_network(&network).unwrap();

        // Verify round-trip
        for idx in 0..ybus.n_bus() {
            let bus_id = ybus.bus_id(idx).unwrap();
            let back_idx = ybus.bus_index(bus_id).unwrap();
            assert_eq!(idx, back_idx);
        }
    }
}
