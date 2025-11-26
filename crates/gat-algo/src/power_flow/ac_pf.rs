//! # AC Power Flow Solver with Newton-Raphson Method
//!
//! This module implements a full Newton-Raphson AC power flow solver with support
//! for reactive power limit enforcement (PV-PQ bus switching). Power flow analysis
//! is the foundation of all power system studies - it determines the steady-state
//! voltage magnitudes and angles at all buses given specified generation and load.
//!
//! ## Why Power Flow Matters
//!
//! Every operational decision in power systems starts with a power flow:
//! - **Operations**: Real-time state estimation and security monitoring
//! - **Markets**: Calculating LMPs and congestion
//! - **Planning**: Evaluating transmission upgrades and generation additions
//! - **Contingency**: N-1/N-2 analysis for reliability assessment
//!
//! ## Bus Classifications
//!
//! Power flow classifies each bus into one of three types:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  BUS TYPE  │  SPECIFIED         │  CALCULATED        │  TYPICAL USE     │
//! │────────────│────────────────────│────────────────────│──────────────────│
//! │  SLACK     │  V, θ (θ = 0)      │  P, Q              │  One per island  │
//! │  PV        │  P, |V|            │  Q, θ              │  Generators      │
//! │  PQ        │  P, Q              │  |V|, θ            │  Loads           │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! **Physical meaning:**
//! - **Slack bus**: The "swing" bus that absorbs generation-load mismatch.
//!   One slack bus per electrical island provides the angle reference.
//! - **PV bus**: Voltage-controlled generator. The AVR (Automatic Voltage
//!   Regulator) adjusts excitation to maintain voltage setpoint.
//! - **PQ bus**: Load bus with specified demand. Most buses are PQ.
//!
//! ## The Newton-Raphson Algorithm
//!
//! Power flow is a system of nonlinear equations. Newton-Raphson solves these
//! by iteratively linearizing around the current solution:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  NEWTON-RAPHSON ITERATION                                                │
//! │  ────────────────────────                                                │
//! │                                                                           │
//! │  Given: Current estimate (V^(k), θ^(k))                                  │
//! │  Find:  Correction (ΔV, Δθ) such that mismatches approach zero           │
//! │                                                                           │
//! │  Step 1: Compute power mismatches                                        │
//! │          ΔP = P_specified - P_calculated(V, θ)                           │
//! │          ΔQ = Q_specified - Q_calculated(V, θ)                           │
//! │                                                                           │
//! │  Step 2: Form Jacobian matrix (linearization of power equations)          │
//! │          J = [ ∂P/∂θ   ∂P/∂V ]                                           │
//! │              [ ∂Q/∂θ   ∂Q/∂V ]                                           │
//! │                                                                           │
//! │  Step 3: Solve linear system for corrections                              │
//! │          J × [Δθ, ΔV]ᵀ = [ΔP, ΔQ]ᵀ                                       │
//! │                                                                           │
//! │  Step 4: Update voltages                                                  │
//! │          θ^(k+1) = θ^(k) + Δθ                                            │
//! │          V^(k+1) = V^(k) + ΔV                                            │
//! │                                                                           │
//! │  Step 5: Check convergence                                                │
//! │          If max(|ΔP|, |ΔQ|) < tolerance: CONVERGED                       │
//! │          Otherwise: go to Step 1                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Convergence Properties
//!
//! Newton-Raphson has **quadratic convergence** near the solution:
//! - Error decreases as ε^(k+1) ≈ C × (ε^(k))²
//! - Typically converges in 3-5 iterations for well-conditioned networks
//! - May diverge if started too far from solution or near voltage collapse
//!
//! ## Q-Limit Enforcement (PV-PQ Switching)
//!
//! Real generators have reactive power limits (the "capability curve"):
//!
//! ```text
//!        Q_max ←───────────────────────────────────────→
//!              │                                        │
//!              │     ╭─────────────────────────────╮    │
//!              │    ╱                               ╲   │
//!              │   ╱         OPERATING              ╲  │
//!              │  ╱            REGION                ╲ │
//!              │ ╱                                    ╲│
//!        ──────│╱──────────────────────────────────────╲───→ P
//!              │╲                                      ╱
//!              │ ╲                                    ╱
//!              │  ╲                                  ╱
//!              │   ╰────────────────────────────────╯
//!        Q_min ←───────────────────────────────────────→
//! ```
//!
//! When a PV bus generator exceeds its Q limit:
//! 1. The bus converts from PV to PQ
//! 2. Q is fixed at the violated limit (Qmin or Qmax)
//! 3. Voltage is no longer controlled (will deviate from setpoint)
//! 4. Power flow is re-solved with the new bus classification
//!
//! This models the physical reality: an overloaded generator cannot
//! maintain voltage and must either reduce reactive output or trip.
//!
//! ## References
//!
//! - **Tinney & Hart (1967)**: "Power Flow Solution by Newton's Method"
//!   IEEE Trans. PAS, 86(11), 1449-1460. The classic reference.
//!   DOI: [10.1109/TPAS.1967.291823](https://doi.org/10.1109/TPAS.1967.291823)
//!
//! - **Stott (1974)**: "Review of Load-Flow Calculation Methods"
//!   Proceedings of the IEEE, 62(7), 916-929. Comprehensive survey.
//!   DOI: [10.1109/PROC.1974.9544](https://doi.org/10.1109/PROC.1974.9544)
//!
//! - **Van Cutsem & Vournas (1998)**: "Voltage Stability of Electric Power Systems"
//!   Springer. Q-limit enforcement and voltage collapse analysis.
//!   DOI: [10.1007/978-0-387-75536-6](https://doi.org/10.1007/978-0-387-75536-6)

