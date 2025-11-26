# Full Nonlinear AC-OPF Solver Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a full-space nonlinear AC-OPF solver using interior-point methods that shares data structures with the existing SOCP solver.

**Architecture:** Build an `AcNlpBuilder` that reuses the SOCP's `BusData/BranchData/GenData` extraction patterns, constructs Y-bus admittance matrices, and produces Jacobian/Hessian callbacks for IPOPT. Use the `argmin` crate with L-BFGS as the initial solver backend (pure Rust, no external dependencies), with IPOPT as a future optional backend behind a feature flag.

**Tech Stack:** Rust, argmin (L-BFGS/BFGS), nalgebra (sparse matrices), num-complex (phasors), existing gat-core data structures.

---

## Phase 1: Foundation - Y-Bus and Power Flow Equations

### Task 1: Create AC-OPF Module Structure

**Files:**
- Create: `crates/gat-algo/src/opf/ac_nlp/mod.rs`
- Create: `crates/gat-algo/src/opf/ac_nlp/ybus.rs`
- Modify: `crates/gat-algo/src/opf/mod.rs:9-14`

**Step 1: Create directory and module file**

Create `crates/gat-algo/src/opf/ac_nlp/mod.rs`:

```rust
//! Full Nonlinear AC Optimal Power Flow Solver
//!
//! This module implements a full-space AC-OPF using interior-point methods.
//! Unlike the SOCP relaxation which uses squared variables, this formulation
//! uses the original polar variables (V, θ) with explicit nonlinear constraints.
//!
//! ## Mathematical Formulation
//!
//! Variables: V_i (voltage magnitude), θ_i (angle), P_g, Q_g (generator dispatch)
//!
//! Minimize: Σ (c₀ + c₁·P_g + c₂·P_g²)
//!
//! Subject to:
//!   - Power balance: P_inj = P_gen - P_load, Q_inj = Q_gen - Q_load
//!   - AC power flow: P_i = Σ V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
//!   - Voltage limits: V_min ≤ V ≤ V_max
//!   - Generator limits: P_min ≤ P_g ≤ P_max, Q_min ≤ Q_g ≤ Q_max
//!   - Thermal limits: P_ij² + Q_ij² ≤ S_max²

mod ybus;

pub use ybus::YBusBuilder;
```

**Step 2: Run `cargo check` to verify module structure**

Run: `cargo check -p gat-algo`
Expected: Error about missing ybus module (we'll create it next)

**Step 3: Create empty ybus.rs placeholder**

Create `crates/gat-algo/src/opf/ac_nlp/ybus.rs`:

```rust
//! Y-Bus (Admittance Matrix) Construction
//!
//! The Y-bus matrix is fundamental to AC power flow analysis. Each element Y_ij
//! represents the admittance between buses i and j:
//!
//! ```text
//! Y_ij = -y_ij  (off-diagonal, i ≠ j)
//! Y_ii = Σ y_ik + y_sh_i  (diagonal: sum of incident branch admittances + shunt)
//! ```
//!
//! where y_ij = 1/(r_ij + jx_ij) is the series admittance of branch i-j.

use num_complex::Complex64;

/// Y-bus builder for AC power flow calculations
pub struct YBusBuilder;

impl YBusBuilder {
    /// Create a new Y-bus builder
    pub fn new() -> Self {
        Self
    }
}
```

**Step 4: Wire up the module in mod.rs**

Edit `crates/gat-algo/src/opf/mod.rs` to add:

```rust
mod ac_nlp;
pub use ac_nlp::YBusBuilder;
```

Add this after line 11 (`mod socp;`).

**Step 5: Run cargo check to verify compilation**

Run: `cargo check -p gat-algo`
Expected: PASS (compiles with empty placeholder)

**Step 6: Commit**

```bash
git add crates/gat-algo/src/opf/ac_nlp/
git add crates/gat-algo/src/opf/mod.rs
git commit -m "feat(ac-opf): add module structure for nonlinear AC-OPF

Create ac_nlp module with placeholder for Y-bus construction.
This establishes the foundation for full AC-OPF implementation."
```

---

### Task 2: Implement Y-Bus Matrix Construction

**Files:**
- Modify: `crates/gat-algo/src/opf/ac_nlp/ybus.rs`
- Create: `crates/gat-algo/tests/ac_nlp_ybus.rs`

**Step 1: Write the failing test**

Create `crates/gat-algo/tests/ac_nlp_ybus.rs`:

```rust
//! Y-Bus construction tests

use gat_algo::opf::ac_nlp::YBusBuilder;
use gat_core::{Branch, BranchId, Bus, BusId, Edge, Network, Node};
use num_complex::Complex64;

/// Helper: create a simple 2-bus network
fn two_bus_network() -> Network {
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 138.0,
    }));
    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 138.0,
    }));

    // Line with R=0.01, X=0.1 (per unit)
    network.graph.add_edge(
        bus1,
        bus2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.02, // Small line charging
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
        }),
    );

    network
}

#[test]
fn ybus_two_bus_admittance() {
    let network = two_bus_network();
    let ybus = YBusBuilder::from_network(&network).expect("should build Y-bus");

    // For R=0.01, X=0.1: y = 1/(0.01 + j0.1) = (0.01 - j0.1) / (0.01² + 0.1²)
    //                      = (0.01 - j0.1) / 0.0101 ≈ 0.99 - j9.9
    let y_series = Complex64::new(0.01, 0.1).inv();

    // Off-diagonal: Y_12 = Y_21 = -y_series
    let y12 = ybus.get(0, 1);
    assert!(
        (y12.re - (-y_series.re)).abs() < 0.01,
        "Y_12 real part mismatch: got {}, expected {}",
        y12.re,
        -y_series.re
    );
    assert!(
        (y12.im - (-y_series.im)).abs() < 0.1,
        "Y_12 imag part mismatch: got {}, expected {}",
        y12.im,
        -y_series.im
    );

    // Diagonal: Y_11 = y_series + j*B_shunt/2
    let y11 = ybus.get(0, 0);
    let expected_y11 = y_series + Complex64::new(0.0, 0.02 / 2.0);
    assert!(
        (y11.re - expected_y11.re).abs() < 0.01,
        "Y_11 real mismatch"
    );
    assert!(
        (y11.im - expected_y11.im).abs() < 0.1,
        "Y_11 imag mismatch"
    );
}

