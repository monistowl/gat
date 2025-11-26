//! # Y-Bus (Admittance Matrix) Construction
//!
//! The Y-bus matrix is the foundation of AC power flow analysis. It encodes
//! the network topology and electrical parameters in a single matrix that
//! relates bus voltages to current injections via Ohm's law: **I = Y · V**.
//!
//! ## Physical Intuition
//!
//! In AC circuits, the relationship between voltage and current is governed
//! by **impedance** (Z = R + jX), which combines resistance (R) and reactance (X).
//! For power system analysis, we work with **admittance** (Y = 1/Z = G + jB),
//! the inverse of impedance:
//!
//! - **G (conductance)**: Real power losses (I²R heating)
//! - **B (susceptance)**: Reactive power exchange (magnetic/electric fields)
//!
//! ## Matrix Structure
//!
//! For an n-bus network, Y-bus is an n×n complex matrix:
//!
//! ```text
//!     ┌                                           ┐
//!     │  Y₁₁    Y₁₂    Y₁₃   ...   Y₁ₙ          │
//!     │  Y₂₁    Y₂₂    Y₂₃   ...   Y₂ₙ          │
//! Y = │  Y₃₁    Y₃₂    Y₃₃   ...   Y₃ₙ          │
//!     │   ⋮      ⋮      ⋮     ⋱    ⋮            │
//!     │  Yₙ₁    Yₙ₂    Yₙ₃   ...   Yₙₙ          │
//!     └                                           ┘
//! ```
//!
//! **Key properties:**
//! - **Diagonal** (Y_ii): Sum of admittances connected to bus i (self-admittance)
//! - **Off-diagonal** (Y_ij): Negative of branch admittance between buses i and j
//! - **Sparse**: Only non-zero where branches exist (typically 2-6 connections per bus)
//! - **Symmetric** for networks without phase-shifting transformers
//!
//! ## Building Blocks
//!
//! ### Simple Transmission Line (π-model)
//!
//! ```text
//!        ┌───[R + jX]───┐
//!        │              │
//!   i ───┼──┬────────┬──┼─── j
//!        │  │        │  │
//!        │ ═══      ═══ │    (jB/2 shunt at each end)
//!        │  │        │  │
//!        └──┴────────┴──┘
//!            ⏊        ⏊
//! ```
//!
//! For this branch:
//! - Series admittance: y = 1/(R + jX) = (R - jX)/(R² + X²)
//! - Shunt admittance: y_sh = jB/2 at each end
//!
//! **Y-bus contributions:**
//! ```text
//! Y_ii += y + jB/2      (diagonal at bus i)
//! Y_jj += y + jB/2      (diagonal at bus j)
//! Y_ij = Y_ji = -y      (off-diagonal, negative of series admittance)
//! ```
//!
//! ### Transformer Model
//!
//! Transformers introduce voltage transformation (tap ratio τ) and optional
//! phase shift (φ) for phase-shifting transformers (PSTs):
//!
//! ```text
//!     Bus i        τ:1, φ          Bus j
//!       ○────────[======]────────○
//!       |          ╲  ╱          |
//!      V_i          ╲╱          V_j
//!       |                        |
//!       ⏊                        ⏊
//! ```
//!
//! The transformer model in the Y-bus becomes:
//!
//! ```text
//! Y_ii += y/τ² + y_shunt           (from-bus diagonal)
//! Y_jj += y + y_shunt              (to-bus diagonal)
//! Y_ij += -y/(τ · e^{jφ})          (off-diagonal, affected by tap and phase)
//! Y_ji += -y/(τ · e^{-jφ})         (conjugate for the reverse direction)
//! ```
//!
//! **Physical meaning:**
//! - τ > 1: Step-up transformer (boosts voltage to secondary)
//! - τ < 1: Step-down transformer (reduces voltage)
//! - φ ≠ 0: Phase-shifting transformer (controls real power flow direction)
//!
//! ## Why Complex Numbers?
//!
//! AC power systems operate with sinusoidal voltages and currents. Instead of
//! tracking time-varying waveforms, we use **phasors** (complex numbers) that
//! capture magnitude and phase angle:
//!
//! ```text
//! v(t) = V_max · cos(ωt + θ)  ←→  V = (V_max/√2) · e^{jθ}
//!        └─────────────────┘       └──────────────────────┘
//!         time domain               phasor (complex)
//! ```
//!
//! The Y-bus naturally works with these phasors, making power flow equations
//! elegant and computationally efficient.
//!
//! ## References
//!
//! - **Grainger & Stevenson**: "Power System Analysis", Chapter 8
//!   The definitive textbook treatment of Y-bus construction
//!
//! - **Bergen & Vittal**: "Power Systems Analysis", 2nd Ed., Chapter 9
//!   DOI: [10.1073/pnas.93.24.13774](https://doi.org/10.1073/pnas.93.24.13774)
//!
//! - **Arrillaga & Arnold**: "Computer Modelling of Electrical Power Systems"
//!   Comprehensive transformer modeling details
//!   DOI: [10.1002/9781118878286](https://doi.org/10.1002/9781118878286)
//!
//! - **Tinney & Hart (1967)**: "Power Flow Solution by Newton's Method"
//!   IEEE Trans. PAS, 86(11), 1449-1460
//!   DOI: [10.1109/TPAS.1967.291823](https://doi.org/10.1109/TPAS.1967.291823)