use anyhow::{anyhow, Result};
use faer::prelude::SpSolver;
use faer::{FaerMat, Mat};
use gat_core::{BusId, Edge, GenId, Network, Node};
use num_complex::{Complex64, ComplexFloat};
use sprs::{CsMat, TriMat};
use std::collections::HashMap;

/// Bus type classification for power flow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    /// Slack bus: V and θ are fixed, P and Q are calculated
    Slack,
    /// PV bus: P and V are specified, Q and θ are calculated
    PV,
    /// PQ bus: P and Q are specified, V and θ are calculated
    PQ,
}

/// AC Power Flow solution
#[derive(Debug, Clone)]
pub struct AcPowerFlowSolution {
    /// Did the solver converge?
    pub converged: bool,
    /// Number of iterations
    pub iterations: usize,
    /// Maximum power mismatch at convergence
    pub max_mismatch: f64,
    /// Bus voltage magnitudes (p.u.)
    pub bus_voltage_magnitude: HashMap<BusId, f64>,
    /// Bus voltage angles (radians)
    pub bus_voltage_angle: HashMap<BusId, f64>,
    /// Generator reactive power output (MVAR)
    pub generator_q_mvar: HashMap<GenId, f64>,
    /// Generator active power output (MW)
    pub generator_p_mw: HashMap<GenId, f64>,
    /// Final bus types (may differ from initial if Q-limits enforced)
    pub bus_types: HashMap<BusId, BusType>,
    /// Bus reactive power injection (MVAR)
    pub bus_q_injection: HashMap<BusId, f64>,
}

impl Default for AcPowerFlowSolution {
    fn default() -> Self {
        Self {
            converged: false,
            iterations: 0,
            max_mismatch: f64::INFINITY,
            bus_voltage_magnitude: HashMap::new(),
            bus_voltage_angle: HashMap::new(),
            generator_q_mvar: HashMap::new(),
            generator_p_mw: HashMap::new(),
            bus_types: HashMap::new(),
            bus_q_injection: HashMap::new(),
        }
    }
}

/// AC Power Flow Solver configuration
#[derive(Debug, Clone)]
pub struct AcPowerFlowSolver {
    /// Convergence tolerance for power mismatches
    pub tolerance: f64,
    /// Maximum Newton-Raphson iterations
    pub max_iterations: usize,
    /// Whether to enforce generator Q limits (PV-PQ switching)
    pub enforce_q_limits: bool,
    /// Maximum outer iterations for Q-limit enforcement
    pub max_q_iterations: usize,
    /// Voltage setpoint for PV buses (p.u.)
    pub pv_voltage_setpoint: f64,
    /// System MVA base for per-unit conversion (default: 100 MVA)
    pub base_mva: f64,
}