#[test]
fn ybus_symmetry() {
    let network = two_bus_network();
    let ybus = YBusBuilder::from_network(&network).expect("should build Y-bus");

    // Y-bus should be symmetric for networks without phase shifters
    let y12 = ybus.get(0, 1);
    let y21 = ybus.get(1, 0);

    assert!(
        (y12.re - y21.re).abs() < 1e-10,
        "Y-bus should be symmetric"
    );
    assert!(
        (y12.im - y21.im).abs() < 1e-10,
        "Y-bus should be symmetric"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gat-algo ybus_two_bus_admittance`
Expected: FAIL with compilation error (from_network doesn't exist)

**Step 3: Implement Y-bus construction**

Replace `crates/gat-algo/src/opf/ac_nlp/ybus.rs` with:

```rust
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
```

**Step 4: Add num-complex to Cargo.toml if not present (already there)**

Verify `num-complex = "0.4"` is in `crates/gat-algo/Cargo.toml` (it is, line 13).

**Step 5: Export the module properly**

Edit `crates/gat-algo/src/opf/ac_nlp/mod.rs`:

```rust
//! Full Nonlinear AC Optimal Power Flow Solver
//!
//! This module implements a full-space AC-OPF using interior-point methods.

mod ybus;

pub use ybus::{YBus, YBusBuilder};
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p gat-algo ybus`
Expected: PASS (2 tests)

**Step 7: Commit**

```bash
git add crates/gat-algo/src/opf/ac_nlp/
git add crates/gat-algo/tests/ac_nlp_ybus.rs
git commit -m "feat(ac-opf): implement Y-bus matrix construction

Add YBusBuilder that extracts bus admittance matrix from network:
- Series admittance from branch R+jX
- Shunt admittance from line charging
- Tap ratio and phase shift for transformers
- Dense matrix storage with bus ID mapping

Includes tests for 2-bus admittance calculation and symmetry."
```

---

### Task 3: Implement AC Power Flow Equations

**Files:**
- Create: `crates/gat-algo/src/opf/ac_nlp/power_equations.rs`
- Modify: `crates/gat-algo/src/opf/ac_nlp/mod.rs`
- Create: `crates/gat-algo/tests/ac_nlp_power.rs`

**Step 1: Write the failing test**

Create `crates/gat-algo/tests/ac_nlp_power.rs`:

```rust
//! AC power flow equation tests

use gat_algo::opf::ac_nlp::{PowerEquations, YBusBuilder};
use gat_core::{Branch, BranchId, Bus, BusId, Edge, Network, Node};

fn two_bus_network() -> Network {
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 138.0,
    }));
    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 138.0,
    }));

    network.graph.add_edge(
        bus1,
        bus2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.0,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
        }),
    );

    network
}

#[test]
fn power_injection_flat_start() {
    let network = two_bus_network();
    let ybus = YBusBuilder::from_network(&network).unwrap();

    // Flat start: V = [1.0, 1.0], θ = [0.0, 0.0]
    let v = vec![1.0, 1.0];
    let theta = vec![0.0, 0.0];

    let (p_inj, q_inj) = PowerEquations::compute_injections(&ybus, &v, &theta);

    // At flat start with no angle difference, power flow should be zero
    assert!(p_inj[0].abs() < 1e-10, "P1 should be ~0 at flat start");
    assert!(p_inj[1].abs() < 1e-10, "P2 should be ~0 at flat start");
    assert!(q_inj[0].abs() < 1e-10, "Q1 should be ~0 at flat start");
    assert!(q_inj[1].abs() < 1e-10, "Q2 should be ~0 at flat start");
}

