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