use crate::opf::OpfError;
use gat_core::{BusId, Edge, Network, Node};
use num_complex::Complex64;
use std::collections::HashMap;

// ============================================================================
// Y-BUS DATA STRUCTURE
// ============================================================================

/// Dense Y-bus matrix for AC power flow calculations.
///
/// The Y-bus matrix relates bus currents to bus voltages: **I = Y · V**
///
/// For power flow, we typically decompose Y into real (G) and imaginary (B) parts:
/// - Y_ij = G_ij + j·B_ij
/// - G: Conductance matrix (dissipative, causes real power losses)
/// - B: Susceptance matrix (reactive, stores/releases energy in fields)
///
/// # Implementation Notes
///
/// We use **dense storage** for simplicity. For large networks (>1000 buses),
/// sparse storage (CSR/CSC format) would be more memory-efficient since
/// Y-bus is typically 1-3% dense (each bus connects to 2-6 neighbors).
///
/// Memory requirement: O(n²) complex values = O(16n²) bytes
/// - 100 buses: ~160 KB
/// - 1000 buses: ~16 MB
/// - 10000 buses: ~1.6 GB (sparse would be ~50 MB)
#[derive(Debug, Clone)]
pub struct YBus {
    /// Number of buses in the network
    n_bus: usize,

    /// Dense matrix storage in row-major order.
    /// Element (i, j) is at index `i * n_bus + j`.
    /// Each element is a complex admittance Y_ij = G_ij + j·B_ij
    data: Vec<Complex64>,

    /// Mapping from external bus IDs to internal 0-based indices.
    /// This allows the network to use arbitrary bus numbering (e.g., 101, 102, 201)
    /// while we use contiguous 0..n_bus indices internally.
    bus_map: HashMap<BusId, usize>,
}

impl YBus {
    /// Retrieve the admittance Y_ij between buses i and j.
    ///
    /// # Physical Interpretation
    ///
    /// - **Y_ii (diagonal)**: Total admittance connected to bus i
    ///   - Large |Y_ii| means bus i is "well connected" (low impedance to neighbors)
    ///   - Includes series admittances of all branches plus any shunt elements
    ///
    /// - **Y_ij (off-diagonal, i ≠ j)**: Negative of branch admittance
    ///   - Y_ij = 0 means no direct connection between buses i and j
    ///   - Y_ij ≠ 0 means there's a branch (line or transformer)
    ///   - Magnitude |Y_ij| indicates "strength" of connection (1/|Z_ij|)
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> Complex64 {
        self.data[row * self.n_bus + col]
    }