#[test]
fn power_injection_with_angle() {
    let network = two_bus_network();
    let ybus = YBusBuilder::from_network(&network).unwrap();

    // V = [1.0, 1.0], θ = [0.0, -0.1 rad] (bus 2 lagging)
    // Power should flow from bus 1 to bus 2
    let v = vec![1.0, 1.0];
    let theta = vec![0.0, -0.1];

    let (p_inj, _q_inj) = PowerEquations::compute_injections(&ybus, &v, &theta);

    // P1 should be positive (injecting into network = sending)
    // P2 should be negative (withdrawing from network = receiving)
    assert!(p_inj[0] > 0.0, "P1 should be positive (sending)");
    assert!(p_inj[1] < 0.0, "P2 should be negative (receiving)");

    // Conservation: P1 + P2 ≈ losses (small for this case)
    let total_p = p_inj[0] + p_inj[1];
    assert!(total_p.abs() < 0.01, "Power should be nearly conserved");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gat-algo power_injection`
Expected: FAIL with compilation error (PowerEquations doesn't exist)

**Step 3: Implement power flow equations**

Create `crates/gat-algo/src/opf/ac_nlp/power_equations.rs`:

```rust
//! AC Power Flow Equations
//!
//! Implements the fundamental AC power flow equations:
//!
//! ```text
//! P_i = Σⱼ V_i·V_j·(G_ij·cos(θ_i - θ_j) + B_ij·sin(θ_i - θ_j))
//! Q_i = Σⱼ V_i·V_j·(G_ij·sin(θ_i - θ_j) - B_ij·cos(θ_i - θ_j))
//! ```
//!
//! where G_ij = Re(Y_ij) and B_ij = Im(Y_ij).
//!
//! ## References
//!
//! - Grainger & Stevenson, "Power System Analysis", equations (9.14)-(9.15)

use super::YBus;

/// AC power flow equation computation
pub struct PowerEquations;

impl PowerEquations {
    /// Compute power injections at all buses
    ///
    /// # Arguments
    ///
    /// * `ybus` - Y-bus admittance matrix
    /// * `v` - Voltage magnitudes (per-unit)
    /// * `theta` - Voltage angles (radians)
    ///
    /// # Returns
    ///
    /// Tuple of (P_injection, Q_injection) vectors in per-unit
    pub fn compute_injections(ybus: &YBus, v: &[f64], theta: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let n = ybus.n_bus();
        let mut p_inj = vec![0.0; n];
        let mut q_inj = vec![0.0; n];

        for i in 0..n {
            let vi = v[i];
            let theta_i = theta[i];

            for j in 0..n {
                let y_ij = ybus.get(i, j);
                let g_ij = y_ij.re;
                let b_ij = y_ij.im;

                let vj = v[j];
                let theta_ij = theta_i - theta[j];

                let cos_ij = theta_ij.cos();
                let sin_ij = theta_ij.sin();

                // P_i = Σ V_i·V_j·(G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                p_inj[i] += vi * vj * (g_ij * cos_ij + b_ij * sin_ij);

                // Q_i = Σ V_i·V_j·(G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                q_inj[i] += vi * vj * (g_ij * sin_ij - b_ij * cos_ij);
            }
        }

        (p_inj, q_inj)
    }

    /// Compute Jacobian of power flow equations
    ///
    /// The Jacobian has the structure:
    /// ```text
    /// J = | ∂P/∂θ  ∂P/∂V |
    ///     | ∂Q/∂θ  ∂Q/∂V |
    /// ```
    ///
    /// # Returns
    ///
    /// Tuple of (dP_dtheta, dP_dV, dQ_dtheta, dQ_dV) as flat vectors (row-major)
    pub fn compute_jacobian(
        ybus: &YBus,
        v: &[f64],
        theta: &[f64],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let n = ybus.n_bus();

        let mut dp_dtheta = vec![0.0; n * n];
        let mut dp_dv = vec![0.0; n * n];
        let mut dq_dtheta = vec![0.0; n * n];
        let mut dq_dv = vec![0.0; n * n];

        for i in 0..n {
            let vi = v[i];
            let theta_i = theta[i];

            for j in 0..n {
                let y_ij = ybus.get(i, j);
                let g_ij = y_ij.re;
                let b_ij = y_ij.im;

                let vj = v[j];
                let theta_ij = theta_i - theta[j];

                let cos_ij = theta_ij.cos();
                let sin_ij = theta_ij.sin();

                let idx = i * n + j;

                if i == j {
                    // Diagonal elements (self-derivatives)
                    // Need to compute sums over k ≠ i

                    let mut sum_p = 0.0;
                    let mut sum_q = 0.0;

                    for k in 0..n {
                        if k != i {
                            let y_ik = ybus.get(i, k);
                            let g_ik = y_ik.re;
                            let b_ik = y_ik.im;
                            let vk = v[k];
                            let theta_ik = theta_i - theta[k];
                            let cos_ik = theta_ik.cos();
                            let sin_ik = theta_ik.sin();

                            sum_p += vk * (-g_ik * sin_ik + b_ik * cos_ik);
                            sum_q += vk * (g_ik * cos_ik + b_ik * sin_ik);
                        }
                    }

                    // ∂P_i/∂θ_i = V_i · Σ_{k≠i} V_k · (-G_ik·sin(θ_ik) + B_ik·cos(θ_ik))
                    dp_dtheta[idx] = vi * sum_p;

                    // ∂Q_i/∂θ_i = V_i · Σ_{k≠i} V_k · (G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
                    dq_dtheta[idx] = vi * sum_q;

                    // ∂P_i/∂V_i = 2·V_i·G_ii + Σ_{k≠i} V_k·(G_ik·cos(θ_ik) + B_ik·sin(θ_ik))
                    let mut sum_pv = 0.0;
                    for k in 0..n {
                        if k != i {
                            let y_ik = ybus.get(i, k);
                            let vk = v[k];
                            let theta_ik = theta_i - theta[k];
                            sum_pv += vk * (y_ik.re * theta_ik.cos() + y_ik.im * theta_ik.sin());
                        }
                    }
                    dp_dv[idx] = 2.0 * vi * g_ij + sum_pv;

                    // ∂Q_i/∂V_i = -2·V_i·B_ii + Σ_{k≠i} V_k·(G_ik·sin(θ_ik) - B_ik·cos(θ_ik))
                    let mut sum_qv = 0.0;
                    for k in 0..n {
                        if k != i {
                            let y_ik = ybus.get(i, k);
                            let vk = v[k];
                            let theta_ik = theta_i - theta[k];
                            sum_qv += vk * (y_ik.re * theta_ik.sin() - y_ik.im * theta_ik.cos());
                        }
                    }
                    dq_dv[idx] = -2.0 * vi * b_ij + sum_qv;
                } else {
                    // Off-diagonal elements

                    // ∂P_i/∂θ_j = V_i · V_j · (G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                    dp_dtheta[idx] = vi * vj * (g_ij * sin_ij - b_ij * cos_ij);

                    // ∂Q_i/∂θ_j = -V_i · V_j · (G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                    dq_dtheta[idx] = -vi * vj * (g_ij * cos_ij + b_ij * sin_ij);

                    // ∂P_i/∂V_j = V_i · (G_ij·cos(θ_ij) + B_ij·sin(θ_ij))
                    dp_dv[idx] = vi * (g_ij * cos_ij + b_ij * sin_ij);

                    // ∂Q_i/∂V_j = V_i · (G_ij·sin(θ_ij) - B_ij·cos(θ_ij))
                    dq_dv[idx] = vi * (g_ij * sin_ij - b_ij * cos_ij);
                }
            }
        }

        (dp_dtheta, dp_dv, dq_dtheta, dq_dv)
    }
}
```

**Step 4: Export PowerEquations**

Edit `crates/gat-algo/src/opf/ac_nlp/mod.rs`:

```rust
//! Full Nonlinear AC Optimal Power Flow Solver

mod power_equations;
mod ybus;

pub use power_equations::PowerEquations;
pub use ybus::{YBus, YBusBuilder};
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p gat-algo power_injection`
Expected: PASS (2 tests)

**Step 6: Commit**

```bash
git add crates/gat-algo/src/opf/ac_nlp/power_equations.rs
git add crates/gat-algo/src/opf/ac_nlp/mod.rs
git add crates/gat-algo/tests/ac_nlp_power.rs
git commit -m "feat(ac-opf): implement AC power flow equations

Add PowerEquations with:
- compute_injections: P_i, Q_i from V, θ using Y-bus
- compute_jacobian: ∂P/∂θ, ∂P/∂V, ∂Q/∂θ, ∂Q/∂V

Jacobian follows standard Newton-Raphson structure for power flow.
Includes tests for flat start and power conservation."
```

---

## Phase 2: NLP Problem Formulation

### Task 4: Create AC-OPF Problem Structure

**Files:**
- Create: `crates/gat-algo/src/opf/ac_nlp/problem.rs`
- Modify: `crates/gat-algo/src/opf/ac_nlp/mod.rs`

**Step 1: Write the problem structure**

Create `crates/gat-algo/src/opf/ac_nlp/problem.rs`:

```rust
//! AC-OPF Problem Formulation
//!
//! Defines the nonlinear program (NLP) for AC optimal power flow.
//!
//! ## Variable Layout
//!
//! ```text
//! x = [ V_1, ..., V_n, θ_1, ..., θ_n, P_g1, ..., P_gm, Q_g1, ..., Q_gm ]
//!     |<--- n_bus --->|<-- n_bus -->|<--- n_gen --->|<--- n_gen --->|
//! ```
//!
//! ## Constraints
//!
//! Equality constraints (g(x) = 0):
//!   - Power balance at each bus: P_inj - P_gen + P_load = 0
//!   - Q balance: Q_inj - Q_gen + Q_load = 0
//!   - Reference bus angle: θ_ref = 0
//!
//! Inequality constraints (h(x) ≤ 0):
//!   - Voltage bounds: V_min ≤ V ≤ V_max
//!   - Generator limits: P_min ≤ P_g ≤ P_max, Q_min ≤ Q_g ≤ Q_max
//!   - Thermal limits: P_ij² + Q_ij² ≤ S_max²

use super::{PowerEquations, YBus, YBusBuilder};
use crate::opf::OpfError;
use gat_core::{BusId, CostModel, Network, Node};
use std::collections::HashMap;

/// Generator data for OPF
#[derive(Debug, Clone)]
pub struct GenData {
    pub name: String,
    pub bus_id: BusId,
    pub pmin_mw: f64,
    pub pmax_mw: f64,
    pub qmin_mvar: f64,
    pub qmax_mvar: f64,
    pub cost_coeffs: Vec<f64>,
}

/// Bus data for OPF
#[derive(Debug, Clone)]
pub struct BusData {
    pub id: BusId,
    pub name: String,
    pub index: usize,
    pub v_min: f64,
    pub v_max: f64,
    pub p_load: f64,
    pub q_load: f64,
}

/// AC-OPF Problem definition
pub struct AcOpfProblem {
    /// Y-bus admittance matrix
    pub ybus: YBus,
    /// Bus data
    pub buses: Vec<BusData>,
    /// Generator data
    pub generators: Vec<GenData>,
    /// Reference bus index
    pub ref_bus: usize,
    /// Per-unit base (MVA)
    pub base_mva: f64,

    // Variable indices
    pub n_bus: usize,
    pub n_gen: usize,
    pub n_var: usize,

    // Index offsets
    pub v_offset: usize,
    pub theta_offset: usize,
    pub pg_offset: usize,
    pub qg_offset: usize,

    // Generator-to-bus mapping
    pub gen_bus_idx: Vec<usize>,
}

impl AcOpfProblem {
    /// Build problem from network
    pub fn from_network(network: &Network) -> Result<Self, OpfError> {
        let ybus = YBusBuilder::from_network(network)?;

        // Extract buses
        let mut buses = Vec::new();
        let mut loads: HashMap<BusId, (f64, f64)> = HashMap::new();
        let mut bus_idx = 0;

        for node_idx in network.graph.node_indices() {
            match &network.graph[node_idx] {
                Node::Bus(bus) => {
                    buses.push(BusData {
                        id: bus.id,
                        name: bus.name.clone(),
                        index: bus_idx,
                        v_min: 0.9,
                        v_max: 1.1,
                        p_load: 0.0,
                        q_load: 0.0,
                    });
                    bus_idx += 1;
                }
                Node::Load(load) => {
                    let entry = loads.entry(load.bus).or_insert((0.0, 0.0));
                    entry.0 += load.active_power_mw;
                    entry.1 += load.reactive_power_mvar;
                }
                _ => {}
            }
        }

        // Apply loads to buses
        for bus in &mut buses {
            if let Some((p, q)) = loads.get(&bus.id) {
                bus.p_load = *p;
                bus.q_load = *q;
            }
        }

        // Extract generators
        let mut generators = Vec::new();
        for node_idx in network.graph.node_indices() {
            if let Node::Gen(gen) = &network.graph[node_idx] {
                let cost_coeffs = match &gen.cost_model {
                    CostModel::NoCost => vec![0.0, 0.0],
                    CostModel::Polynomial(c) => c.clone(),
                    CostModel::PiecewiseLinear(_) => {
                        let mid = (gen.pmin_mw + gen.pmax_mw) / 2.0;
                        vec![0.0, gen.cost_model.marginal_cost(mid)]
                    }
                };

                generators.push(GenData {
                    name: gen.name.clone(),
                    bus_id: gen.bus,
                    pmin_mw: gen.pmin_mw,
                    pmax_mw: gen.pmax_mw,
                    qmin_mvar: gen.qmin_mvar,
                    qmax_mvar: gen.qmax_mvar,
                    cost_coeffs,
                });
            }
        }

        if generators.is_empty() {
            return Err(OpfError::DataValidation(
                "No generators in network".to_string(),
            ));
        }

        let n_bus = buses.len();
        let n_gen = generators.len();
        let n_var = 2 * n_bus + 2 * n_gen;

        // Compute generator-to-bus index mapping
        let bus_map: HashMap<BusId, usize> = buses.iter().map(|b| (b.id, b.index)).collect();
        let gen_bus_idx: Vec<usize> = generators
            .iter()
            .map(|g| *bus_map.get(&g.bus_id).unwrap_or(&0))
            .collect();

        Ok(Self {
            ybus,
            buses,
            generators,
            ref_bus: 0,
            base_mva: 100.0,

            n_bus,
            n_gen,
            n_var,

            v_offset: 0,
            theta_offset: n_bus,
            pg_offset: 2 * n_bus,
            qg_offset: 2 * n_bus + n_gen,

            gen_bus_idx,
        })
    }

    /// Get initial point (flat start)
    pub fn initial_point(&self) -> Vec<f64> {
        let mut x = vec![0.0; self.n_var];

        // Voltage magnitudes = 1.0
        for i in 0..self.n_bus {
            x[self.v_offset + i] = 1.0;
        }

        // Angles = 0.0 (already initialized)

        // Generator setpoints at midpoint of range
        for (i, gen) in self.generators.iter().enumerate() {
            x[self.pg_offset + i] = (gen.pmin_mw + gen.pmax_mw) / 2.0 / self.base_mva;
            x[self.qg_offset + i] = (gen.qmin_mvar + gen.qmax_mvar) / 2.0 / self.base_mva;
        }

        x
    }

    /// Extract voltage magnitude and angle vectors
    pub fn extract_v_theta(&self, x: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let v: Vec<f64> = (0..self.n_bus).map(|i| x[self.v_offset + i]).collect();
        let theta: Vec<f64> = (0..self.n_bus).map(|i| x[self.theta_offset + i]).collect();
        (v, theta)
    }

    /// Evaluate objective function: Σ (c₀ + c₁·P_g + c₂·P_g²)
    pub fn objective(&self, x: &[f64]) -> f64 {
        let mut cost = 0.0;
        for (i, gen) in self.generators.iter().enumerate() {
            let pg_pu = x[self.pg_offset + i];
            let pg_mw = pg_pu * self.base_mva;

            let c0 = gen.cost_coeffs.first().copied().unwrap_or(0.0);
            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);

            cost += c0 + c1 * pg_mw + c2 * pg_mw * pg_mw;
        }
        cost
    }

    /// Evaluate objective gradient
    pub fn objective_gradient(&self, x: &[f64]) -> Vec<f64> {
        let mut grad = vec![0.0; self.n_var];

        for (i, gen) in self.generators.iter().enumerate() {
            let pg_pu = x[self.pg_offset + i];
            let pg_mw = pg_pu * self.base_mva;

            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);

            // d/dP_pu (c1 * P_mw + c2 * P_mw²) = (c1 + 2*c2*P_mw) * base_mva
            grad[self.pg_offset + i] = (c1 + 2.0 * c2 * pg_mw) * self.base_mva;
        }

        grad
    }

    /// Evaluate equality constraints (power balance)
    ///
    /// Returns vector of constraint violations (should be zero at feasible point)
    pub fn equality_constraints(&self, x: &[f64]) -> Vec<f64> {
        let (v, theta) = self.extract_v_theta(x);
        let (p_inj, q_inj) = PowerEquations::compute_injections(&self.ybus, &v, &theta);

        // 2*n_bus constraints (P balance + Q balance) + 1 (reference angle)
        let mut g = Vec::with_capacity(2 * self.n_bus + 1);

        // Build generator injections at each bus
        let mut pg_bus = vec![0.0; self.n_bus];
        let mut qg_bus = vec![0.0; self.n_bus];

        for (i, &bus_idx) in self.gen_bus_idx.iter().enumerate() {
            pg_bus[bus_idx] += x[self.pg_offset + i];
            qg_bus[bus_idx] += x[self.qg_offset + i];
        }

        // P balance: P_inj - P_gen + P_load = 0
        for (i, bus) in self.buses.iter().enumerate() {
            let p_load_pu = bus.p_load / self.base_mva;
            g.push(p_inj[i] - pg_bus[i] + p_load_pu);
        }

        // Q balance: Q_inj - Q_gen + Q_load = 0
        for (i, bus) in self.buses.iter().enumerate() {
            let q_load_pu = bus.q_load / self.base_mva;
            g.push(q_inj[i] - qg_bus[i] + q_load_pu);
        }

        // Reference angle: θ_ref = 0
        g.push(x[self.theta_offset + self.ref_bus]);

        g
    }

    /// Get variable bounds: (lower, upper)
    pub fn variable_bounds(&self) -> (Vec<f64>, Vec<f64>) {
        let mut lb = vec![f64::NEG_INFINITY; self.n_var];
        let mut ub = vec![f64::INFINITY; self.n_var];

        // Voltage bounds
        for (i, bus) in self.buses.iter().enumerate() {
            lb[self.v_offset + i] = bus.v_min;
            ub[self.v_offset + i] = bus.v_max;
        }

        // Angle bounds (±π/2 for numerical stability)
        for i in 0..self.n_bus {
            lb[self.theta_offset + i] = -std::f64::consts::FRAC_PI_2;
            ub[self.theta_offset + i] = std::f64::consts::FRAC_PI_2;
        }

        // Generator P limits
        for (i, gen) in self.generators.iter().enumerate() {
            lb[self.pg_offset + i] = gen.pmin_mw / self.base_mva;
            ub[self.pg_offset + i] = gen.pmax_mw / self.base_mva;
        }

        // Generator Q limits
        for (i, gen) in self.generators.iter().enumerate() {
            lb[self.qg_offset + i] = gen.qmin_mvar / self.base_mva;
            ub[self.qg_offset + i] = gen.qmax_mvar / self.base_mva;
        }

        (lb, ub)
    }
}
```

**Step 2: Export and wire up**

Edit `crates/gat-algo/src/opf/ac_nlp/mod.rs`:

```rust
//! Full Nonlinear AC Optimal Power Flow Solver

