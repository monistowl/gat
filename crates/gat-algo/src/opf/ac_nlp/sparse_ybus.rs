//! Sparse Y-bus matrix for large-scale AC power flow.
//!
//! Uses CSR (Compressed Sparse Row) format via `sprs` for O(nnz) storage
//! instead of O(n²) dense storage. Critical for networks > 500 buses.

use crate::opf::OpfError;
use gat_core::{BusId, Edge, Network, Node};
use num_complex::Complex64;
use sprs::{CsMat, TriMat};
use std::collections::HashMap;

/// Sparse Y-bus matrix in CSR format.
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
}

impl SparseYBus {
    /// Build sparse Y-bus from network.
    pub fn from_network(network: &Network) -> Result<Self, OpfError> {
        // Index buses
        let mut bus_map: HashMap<BusId, usize> = HashMap::new();
        let mut bus_idx = 0;
        for node_idx in network.graph.node_indices() {
            if let Node::Bus(bus) = &network.graph[node_idx] {
                bus_map.insert(bus.id, bus_idx);
                bus_idx += 1;
            }
        }
        let n_bus = bus_map.len();
        if n_bus == 0 {
            return Err(OpfError::DataValidation("No buses in network".to_string()));
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

                let from_idx = *bus_map.get(&branch.from_bus).ok_or_else(|| {
                    OpfError::DataValidation(format!(
                        "Unknown from_bus {}",
                        branch.from_bus.value()
                    ))
                })?;
                let to_idx = *bus_map.get(&branch.to_bus).ok_or_else(|| {
                    OpfError::DataValidation(format!("Unknown to_bus {}", branch.to_bus.value()))
                })?;

                // Series admittance y = 1/(r + jx)
                let z = Complex64::new(branch.resistance, branch.reactance);
                if z.norm() < 1e-12 {
                    return Err(OpfError::DataValidation(format!(
                        "Branch {} has zero impedance",
                        branch.name
                    )));
                }
                let y_series = z.inv();

                let tau = branch.tap_ratio;
                let phi = branch.phase_shift_rad;
                let tau2 = tau * tau;
                let shift = Complex64::from_polar(1.0, -phi);

                let y_shunt_half = Complex64::new(0.0, branch.charging_b_pu / 2.0);

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

        Ok(Self {
            n_bus,
            g_matrix: g_triplet.to_csr(),
            b_matrix: b_triplet.to_csr(),
            bus_map,
        })
    }

    /// Number of buses
    pub fn n_bus(&self) -> usize {
        self.n_bus
    }

    /// Get G_ij (conductance)
    pub fn g(&self, i: usize, j: usize) -> f64 {
        self.g_matrix.get(i, j).copied().unwrap_or(0.0)
    }

    /// Get B_ij (susceptance)
    pub fn b(&self, i: usize, j: usize) -> f64 {
        self.b_matrix.get(i, j).copied().unwrap_or(0.0)
    }

    /// Get bus index from ID
    pub fn bus_index(&self, id: BusId) -> Option<usize> {
        self.bus_map.get(&id).copied()
    }

    /// Iterate over non-zero entries in row i of G matrix
    pub fn g_row_iter(&self, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.g_matrix
            .outer_view(i)
            .map(|row| row.iter().map(|(j, &v)| (j, v)).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    /// Iterate over non-zero entries in row i of B matrix
    pub fn b_row_iter(&self, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.b_matrix
            .outer_view(i)
            .map(|row| row.iter().map(|(j, &v)| (j, v)).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus, BusId, CostModel, Gen, GenId, Load, LoadId};

    /// Create a simple 3-bus test network inline
    fn create_3bus_network() -> Network {
        let mut network = Network::new();

        // Add 3 buses
        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            vmin_pu: Some(0.95),
            vmax_pu: Some(1.05),
            area_id: None,
            zone_id: None,
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            vmin_pu: Some(0.95),
            vmax_pu: Some(1.05),
            area_id: None,
            zone_id: None,
        }));
        let bus3_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(3),
            name: "Bus3".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            vmin_pu: Some(0.95),
            vmax_pu: Some(1.05),
            area_id: None,
            zone_id: None,
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
                charging_b_pu: 0.02,
                s_max_mva: Some(100.0),
                rating_a_mva: Some(100.0),
                tap_ratio: 1.0,
                phase_shift_rad: 0.0,
                status: true,
                ..Default::default()
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
                charging_b_pu: 0.02,
                s_max_mva: Some(100.0),
                rating_a_mva: Some(100.0),
                tap_ratio: 1.0,
                phase_shift_rad: 0.0,
                status: true,
                ..Default::default()
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
                charging_b_pu: 0.02,
                s_max_mva: Some(100.0),
                rating_a_mva: Some(100.0),
                tap_ratio: 1.0,
                phase_shift_rad: 0.0,
                status: true,
                ..Default::default()
            }),
        );

        // Add generator at bus 1
        network.graph.add_node(Node::Gen(
            Gen::new(GenId::new(1), "Gen1".to_string(), BusId::new(1))
                .with_p_limits(0.0, 100.0)
                .with_q_limits(-50.0, 50.0)
                .with_cost(CostModel::quadratic(0.0, 20.0, 0.01)),
        ));

        // Add load at bus 3
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load3".to_string(),
            bus: BusId::new(3),
            active_power_mw: 50.0,
            reactive_power_mvar: 20.0,
        }));

        network
    }

    #[test]
    fn test_sparse_ybus_construction() {
        let network = create_3bus_network();
        let ybus = SparseYBus::from_network(&network).unwrap();
        assert_eq!(ybus.n_bus(), 3);
        // Diagonal should be non-zero
        assert!(ybus.g(0, 0).abs() > 0.0 || ybus.b(0, 0).abs() > 0.0);
    }

    #[test]
    fn test_sparse_ybus_symmetry_no_phase_shift() {
        let network = create_3bus_network();
        let ybus = SparseYBus::from_network(&network).unwrap();
        // Without phase shifters, Y-bus should be symmetric
        for i in 0..ybus.n_bus() {
            for j in 0..ybus.n_bus() {
                let g_diff = (ybus.g(i, j) - ybus.g(j, i)).abs();
                let b_diff = (ybus.b(i, j) - ybus.b(j, i)).abs();
                assert!(
                    g_diff < 1e-10,
                    "G[{},{}]={} != G[{},{}]={}, diff={}",
                    i,
                    j,
                    ybus.g(i, j),
                    j,
                    i,
                    ybus.g(j, i),
                    g_diff
                );
                assert!(
                    b_diff < 1e-10,
                    "B[{},{}]={} != B[{},{}]={}, diff={}",
                    i,
                    j,
                    ybus.b(i, j),
                    j,
                    i,
                    ybus.b(j, i),
                    b_diff
                );
            }
        }
    }

    #[test]
    fn test_sparse_ybus_bus_mapping() {
        let network = create_3bus_network();
        let ybus = SparseYBus::from_network(&network).unwrap();

        // Verify bus ID to index mapping
        assert_eq!(ybus.bus_index(BusId::new(1)), Some(0));
        assert_eq!(ybus.bus_index(BusId::new(2)), Some(1));
        assert_eq!(ybus.bus_index(BusId::new(3)), Some(2));
        assert_eq!(ybus.bus_index(BusId::new(99)), None);
    }

    #[test]
    fn test_sparse_ybus_off_diagonal() {
        let network = create_3bus_network();
        let ybus = SparseYBus::from_network(&network).unwrap();

        // For a 3-bus triangle network, all buses are connected
        // So off-diagonal elements should be non-zero for connected buses
        assert!(
            ybus.g(0, 1).abs() > 1e-10 || ybus.b(0, 1).abs() > 1e-10,
            "Bus 0-1 should be connected"
        );
        assert!(
            ybus.g(1, 2).abs() > 1e-10 || ybus.b(1, 2).abs() > 1e-10,
            "Bus 1-2 should be connected"
        );
        assert!(
            ybus.g(0, 2).abs() > 1e-10 || ybus.b(0, 2).abs() > 1e-10,
            "Bus 0-2 should be connected"
        );
    }

    #[test]
    fn test_sparse_ybus_row_iteration() {
        let network = create_3bus_network();
        let ybus = SparseYBus::from_network(&network).unwrap();

        // Test G matrix row iteration
        let g_row_0: Vec<_> = ybus.g_row_iter(0).collect();
        assert!(!g_row_0.is_empty(), "Row 0 should have non-zero G entries");

        // Test B matrix row iteration
        let b_row_0: Vec<_> = ybus.b_row_iter(0).collect();
        assert!(!b_row_0.is_empty(), "Row 0 should have non-zero B entries");
    }
}