impl Default for AcPowerFlowSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl AcPowerFlowSolver {
    /// Create a new AC power flow solver with default settings
    pub fn new() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 20,
            enforce_q_limits: false,
            max_q_iterations: 10,
            pv_voltage_setpoint: 1.0,
            base_mva: 100.0,
        }
    }

    /// Set convergence tolerance
    pub fn with_tolerance(mut self, tol: f64) -> Self {
        self.tolerance = tol;
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max_iter: usize) -> Self {
        self.max_iterations = max_iter;
        self
    }

    /// Enable or disable Q-limit enforcement
    pub fn with_q_limit_enforcement(mut self, enable: bool) -> Self {
        self.enforce_q_limits = enable;
        self
    }

    /// Set PV bus voltage setpoint
    pub fn with_pv_voltage_setpoint(mut self, v_pu: f64) -> Self {
        self.pv_voltage_setpoint = v_pu;
        self
    }

    /// Set system MVA base for per-unit conversion
    ///
    /// The MVA base is used to convert between MW/MVAR and per-unit values.
    /// Standard values are typically 100 MVA (default) or 1000 MVA for large systems.
    pub fn with_base_mva(mut self, base_mva: f64) -> Self {
        self.base_mva = base_mva;
        self
    }

    /// Solve AC power flow for the given network
    pub fn solve(&self, network: &Network) -> Result<AcPowerFlowSolution> {
        // Build network data structures
        let (buses, bus_idx_map) = self.collect_buses(network);
        let generators = self.collect_generators(network);
        let loads = self.collect_loads(network);
        let branches = self.collect_branches(network);

        if buses.is_empty() {
            return Err(anyhow!("Network has no buses"));
        }

        // Initialize bus types
        let mut bus_types = self.classify_buses(&buses, &generators);

        // Initialize voltage state
        let n = buses.len();
        let mut v_mag = vec![1.0; n]; // Voltage magnitudes (p.u.)
        let mut v_ang = vec![0.0; n]; // Voltage angles (radians)

        // Set PV bus voltages to setpoint
        for (i, bus_id) in buses.iter().enumerate() {
            if bus_types.get(bus_id) == Some(&BusType::PV) {
                v_mag[i] = self.pv_voltage_setpoint;
            }
        }

        // Build admittance matrix
        let y_bus = self.build_y_bus(&buses, &bus_idx_map, &branches);

        // Compute net injections (P, Q specified)
        let (p_spec, q_spec) =
            self.compute_specified_power(&buses, &bus_idx_map, &generators, &loads);

        // Generator Q limits
        let gen_q_limits: HashMap<GenId, (f64, f64)> = generators
            .iter()
            .map(|g| (g.id, (g.qmin_mvar, g.qmax_mvar)))
            .collect();

        // Generator to bus mapping
        let gen_bus_map: HashMap<GenId, BusId> = generators.iter().map(|g| (g.id, g.bus)).collect();

        // Track which generators are Q-limited (and at which limit)
        // When a generator is Q-limited, its Q is fixed at the limit
        let mut gen_q_fixed: HashMap<GenId, f64> = HashMap::new();

        // Mutable q_spec that gets updated when generators hit limits
        let mut q_spec = q_spec;

        // Q-limit outer loop
        for q_iter in 0..self.max_q_iterations {
            // Run Newton-Raphson with current bus types
            let nr_result = self.newton_raphson(
                &buses,
                &bus_idx_map,
                &bus_types,
                &y_bus,
                &p_spec,
                &q_spec,
                &mut v_mag,
                &mut v_ang,
            )?;

            if !nr_result.converged {
                return Err(anyhow!(
                    "Newton-Raphson did not converge after {} iterations (max mismatch: {:.6})",
                    nr_result.iterations,
                    nr_result.max_mismatch
                ));
            }

            // Compute generator Q from power balance
            let gen_q = self.compute_generator_q(
                &buses,
                &bus_idx_map,
                &generators,
                &loads,
                &y_bus,
                &v_mag,
                &v_ang,
            );

            // For Q-limited generators, use the fixed Q value
            let mut final_gen_q = gen_q.clone();
            for (gen_id, &fixed_q) in &gen_q_fixed {
                final_gen_q.insert(*gen_id, fixed_q);
            }

            // Build partial solution
            let solution = self.build_solution(
                &buses,
                &bus_idx_map,
                &bus_types,
                &generators,
                &v_mag,
                &v_ang,
                &final_gen_q,
                &nr_result,
            );

            if !self.enforce_q_limits {
                return Ok(solution);
            }

            // Check Q limits and switch bus types if needed
            let switched = self.check_q_limits_and_fix(
                &generators,
                &gen_bus_map,
                &gen_q,
                &gen_q_limits,
                &mut bus_types,
                &mut gen_q_fixed,
                &mut q_spec,
                &bus_idx_map,
                &loads,
            );

            if !switched {
                // No more switches needed - converged
                return Ok(solution);
            }

            eprintln!(
                "Q-limit iteration {}: buses switched, re-solving",
                q_iter + 1
            );
        }

        Err(anyhow!(
            "Q-limit enforcement did not converge in {} iterations",
            self.max_q_iterations
        ))
    }

    /// Collect bus data from network
    fn collect_buses(&self, network: &Network) -> (Vec<BusId>, HashMap<BusId, usize>) {
        let mut buses = Vec::new();
        for node in network.graph.node_weights() {
            if let Node::Bus(bus) = node {
                buses.push(bus.id);
            }
        }
        buses.sort_by_key(|b| b.value());

        let bus_idx_map: HashMap<BusId, usize> =
            buses.iter().enumerate().map(|(i, &id)| (id, i)).collect();

        (buses, bus_idx_map)
    }

    /// Collect generator data from network
    fn collect_generators(&self, network: &Network) -> Vec<GeneratorData> {
        let mut generators = Vec::new();
        for node in network.graph.node_weights() {
            if let Node::Gen(gen) = node {
                generators.push(GeneratorData {
                    id: gen.id,
                    bus: gen.bus,
                    p_mw: gen.active_power_mw,
                    q_mvar: gen.reactive_power_mvar,
                    qmin_mvar: gen.qmin_mvar,
                    qmax_mvar: gen.qmax_mvar,
                });
            }
        }
        generators
    }

    /// Collect load data from network
    fn collect_loads(&self, network: &Network) -> Vec<LoadData> {
        let mut loads = Vec::new();
        for node in network.graph.node_weights() {
            if let Node::Load(load) = node {
                loads.push(LoadData {
                    bus: load.bus,
                    p_mw: load.active_power_mw,
                    q_mvar: load.reactive_power_mvar,
                });
            }
        }
        loads
    }

    /// Collect branch data from network
    fn collect_branches(&self, network: &Network) -> Vec<BranchData> {
        let mut branches = Vec::new();
        for edge in network.graph.edge_weights() {
            if let Edge::Branch(branch) = edge {
                if branch.status {
                    branches.push(BranchData {
                        from_bus: branch.from_bus,
                        to_bus: branch.to_bus,
                        r_pu: branch.resistance,
                        x_pu: branch.reactance,
                        b_pu: branch.charging_b_pu,
                        tap: branch.tap_ratio,
                        shift: branch.phase_shift_rad,
                    });
                }
            }
        }
        branches
    }

    /// Classify buses into Slack, PV, or PQ
    fn classify_buses(
        &self,
        buses: &[BusId],
        generators: &[GeneratorData],
    ) -> HashMap<BusId, BusType> {
        let mut bus_types = HashMap::new();

        // Initially all buses are PQ
        for bus_id in buses {
            bus_types.insert(*bus_id, BusType::PQ);
        }

        // Buses with generators become PV (first generator's bus becomes slack)
        let mut has_slack = false;
        for gen in generators {
            if !has_slack {
                bus_types.insert(gen.bus, BusType::Slack);
                has_slack = true;
            } else {
                // Only mark as PV if not already slack
                if bus_types.get(&gen.bus) != Some(&BusType::Slack) {
                    bus_types.insert(gen.bus, BusType::PV);
                }
            }
        }

        bus_types
    }

    /// Build the bus admittance matrix Y_bus
    fn build_y_bus(
        &self,
        buses: &[BusId],
        bus_idx_map: &HashMap<BusId, usize>,
        branches: &[BranchData],
    ) -> Vec<Vec<(f64, f64)>> {
        let n = buses.len();
        // Y_bus[i][j] = (G_ij, B_ij) - conductance and susceptance
        let mut y_bus = vec![vec![(0.0, 0.0); n]; n];

        for branch in branches {
            let Some(&i) = bus_idx_map.get(&branch.from_bus) else {
                continue;
            };
            let Some(&j) = bus_idx_map.get(&branch.to_bus) else {
                continue;
            };

            // Series admittance
            let z = Complex64::new(branch.r_pu, branch.x_pu);
            if z.norm_sqr() < 1e-12 {
                continue; // Skip zero impedance branches
            }
            let y_series = z.recip();

            // Shunt admittance (line charging)
            let b_shunt = branch.b_pu / 2.0;

            // Tap ratio + phase shift handling
            let tap_mag = if branch.tap > 0.0 { branch.tap } else { 1.0 };
            let phase = branch.shift;
            let tap = Complex64::from_polar(tap_mag, phase);
            let tap_conj = tap.conj();
            let tap_mag_sq = tap_mag * tap_mag;

            // Off-diagonal elements (negative of branch admittance)
            let y_off_ij = -(y_series / tap_conj);
            let y_off_ji = -(y_series / tap);
            y_bus[i][j].0 += y_off_ij.re;
            y_bus[i][j].1 += y_off_ij.im;
            y_bus[j][i].0 += y_off_ji.re;
            y_bus[j][i].1 += y_off_ji.im;

            // Diagonal elements
            let y_ii = y_series / tap_mag_sq + Complex64::new(0.0, b_shunt);
            let y_jj = y_series + Complex64::new(0.0, b_shunt);
            y_bus[i][i].0 += y_ii.re;
            y_bus[i][i].1 += y_ii.im;
            y_bus[j][j].0 += y_jj.re;
            y_bus[j][j].1 += y_jj.im;
        }

        y_bus
    }

    /// Compute specified power injections at each bus
    fn compute_specified_power(
        &self,
        buses: &[BusId],
        bus_idx_map: &HashMap<BusId, usize>,
        generators: &[GeneratorData],
        loads: &[LoadData],
    ) -> (Vec<f64>, Vec<f64>) {
        let n = buses.len();
        let mut p_spec = vec![0.0; n];
        let mut q_spec = vec![0.0; n];

        // Add generator injections (positive)
        for gen in generators {
            if let Some(&idx) = bus_idx_map.get(&gen.bus) {
                p_spec[idx] += gen.p_mw;
                q_spec[idx] += gen.q_mvar;
            }
        }

        // Subtract load (negative injection)
        for load in loads {
            if let Some(&idx) = bus_idx_map.get(&load.bus) {
                p_spec[idx] -= load.p_mw;
                q_spec[idx] -= load.q_mvar;
            }
        }

        // Convert to per-unit using configured MVA base
        for i in 0..n {
            p_spec[i] /= self.base_mva;
            q_spec[i] /= self.base_mva;
        }

        (p_spec, q_spec)
    }

    /// Run Newton-Raphson iteration
    fn newton_raphson(
        &self,
        buses: &[BusId],
        _bus_idx_map: &HashMap<BusId, usize>,
        bus_types: &HashMap<BusId, BusType>,
        y_bus: &[Vec<(f64, f64)>],
        p_spec: &[f64],
        q_spec: &[f64],
        v_mag: &mut [f64],
        v_ang: &mut [f64],
    ) -> Result<NRResult> {
        let n = buses.len();
        if n == 0 {
            return Ok(NRResult {
                converged: true,
                iterations: 0,
                max_mismatch: 0.0,
            });
        }

        // Identify non-slack buses for P equations and PQ buses for Q equations
        let mut p_buses: Vec<usize> = Vec::new();
        let mut q_buses: Vec<usize> = Vec::new();

        for (i, bus_id) in buses.iter().enumerate() {
            let bus_type = bus_types.get(bus_id).unwrap_or(&BusType::PQ);
            if *bus_type != BusType::Slack {
                p_buses.push(i);
            }
            if *bus_type == BusType::PQ {
                q_buses.push(i);
            }
        }

        let n_p = p_buses.len();
        let n_q = q_buses.len();
        let n_vars = n_p + n_q;

        if n_vars == 0 {
            return Ok(NRResult {
                converged: true,
                iterations: 0,
                max_mismatch: 0.0,
            });
        }

        for iter in 0..self.max_iterations {
            // Compute power mismatches
            let (p_calc, q_calc) = self.compute_power(y_bus, v_mag, v_ang);

            let mut mismatch = vec![0.0; n_vars];
            let mut max_mismatch: f64 = 0.0;

            // ΔP for non-slack buses
            for (k, &i) in p_buses.iter().enumerate() {
                mismatch[k] = p_spec[i] - p_calc[i];
                max_mismatch = max_mismatch.max(mismatch[k].abs());
            }

            // ΔQ for PQ buses
            for (k, &i) in q_buses.iter().enumerate() {
                mismatch[n_p + k] = q_spec[i] - q_calc[i];
                max_mismatch = max_mismatch.max(mismatch[n_p + k].abs());
            }

            if max_mismatch < self.tolerance {
                return Ok(NRResult {
                    converged: true,
                    iterations: iter + 1,
                    max_mismatch,
                });
            }

            // Build Jacobian matrix
            let jacobian = self.build_jacobian(y_bus, v_mag, v_ang, &p_buses, &q_buses);

            // Solve Jacobian system: J × Δx = mismatch
            let delta = self.solve_linear_system_faer(&jacobian, &mismatch)?;

            // Update angles for non-slack buses
            for (k, &i) in p_buses.iter().enumerate() {
                v_ang[i] += delta[k];
            }

            // Update voltage magnitudes for PQ buses
            for (k, &i) in q_buses.iter().enumerate() {
                v_mag[i] += delta[n_p + k];
            }
        }

        // Compute final mismatch for reporting
        let (p_calc, q_calc) = self.compute_power(y_bus, v_mag, v_ang);
        let mut max_mismatch: f64 = 0.0;
        for &i in &p_buses {
            max_mismatch = max_mismatch.max((p_spec[i] - p_calc[i]).abs());
        }
        for &i in &q_buses {
            max_mismatch = max_mismatch.max((q_spec[i] - q_calc[i]).abs());
        }

        Ok(NRResult {
            converged: false,
            iterations: self.max_iterations,
            max_mismatch,
        })
    }

    /// Compute P and Q injections from current voltage state
    fn compute_power(
        &self,
        y_bus: &[Vec<(f64, f64)>],
        v_mag: &[f64],
        v_ang: &[f64],
    ) -> (Vec<f64>, Vec<f64>) {
        let n = v_mag.len();
        let mut p = vec![0.0; n];
        let mut q = vec![0.0; n];

        for i in 0..n {
            for j in 0..n {
                let (g_ij, b_ij) = y_bus[i][j];
                let theta_ij = v_ang[i] - v_ang[j];
                let cos_theta = theta_ij.cos();
                let sin_theta = theta_ij.sin();

                // P_i = Σ V_i × V_j × (G_ij × cos(θ_ij) + B_ij × sin(θ_ij))
                p[i] += v_mag[i] * v_mag[j] * (g_ij * cos_theta + b_ij * sin_theta);
                // Q_i = Σ V_i × V_j × (G_ij × sin(θ_ij) - B_ij × cos(θ_ij))
                q[i] += v_mag[i] * v_mag[j] * (g_ij * sin_theta - b_ij * cos_theta);
            }
        }

        (p, q)
    }

    /// Build Jacobian matrix for Newton-Raphson
    fn build_jacobian(
        &self,
        y_bus: &[Vec<(f64, f64)>],
        v_mag: &[f64],
        v_ang: &[f64],
        p_buses: &[usize],
        q_buses: &[usize],
    ) -> Vec<Vec<f64>> {
        let _n = v_mag.len();
        let n_p = p_buses.len();
        let n_q = q_buses.len();
        let n_vars = n_p + n_q;

        let mut jacobian = vec![vec![0.0; n_vars]; n_vars];

        // J11: ∂P/∂θ (for non-slack buses)
        for (row, &i) in p_buses.iter().enumerate() {
            for (col, &j) in p_buses.iter().enumerate() {
                jacobian[row][col] = self.dp_dtheta(y_bus, v_mag, v_ang, i, j);
            }
        }

        // J12: ∂P/∂V (for PQ buses)
        for (row, &i) in p_buses.iter().enumerate() {
            for (col, &j) in q_buses.iter().enumerate() {
                jacobian[row][n_p + col] = self.dp_dv(y_bus, v_mag, v_ang, i, j);
            }
        }

        // J21: ∂Q/∂θ (for PQ buses)
        for (row, &i) in q_buses.iter().enumerate() {
            for (col, &j) in p_buses.iter().enumerate() {
                jacobian[n_p + row][col] = self.dq_dtheta(y_bus, v_mag, v_ang, i, j);
            }
        }

        // J22: ∂Q/∂V (for PQ buses)
        for (row, &i) in q_buses.iter().enumerate() {
            for (col, &j) in q_buses.iter().enumerate() {
                jacobian[n_p + row][n_p + col] = self.dq_dv(y_bus, v_mag, v_ang, i, j);
            }
        }

        jacobian
    }

    /// Build sparse Jacobian matrix for Newton-Raphson
    ///
    /// Uses CSR (Compressed Sparse Row) format for efficient storage and
    /// matrix-vector multiplication. For power systems, Jacobian sparsity
    /// follows the network topology - only connected buses have non-zero entries.
    fn build_jacobian_sparse(
        &self,
        y_bus: &[Vec<(f64, f64)>],
        v_mag: &[f64],
        v_ang: &[f64],
        p_buses: &[usize],
        q_buses: &[usize],
    ) -> CsMat<f64> {
        let n_p = p_buses.len();
        let n_q = q_buses.len();
        let n_vars = n_p + n_q;

        // Use triplet format for construction, then convert to CSR
        let mut triplets = TriMat::new((n_vars, n_vars));

        // J11: dP/dtheta (for non-slack buses)
        for (row, &i) in p_buses.iter().enumerate() {
            for (col, &j) in p_buses.iter().enumerate() {
                let val = self.dp_dtheta(y_bus, v_mag, v_ang, i, j);
                if val.abs() > 1e-14 {
                    triplets.add_triplet(row, col, val);
                }
            }
        }

        // J12: dP/dV (for PQ buses)
        for (row, &i) in p_buses.iter().enumerate() {
            for (col, &j) in q_buses.iter().enumerate() {
                let val = self.dp_dv(y_bus, v_mag, v_ang, i, j);
                if val.abs() > 1e-14 {
                    triplets.add_triplet(row, n_p + col, val);
                }
            }
        }

        // J21: dQ/dtheta (for PQ buses)
        for (row, &i) in q_buses.iter().enumerate() {
            for (col, &j) in p_buses.iter().enumerate() {
                let val = self.dq_dtheta(y_bus, v_mag, v_ang, i, j);
                if val.abs() > 1e-14 {
                    triplets.add_triplet(n_p + row, col, val);
                }
            }
        }

        // J22: dQ/dV (for PQ buses)
        for (row, &i) in q_buses.iter().enumerate() {
            for (col, &j) in q_buses.iter().enumerate() {
                let val = self.dq_dv(y_bus, v_mag, v_ang, i, j);
                if val.abs() > 1e-14 {
                    triplets.add_triplet(n_p + row, n_p + col, val);
                }
            }
        }

        triplets.to_csr()
    }

    /// ∂P_i/∂θ_j
    fn dp_dtheta(
        &self,
        y_bus: &[Vec<(f64, f64)>],
        v_mag: &[f64],
        v_ang: &[f64],
        i: usize,
        j: usize,
    ) -> f64 {
        let (g_ij, b_ij) = y_bus[i][j];
        let theta_ij = v_ang[i] - v_ang[j];

        if i == j {
            // Diagonal: ∂P_i/∂θ_i = -Q_i - B_ii × V_i²
            let n = v_mag.len();
            let mut q_i = 0.0;
            for k in 0..n {
                let (g_ik, b_ik) = y_bus[i][k];
                let theta_ik = v_ang[i] - v_ang[k];
                q_i += v_mag[i] * v_mag[k] * (g_ik * theta_ik.sin() - b_ik * theta_ik.cos());
            }
            -q_i - b_ij * v_mag[i] * v_mag[i]
        } else {
            // Off-diagonal: ∂P_i/∂θ_j = V_i × V_j × (G_ij × sin(θ_ij) - B_ij × cos(θ_ij))
            v_mag[i] * v_mag[j] * (g_ij * theta_ij.sin() - b_ij * theta_ij.cos())
        }
    }

    /// ∂P_i/∂V_j
    fn dp_dv(
        &self,
        y_bus: &[Vec<(f64, f64)>],
        v_mag: &[f64],
        v_ang: &[f64],
        i: usize,
        j: usize,
    ) -> f64 {
        let (g_ij, b_ij) = y_bus[i][j];
        let theta_ij = v_ang[i] - v_ang[j];

        if i == j {
            // Diagonal: ∂P_i/∂V_i = P_i/V_i + G_ii × V_i
            let n = v_mag.len();
            let mut p_i = 0.0;
            for k in 0..n {
                let (g_ik, b_ik) = y_bus[i][k];
                let theta_ik = v_ang[i] - v_ang[k];
                p_i += v_mag[i] * v_mag[k] * (g_ik * theta_ik.cos() + b_ik * theta_ik.sin());
            }
            p_i / v_mag[i] + g_ij * v_mag[i]
        } else {
            // Off-diagonal: ∂P_i/∂V_j = V_i × (G_ij × cos(θ_ij) + B_ij × sin(θ_ij))
            v_mag[i] * (g_ij * theta_ij.cos() + b_ij * theta_ij.sin())
        }
    }

    /// ∂Q_i/∂θ_j
    fn dq_dtheta(
        &self,
        y_bus: &[Vec<(f64, f64)>],
        v_mag: &[f64],
        v_ang: &[f64],
        i: usize,
        j: usize,
    ) -> f64 {
        let (g_ij, b_ij) = y_bus[i][j];
        let theta_ij = v_ang[i] - v_ang[j];

        if i == j {
            // Diagonal: ∂Q_i/∂θ_i = P_i - G_ii × V_i²
            let n = v_mag.len();
            let mut p_i = 0.0;
            for k in 0..n {
                let (g_ik, b_ik) = y_bus[i][k];
                let theta_ik = v_ang[i] - v_ang[k];
                p_i += v_mag[i] * v_mag[k] * (g_ik * theta_ik.cos() + b_ik * theta_ik.sin());
            }
            p_i - g_ij * v_mag[i] * v_mag[i]
        } else {
            // Off-diagonal: ∂Q_i/∂θ_j = -V_i × V_j × (G_ij × cos(θ_ij) + B_ij × sin(θ_ij))
            -v_mag[i] * v_mag[j] * (g_ij * theta_ij.cos() + b_ij * theta_ij.sin())
        }
    }

    /// ∂Q_i/∂V_j
    fn dq_dv(
        &self,
        y_bus: &[Vec<(f64, f64)>],
        v_mag: &[f64],
        v_ang: &[f64],
        i: usize,
        j: usize,
    ) -> f64 {
        let (g_ij, b_ij) = y_bus[i][j];
        let theta_ij = v_ang[i] - v_ang[j];

        if i == j {
            // Diagonal: ∂Q_i/∂V_i = Q_i/V_i - B_ii × V_i
            let n = v_mag.len();
            let mut q_i = 0.0;
            for k in 0..n {
                let (g_ik, b_ik) = y_bus[i][k];
                let theta_ik = v_ang[i] - v_ang[k];
                q_i += v_mag[i] * v_mag[k] * (g_ik * theta_ik.sin() - b_ik * theta_ik.cos());
            }
            q_i / v_mag[i] - b_ij * v_mag[i]
        } else {
            // Off-diagonal: ∂Q_i/∂V_j = V_i × (G_ij × sin(θ_ij) - B_ij × cos(θ_ij))
            v_mag[i] * (g_ij * theta_ij.sin() - b_ij * theta_ij.cos())
        }
    }

    /// Solve linear system Ax = b using Gaussian elimination
    fn solve_linear_system(&self, a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>> {
        let n = b.len();
        if n == 0 {
            return Ok(vec![]);
        }

        // Create augmented matrix
        let mut aug: Vec<Vec<f64>> = a.iter().cloned().collect();
        for i in 0..n {
            aug[i].push(b[i]);
        }

        // Forward elimination with partial pivoting
        for col in 0..n {
            // Find pivot
            let mut max_row = col;
            let mut max_val = aug[col][col].abs();
            for row in (col + 1)..n {
                if aug[row][col].abs() > max_val {
                    max_val = aug[row][col].abs();
                    max_row = row;
                }
            }

            if max_val < 1e-12 {
                return Err(anyhow!("Singular Jacobian matrix"));
            }

            // Swap rows
            aug.swap(col, max_row);

            // Eliminate
            for row in (col + 1)..n {
                let factor = aug[row][col] / aug[col][col];
                for j in col..=n {
                    aug[row][j] -= factor * aug[col][j];
                }
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = aug[i][n];
            for j in (i + 1)..n {
                sum -= aug[i][j] * x[j];
            }
            x[i] = sum / aug[i][i];
        }

        Ok(x)
    }

    /// Solve linear system Ax = b using faer's optimized LU decomposition
    ///
    /// This is significantly faster than hand-rolled Gaussian elimination
    /// for larger systems, with better numerical stability.
    fn solve_linear_system_faer(&self, a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>> {
        let n = b.len();
        if n == 0 {
            return Ok(vec![]);
        }

        // Convert to faer Mat
        let mut mat = Mat::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                mat.write(i, j, a[i][j]);
            }
        }

        // Convert b to faer column vector
        let mut rhs = Mat::zeros(n, 1);
        for i in 0..n {
            rhs.write(i, 0, b[i]);
        }

        // Solve using LU decomposition with partial pivoting
        let lu = mat.partial_piv_lu();
        let solution = lu.solve(&rhs);

        // Extract solution
        let x: Vec<f64> = (0..n).map(|i| solution.read(i, 0)).collect();

        // Check for NaN/Inf (indicates singular matrix)
        if x.iter().any(|&v| !v.is_finite()) {
            return Err(anyhow!("Singular Jacobian matrix (faer solver)"));
        }

        Ok(x)
    }

    /// Compute generator reactive power from power balance
    fn compute_generator_q(
        &self,
        _buses: &[BusId],
        bus_idx_map: &HashMap<BusId, usize>,
        generators: &[GeneratorData],
        loads: &[LoadData],
        y_bus: &[Vec<(f64, f64)>],
        v_mag: &[f64],
        v_ang: &[f64],
    ) -> HashMap<GenId, f64> {
        let (_, q_calc) = self.compute_power(y_bus, v_mag, v_ang);

        // Build load Q at each bus
        let mut load_q: HashMap<BusId, f64> = HashMap::new();
        for load in loads {
            *load_q.entry(load.bus).or_insert(0.0) += load.q_mvar;
        }

        // For each generator, Q = Q_calc_at_bus + Q_load_at_bus
        // (Note: if multiple generators at same bus, this is an approximation)
        let mut gen_q = HashMap::new();
        for gen in generators {
            if let Some(&idx) = bus_idx_map.get(&gen.bus) {
                // Q injected by network at this bus (in p.u.)
                let q_net_pu = q_calc[idx];
                // Convert to MVAR using configured MVA base
                let q_net_mvar = q_net_pu * self.base_mva;
                // Generator Q = network Q + load Q
                let q_load = load_q.get(&gen.bus).copied().unwrap_or(0.0);
                gen_q.insert(gen.id, q_net_mvar + q_load);
            }
        }

        gen_q
    }

    /// Check generator Q limits, switch bus types, and fix Q at limits
    ///
    /// When a generator hits its Q limit:
    /// 1. Switch bus from PV to PQ
    /// 2. Record the fixed Q value
    /// 3. Update q_spec to use the fixed Q
    #[allow(clippy::too_many_arguments)]
    fn check_q_limits_and_fix(
        &self,
        generators: &[GeneratorData],
        gen_bus_map: &HashMap<GenId, BusId>,
        gen_q: &HashMap<GenId, f64>,
        gen_q_limits: &HashMap<GenId, (f64, f64)>,
        bus_types: &mut HashMap<BusId, BusType>,
        gen_q_fixed: &mut HashMap<GenId, f64>,
        q_spec: &mut Vec<f64>,
        bus_idx_map: &HashMap<BusId, usize>,
        loads: &[LoadData],
    ) -> bool {
        let mut switched = false;

        // Build load Q at each bus for updating q_spec
        let mut load_q: HashMap<BusId, f64> = HashMap::new();
        for load in loads {
            *load_q.entry(load.bus).or_insert(0.0) += load.q_mvar;
        }

        for gen in generators {
            let q = gen_q.get(&gen.id).copied().unwrap_or(0.0);
            let (qmin, qmax) = gen_q_limits
                .get(&gen.id)
                .copied()
                .unwrap_or((f64::NEG_INFINITY, f64::INFINITY));

            let bus_id = gen_bus_map.get(&gen.id).copied().unwrap_or(gen.bus);
            let current_type = bus_types.get(&bus_id).copied().unwrap_or(BusType::PQ);

            // Only check PV buses (slack bus doesn't get converted)
            if current_type != BusType::PV {
                continue;
            }

            let fixed_q = if q > qmax {
                // Hit upper limit - switch to PQ mode, fix Q at Qmax
                bus_types.insert(bus_id, BusType::PQ);
                switched = true;
                eprintln!(
                    "Bus {} switched PV->PQ: Q={:.2} > Qmax={:.2}",
                    bus_id.value(),
                    q,
                    qmax
                );
                Some(qmax)
            } else if q < qmin {
                // Hit lower limit - switch to PQ mode, fix Q at Qmin
                bus_types.insert(bus_id, BusType::PQ);
                switched = true;
                eprintln!(
                    "Bus {} switched PV->PQ: Q={:.2} < Qmin={:.2}",
                    bus_id.value(),
                    q,
                    qmin
                );
                Some(qmin)
            } else {
                None
            };

            if let Some(q_limit) = fixed_q {
                // Record the fixed Q for this generator
                gen_q_fixed.insert(gen.id, q_limit);

                // Update q_spec at this bus to reflect the fixed generator Q
                // q_spec = Q_gen - Q_load (in per unit)
                if let Some(&bus_idx) = bus_idx_map.get(&bus_id) {
                    let q_load = load_q.get(&bus_id).copied().unwrap_or(0.0);
                    q_spec[bus_idx] = (q_limit - q_load) / self.base_mva;
                }
            }
        }

        switched
    }

    /// Build the final solution structure
    fn build_solution(
        &self,
        buses: &[BusId],
        _bus_idx_map: &HashMap<BusId, usize>,
        bus_types: &HashMap<BusId, BusType>,
        generators: &[GeneratorData],
        v_mag: &[f64],
        v_ang: &[f64],
        gen_q: &HashMap<GenId, f64>,
        nr_result: &NRResult,
    ) -> AcPowerFlowSolution {
        let mut solution = AcPowerFlowSolution {
            converged: nr_result.converged,
            iterations: nr_result.iterations,
            max_mismatch: nr_result.max_mismatch,
            ..Default::default()
        };

        // Bus voltages
        for (i, bus_id) in buses.iter().enumerate() {
            solution.bus_voltage_magnitude.insert(*bus_id, v_mag[i]);
            solution.bus_voltage_angle.insert(*bus_id, v_ang[i]);
            if let Some(&bt) = bus_types.get(bus_id) {
                solution.bus_types.insert(*bus_id, bt);
            }
        }

        // Generator outputs
        for gen in generators {
            solution.generator_p_mw.insert(gen.id, gen.p_mw);
            let q = gen_q.get(&gen.id).copied().unwrap_or(0.0);
            solution.generator_q_mvar.insert(gen.id, q);
        }

        solution
    }
}