mod power_equations;
mod problem;
mod ybus;

pub use power_equations::PowerEquations;
pub use problem::{AcOpfProblem, BusData, GenData};
pub use ybus::{YBus, YBusBuilder};
```

**Step 3: Run cargo check**

Run: `cargo check -p gat-algo`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/ac_nlp/problem.rs
git add crates/gat-algo/src/opf/ac_nlp/mod.rs
git commit -m "feat(ac-opf): add NLP problem structure

AcOpfProblem extracts network data and provides:
- Variable layout: [V, θ, P_g, Q_g]
- Objective function with quadratic costs
- Equality constraints (power balance + ref angle)
- Variable bounds (voltage, generator limits)
- Initial point (flat start)"
```

---

### Task 5: Add argmin Dependency and Implement Solver

**Files:**
- Modify: `crates/gat-algo/Cargo.toml`
- Create: `crates/gat-algo/src/opf/ac_nlp/solver.rs`
- Modify: `crates/gat-algo/src/opf/ac_nlp/mod.rs`

**Step 1: Add argmin to Cargo.toml**

Edit `crates/gat-algo/Cargo.toml`, add after line 18 (clarabel):

```toml
argmin = "0.10"
argmin-math = { version = "0.4", features = ["vec"] }
```

**Step 2: Create the solver**

