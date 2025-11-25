//! Y-Bus (Admittance Matrix) Construction
//!
//! The Y-bus matrix is fundamental to AC power flow analysis. Each element Y_ij
//! represents the admittance between buses i and j:
//!
//! ```text
//! Y_ij = -y_ij / (τ · e^{jφ})  (off-diagonal, i ≠ j)
//! Y_ii = Σ y_ik / τ² + y_sh_i  (diagonal: sum of incident admittances + shunt)
//! ```
//!
//! where:
//!   - y_ij = 1/(r_ij + jx_ij) is the series admittance
//!   - τ is the transformer tap ratio
//!   - φ is the phase shift angle
//!   - y_sh is the shunt admittance (line charging)
//!
//! ## References
//!
//! - Grainger & Stevenson, "Power System Analysis", Ch. 8
//! - Bergen & Vittal, "Power Systems Analysis", Ch. 9

use crate::opf::OpfError;
use gat_core::{BusId, Edge, Network, Node};
use num_complex::Complex64;
use std::collections::HashMap;

/// Dense Y-bus matrix for AC power flow
///
/// Stores the full admittance matrix in dense form. For large networks,
/// a sparse representation would be more efficient, but dense is simpler
/// for initial implementation and adequate for networks up to ~1000 buses.
#[derive(Debug, Clone)]
pub struct YBus {
    /// Number of buses
    n_bus: usize,
    /// Dense matrix storage (row-major)
    data: Vec<Complex64>,
    /// Bus ID to index mapping
    bus_map: HashMap<BusId, usize>,
}

impl YBus {
    /// Get element at (row, col)
    pub fn get(&self, row: usize, col: usize) -> Complex64 {
        self.data[row * self.n_bus + col]
    }

    /// Get mutable reference to element at (row, col)
    fn get_mut(&mut self, row: usize, col: usize) -> &mut Complex64 {
        &mut self.data[row * self.n_bus + col]
    }

    /// Number of buses
    pub fn n_bus(&self) -> usize {
        self.n_bus
    }

    /// Get bus index from ID
    pub fn bus_index(&self, id: BusId) -> Option<usize> {
        self.bus_map.get(&id).copied()
    }

    /// Get the G matrix (real part of Y-bus)
    pub fn g_matrix(&self) -> Vec<f64> {
        self.data.iter().map(|y| y.re).collect()
    }

    /// Get the B matrix (imaginary part of Y-bus)
    pub fn b_matrix(&self) -> Vec<f64> {
        self.data.iter().map(|y| y.im).collect()
    }
}

/// Builder for Y-bus matrix
pub struct YBusBuilder;

impl YBusBuilder {
    /// Build Y-bus matrix from network
    ///
    /// # Algorithm
    ///
    /// 1. Count buses and assign indices
    /// 2. For each branch:
    ///    - Compute series admittance y = 1/(r + jx)
    ///    - Add -y to off-diagonal Y_ij, Y_ji
    ///    - Add y to diagonal Y_ii, Y_jj
    ///    - Add shunt admittance jB/2 to diagonal at each end
    /// 3. Handle tap ratio and phase shift for transformers
    pub fn from_network(network: &Network) -> Result<YBus, OpfError> {
        // Count buses and build index map
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
            return Err(OpfError::DataValidation(
                "No buses in network".to_string(),
            ));
        }

        // Initialize Y-bus with zeros
        let mut ybus = YBus {
            n_bus,
            data: vec![Complex64::new(0.0, 0.0); n_bus * n_bus],
            bus_map,
        };

        // Process each branch
        for edge_idx in network.graph.edge_indices() {
            if let Edge::Branch(branch) = &network.graph[edge_idx] {
                if !branch.status {
                    continue; // Skip out-of-service branches
                }

                let from_idx = ybus.bus_index(branch.from_bus).ok_or_else(|| {
                    OpfError::DataValidation(format!(
                        "Branch {} references unknown from_bus",
                        branch.name
                    ))
                })?;

                let to_idx = ybus.bus_index(branch.to_bus).ok_or_else(|| {
                    OpfError::DataValidation(format!(
                        "Branch {} references unknown to_bus",
                        branch.name
                    ))
                })?;

                // Compute series admittance: y = 1 / (r + jx)
                let z = Complex64::new(branch.resistance, branch.reactance);
                if z.norm() < 1e-12 {
                    return Err(OpfError::DataValidation(format!(
                        "Branch {} has zero impedance",
                        branch.name
                    )));
                }
                let y_series = z.inv();

                // Tap ratio and phase shift
                let tau = branch.tap_ratio;
                let phi = branch.phase_shift_rad;
                let tau2 = tau * tau;

                // Phase shift phasor: e^{-jφ}
                let shift = Complex64::from_polar(1.0, -phi);

                // Shunt admittance (half at each end)
                let y_shunt_half = Complex64::new(0.0, branch.charging_b_pu / 2.0);

                // Update Y-bus entries using transformer model:
                //
                // For a transformer with tap τ and phase shift φ:
                //   Y_ii += y/τ² + y_shunt
                //   Y_jj += y + y_shunt
                //   Y_ij += -y/(τ·e^{jφ})
                //   Y_ji += -y/(τ·e^{-jφ})
                //
                // For a simple line (τ=1, φ=0):
                //   Y_ii += y + y_shunt
                //   Y_jj += y + y_shunt
                //   Y_ij = Y_ji = -y

                // Diagonal entries
                *ybus.get_mut(from_idx, from_idx) += y_series / tau2 + y_shunt_half;
                *ybus.get_mut(to_idx, to_idx) += y_series + y_shunt_half;

                // Off-diagonal entries
                *ybus.get_mut(from_idx, to_idx) += -y_series / tau * shift.conj();
                *ybus.get_mut(to_idx, from_idx) += -y_series / tau * shift;
            }
        }

        Ok(ybus)
    }
}