/// Newton-Raphson iteration result
struct NRResult {
    converged: bool,
    iterations: usize,
    max_mismatch: f64,
}

/// Internal generator data structure
#[derive(Debug, Clone)]
struct GeneratorData {
    id: GenId,
    bus: BusId,
    p_mw: f64,
    q_mvar: f64,
    // P limits intentionally omitted: AC PF solves feasibility without redispatch.
    qmin_mvar: f64,
    qmax_mvar: f64,
}

/// Internal load data structure
#[derive(Debug, Clone)]
struct LoadData {
    bus: BusId,
    p_mw: f64,
    q_mvar: f64,
}

/// Internal branch data structure
#[derive(Debug, Clone)]
struct BranchData {
    from_bus: BusId,
    to_bus: BusId,
    r_pu: f64,
    x_pu: f64,
    b_pu: f64,
    tap: f64,
    shift: f64,
}

#[cfg(test)]
mod sparse_tests {
    use super::*;

    /// Test that sparse Jacobian produces same result as dense for 3-bus system
    #[test]
    fn test_sparse_jacobian_matches_dense() {
        // Create simple 3-bus Y-bus matrix (admittance)
        // Y = [[2, -1, -1], [-1, 2, -1], [-1, -1, 2]] (simple mesh)
        let y_bus_dense = vec![
            vec![(2.0, -0.5), (-1.0, 0.1), (-1.0, 0.1)],
            vec![(-1.0, 0.1), (2.0, -0.5), (-1.0, 0.1)],
            vec![(-1.0, 0.1), (-1.0, 0.1), (2.0, -0.5)],
        ];

        let v_mag = vec![1.0, 1.0, 1.0];
        let v_ang = vec![0.0, -0.05, -0.1];
        let p_buses = vec![1, 2]; // non-slack buses
        let q_buses = vec![1, 2]; // PQ buses

        let solver = AcPowerFlowSolver::new();

        // Build dense Jacobian (existing method)
        let dense_jacobian =
            solver.build_jacobian(&y_bus_dense, &v_mag, &v_ang, &p_buses, &q_buses);

        // Build sparse Jacobian (new method to implement)
        let sparse_jacobian =
            solver.build_jacobian_sparse(&y_bus_dense, &v_mag, &v_ang, &p_buses, &q_buses);

        // Convert sparse back to dense for comparison
        let n = dense_jacobian.len();
        for i in 0..n {
            for j in 0..n {
                let dense_val = dense_jacobian[i][j];
                let sparse_val = *sparse_jacobian.get(i, j).unwrap_or(&0.0);
                assert!(
                    (dense_val - sparse_val).abs() < 1e-10,
                    "Mismatch at ({}, {}): dense={}, sparse={}",
                    i,
                    j,
                    dense_val,
                    sparse_val
                );
            }
        }
    }