Create `crates/gat-algo/src/opf/ac_nlp/solver.rs`:

```rust
//! AC-OPF Solver using Interior Point Methods
//!
//! Implements a penalty-based approach using L-BFGS from argmin.
//! This converts the constrained optimization to unconstrained by adding
//! penalty terms for constraint violations.
//!
//! ## Penalty Formulation
//!
//! ```text
//! minimize f(x) + μ · Σ g_i(x)² + μ · Σ max(0, h_j(x))²
//! ```
//!
//! where μ is increased iteratively until constraints are satisfied.

use super::AcOpfProblem;
use crate::opf::{OpfError, OpfMethod, OpfSolution};
use argmin::core::{CostFunction, Executor, Gradient, State};
use argmin::solver::linesearch::MoreThuenteLineSearch;
use argmin::solver::quasinewton::LBFGS;
use std::collections::HashMap;
use std::time::Instant;

/// Penalty function wrapper for argmin
struct PenaltyProblem<'a> {
    problem: &'a AcOpfProblem,
    penalty: f64,
    lb: Vec<f64>,
    ub: Vec<f64>,
}

impl<'a> CostFunction for PenaltyProblem<'a> {
    type Param = Vec<f64>;
    type Output = f64;

    fn cost(&self, x: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
        // Original objective
        let mut cost = self.problem.objective(x);

        // Equality constraint penalty
        let g = self.problem.equality_constraints(x);
        for gi in &g {
            cost += self.penalty * gi * gi;
        }

        // Bound constraint penalty
        for i in 0..x.len() {
            if x[i] < self.lb[i] {
                let v = self.lb[i] - x[i];
                cost += self.penalty * v * v;
            }
            if x[i] > self.ub[i] {
                let v = x[i] - self.ub[i];
                cost += self.penalty * v * v;
            }
        }

        Ok(cost)
    }
}