    /// Get mutable reference to element at (row, col).
    /// Used during matrix construction.
    #[inline]
    fn get_mut(&mut self, row: usize, col: usize) -> &mut Complex64 {
        &mut self.data[row * self.n_bus + col]
    }

    /// Number of buses in the network.
    pub fn n_bus(&self) -> usize {
        self.n_bus
    }

    /// Look up the internal matrix index for a given bus ID.
    /// Returns `None` if the bus ID doesn't exist in this Y-bus.
    pub fn bus_index(&self, id: BusId) -> Option<usize> {
        self.bus_map.get(&id).copied()
    }

    /// Extract the **G matrix** (conductance, real part of Y-bus).
    ///
    /// G_ij = Re(Y_ij)
    ///
    /// The G matrix appears in real power equations:
    /// P_i = Σ_j V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
    ///
    /// Physically, G represents dissipative losses (I²R heating in conductors).
    /// For lossless elements (ideal transformers, pure reactors), G_ij = 0.
    pub fn g_matrix(&self) -> Vec<f64> {
        self.data.iter().map(|y| y.re).collect()
    }

    /// Extract the **B matrix** (susceptance, imaginary part of Y-bus).
    ///
    /// B_ij = Im(Y_ij)
    ///
    /// The B matrix appears in reactive power equations:
    /// Q_i = Σ_j V_i·V_j·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
    ///
    /// Physically, B represents energy storage in magnetic fields (inductors)
    /// and electric fields (capacitors):
    /// - B < 0: Inductive (absorbs VARs, current lags voltage)
    /// - B > 0: Capacitive (supplies VARs, current leads voltage)
    ///
    /// For transmission lines, the series element is typically inductive (B < 0)
    /// while the shunt charging capacitance is positive (B > 0).
    pub fn b_matrix(&self) -> Vec<f64> {
        self.data.iter().map(|y| y.im).collect()
    }
}

// ============================================================================
// Y-BUS BUILDER
// ============================================================================

/// Builder for constructing Y-bus from network data.
pub struct YBusBuilder;