    /// Test that faer-based solver produces same result as Gaussian elimination
    #[test]
    fn test_sparse_solver_matches_gaussian() {
        // Simple 3x3 system: Ax = b
        // A = [[4, 1, 0], [1, 4, 1], [0, 1, 4]]
        // b = [1, 2, 1]
        // Expected x ≈ [0.176, 0.412, 0.147]
        let a = vec![
            vec![4.0, 1.0, 0.0],
            vec![1.0, 4.0, 1.0],
            vec![0.0, 1.0, 4.0],
        ];
        let b = vec![1.0, 2.0, 1.0];

        let solver = AcPowerFlowSolver::new();

        // Solve with existing Gaussian elimination
        let x_gauss = solver.solve_linear_system(&a, &b).unwrap();

        // Solve with new faer-based solver
        let x_faer = solver.solve_linear_system_faer(&a, &b).unwrap();

        // Compare results
        for i in 0..3 {
            assert!(
                (x_gauss[i] - x_faer[i]).abs() < 1e-10,
                "Mismatch at {}: gauss={}, faer={}",
                i,
                x_gauss[i],
                x_faer[i]
            );
        }
    }

    /// Test full Newton-Raphson with faer solver on 2-bus network
    #[test]
    fn test_newton_raphson_with_faer_solver() {
        use gat_core::{Branch, BranchId, Bus, Gen, Load, LoadId};

        let mut network = Network::new();

        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(0),
            name: "bus1".to_string(),
            voltage_kv: 100.0,
            ..Bus::default()
        }));

        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "bus2".to_string(),
            voltage_kv: 100.0,
            ..Bus::default()
        }));

        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(0),
                name: "line".to_string(),
                from_bus: BusId::new(0),
                to_bus: BusId::new(1),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        network.graph.add_node(Node::Gen(Gen::new(
            GenId::new(0),
            "gen1".to_string(),
            BusId::new(0),
        )));

        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(0),
            name: "load".to_string(),
            bus: BusId::new(1),
            active_power_mw: 50.0,
            reactive_power_mvar: 10.0,
        }));

        let solver = AcPowerFlowSolver::new()
            .with_tolerance(1e-6)
            .with_max_iterations(20);

        let solution = solver.solve(&network).expect("should converge");
        assert!(solution.converged);
        assert!(solution.iterations <= 10);
    }
}