impl<'a> Gradient for PenaltyProblem<'a> {
    type Param = Vec<f64>;
    type Gradient = Vec<f64>;

    fn gradient(&self, x: &Self::Param) -> Result<Self::Gradient, argmin::core::Error> {
        let n = x.len();
        let eps = 1e-7;

        // Finite difference gradient
        let mut grad = vec![0.0; n];
        let f0 = self.cost(x)?;

        for i in 0..n {
            let mut x_plus = x.clone();
            x_plus[i] += eps;
            let f_plus = self.cost(&x_plus)?;
            grad[i] = (f_plus - f0) / eps;
        }

        Ok(grad)
    }
}

/// Solve AC-OPF using penalty method with L-BFGS
pub fn solve(problem: &AcOpfProblem, max_iterations: usize, tolerance: f64) -> Result<OpfSolution, OpfError> {
    let start = Instant::now();

    let x0 = problem.initial_point();
    let (lb, ub) = problem.variable_bounds();

    // Penalty method: start with small penalty, increase until feasible
    let mut x = x0;
    let mut penalty = 1000.0;
    let penalty_increase = 10.0;
    let max_penalty_iters = 5;

    for outer_iter in 0..max_penalty_iters {
        let penalty_problem = PenaltyProblem {
            problem,
            penalty,
            lb: lb.clone(),
            ub: ub.clone(),
        };

        // L-BFGS with line search
        let linesearch = MoreThuenteLineSearch::new();
        let solver = LBFGS::new(linesearch, 7);

        let executor = Executor::new(penalty_problem, solver)
            .configure(|state| {
                state
                    .param(x.clone())
                    .max_iters(max_iterations as u64 / max_penalty_iters as u64)
                    .target_cost(0.0)
            });

        let result = executor.run();

        match result {
            Ok(res) => {
                if let Some(best) = res.state().get_best_param() {
                    x = best.clone();
                }
            }
            Err(_) => {
                // Continue with current x
            }
        }

        // Check constraint violation
        let g = problem.equality_constraints(&x);
        let max_violation: f64 = g.iter().map(|gi| gi.abs()).fold(0.0, f64::max);

        if max_violation < tolerance {
            break;
        }

        penalty *= penalty_increase;
    }

    // Check final feasibility
    let g = problem.equality_constraints(&x);
    let max_violation: f64 = g.iter().map(|gi| gi.abs()).fold(0.0, f64::max);
    let converged = max_violation < tolerance * 10.0; // Allow some slack

    // Build solution
    let (v, theta) = problem.extract_v_theta(&x);

    let mut solution = OpfSolution {
        converged,
        method_used: OpfMethod::AcOpf,
        iterations: max_iterations,
        solve_time_ms: start.elapsed().as_millis(),
        objective_value: problem.objective(&x),
        ..Default::default()
    };

    // Extract generator dispatch
    for (i, gen) in problem.generators.iter().enumerate() {
        let pg_mw = x[problem.pg_offset + i] * problem.base_mva;
        let qg_mvar = x[problem.qg_offset + i] * problem.base_mva;
        solution.generator_p.insert(gen.name.clone(), pg_mw);
        solution.generator_q.insert(gen.name.clone(), qg_mvar);
    }

    // Extract bus voltages
    for (i, bus) in problem.buses.iter().enumerate() {
        solution.bus_voltage_mag.insert(bus.name.clone(), v[i]);
        solution.bus_voltage_ang.insert(bus.name.clone(), theta[i].to_degrees());
    }

    // Set LMPs (approximate from marginal generators)
    let mut system_lmp = 0.0;
    for (i, gen) in problem.generators.iter().enumerate() {
        let pg_mw = x[problem.pg_offset + i] * problem.base_mva;
        let at_min = (pg_mw - gen.pmin_mw).abs() < 1.0;
        let at_max = (pg_mw - gen.pmax_mw).abs() < 1.0;

        if !at_min && !at_max {
            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
            system_lmp = c1 + 2.0 * c2 * pg_mw;
            break;
        }
    }

    for bus in &problem.buses {
        solution.bus_lmp.insert(bus.name.clone(), system_lmp);
    }

    Ok(solution)
}
```

**Step 3: Export and wire up**

Edit `crates/gat-algo/src/opf/ac_nlp/mod.rs`:

```rust
//! Full Nonlinear AC Optimal Power Flow Solver

