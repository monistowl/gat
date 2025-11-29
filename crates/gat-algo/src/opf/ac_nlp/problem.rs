//! # AC-OPF Problem Formulation
//!
//! This module defines the mathematical optimization problem for AC Optimal Power Flow.
//! It transforms network data (buses, generators, loads, branches) into a structured
//! nonlinear program (NLP) that can be solved by various optimization algorithms.
//!
//! ## Decision Variable Layout
//!
//! The optimization variables are laid out in a single vector `x` as:
//!
//! ```text
//! x = [ V₁, V₂, ..., V_n,  θ₁, θ₂, ..., θ_n,  P_g1, ..., P_gm,  Q_g1, ..., Q_gm ]
//!     |<─── voltages ───>|<─── angles ───>|<── real power ─>|<── reactive ──>|
//!     |      n_bus       |     n_bus      |      n_gen      |      n_gen     |
//!     |<─────────────────────────────────────────────────────────────────────>|
//!                                    n_var = 2*n_bus + 2*n_gen
//! ```
//!
//! **Variable groups:**
//! - **V (voltage magnitudes)**: Per-unit voltage at each bus (typically 0.9-1.1)
//! - **θ (voltage angles)**: Phase angle at each bus in radians (-π/2 to +π/2)
//! - **P_g (real power)**: Generator active power dispatch in per-unit
//! - **Q_g (reactive power)**: Generator reactive power dispatch in per-unit
//!
//! ## Objective Function
//!
//! Minimize total generation cost using polynomial cost functions:
//!
//! ```text
//! minimize  Σ_g [ c₀_g + c₁_g · P_g + c₂_g · P_g² ]
//!
//! where:
//!   c₀ = No-load cost ($/hr) - fixed cost when generator is online
//!   c₁ = Linear cost ($/MWh) - incremental fuel cost
//!   c₂ = Quadratic cost ($/MW²h) - efficiency degradation at high/low output
//! ```
//!
//! **Physical interpretation:** This models the thermal heat rate curve of generators.
//! More efficient generators have lower c₁ coefficients. The quadratic term captures
//! the efficiency reduction when operating far from optimal heat rate.
//!
//! ## Equality Constraints
//!
//! Power balance at each bus (Kirchhoff's current law):
//!
//! ```text
//! P_gen - P_load - P_injected(V, θ) = 0    (n_bus equations)
//! Q_gen - Q_load - Q_injected(V, θ) = 0    (n_bus equations)
//! θ_ref = 0                                 (1 equation)
//! ```
//!
//! The reference angle constraint breaks the symmetry since only angle *differences*
//! affect power flow. Any bus can be chosen as reference; we use bus 0.
//!
//! ## Inequality Constraints
//!
//! Physical operating limits:
//!
//! ```text
//! V_min ≤ V_i ≤ V_max         Voltage limits (equipment insulation, stability)
//! P_min ≤ P_g ≤ P_max         Generator MW limits (turbine capability)
//! Q_min ≤ Q_g ≤ Q_max         Generator MVAr limits (field current, heating)
//! θ_min ≤ θ_i ≤ θ_max         Angle limits (numerical stability, ±π/2)
//! ```
//!
//! **Note:** Branch thermal limits (S_ij ≤ S_max) are not currently implemented
//! as explicit constraints, but could be added as quadratic inequality constraints.
//!
//! ## Per-Unit System
//!
//! All electrical quantities are normalized to a common base, typically 100 MVA:
//!
//! ```text
//! P_pu = P_MW / S_base          where S_base = 100 MVA
//! Q_pu = Q_MVAr / S_base
//! V_pu = V / V_base             where V_base is bus nominal voltage
//! ```
//!
//! This scaling improves numerical conditioning by keeping most values near 1.0.
//!
//! ## References
//!
//! - **Zimmerman et al. (2011)**: "MATPOWER: Steady-State Operations, Planning,
//!   and Analysis Tools for Power Systems Research and Education"
//!   IEEE Trans. Power Systems, 26(1), 12-19
//!   DOI: [10.1109/TPWRS.2010.2051168](https://doi.org/10.1109/TPWRS.2010.2051168)
//!
//! - **Frank et al. (2012)**: "Optimal Power Flow: A Bibliographic Survey"
//!   Energy Systems, 3(3), 221-258
//!   DOI: [10.1007/s12667-012-0056-y](https://doi.org/10.1007/s12667-012-0056-y)