impl YBusBuilder {
    /// Build the Y-bus matrix from a Network graph.
    ///
    /// # Algorithm Overview
    ///
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────────────┐
    /// │  STEP 1: INDEX BUSES                                                 │
    /// │  ─────────────────────                                               │
    /// │  Traverse network graph to find all Bus nodes                        │
    /// │  Assign sequential indices 0, 1, 2, ... to each bus                  │
    /// │  Build bus_map: BusId → index for fast lookup                        │
    /// └─────────────────────────────────────────────────────────────────────┘
    ///                                    │
    ///                                    ▼
    /// ┌─────────────────────────────────────────────────────────────────────┐
    /// │  STEP 2: INITIALIZE ZERO MATRIX                                      │
    /// │  ───────────────────────────────                                     │
    /// │  Create n×n matrix of complex zeros                                  │
    /// │  Y_ij = 0 + j0 for all i, j                                          │
    /// └─────────────────────────────────────────────────────────────────────┘
    ///                                    │
    ///                                    ▼
    /// ┌─────────────────────────────────────────────────────────────────────┐
    /// │  STEP 3: PROCESS EACH BRANCH                                         │
    /// │  ───────────────────────────────                                     │
    /// │  For each in-service branch (line or transformer):                   │
    /// │                                                                       │
    /// │  (a) Compute series admittance:                                       │
    /// │      y = 1/(r + jx) = (r - jx)/(r² + x²)                             │
    /// │                                                                       │
    /// │  (b) Get tap ratio τ and phase shift φ                               │
    /// │                                                                       │
    /// │  (c) Compute shunt admittance contribution:                           │
    /// │      y_shunt = jB/2 (half at each end of line)                       │
    /// │                                                                       │
    /// │  (d) Update Y-bus entries:                                            │
    /// │      Y_ii += y/τ² + y_shunt                                          │
    /// │      Y_jj += y + y_shunt                                             │
    /// │      Y_ij += -y/(τ · e^{jφ})                                         │
    /// │      Y_ji += -y/(τ · e^{-jφ})                                        │
    /// └─────────────────────────────────────────────────────────────────────┘
    /// ```
    ///
    /// # Arguments
    ///
    /// * `network` - Network graph containing buses and branches
    ///
    /// # Returns
    ///
    /// * `Ok(YBus)` - Successfully constructed admittance matrix
    /// * `Err(OpfError)` - If network is invalid (no buses, zero-impedance branches)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ybus = YBusBuilder::from_network(&network)?;
    ///
    /// // Get admittance between buses 0 and 1
    /// let y_01 = ybus.get(0, 1);
    /// println!("G_01 = {:.4}, B_01 = {:.4}", y_01.re, y_01.im);
    /// ```
    pub fn from_network(network: &Network) -> Result<YBus, OpfError> {
        // ====================================================================
        // STEP 1: COUNT BUSES AND BUILD INDEX MAP
        // ====================================================================
        //
        // The network graph may contain buses with arbitrary IDs (e.g., from
        // MATPOWER files with bus numbers 1, 2, 3... or PSS/E with zone-based
        // numbering 10101, 10102...).
        //
        // We need contiguous 0-based indices for the matrix. This mapping is
        // stored in bus_map for later lookup when processing branches.

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

        // ====================================================================
        // STEP 2: INITIALIZE Y-BUS WITH ZEROS
        // ====================================================================
        //
        // Start with an n×n matrix of complex zeros. We'll accumulate
        // contributions from each branch.

        let mut ybus = YBus {
            n_bus,
            data: vec![Complex64::new(0.0, 0.0); n_bus * n_bus],
            bus_map,
        };

        // ====================================================================
        // STEP 3: PROCESS EACH BRANCH
        // ====================================================================
        //
        // Each branch contributes to up to 4 elements of the Y-bus:
        // - Two diagonal elements (self-admittance at each end)
        // - Two off-diagonal elements (mutual admittance between ends)

        for edge_idx in network.graph.edge_indices() {
            if let Edge::Branch(branch) = &network.graph[edge_idx] {
                // Skip out-of-service branches (used for contingency analysis)
                if !branch.status {
                    continue;
                }

                // Look up internal indices for the branch terminals
                let from_idx = ybus.bus_index(branch.from_bus).ok_or_else(|| {
                    OpfError::DataValidation(format!(
                        "Branch {} references unknown from_bus {}",
                        branch.name,
                        branch.from_bus.value()
                    ))
                })?;

                let to_idx = ybus.bus_index(branch.to_bus).ok_or_else(|| {
                    OpfError::DataValidation(format!(
                        "Branch {} references unknown to_bus {}",
                        branch.name,
                        branch.to_bus.value()
                    ))
                })?;

                // ============================================================
                // COMPUTE SERIES ADMITTANCE
                // ============================================================
                //
                // y = 1/z = 1/(r + jx)
                //
                // Using complex arithmetic:
                // y = (r - jx) / (r² + x²) = (r/(r²+x²)) - j(x/(r²+x²))
                //
                // The real part (r/(r²+x²)) is the conductance G
                // The imaginary part (-x/(r²+x²)) is the susceptance B
                //
                // Note: For transmission lines, typically X >> R (X/R ratio 5-15),
                // so B dominates and the line is primarily inductive.

                let z = Complex64::new(branch.resistance, branch.reactance);
                if z.norm() < 1e-12 {
                    return Err(OpfError::DataValidation(format!(
                        "Branch {} has zero impedance. This would be a short circuit. \
                         Use a small positive reactance (e.g., 0.0001 p.u.) for bus ties.",
                        branch.name
                    )));
                }
                let y_series = z.inv(); // Complex inverse: 1/(r + jx)

                // ============================================================
                // TAP RATIO AND PHASE SHIFT
                // ============================================================
                //
                // Transformers modify the voltage relationship:
                //   V_to = V_from / (τ · e^{jφ})
                //
                // Where:
                //   τ (tau) = tap ratio (typically 0.9 to 1.1)
                //   φ (phi) = phase shift angle (radians, typically -30° to +30°)
                //
                // For a simple transmission line: τ = 1, φ = 0
                //
                // The phase shift is used by Phase-Shifting Transformers (PSTs)
                // to control real power flow independent of voltage magnitude.
                // This is crucial for:
                //   - Loop flow control in meshed networks
                //   - Congestion management on specific corridors
                //   - Power exchange between interconnected systems

                let tau = branch.tap_ratio;
                let phi = branch.phase_shift_rad;
                let tau2 = tau * tau;

                // Phase shift phasor: e^{-jφ}
                // This rotates the voltage phasor by angle -φ
                let shift = Complex64::from_polar(1.0, -phi);

                // ============================================================
                // SHUNT ADMITTANCE (LINE CHARGING)
                // ============================================================
                //
                // Long transmission lines have distributed capacitance between
                // conductors and to ground. The π-model lumps this as half the
                // total charging susceptance at each end.
                //
                // Charging is purely reactive (capacitive), so:
                //   y_shunt = j · B_charging / 2
                //
                // This creates a "VAR source" at each bus proportional to V²:
                //   Q_charging = (B/2) · V²
                //
                // For short lines (<80 km), charging is often negligible.
                // For long lines at high voltage (>230 kV), it can be significant
                // and may cause overvoltage under light load (Ferranti effect).

                let y_shunt_half = Complex64::new(0.0, branch.charging_b_pu / 2.0);

                // ============================================================
                // UPDATE Y-BUS ENTRIES
                // ============================================================
                //
                // The transformer-inclusive π-model gives these Y-bus contributions:
                //
                // ┌─────────────────────────────────────────────────────────────┐
                // │  DIAGONAL ENTRIES (self-admittance)                          │
                // │  ────────────────                                            │
                // │  Y_ii += y/τ² + y_shunt                                      │
                // │         └──┘   └───────┘                                     │
                // │         series  shunt at from-bus                            │
                // │                                                               │
                // │  Y_jj += y + y_shunt                                          │
                // │         └┘  └───────┘                                         │
                // │         series (no tap transformation on to-side)             │
                // │                                                               │
                // │  The τ² factor on the from-side accounts for the transformer  │
                // │  referring the series impedance to the from-bus voltage.      │
                // └─────────────────────────────────────────────────────────────┘
                //
                // ┌─────────────────────────────────────────────────────────────┐
                // │  OFF-DIAGONAL ENTRIES (mutual admittance)                    │
                // │  ────────────────────                                        │
                // │  Y_ij += -y/(τ · e^{jφ})                                     │
                // │          └──────────────┘                                    │
                // │          Negative sign: current INTO bus i from bus j        │
                // │          τ factor: tap transformation                         │
                // │          e^{jφ}: phase shift phasor                          │
                // │                                                               │
                // │  Y_ji += -y/(τ · e^{-jφ})                                    │
                // │          └───────────────┘                                   │
                // │          Conjugate phase shift for reverse direction          │
                // │                                                               │
                // │  Note: For τ=1, φ=0: Y_ij = Y_ji = -y (symmetric)            │
                // │  For phase shifters: Y_ij ≠ Y_ji (asymmetric Y-bus)          │
                // └─────────────────────────────────────────────────────────────┘

                // Diagonal entries
                *ybus.get_mut(from_idx, from_idx) += y_series / tau2 + y_shunt_half;
                *ybus.get_mut(to_idx, to_idx) += y_series + y_shunt_half;

                // Off-diagonal entries (note: shift.conj() = e^{+jφ})
                *ybus.get_mut(from_idx, to_idx) += -y_series / tau * shift.conj();
                *ybus.get_mut(to_idx, from_idx) += -y_series / tau * shift;
            }
        }

        Ok(ybus)
    }
}