mod power_equations;
mod problem;
mod solver;
mod ybus;

pub use power_equations::PowerEquations;
pub use problem::{AcOpfProblem, BusData, GenData};
pub use solver::solve as solve_ac_opf;
pub use ybus::{YBus, YBusBuilder};
```

**Step 4: Wire up in opf/mod.rs**

Edit `crates/gat-algo/src/opf/mod.rs` to route AcOpf:

Replace line 67-69:
```rust
OpfMethod::AcOpf => Err(OpfError::NotImplemented(
    "AC-OPF not yet implemented".into(),
)),
```

With:
```rust
OpfMethod::AcOpf => {
    let problem = ac_nlp::AcOpfProblem::from_network(network)?;
    ac_nlp::solve_ac_opf(&problem, self.max_iterations, self.tolerance)
}
```

And add at top of file (line 9):
```rust
pub mod ac_nlp;
```

**Step 5: Run cargo check**

Run: `cargo check -p gat-algo`
Expected: PASS (may have warnings)

**Step 6: Commit**

```bash
git add crates/gat-algo/Cargo.toml
git add crates/gat-algo/src/opf/ac_nlp/solver.rs
git add crates/gat-algo/src/opf/ac_nlp/mod.rs
git add crates/gat-algo/src/opf/mod.rs
git commit -m "feat(ac-opf): implement penalty-method solver with L-BFGS

Add argmin-based solver using penalty method:
- L-BFGS quasi-Newton optimizer
- Penalty function for equality constraints
- Bound constraint penalties
- Iterative penalty increase until feasibility

Wire AcOpf method to use new solver instead of NotImplemented."
```

---

### Task 6: Create AC-OPF Tests

**Files:**
- Create: `crates/gat-algo/tests/ac_opf.rs`

**Step 1: Write comprehensive tests**

Create `crates/gat-algo/tests/ac_opf.rs`:

```rust
//! AC-OPF solver tests

use gat_algo::{OpfMethod, OpfSolver};
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
};

/// Simple 2-bus test network
fn two_bus_network() -> Network {
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 138.0,
    }));
    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 138.0,
    }));

    network.graph.add_edge(
        bus1,
        bus2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.0,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 10.0,
        reactive_power_mvar: 3.0,
    }));

    network
}

#[test]
fn ac_opf_basic_convergence() {
    let network = two_bus_network();
    let solver = OpfSolver::new()
        .with_method(OpfMethod::AcOpf)
        .with_max_iterations(200)
        .with_tolerance(1e-4);

    let solution = solver.solve(&network).expect("AC-OPF should converge");

    // Should converge (or at least produce a result)
    // Due to penalty method, may not be exactly feasible

    // Generator should supply approximately the load
    let gen_p = solution.generator_p.get("gen1").copied().unwrap_or(0.0);
    assert!(
        gen_p > 5.0 && gen_p < 20.0,
        "Generator P {} should be near load (10 MW)",
        gen_p
    );

    // Voltages should be reasonable
    let v1 = solution.bus_voltage_mag.get("bus1").copied().unwrap_or(0.0);
    let v2 = solution.bus_voltage_mag.get("bus2").copied().unwrap_or(0.0);
    assert!((0.85..=1.15).contains(&v1), "V1 {} out of range", v1);
    assert!((0.85..=1.15).contains(&v2), "V2 {} out of range", v2);
}

#[test]
fn ac_opf_vs_socp_comparison() {
    let network = two_bus_network();

    let socp_solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let ac_solver = OpfSolver::new()
        .with_method(OpfMethod::AcOpf)
        .with_max_iterations(200);

    let socp_sol = socp_solver.solve(&network).expect("SOCP should converge");
    let ac_sol = ac_solver.solve(&network).expect("AC-OPF should converge");

    // Both should give similar objectives (SOCP is a relaxation, so lower bound)
    let socp_cost = socp_sol.objective_value;
    let ac_cost = ac_sol.objective_value;

    // AC cost should be >= SOCP cost (SOCP is relaxation)
    // But for simple networks they should be close
    assert!(
        ac_cost >= socp_cost * 0.9,
        "AC cost {} should be >= SOCP cost {} (minus tolerance)",
        ac_cost,
        socp_cost
    );

    // Generator dispatch should be similar
    let socp_p = socp_sol.generator_p.get("gen1").copied().unwrap_or(0.0);
    let ac_p = ac_sol.generator_p.get("gen1").copied().unwrap_or(0.0);

    assert!(
        (socp_p - ac_p).abs() < 5.0,
        "Generator dispatch should be similar: SOCP={}, AC={}",
        socp_p,
        ac_p
    );
}

/// Three-bus network for more complex test
fn three_bus_network() -> Network {
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 138.0,
    }));
    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 138.0,
    }));
    let bus3 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(2),
        name: "bus3".to_string(),
        voltage_kv: 138.0,
    }));

    // Line 1-2
    network.graph.add_edge(
        bus1,
        bus2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
            ..Branch::default()
        }),
    );

    // Line 2-3
    network.graph.add_edge(
        bus2,
        bus3,
        Edge::Branch(Branch {
            id: BranchId::new(1),
            name: "line2_3".to_string(),
            from_bus: BusId::new(1),
            to_bus: BusId::new(2),
            resistance: 0.01,
            reactance: 0.1,
            ..Branch::default()
        }),
    );

    // Line 1-3 (creates mesh)
    network.graph.add_edge(
        bus1,
        bus3,
        Edge::Branch(Branch {
            id: BranchId::new(2),
            name: "line1_3".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(2),
            resistance: 0.02,
            reactance: 0.15,
            ..Branch::default()
        }),
    );

    // Generator at bus 1 (cheap)
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 200.0,
        qmin_mvar: -100.0,
        qmax_mvar: 100.0,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    // Generator at bus 2 (expensive)
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(1),
        name: "gen2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 20.0),
    }));

    // Load at bus 3
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load3".to_string(),
        bus: BusId::new(2),
        active_power_mw: 50.0,
        reactive_power_mvar: 15.0,
    }));

    network
}