use super::{PowerEquations, YBus, YBusBuilder};
use crate::opf::OpfError;
use gat_core::{BusId, CostModel, Edge, Network, Node};
use std::collections::HashMap;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Interpolate Q limits at given P from capability curve.
///
/// Returns (q_min, q_max) in MVAr at the specified P operating point.
/// If curve is empty, returns the default rectangular limits.
pub fn interpolate_q_limits(
    curve: &[CapabilityCurvePoint],
    p_mw: f64,
    default_qmin: f64,
    default_qmax: f64,
) -> (f64, f64) {
    if curve.is_empty() {
        return (default_qmin, default_qmax);
    }
    if curve.len() == 1 {
        return (curve[0].qmin_mvar, curve[0].qmax_mvar);
    }

    // Find bracketing points
    for i in 0..curve.len() - 1 {
        if p_mw >= curve[i].p_mw && p_mw <= curve[i + 1].p_mw {
            // Linear interpolation
            let t = (p_mw - curve[i].p_mw) / (curve[i + 1].p_mw - curve[i].p_mw);
            let qmin = curve[i].qmin_mvar + t * (curve[i + 1].qmin_mvar - curve[i].qmin_mvar);
            let qmax = curve[i].qmax_mvar + t * (curve[i + 1].qmax_mvar - curve[i].qmax_mvar);
            return (qmin, qmax);
        }
    }

    // Extrapolate from endpoints
    if p_mw < curve[0].p_mw {
        (curve[0].qmin_mvar, curve[0].qmax_mvar)
    } else {
        let last = curve.last().unwrap();
        (last.qmin_mvar, last.qmax_mvar)
    }
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Point on generator capability curve (P, Q_min, Q_max).
///
/// Capability curves define non-rectangular P-Q operating limits that
/// more accurately model generator physical constraints like field current
/// heating and armature heating limits.
#[derive(Debug, Clone)]
pub struct CapabilityCurvePoint {
    /// Real power output (MW)
    pub p_mw: f64,
    /// Minimum reactive power at this P (MVAr)
    pub qmin_mvar: f64,
    /// Maximum reactive power at this P (MVAr)
    pub qmax_mvar: f64,
}

/// Generator data extracted from network for OPF optimization.
///
/// Contains the essential parameters needed for dispatch optimization:
/// operating limits and cost function.
#[derive(Debug, Clone)]
pub struct GenData {
    /// Human-readable generator identifier
    pub name: String,

    /// Bus where generator connects (injection point)
    pub bus_id: BusId,

    /// Minimum real power output (MW).
    /// Below this, the generator must shut down (minimum stable operation).
    pub pmin_mw: f64,

    /// Maximum real power output (MW).
    /// Limited by turbine size, boiler capacity, or heat rate degradation.
    pub pmax_mw: f64,

    /// Minimum reactive power output (MVAr).
    /// Limited by under-excitation (leading power factor) capability.
    pub qmin_mvar: f64,

    /// Maximum reactive power output (MVAr).
    /// Limited by field current heating (lagging power factor).
    pub qmax_mvar: f64,

    /// Cost function coefficients [c₀, c₁, c₂, ...] for polynomial costs.
    /// Cost = c₀ + c₁·P + c₂·P² + ...
    /// Units: c₀ in $/hr, c₁ in $/MWh, c₂ in $/MW²h
    /// Note: For piecewise-linear costs, this is ignored and `cost_model` is used.
    pub cost_coeffs: Vec<f64>,

    /// Full cost model supporting both polynomial and piecewise-linear costs.
    /// When present, this takes precedence over `cost_coeffs`.
    pub cost_model: CostModel,

    /// Capability curve points (if empty, use rectangular limits).
    /// Points should be sorted by p_mw in ascending order.
    pub capability_curve: Vec<CapabilityCurvePoint>,
}

/// Branch data for thermal limit constraints.
///
/// Contains branch parameters needed for computing power flows
/// and enforcing thermal limits |S_ij| ≤ S_max.
#[derive(Debug, Clone)]
pub struct BranchData {
    /// Branch name/identifier
    pub name: String,
    /// From bus index (internal 0-based)
    pub from_idx: usize,
    /// To bus index (internal 0-based)
    pub to_idx: usize,
    /// Series resistance (p.u.)
    pub r: f64,
    /// Series reactance (p.u.)
    pub x: f64,
    /// Line charging susceptance (p.u.)
    pub b_charging: f64,
    /// Tap ratio (1.0 for transmission lines)
    pub tap: f64,
    /// Phase shift (radians)
    pub shift: f64,
    /// Thermal limit (MVA), 0 = unlimited
    pub rate_mva: f64,
    /// Maximum angle difference (radians), 0 = no limit
    /// Typical values: 30-60 degrees (0.52-1.05 radians)
    pub angle_diff_max: f64,
}

/// Bus data extracted from network for OPF optimization.
///
/// Represents an electrical node with load injection and voltage limits.
#[derive(Debug, Clone)]
pub struct BusData {
    /// External bus identifier (from input data)
    pub id: BusId,

    /// Human-readable bus name
    pub name: String,

    /// Internal 0-based index for matrix operations
    pub index: usize,

    /// Minimum allowed voltage magnitude (per-unit).
    /// Typically 0.9-0.95 p.u. to prevent voltage collapse and equipment damage.
    pub v_min: f64,

    /// Maximum allowed voltage magnitude (per-unit).
    /// Typically 1.05-1.1 p.u. to prevent insulation breakdown.
    pub v_max: f64,

    /// Real power load at this bus (MW).
    /// Positive value = power consumed (sink).
    pub p_load: f64,

    /// Reactive power load at this bus (MVAr).
    /// Positive = inductive load (absorbs VARs).
    pub q_load: f64,

    /// Bus shunt conductance (per-unit on system MVA base).
    /// Represents fixed shunt loads that draw real power: P_shunt = gs_pu * V²
    /// Typically zero, but can represent constant-impedance loads.
    pub gs_pu: f64,

    /// Bus shunt susceptance (per-unit on system MVA base).
    /// Represents fixed reactive power compensation:
    /// - bs_pu > 0: Capacitor bank (supplies VARs, raises voltage)
    /// - bs_pu < 0: Reactor (absorbs VARs, lowers voltage)
    /// Power contribution: Q_shunt = bs_pu * V² (injected at bus)
    pub bs_pu: f64,
}

// ============================================================================
// AC-OPF PROBLEM DEFINITION
// ============================================================================

/// Complete AC-OPF problem specification.
///
/// This struct packages all information needed to solve the optimization:
/// - Network topology (Y-bus)
/// - Bus and generator data
/// - Variable indexing scheme
///
/// # Example
///
/// ```ignore
/// let problem = AcOpfProblem::from_network(&network)?;
///
/// // Get initial guess
/// let x0 = problem.initial_point();
///
/// // Evaluate objective and constraints
/// let cost = problem.objective(&x0);
/// let violations = problem.equality_constraints(&x0);
/// ```
#[derive(Clone)]
pub struct AcOpfProblem {
    /// Y-bus admittance matrix encoding network topology and impedances.
    /// Used to compute power flow equations.
    pub ybus: YBus,

    /// Bus data including loads and voltage limits.
    /// Indexed by internal bus index (0 to n_bus-1).
    pub buses: Vec<BusData>,

    /// Generator data including limits and costs.
    /// Indexed by generator index (0 to n_gen-1).
    pub generators: Vec<GenData>,

    /// Index of reference bus (angle fixed to 0).
    /// Typically the largest generator or a well-connected bus.
    pub ref_bus: usize,

    /// Per-unit base power (MVA).
    /// All MW/MVAr values are divided by this for normalization.
    /// Standard value: 100 MVA.
    pub base_mva: f64,

    // ========================================================================
    // PROBLEM DIMENSIONS
    // ========================================================================
    /// Number of buses in the network
    pub n_bus: usize,

    /// Number of generators (dispatchable units)
    pub n_gen: usize,

    /// Total number of optimization variables = 2*n_bus + 2*n_gen
    pub n_var: usize,

    // ========================================================================
    // VARIABLE INDEX OFFSETS
    // ========================================================================
    //
    // These offsets define where each variable group starts in the x vector.
    // Using offsets allows efficient extraction of subvectors.
    /// Offset to voltage magnitudes: x[v_offset + i] = V_i
    pub v_offset: usize,

    /// Offset to voltage angles: x[theta_offset + i] = θ_i
    pub theta_offset: usize,

    /// Offset to generator P: x[pg_offset + g] = P_g
    pub pg_offset: usize,

    /// Offset to generator Q: x[qg_offset + g] = Q_g
    pub qg_offset: usize,

    // ========================================================================
    // GENERATOR-BUS MAPPING
    // ========================================================================
    /// Maps generator index to bus index where it injects power.
    /// gen_bus_idx[g] = internal bus index for generator g
    pub gen_bus_idx: Vec<usize>,

    /// Branch data for thermal constraints
    pub branches: Vec<BranchData>,

    /// Number of branches in the network
    pub n_branch: usize,
}

impl AcOpfProblem {
    /// Build OPF problem from Network graph.
    ///
    /// Extracts buses, generators, and loads from the network graph,
    /// builds the Y-bus matrix, and sets up variable indexing.
    ///
    /// # Arguments
    ///
    /// * `network` - Network graph with Bus, Gen, Load, and Branch nodes/edges
    ///
    /// # Returns
    ///
    /// * `Ok(AcOpfProblem)` - Ready to solve
    /// * `Err(OpfError)` - Invalid network (no buses, no generators, etc.)
    ///
    /// # Algorithm
    ///
    /// 1. Build Y-bus from network branches
    /// 2. Extract bus data and assign internal indices
    /// 3. Sum loads at each bus (may have multiple loads per bus)
    /// 4. Extract generator data and cost models
    /// 5. Set up variable offsets
    pub fn from_network(network: &Network) -> Result<Self, OpfError> {
        // ====================================================================
        // BUILD Y-BUS MATRIX
        // ====================================================================
        //
        // The Y-bus encodes network topology and branch impedances.
        // This is computed first because it determines bus indexing.

        let ybus = YBusBuilder::from_network(network)?;

        // ====================================================================
        // EXTRACT BUS DATA
        // ====================================================================
        //
        // Iterate through network nodes to find all Bus elements.
        // Assign sequential internal indices (0, 1, 2, ...) for matrix operations.

        let mut buses = Vec::new();
        let mut loads: HashMap<BusId, (f64, f64)> = HashMap::new();
        let mut shunts: HashMap<BusId, (f64, f64)> = HashMap::new(); // (gs_pu, bs_pu)
        let mut bus_idx = 0;

        for node_idx in network.graph.node_indices() {
            match &network.graph[node_idx] {
                Node::Bus(bus) => {
                    // Use actual voltage limits from case data, with sensible defaults
                    // PGLib cases typically use 0.94-1.06 for IEEE cases
                    let v_min = bus.vmin_pu.unwrap_or(0.9);
                    let v_max = bus.vmax_pu.unwrap_or(1.1);
                    buses.push(BusData {
                        id: bus.id,
                        name: bus.name.clone(),
                        index: bus_idx,
                        v_min,
                        v_max,
                        p_load: 0.0,
                        q_load: 0.0,
                        gs_pu: 0.0,
                        bs_pu: 0.0,
                    });
                    bus_idx += 1;
                }
                Node::Load(load) => {
                    // Accumulate loads at each bus (there may be multiple)
                    // This handles distributed loads modeled as separate entities
                    let entry = loads.entry(load.bus).or_insert((0.0, 0.0));
                    entry.0 += load.active_power_mw;
                    entry.1 += load.reactive_power_mvar;
                }
                Node::Shunt(shunt) => {
                    // Accumulate shunts at each bus (there may be multiple)
                    // Shunts represent fixed reactive power compensation:
                    // - gs_pu: conductance (draws real power)
                    // - bs_pu > 0: capacitor bank (supplies VARs)
                    // - bs_pu < 0: reactor (absorbs VARs)
                    let entry = shunts.entry(shunt.bus).or_insert((0.0, 0.0));
                    entry.0 += shunt.gs_pu;
                    entry.1 += shunt.bs_pu;
                }
                _ => {}
            }
        }

        // Apply accumulated loads to bus data
        for bus in &mut buses {
            if let Some((p, q)) = loads.get(&bus.id) {
                bus.p_load = *p;
                bus.q_load = *q;
            }
        }

        // Apply accumulated shunts to bus data (already in per-unit)
        for bus in &mut buses {
            if let Some((gs, bs)) = shunts.get(&bus.id) {
                bus.gs_pu = *gs;
                bus.bs_pu = *bs;
            }
        }

        // ====================================================================
        // EXTRACT GENERATOR DATA
        // ====================================================================
        //
        // Each generator contributes:
        // - Decision variables (P_g, Q_g)
        // - Objective function terms (cost model)
        // - Constraints (operating limits)

        let mut generators = Vec::new();
        for node_idx in network.graph.node_indices() {
            if let Node::Gen(gen) = &network.graph[node_idx] {
                // Extract polynomial coefficients (for backwards compatibility)
                // Piecewise-linear costs use the full cost_model instead
                let cost_coeffs = match &gen.cost_model {
                    CostModel::NoCost => vec![0.0, 0.0],
                    CostModel::Polynomial(c) => c.clone(),
                    CostModel::PiecewiseLinear(_) => {
                        // For piecewise-linear, coeffs are placeholder only
                        // The objective() function uses cost_model directly
                        vec![0.0, 0.0]
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
                    cost_model: gen.cost_model.clone(),
                    capability_curve: Vec::new(), // Default: use rectangular limits
                });
            }
        }

        if generators.is_empty() {
            return Err(OpfError::DataValidation(
                "No generators in network".to_string(),
            ));
        }

        // ====================================================================
        // COMPUTE DIMENSIONS AND OFFSETS
        // ====================================================================
        //
        // Variable layout: [V | θ | P_g | Q_g]
        //
        // Offsets enable O(1) access to any variable type:
        //   x[v_offset + i]     → voltage at bus i
        //   x[theta_offset + i] → angle at bus i
        //   x[pg_offset + g]    → real power of generator g
        //   x[qg_offset + g]    → reactive power of generator g

        let n_bus = buses.len();
        let n_gen = generators.len();
        let n_var = 2 * n_bus + 2 * n_gen;

        // Build generator-to-bus index mapping for power balance equations
        let bus_map: HashMap<BusId, usize> = buses.iter().map(|b| (b.id, b.index)).collect();
        let gen_bus_idx: Vec<usize> = generators
            .iter()
            .map(|g| *bus_map.get(&g.bus_id).unwrap_or(&0))
            .collect();

        // ====================================================================
        // EXTRACT BRANCH DATA
        // ====================================================================
        //
        // Branches are needed for:
        // - Thermal limit constraints (|S_ij| ≤ S_max)
        // - Branch flow reporting
        // - Angle difference constraints

        let mut branches = Vec::new();
        for edge_idx in network.graph.edge_indices() {
            if let Edge::Branch(branch) = &network.graph[edge_idx] {
                if !branch.status {
                    continue; // Skip offline branches
                }
                let from_idx = *bus_map.get(&branch.from_bus).unwrap_or(&0);
                let to_idx = *bus_map.get(&branch.to_bus).unwrap_or(&0);

                branches.push(BranchData {
                    name: branch.name.clone(),
                    from_idx,
                    to_idx,
                    r: branch.resistance,
                    x: branch.reactance,
                    b_charging: branch.charging_b_pu,
                    tap: branch.tap_ratio,
                    shift: branch.phase_shift_rad,
                    rate_mva: branch.rating_a_mva.unwrap_or(0.0),
                    angle_diff_max: 0.0, // Default: no limit (could be extracted from Branch if available)
                });
            }
        }
        let n_branch = branches.len();

        Ok(Self {
            ybus,
            buses,
            generators,
            ref_bus: 0,      // Use first bus as reference (could be configurable)
            base_mva: 100.0, // Standard per-unit base

            n_bus,
            n_gen,
            n_var,

            v_offset: 0,
            theta_offset: n_bus,
            pg_offset: 2 * n_bus,
            qg_offset: 2 * n_bus + n_gen,

            gen_bus_idx,

            branches,
            n_branch,
        })
    }

    /// Generate a "flat start" initial point.
    ///
    /// A flat start assumes:
    /// - All voltages at 1.0 p.u. (nominal)
    /// - All angles at 0 radians (synchronized)
    /// - Generators at midpoint of operating range
    ///
    /// This is a common initialization strategy that works well for most networks.
    /// For difficult cases, a DC power flow solution may provide a better start.
    ///
    /// # Returns
    ///
    /// Vector of length n_var with initial values for all decision variables.
    pub fn initial_point(&self) -> Vec<f64> {
        let mut x = vec![0.0; self.n_var];

        // ====================================================================
        // VOLTAGE MAGNITUDES: FLAT START (V = 1.0)
        // ====================================================================
        //
        // Starting at 1.0 p.u. is reasonable because:
        // - Most systems operate within ±10% of nominal
        // - This is the center of typical voltage bands
        // - Ensures initial point is within bounds

        for i in 0..self.n_bus {
            x[self.v_offset + i] = 1.0;
        }

        // ====================================================================
        // VOLTAGE ANGLES: ZERO START (θ = 0)
        // ====================================================================
        //
        // All angles start at zero (synchronized system).
        // Already initialized by vec![0.0; n_var]

        // ====================================================================
        // GENERATOR DISPATCH: MIDPOINT START
        // ====================================================================
        //
        // Starting at midpoint of [P_min, P_max] is a safe choice:
        // - Guaranteed to be feasible
        // - Provides room to adjust up or down
        // - Roughly balances load with generation

        for (i, gen) in self.generators.iter().enumerate() {
            x[self.pg_offset + i] = (gen.pmin_mw + gen.pmax_mw) / 2.0 / self.base_mva;
            x[self.qg_offset + i] = (gen.qmin_mvar + gen.qmax_mvar) / 2.0 / self.base_mva;
        }

        x
    }

    /// Create a warm-start initial point from a previous OPF solution (DC or SOCP).
    ///
    /// Warm-starting significantly improves AC-OPF convergence by providing an
    /// initial point that is closer to the optimal solution. This is especially
    /// valuable for:
    /// - Multi-period optimization (sequential solves)
    /// - Contingency analysis (similar solutions)
    /// - DC→AC refinement workflows
    ///
    /// # Arguments
    ///
    /// * `solution` - A previous OPF solution (from DC-OPF, SOCP, or prior AC-OPF)
    ///
    /// # Returns
    ///
    /// Vector of length n_var with initial values extracted from the solution.
    /// Missing values default to the flat-start values.
    ///
    /// # How Values Are Used
    ///
    /// | Variable | DC-OPF | SOCP | AC-OPF |
    /// |----------|--------|------|--------|
    /// | V_i      | 1.0 (assumed) | Relaxed | Direct |
    /// | θ_i      | From LP (degrees→radians) | From conic | Direct |
    /// | P_g      | Direct | Direct | Direct |
    /// | Q_g      | Midpoint (not solved) | Direct | Direct |
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Solve DC-OPF first (fast, globally optimal)
    /// let dc_solution = solve_dc_opf(&network)?;
    ///
    /// // Use DC solution as warm-start for AC-OPF
    /// let ac_problem = AcOpfProblem::from_network(&network)?;
    /// let x0 = ac_problem.warm_start_from_solution(&dc_solution);
    /// let ac_solution = solve_ac_opf_with_start(&ac_problem, x0)?;
    /// ```
    pub fn warm_start_from_solution(&self, solution: &crate::opf::OpfSolution) -> Vec<f64> {
        // Start with flat-start as fallback
        let mut x = self.initial_point();

        // ====================================================================
        // VOLTAGE MAGNITUDES FROM SOLUTION
        // ====================================================================
        //
        // DC-OPF assumes V=1.0, so these will be empty/default.
        // SOCP/AC-OPF provide actual voltage magnitudes.

        for (i, bus) in self.buses.iter().enumerate() {
            if let Some(&v_mag) = solution.bus_voltage_mag.get(&bus.name) {
                // Clamp to valid range to avoid infeasible start
                x[self.v_offset + i] = v_mag.max(bus.v_min).min(bus.v_max);
            }
        }

        // ====================================================================
        // VOLTAGE ANGLES FROM SOLUTION (DEGREES → RADIANS)
        // ====================================================================
        //
        // OpfSolution stores angles in degrees, AC-OPF uses radians.

        for (i, bus) in self.buses.iter().enumerate() {
            if let Some(&theta_deg) = solution.bus_voltage_ang.get(&bus.name) {
                x[self.theta_offset + i] = theta_deg.to_radians();
            }
        }

        // ====================================================================
        // GENERATOR REAL POWER FROM SOLUTION
        // ====================================================================
        //
        // This is usually available from all OPF methods.
        // Convert from MW to per-unit and clamp to bounds.

        for (i, gen) in self.generators.iter().enumerate() {
            if let Some(&pg_mw) = solution.generator_p.get(&gen.name) {
                let pg_clamped = pg_mw.max(gen.pmin_mw).min(gen.pmax_mw);
                x[self.pg_offset + i] = pg_clamped / self.base_mva;
            }
        }

        // ====================================================================
        // GENERATOR REACTIVE POWER FROM SOLUTION
        // ====================================================================
        //
        // DC-OPF doesn't solve Q, so this will be empty.
        // SOCP/AC-OPF provide reactive power dispatch.

        for (i, gen) in self.generators.iter().enumerate() {
            if let Some(&qg_mvar) = solution.generator_q.get(&gen.name) {
                let qg_clamped = qg_mvar.max(gen.qmin_mvar).min(gen.qmax_mvar);
                x[self.qg_offset + i] = qg_clamped / self.base_mva;
            }
            // If Q not available (DC-OPF), keep the midpoint from flat-start
        }

        x
    }

    /// Extract voltage magnitude and angle vectors from decision variable vector.
    ///
    /// # Arguments
    ///
    /// * `x` - Full decision variable vector
    ///
    /// # Returns
    ///
    /// Tuple `(V, θ)` where:
    /// * `V[i]` = voltage magnitude at bus i (p.u.)
    /// * `θ[i]` = voltage angle at bus i (radians)
    pub fn extract_v_theta(&self, x: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let v: Vec<f64> = (0..self.n_bus).map(|i| x[self.v_offset + i]).collect();
        let theta: Vec<f64> = (0..self.n_bus).map(|i| x[self.theta_offset + i]).collect();
        (v, theta)
    }

    /// Evaluate the objective function (total generation cost).
    ///
    /// Supports both polynomial and piecewise-linear cost models:
    /// - **Polynomial**: f(P) = c₀ + c₁·P + c₂·P² + ...
    /// - **Piecewise-linear**: Interpolated from (MW, $/hr) breakpoints
    ///
    /// # Arguments
    ///
    /// * `x` - Decision variable vector
    ///
    /// # Returns
    ///
    /// Total generation cost in $/hr
    ///
    /// # Note
    ///
    /// Generator dispatch values P_g are stored in per-unit in x, but cost
    /// models are defined in MW. We convert back to MW for cost evaluation.
    pub fn objective(&self, x: &[f64]) -> f64 {
        let mut cost = 0.0;

        for (i, gen) in self.generators.iter().enumerate() {
            // Extract dispatch in per-unit and convert to MW
            let pg_pu = x[self.pg_offset + i];
            let pg_mw = pg_pu * self.base_mva;

            // Use the full cost model which handles both polynomial and
            // piecewise-linear costs correctly via CostModel::evaluate()
            cost += gen.cost_model.evaluate(pg_mw);
        }

        cost
    }

    /// Compute the gradient of the objective function.
    ///
    /// Supports both polynomial and piecewise-linear cost models:
    /// - **Polynomial**: ∂f/∂P = c₁ + 2·c₂·P + ... (marginal cost)
    /// - **Piecewise-linear**: Slope of the segment containing current P
    ///
    /// The scaling by S_base accounts for the per-unit representation:
    /// ```text
    /// ∂f/∂P_pu = marginal_cost(P_MW) · S_base
    /// ```
    ///
    /// # Arguments
    ///
    /// * `x` - Decision variable vector
    ///
    /// # Returns
    ///
    /// Gradient vector of length n_var. Only entries at pg_offset are non-zero.
    pub fn objective_gradient(&self, x: &[f64]) -> Vec<f64> {
        let mut grad = vec![0.0; self.n_var];

        for (i, gen) in self.generators.iter().enumerate() {
            let pg_pu = x[self.pg_offset + i];
            let pg_mw = pg_pu * self.base_mva;

            // Use marginal_cost() which handles both polynomial and
            // piecewise-linear costs correctly
            // Chain rule: ∂f/∂P_pu = ∂f/∂P_MW · ∂P_MW/∂P_pu = marginal_cost · S_base
            grad[self.pg_offset + i] = gen.cost_model.marginal_cost(pg_mw) * self.base_mva;
        }

        grad
    }

    // ========================================================================
    // BRANCH FLOW COMPUTATION
    // ========================================================================

    /// Compute power flow on a branch (from side) in per-unit.
    ///
    /// For a branch from bus i to bus j with:
    /// - Series admittance: g + jb = 1/(r + jx)
    /// - Line charging: bc (total charging susceptance, split half to each side)
    /// - Tap ratio: a (1.0 for transmission lines)
    /// - Phase shift: θ_s (radians)
    ///
    /// The "from" side power injection is:
    /// ```text
    /// P_ij = (Vi²/a²) · g - (Vi·Vj/a) · [g·cos(θi-θj-θs) + b·sin(θi-θj-θs)]
    /// Q_ij = -(Vi²/a²) · (b + bc/2) - (Vi·Vj/a) · [g·sin(θi-θj-θs) - b·cos(θi-θj-θs)]
    /// ```
    ///
    /// # Arguments
    ///
    /// * `branch` - Branch data (impedance, tap, limits)
    /// * `vi` - Voltage magnitude at from bus (p.u.)
    /// * `vj` - Voltage magnitude at to bus (p.u.)
    /// * `theta_i` - Voltage angle at from bus (radians)
    /// * `theta_j` - Voltage angle at to bus (radians)
    ///
    /// # Returns
    ///
    /// Tuple `(P_ij, Q_ij)` - real and reactive power flow in per-unit
    pub fn branch_flow_from(&self, branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64) {
        // Compute series admittance g + jb = 1/(r + jx)
        let z_sq = branch.r * branch.r + branch.x * branch.x;
        let g = branch.r / z_sq;
        let b = -branch.x / z_sq;

        // Tap ratio (default 1.0 for lines)
        let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };
        let a_sq = a * a;

        // Angle difference including phase shift
        let theta_diff = theta_i - theta_j - branch.shift;
        let cos_diff = theta_diff.cos();
        let sin_diff = theta_diff.sin();

        // Voltage products
        let vi_sq = vi * vi;
        let vi_vj = vi * vj;

        // Real power: P_ij = (Vi²/a²)·g - (Vi·Vj/a)·[g·cos + b·sin]
        let p_ij = (vi_sq / a_sq) * g - (vi_vj / a) * (g * cos_diff + b * sin_diff);

        // Reactive power: Q_ij = -(Vi²/a²)·(b + bc/2) - (Vi·Vj/a)·[g·sin - b·cos]
        let bc_half = branch.b_charging / 2.0;
        let q_ij = -(vi_sq / a_sq) * (b + bc_half) - (vi_vj / a) * (g * sin_diff - b * cos_diff);

        (p_ij, q_ij)
    }

    /// Compute power flow on a branch (to side) in per-unit.
    ///
    /// The "to" side power flow differs because the tap and phase shift
    /// are on the from side. The formulas become:
    /// ```text
    /// P_ji = Vj² · g - (Vi·Vj/a) · [g·cos(θj-θi+θs) + b·sin(θj-θi+θs)]
    /// Q_ji = -Vj² · (b + bc/2) - (Vi·Vj/a) · [g·sin(θj-θi+θs) - b·cos(θj-θi+θs)]
    /// ```
    ///
    /// # Returns
    ///
    /// Tuple `(P_ji, Q_ji)` - real and reactive power flow in per-unit
    pub fn branch_flow_to(&self, branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> (f64, f64) {
        // Compute series admittance
        let z_sq = branch.r * branch.r + branch.x * branch.x;
        let g = branch.r / z_sq;
        let b = -branch.x / z_sq;

        // Tap ratio
        let a = if branch.tap > 0.0 { branch.tap } else { 1.0 };

        // Angle difference (reversed direction, plus phase shift)
        let theta_diff = theta_j - theta_i + branch.shift;
        let cos_diff = theta_diff.cos();
        let sin_diff = theta_diff.sin();

        // Voltage products
        let vj_sq = vj * vj;
        let vi_vj = vi * vj;

        // Real power: P_ji = Vj²·g - (Vi·Vj/a)·[g·cos + b·sin]
        let p_ji = vj_sq * g - (vi_vj / a) * (g * cos_diff + b * sin_diff);

        // Reactive power: Q_ji = -Vj²·(b + bc/2) - (Vi·Vj/a)·[g·sin - b·cos]
        let bc_half = branch.b_charging / 2.0;
        let q_ji = -vj_sq * (b + bc_half) - (vi_vj / a) * (g * sin_diff - b * cos_diff);

        (p_ji, q_ji)
    }

    /// Compute squared apparent power flow on branch (from side) in per-unit².
    ///
    /// S²_ij = P²_ij + Q²_ij
    ///
    /// Used for thermal limit constraints: S²_ij ≤ S²_max
    pub fn branch_flow_sq_from(&self, branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> f64 {
        let (p, q) = self.branch_flow_from(branch, vi, vj, theta_i, theta_j);
        p * p + q * q
    }

    /// Compute squared apparent power flow on branch (to side) in per-unit².
    pub fn branch_flow_sq_to(&self, branch: &BranchData, vi: f64, vj: f64, theta_i: f64, theta_j: f64) -> f64 {
        let (p, q) = self.branch_flow_to(branch, vi, vj, theta_i, theta_j);
        p * p + q * q
    }

    /// Evaluate thermal limit inequality constraints.
    ///
    /// For each branch with a thermal limit (rate_mva > 0), we add constraints:
    /// - S²_ij - S²_max ≤ 0  (from side)
    /// - S²_ji - S²_max ≤ 0  (to side)
    ///
    /// # Arguments
    ///
    /// * `x` - Decision variable vector
    ///
    /// # Returns
    ///
    /// Vector of constraint values (should be ≤ 0 for feasibility).
    /// Length = 2 * number of branches with thermal limits.
    pub fn thermal_constraints(&self, x: &[f64]) -> Vec<f64> {
        let (v, theta) = self.extract_v_theta(x);
        let mut h = Vec::new();

        for branch in &self.branches {
            // Skip branches without thermal limits
            if branch.rate_mva <= 0.0 {
                continue;
            }

            let vi = v[branch.from_idx];
            let vj = v[branch.to_idx];
            let theta_i = theta[branch.from_idx];
            let theta_j = theta[branch.to_idx];

            // Thermal limit in per-unit squared
            let s_max_pu = branch.rate_mva / self.base_mva;
            let s_max_sq = s_max_pu * s_max_pu;

            // From side: S²_ij - S²_max ≤ 0
            let s_sq_from = self.branch_flow_sq_from(branch, vi, vj, theta_i, theta_j);
            h.push(s_sq_from - s_max_sq);

            // To side: S²_ji - S²_max ≤ 0
            let s_sq_to = self.branch_flow_sq_to(branch, vi, vj, theta_i, theta_j);
            h.push(s_sq_to - s_max_sq);
        }

        h
    }

    /// Count branches with thermal limits.
    ///
    /// Returns the number of branches that have rate_mva > 0, which determines
    /// how many inequality constraints we need (2 per branch: from and to sides).
    pub fn n_thermal_constrained_branches(&self) -> usize {
        self.branches.iter().filter(|b| b.rate_mva > 0.0).count()
    }

    /// Get indices of branches with thermal limits.
    pub fn thermal_constrained_branch_indices(&self) -> Vec<usize> {
        self.branches
            .iter()
            .enumerate()
            .filter(|(_, b)| b.rate_mva > 0.0)
            .map(|(i, _)| i)
            .collect()
    }

    /// Evaluate equality constraints (power balance equations).
    ///
    /// The constraints enforce:
    /// ```text
    /// g₁: P_gen - P_load - P_inj(V,θ) = 0  for each bus
    /// g₂: Q_gen - Q_load - Q_inj(V,θ) = 0  for each bus
    /// g₃: θ_ref = 0                         reference angle
    /// ```
    ///
    /// # Arguments
    ///
    /// * `x` - Decision variable vector
    ///
    /// # Returns
    ///
    /// Constraint violation vector of length 2*n_bus + 1.
    /// A feasible point has all entries near zero.
    pub fn equality_constraints(&self, x: &[f64]) -> Vec<f64> {
        // Extract voltages and angles
        let (v, theta) = self.extract_v_theta(x);

        // Compute power injections from AC power flow equations
        let (p_inj, q_inj) = PowerEquations::compute_injections(&self.ybus, &v, &theta);

        // Pre-allocate constraint vector
        // Layout: [P balance for each bus | Q balance for each bus | ref angle]
        let mut g = Vec::with_capacity(2 * self.n_bus + 1);

        // ====================================================================
        // SUM GENERATOR INJECTIONS AT EACH BUS
        // ====================================================================
        //
        // Multiple generators may connect to the same bus.
        // We sum their contributions before forming the balance equation.

        let mut pg_bus = vec![0.0; self.n_bus];
        let mut qg_bus = vec![0.0; self.n_bus];

        for (i, &bus_idx) in self.gen_bus_idx.iter().enumerate() {
            pg_bus[bus_idx] += x[self.pg_offset + i];
            qg_bus[bus_idx] += x[self.qg_offset + i];
        }

        // ====================================================================
        // REAL POWER BALANCE: P_inj - P_gen + P_load + P_shunt = 0
        // ====================================================================
        //
        // Rearranged from: P_gen = P_load + P_shunt + P_inj
        // At a feasible point, generation equals load, shunt consumption, plus network injection.
        // P_shunt = gs_pu * V² represents power consumed by shunt conductance (like a load).

        for (i, bus) in self.buses.iter().enumerate() {
            let p_load_pu = bus.p_load / self.base_mva;
            let v_sq = v[i] * v[i];
            let p_shunt_pu = bus.gs_pu * v_sq; // Power consumed by shunt conductance
            g.push(p_inj[i] - pg_bus[i] + p_load_pu + p_shunt_pu);
        }

        // ====================================================================
        // REACTIVE POWER BALANCE: Q_inj - Q_gen + Q_load - Q_shunt = 0
        // ====================================================================
        //
        // Q_shunt = bs_pu * V² is reactive power supplied by the shunt:
        // - bs_pu > 0 (capacitor): supplies VARs, reduces effective Q load
        // - bs_pu < 0 (reactor): absorbs VARs, increases effective Q load

        for (i, bus) in self.buses.iter().enumerate() {
            let q_load_pu = bus.q_load / self.base_mva;
            let v_sq = v[i] * v[i];
            let q_shunt_pu = bus.bs_pu * v_sq; // Reactive power supplied by shunt
            g.push(q_inj[i] - qg_bus[i] + q_load_pu - q_shunt_pu);
        }

        // ====================================================================
        // REFERENCE ANGLE: θ_ref = 0
        // ====================================================================
        //
        // One angle must be fixed because only angle differences affect power flow.
        // This breaks the rotational symmetry of the problem.

        g.push(x[self.theta_offset + self.ref_bus]);

        g
    }

    /// Get variable bounds for box constraints.
    ///
    /// # Returns
    ///
    /// Tuple `(lb, ub)` where:
    /// * `lb[i]` = lower bound for x[i]
    /// * `ub[i]` = upper bound for x[i]
    pub fn variable_bounds(&self) -> (Vec<f64>, Vec<f64>) {
        let mut lb = vec![f64::NEG_INFINITY; self.n_var];
        let mut ub = vec![f64::INFINITY; self.n_var];

        // ====================================================================
        // VOLTAGE BOUNDS
        // ====================================================================
        //
        // V_min ≤ V_i ≤ V_max
        //
        // Typical range: 0.9-1.1 p.u. (±10% of nominal)
        // - Too low: voltage collapse, motor stalling
        // - Too high: insulation breakdown, equipment damage

        for (i, bus) in self.buses.iter().enumerate() {
            lb[self.v_offset + i] = bus.v_min;
            ub[self.v_offset + i] = bus.v_max;
        }

        // ====================================================================
        // ANGLE BOUNDS
        // ====================================================================
        //
        // -π/2 ≤ θ_i ≤ π/2
        //
        // Practical limits for numerical stability. Real systems rarely
        // exceed ±30° angle differences between adjacent buses.
        // The ±90° limit prevents wrap-around issues with trig functions.

        for i in 0..self.n_bus {
            lb[self.theta_offset + i] = -std::f64::consts::FRAC_PI_2;
            ub[self.theta_offset + i] = std::f64::consts::FRAC_PI_2;
        }

        // ====================================================================
        // GENERATOR REAL POWER LIMITS
        // ====================================================================
        //
        // P_min ≤ P_g ≤ P_max
        //
        // P_min: Minimum stable generation (below this, must shut down)
        // P_max: Maximum capacity (turbine/boiler limit)

        for (i, gen) in self.generators.iter().enumerate() {
            lb[self.pg_offset + i] = gen.pmin_mw / self.base_mva;
            ub[self.pg_offset + i] = gen.pmax_mw / self.base_mva;
        }

        // ====================================================================
        // GENERATOR REACTIVE POWER LIMITS
        // ====================================================================
        //
        // Q_min ≤ Q_g ≤ Q_max
        //
        // Q_min: Under-excitation limit (leading PF capability)
        // Q_max: Over-excitation limit (field heating constraint)
        //
        // These limits form the generator capability curve (D-curve).

        for (i, gen) in self.generators.iter().enumerate() {
            lb[self.qg_offset + i] = gen.qmin_mvar / self.base_mva;
            ub[self.qg_offset + i] = gen.qmax_mvar / self.base_mva;
        }

        (lb, ub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piecewise_linear_cost_evaluation() {
        // Create a piecewise-linear cost curve:
        // 0 MW -> $0/hr
        // 50 MW -> $500/hr (slope = $10/MWh)
        // 100 MW -> $1500/hr (slope = $20/MWh - more expensive at higher output)
        let pwl_cost = CostModel::PiecewiseLinear(vec![
            (0.0, 0.0),      // P=0, cost=0
            (50.0, 500.0),   // P=50, cost=500
            (100.0, 1500.0), // P=100, cost=1500
        ]);

        // Test cost evaluation at breakpoints
        assert!((pwl_cost.evaluate(0.0) - 0.0).abs() < 1e-9);
        assert!((pwl_cost.evaluate(50.0) - 500.0).abs() < 1e-9);
        assert!((pwl_cost.evaluate(100.0) - 1500.0).abs() < 1e-9);

        // Test interpolation between breakpoints
        // At 25 MW: should be 250 (halfway between 0 and 500)
        assert!((pwl_cost.evaluate(25.0) - 250.0).abs() < 1e-9);

        // At 75 MW: should be 1000 (halfway between 500 and 1500)
        assert!((pwl_cost.evaluate(75.0) - 1000.0).abs() < 1e-9);

        // Test marginal cost (derivative)
        // First segment: slope = 500/50 = 10 $/MWh
        assert!((pwl_cost.marginal_cost(25.0) - 10.0).abs() < 1e-9);

        // Second segment: slope = 1000/50 = 20 $/MWh
        assert!((pwl_cost.marginal_cost(75.0) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_polynomial_vs_piecewise_cost_models() {
        // Create a GenData with piecewise-linear cost
        let pwl_gen = GenData {
            name: "PWL_Gen".to_string(),
            bus_id: BusId::new(1),
            pmin_mw: 0.0,
            pmax_mw: 100.0,
            qmin_mvar: -50.0,
            qmax_mvar: 50.0,
            cost_coeffs: vec![], // Ignored for PWL
            cost_model: CostModel::PiecewiseLinear(vec![
                (0.0, 100.0),    // $100/hr no-load cost
                (100.0, 1100.0), // $1100/hr at 100 MW (avg $10/MWh)
            ]),
            capability_curve: Vec::new(),
        };

        // Create a GenData with polynomial cost (linear: $100 + $10*P)
        let poly_gen = GenData {
            name: "Poly_Gen".to_string(),
            bus_id: BusId::new(1),
            pmin_mw: 0.0,
            pmax_mw: 100.0,
            qmin_mvar: -50.0,
            qmax_mvar: 50.0,
            cost_coeffs: vec![100.0, 10.0, 0.0],
            cost_model: CostModel::linear(100.0, 10.0),
            capability_curve: Vec::new(),
        };

        // For linear PWL, both should give the same cost at any P
        assert!(
            (pwl_gen.cost_model.evaluate(0.0) - poly_gen.cost_model.evaluate(0.0)).abs() < 1e-9
        );
        assert!(
            (pwl_gen.cost_model.evaluate(50.0) - poly_gen.cost_model.evaluate(50.0)).abs() < 1e-9
        );
        assert!(
            (pwl_gen.cost_model.evaluate(100.0) - poly_gen.cost_model.evaluate(100.0)).abs() < 1e-9
        );

        // Marginal costs should also match
        assert!(
            (pwl_gen.cost_model.marginal_cost(50.0) - poly_gen.cost_model.marginal_cost(50.0))
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn test_warm_start_from_solution() {
        use crate::opf::{OpfMethod, OpfSolution};
        use gat_core::{Branch, BranchId, Bus, Edge, Gen, GenId, Network, Node};

        // Create a minimal network to test warm-start
        let mut network = Network::new();

        // Add a bus
        let bus_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));

        // Add a generator
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen1".to_string(),
            bus: BusId::new(1),
            pmin_mw: 10.0,
            pmax_mw: 100.0,
            qmin_mvar: -50.0,
            qmax_mvar: 50.0,
            cost_model: CostModel::linear(0.0, 10.0),
            ..Gen::default()
        }));

        // Add a second bus and branch (needed for valid network)
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));

        network.graph.add_edge(
            bus_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Line1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                status: true,
                ..Branch::default()
            }),
        );

        // Build the problem
        let problem = AcOpfProblem::from_network(&network).unwrap();

        // Create a mock DC-OPF solution
        let mut dc_solution = OpfSolution {
            converged: true,
            method_used: OpfMethod::DcOpf,
            ..Default::default()
        };

        // Set values from "DC-OPF"
        dc_solution.bus_voltage_ang.insert("Bus1".to_string(), 5.0); // 5 degrees
        dc_solution.bus_voltage_ang.insert("Bus2".to_string(), 3.0); // 3 degrees
        dc_solution.generator_p.insert("Gen1".to_string(), 75.0); // 75 MW

        // Get warm-start vector
        let x0 = problem.warm_start_from_solution(&dc_solution);

        // Verify that values are extracted correctly
        // n_var = 2 * n_bus + 2 * n_gen = 2*2 + 2*1 = 6
        assert_eq!(x0.len(), 6);

        // Voltage magnitudes should be 1.0 (DC doesn't provide V)
        assert!((x0[problem.v_offset] - 1.0).abs() < 1e-9);
        assert!((x0[problem.v_offset + 1] - 1.0).abs() < 1e-9);

        // Angles should be converted from degrees to radians
        assert!((x0[problem.theta_offset] - 5.0_f64.to_radians()).abs() < 1e-9);
        assert!((x0[problem.theta_offset + 1] - 3.0_f64.to_radians()).abs() < 1e-9);

        // P should be 75 MW in per-unit (75/100 = 0.75)
        assert!((x0[problem.pg_offset] - 0.75).abs() < 1e-9);

        // Q should be midpoint since DC doesn't solve Q
        let gen = &problem.generators[0];
        let q_midpoint = (gen.qmin_mvar + gen.qmax_mvar) / 2.0 / problem.base_mva;
        assert!((x0[problem.qg_offset] - q_midpoint).abs() < 1e-9);
    }
}