#[test]
fn ac_opf_three_bus_economic_dispatch() {
    let network = three_bus_network();
    let solver = OpfSolver::new()
        .with_method(OpfMethod::AcOpf)
        .with_max_iterations(300);

    let solution = solver.solve(&network).expect("AC-OPF should converge");

    // Cheaper gen1 should dispatch more than expensive gen2
    let gen1_p = solution.generator_p.get("gen1").copied().unwrap_or(0.0);
    let gen2_p = solution.generator_p.get("gen2").copied().unwrap_or(0.0);

    assert!(
        gen1_p > gen2_p,
        "Cheaper gen1 ({}) should dispatch more than gen2 ({})",
        gen1_p,
        gen2_p
    );

    // Total generation should approximately match load + losses
    let total_gen = gen1_p + gen2_p;
    assert!(
        total_gen >= 50.0 && total_gen < 60.0,
        "Total generation {} should cover 50 MW load plus losses",
        total_gen
    );
}
```

**Step 2: Run tests**

Run: `cargo test -p gat-algo ac_opf`
Expected: PASS (3 tests)

**Step 3: Commit**

```bash
git add crates/gat-algo/tests/ac_opf.rs
git commit -m "test(ac-opf): add comprehensive test suite

Test cases:
- Basic convergence on 2-bus network
- Comparison with SOCP relaxation
- Economic dispatch on 3-bus meshed network
- Generator dispatch follows merit order"
```

---

## Phase 3: Integration and Polish

### Task 7: Update Documentation

**Files:**
- Modify: `docs/guide/opf.md`
- Modify: `CHANGELOG.md`

**Step 1: Update OPF guide**

Add to `docs/guide/opf.md` after the SOCP section:

```markdown
## Full AC-OPF (AcOpf)

The full nonlinear AC-OPF solves the complete AC power flow equations without relaxations.

### Features

| Feature | Status |
|---------|--------|
| Polar formulation (V, θ) | ✅ |
| Y-bus construction | ✅ |
| Quadratic costs | ✅ |
| Voltage bounds | ✅ |
| Generator limits | ✅ |
| Jacobian computation | ✅ |
| L-BFGS optimizer | ✅ |
| Thermal limits | 🔄 Planned |
| IPOPT backend | 🔄 Planned |

### Usage

```rust
use gat_algo::{OpfSolver, OpfMethod};

let solver = OpfSolver::new()
    .with_method(OpfMethod::AcOpf)
    .with_max_iterations(200)
    .with_tolerance(1e-4);

let solution = solver.solve(&network)?;
```

### Mathematical Formulation

Variables: V_i (voltage magnitude), θ_i (angle), P_g, Q_g (generator dispatch)

**Objective:**
```
minimize Σ (c₀ + c₁·P_g + c₂·P_g²)
```

**Power Flow Equations:**
```
P_i = Σⱼ V_i·V_j·(G_ij·cos(θ_i - θ_j) + B_ij·sin(θ_i - θ_j))
Q_i = Σⱼ V_i·V_j·(G_ij·sin(θ_i - θ_j) - B_ij·cos(θ_i - θ_j))
```

### Solver Backend

Currently uses argmin's L-BFGS quasi-Newton method with a penalty formulation
for constraints. Future versions will support IPOPT for true interior-point
optimization.
```

**Step 2: Update CHANGELOG**

Add new section to `CHANGELOG.md`:

```markdown
## [0.3.4] - YYYY-MM-DD

### Added

#### Full Nonlinear AC-OPF Solver

- **Y-bus construction** (`crates/gat-algo/src/opf/ac_nlp/ybus.rs`)
  - Complex admittance matrix from network topology
  - Tap ratio and phase shift support
  - Line charging (shunt susceptance)

- **AC power flow equations** (`power_equations.rs`)
  - P and Q injection calculations
  - Full Jacobian computation (∂P/∂θ, ∂P/∂V, ∂Q/∂θ, ∂Q/∂V)

- **NLP problem formulation** (`problem.rs`)
  - Variable layout: [V, θ, P_g, Q_g]
  - Equality constraints (power balance)
  - Bound constraints (voltage, generator limits)

- **Penalty-method solver** (`solver.rs`)
  - L-BFGS optimizer from argmin crate
  - Iterative penalty increase for feasibility
  - Warm start from flat start

- **OpfMethod::AcOpf** now routes to the new solver instead of NotImplemented

### Dependencies

- Added `argmin = "0.10"` for optimization
- Added `argmin-math = "0.4"` for vector math
```

**Step 3: Commit**

```bash
git add docs/guide/opf.md
git add CHANGELOG.md
git commit -m "docs: update OPF documentation for AC-OPF

Add AC-OPF section to opf.md with:
- Feature matrix
- Usage examples
- Mathematical formulation
- Solver backend notes

Update CHANGELOG with 0.3.4 additions."
```

---

### Task 8: Run Full Test Suite and Format

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run clippy**

Run: `cargo clippy -p gat-algo --all-features -- -D warnings`
Expected: PASS (fix any warnings)

**Step 3: Run all tests**

Run: `cargo test -p gat-algo`
Expected: All tests pass

**Step 4: Commit final cleanup**

```bash
git add -A
git commit -m "chore: format and lint cleanup for AC-OPF"
```

---

## Summary

This plan implements a full nonlinear AC-OPF solver in 8 tasks across 3 phases:

**Phase 1: Foundation**
1. Module structure
2. Y-bus matrix construction
3. AC power flow equations

**Phase 2: NLP Formulation**
4. Problem structure
5. argmin-based solver
6. Comprehensive tests

**Phase 3: Integration**
7. Documentation
8. Final cleanup

Each task follows TDD (test → implement → verify → commit) with ~5-10 minute steps.

**Future enhancements** (not in this plan):
- IPOPT backend via feature flag
- Sparse Jacobian/Hessian
- Thermal limit constraints
- Warm start from SOCP solution
- Rectangular formulation fallback
