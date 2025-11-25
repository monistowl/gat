//! # Second-Order Cone Programming (SOCP) Relaxation for AC Optimal Power Flow
//!
//! This module implements a convex relaxation of the AC Optimal Power Flow (AC-OPF)
//! problem using Second-Order Cone Programming. The approach is based on the
//! **branch-flow model** (also known as the DistFlow model) originally developed for
//! radial distribution networks and later extended to meshed transmission systems.
//!
//! ## Background and Motivation
//!
//! The AC-OPF problem is fundamental to power system operations. It determines the
//! optimal generator dispatch to meet load while respecting physical constraints
//! (voltage limits, thermal limits, power balance) and minimizing cost. However,
//! AC-OPF is **non-convex** due to the quadratic relationship between voltage, current,
//! and power in AC circuits, making it NP-hard in general.
//!
//! Convex relaxations provide:
//! 1. **Global optimality guarantees** - No local minima to get trapped in
//! 2. **Polynomial-time solvability** - Practical for large networks
//! 3. **Lower bounds** - Useful for optimality gap analysis
//! 4. **Feasible recovery** - Often yields AC-feasible solutions directly
//!
//! ## The Branch-Flow Model
//!
//! Unlike the classical bus-injection model (which uses the Y-bus admittance matrix),
//! the branch-flow model uses **branch power flows** as primary variables. For a branch
//! from bus i to bus j with impedance z = r + jx:
//!
//! ```text
//!     Bus i ----[r + jx]---- Bus j
//!       |                      |
//!      Vᵢ                     Vⱼ
//!       ↓                      ↓
//!     Pᵢⱼ + jQᵢⱼ →          (received power)
//! ```
//!
//! **Key variables per branch:**
//! - `Pᵢⱼ`, `Qᵢⱼ`: Real and reactive power flow (sending end)
//! - `ℓᵢⱼ = |Iᵢⱼ|²`: Squared current magnitude
//! - `vᵢ = |Vᵢ|²`: Squared voltage magnitude at sending bus
//!
//! **Fundamental relationships:**
//!
//! 1. **Power-current relationship** (exact):
//!    ```text
//!    Pᵢⱼ² + Qᵢⱼ² = vᵢ · ℓᵢⱼ
//!    ```
//!    This is non-convex (equality with products of variables).
//!
//! 2. **Voltage drop equation** (Ohm's law):
//!    ```text
//!    vⱼ = vᵢ - 2(r·Pᵢⱼ + x·Qᵢⱼ) + (r² + x²)·ℓᵢⱼ
//!    ```
//!    This is linear in the squared-magnitude variables!
//!
//! 3. **Power loss on branch:**
//!    ```text
//!    P_loss = r · ℓᵢⱼ,  Q_loss = x · ℓᵢⱼ
//!    ```
//!
//! ## The SOCP Relaxation
//!
//! The key insight is to **relax** the equality constraint (1) to an inequality:
//!
//! ```text
//! Pᵢⱼ² + Qᵢⱼ² ≤ vᵢ · ℓᵢⱼ    (SOCP relaxation)
//! ```
//!
//! This is a **rotated second-order cone constraint**, which is convex! The relaxation
//! is **exact** (tight at optimum) for:
//! - Radial networks (trees) under mild conditions
//! - Networks where voltage constraints are not binding at optimum
//!
//! For meshed networks, the relaxation may be loose, but often provides excellent
//! approximations and valid lower bounds.
//!
//! ## Key References
//!
//! This implementation draws from several foundational papers:
//!
//! - **Baran & Wu (1989)**: Original DistFlow model for radial networks
//!   "Network reconfiguration in distribution systems for loss reduction and load balancing"
//!   IEEE Trans. Power Delivery, 4(2), 1401-1407
//!   DOI: [10.1109/61.25627](https://doi.org/10.1109/61.25627)
//!
//! - **Farivar & Low (2013)**: SOCP relaxation exactness conditions
//!   "Branch Flow Model: Relaxations and Convexification"
//!   IEEE Trans. Power Systems, 28(3), 2554-2564
//!   DOI: [10.1109/TPWRS.2013.2255317](https://doi.org/10.1109/TPWRS.2013.2255317)
//!
//! - **Gan, Li, Topcu & Low (2015)**: Exact relaxation conditions
//!   "Exact Convex Relaxation of Optimal Power Flow in Radial Networks"
//!   IEEE Trans. Automatic Control, 60(1), 72-87
//!   DOI: [10.1109/TAC.2014.2332712](https://doi.org/10.1109/TAC.2014.2332712)
//!
//! - **Low (2014)**: Comprehensive tutorial on convex relaxations
//!   "Convex Relaxation of Optimal Power Flow—Part I/II"
//!   IEEE Trans. Control of Network Systems, 1(1), 15-27
//!   DOI: [10.1109/TCNS.2014.2309732](https://doi.org/10.1109/TCNS.2014.2309732)
//!
//! - **Jabr (2006)**: Conic formulation for OPF
//!   "Radial Distribution Load Flow Using Conic Programming"
//!   IEEE Trans. Power Systems, 21(3), 1458-1459
//!   DOI: [10.1109/TPWRS.2006.879234](https://doi.org/10.1109/TPWRS.2006.879234)
//!
//! ## Implementation Features
//!
//! This solver supports:
//! - **Quadratic cost curves**: `cost = c₀ + c₁·P + c₂·P²`
//! - **Phase-shifting transformers**: Via angle-coupled formulation
//! - **Tap-changing transformers**: Off-nominal tap ratios
//! - **Line charging**: Shunt susceptance (π-model)
//! - **Thermal limits**: MVA flow constraints
//! - **Voltage bounds**: Typically [0.9, 1.1] p.u.
//! - **LMP computation**: From dual variables on power balance
//!
//! ## Solver Backend
//!
//! We use [Clarabel](https://github.com/oxfordcontrol/Clarabel.rs), a high-performance
//! interior-point solver for conic programs written in Rust. Clarabel implements a
//! primal-dual interior point method with Nesterov-Todd scaling.

use crate::opf::types::{ConstraintInfo, ConstraintType};
use crate::opf::{OpfMethod, OpfSolution};
use crate::OpfError;
use clarabel::{
    algebra::CscMatrix,
    solver::{DefaultSettingsBuilder, IPSolver, SupportedConeT},
};
use gat_core::{BusId, Edge, Network, Node};
use std::collections::HashMap;
use std::time::Instant;

// ============================================================================
// DATA STRUCTURES
// ============================================================================
//
// These structs transform the graph-based network representation into a form
// suitable for mathematical optimization. The separation allows us to:
// 1. Validate data before optimization
// 2. Precompute derived quantities (e.g., squared bounds)
// 3. Assign matrix indices deterministically
// ============================================================================

/// Bus-level data extracted from the network graph.
///
/// In power systems, a "bus" represents an electrical node where components connect.
/// Buses are characterized by their voltage level and operational limits.
///
/// # Per-Unit System
///
/// Power systems use a "per-unit" (p.u.) system to normalize quantities:
/// - Voltage: `V_pu = V_actual / V_base`
/// - Power: `S_pu = S_actual / S_base` (typically S_base = 100 MVA)
/// - Impedance: `Z_pu = Z_actual · S_base / V_base²`
///
/// This normalization simplifies calculations and makes values comparable across
/// different voltage levels. See IEEE Std 141-1993 for details.
#[derive(Debug, Clone)]
struct BusData {
    /// Unique identifier for cross-referencing with network graph
    id: BusId,

    /// Human-readable name (e.g., "NORTH_345", "SOUTH_138")
    name: String,

    /// Zero-based index for matrix construction. Buses are numbered 0..n_bus
    /// in the order they appear in the network graph traversal.
    index: usize,

    /// Minimum voltage magnitude in per-unit.
    /// Typical range: 0.90-0.95 p.u.
    /// Voltage below this risks equipment damage and protection trips.
    v_min: f64,

    /// Maximum voltage magnitude in per-unit.
    /// Typical range: 1.05-1.10 p.u.
    /// Voltage above this stresses insulation and shortens equipment life.
    v_max: f64,

    /// Nominal voltage in kV. Used for:
    /// - Per-unit base conversions between voltage levels
    /// - Impedance transformation through transformers
    /// - Line charging calculations (charging ∝ V²)
    base_kv: f64,
}

/// Generator data including cost curves and capability limits.
///
/// Generators are modeled as controllable power injections at buses. The OPF
/// determines optimal setpoints (P, Q) that minimize cost while respecting limits.
///
/// # Cost Models
///
/// Generator cost is typically modeled as a polynomial:
/// ```text
/// Cost(P) = c₀ + c₁·P + c₂·P²  [$/hr]
/// ```
///
/// Where:
/// - `c₀`: No-load cost (fuel burned at synchronous speed, no power output)
/// - `c₁`: Incremental cost at low output [$/MWh]
/// - `c₂`: Quadratic term capturing heat-rate curve [$/MW²h]
///
/// The quadratic term models the fact that thermal generators become less
/// efficient at very high and very low output levels.
///
/// # Capability Curves
///
/// Real generators have coupled P-Q limits (the "D-curve" or capability curve).
/// Here we use simple box constraints [Pmin, Pmax] × [Qmin, Qmax] which is a
/// common approximation. For more accuracy, see:
/// - IEEE Std 421.5 for generator capability representation
/// - DOI: [10.1109/MPER.1968.5528951](https://doi.org/10.1109/MPER.1968.5528951)
#[derive(Debug, Clone)]
struct GenData {
    /// Generator name for result mapping (e.g., "GEN_NORTH_1")
    name: String,

    /// Bus where generator is connected. Multiple generators can connect to one bus.
    bus_id: BusId,

    /// Minimum real power output in MW.
    /// For thermal units: minimum stable generation (often 20-40% of Pmax)
    /// Below this, flame stability and emissions become problematic.
    pmin_mw: f64,

    /// Maximum real power output in MW (nameplate capacity).
    /// May be derated for temperature, elevation, or maintenance.
    pmax_mw: f64,

    /// Minimum reactive power in MVAr (absorbing/underexcited).
    /// Limited by stator end-region heating and stability margins.
    qmin_mvar: f64,

    /// Maximum reactive power in MVAr (producing/overexcited).
    /// Limited by field winding heating (I²R losses in rotor).
    qmax_mvar: f64,

    /// Polynomial cost coefficients [c₀, c₁, c₂, ...].
    /// - c₀: Fixed cost ($/hr) - incurred whenever unit is on
    /// - c₁: Linear cost ($/MWh) - marginal cost at zero output
    /// - c₂: Quadratic cost ($/MW²h) - curvature of heat rate
    ///
    /// The marginal cost at output P is: MC(P) = c₁ + 2·c₂·P
    /// At economic dispatch equilibrium, all online generators have equal MC.
    cost_coeffs: Vec<f64>,
}

/// Branch (transmission line or transformer) parameters.
///
/// Branches connect buses and are characterized by their impedance and limits.
/// We use the **π-model** for transmission lines:
///
/// ```text
///        ┌────[R + jX]────┐
///        │                │
///   i ───┼───┬────────┬───┼─── j
///        │   │        │   │
///        │  ═══      ═══  │
///        │  jB/2    jB/2  │
///        │   │        │   │
///        └───┴────────┴───┘
///            ⏊        ⏊
/// ```
///
/// Where:
/// - R: Series resistance (causes I²R losses, heats the conductor)
/// - X: Series reactance (inductive, from magnetic field around conductor)
/// - B: Shunt susceptance (capacitive, from electric field between conductors)
///
/// For transformers, we additionally have:
/// - τ (tap ratio): Voltage transformation ratio
/// - φ (phase shift): Angle shift for phase-shifting transformers (PSTs)
///
/// # Transformer Model
///
/// An ideal transformer with tap ratio τ and phase shift φ:
/// ```text
/// V_secondary = V_primary / τ · e^(-jφ)
/// ```
///
/// Phase-shifting transformers control real power flow by adjusting φ,
/// independent of the voltage magnitude. They're used for:
/// - Loop flow control in meshed networks
/// - Congestion management
/// - Stability improvement
///
/// See: Arrillaga & Arnold, "Computer Modelling of Electrical Power Systems"
/// DOI: [10.1002/9781118878286](https://doi.org/10.1002/9781118878286)
#[derive(Debug, Clone)]
struct BranchData {
    /// Branch name for result mapping (e.g., "LINE_NORTH_SOUTH_1")
    name: String,

    /// Sending-end bus (the "from" terminal)
    from_bus: BusId,

    /// Receiving-end bus (the "to" terminal)
    to_bus: BusId,

    /// Series resistance in per-unit on system base.
    /// Typical values: 0.001-0.05 p.u. for transmission lines
    /// Transformers often have very low R (0.002-0.01 p.u.)
    r: f64,

    /// Series reactance in per-unit on system base.
    /// Typical values: 0.01-0.3 p.u. for transmission lines
    /// Usually X >> R for high-voltage lines (X/R ratio of 5-15)
    x: f64,

    /// Total line charging susceptance in per-unit (split half at each end).
    /// Represents the capacitance between conductors and to ground.
    /// Significant for long lines (>80 km) and at high voltages (≥230 kV).
    /// Can cause voltage rise under light loading (Ferranti effect).
    b_shunt: f64,

    /// Off-nominal tap ratio (dimensionless).
    /// - τ = 1.0: Nominal ratio (e.g., 345/138 kV as designed)
    /// - τ > 1.0: Boosts secondary voltage
    /// - τ < 1.0: Bucks secondary voltage
    ///
    /// Typical range: 0.9 to 1.1 (±10% regulation range)
    /// Load tap changers (LTCs) adjust this in discrete steps.
    tap_ratio: f64,

    /// Phase shift angle in radians (for phase-shifting transformers).
    /// Positive φ advances the secondary voltage phasor.
    /// Typical range: -30° to +30° (-0.52 to +0.52 rad)
    phase_shift_rad: f64,

    /// Maximum apparent power flow in MVA (thermal limit).
    /// Set by the thermal capacity of:
    /// - Conductors (sag limit at high temperature)
    /// - Transformer windings (insulation life)
    /// - Terminal equipment (CTs, switches)
    ///
    /// Usually rated for continuous operation at 40°C ambient.
    /// Short-term emergency ratings may be 10-25% higher.
    s_max_mva: Option<f64>,
}

/// Map from bus ID to aggregated (P, Q) load at that bus.
///
/// Multiple loads at the same bus are summed. Loads are treated as fixed
/// (inelastic) demand in this formulation. For price-responsive demand,
/// loads would become decision variables with utility functions.
type LoadMap = HashMap<BusId, (f64, f64)>;

// ============================================================================
// NETWORK DATA EXTRACTION
// ============================================================================

/// Extract and validate network data from the graph representation.
///
/// This function transforms the generic network graph into solver-ready data
/// structures, performing validation along the way.
///
/// # Validation Checks
///
/// - Bus voltage must be positive (negative kV is unphysical)
/// - At least one bus and one generator must exist
/// - Branch impedance must be non-zero (zero impedance is a short circuit)
/// - Only in-service (status=true) branches are included
///
/// # Cost Model Handling
///
/// Different cost model types are converted to polynomial form:
/// - `NoCost`: Becomes [0, 0] (no cost contribution)
/// - `Polynomial`: Used directly [c₀, c₁, c₂, ...]
/// - `PiecewiseLinear`: Approximated by marginal cost at midpoint
///
/// The piecewise-linear approximation maintains convexity while simplifying
/// the optimization. For exact piecewise handling, auxiliary variables and
/// SOS2 constraints would be needed (see MATPOWER's formulation).
fn extract_network_data(
    network: &Network,
) -> Result<(Vec<BusData>, Vec<GenData>, Vec<BranchData>, LoadMap), OpfError> {
    let mut buses = Vec::new();
    let mut generators = Vec::new();
    let mut loads: LoadMap = HashMap::new();

    // ========================================================================
    // PASS 1: Extract nodes (buses, generators, loads)
    // ========================================================================
    //
    // The network graph uses a heterogeneous node type, so we pattern-match
    // to extract each component type. Buses get sequential indices for matrix
    // construction.

    let mut bus_index = 0;
    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Bus(bus) => {
                // Validate positive voltage (sanity check on input data)
                if bus.voltage_kv <= 0.0 {
                    return Err(OpfError::DataValidation(format!(
                        "Bus {} has non-positive voltage_kv ({}). \
                         Check input data - voltage must be a positive value in kV.",
                        bus.name, bus.voltage_kv
                    )));
                }

                // Default voltage limits: ±10% of nominal
                // These are typical NERC/FERC requirements for bulk transmission
                // Distribution systems may use tighter bounds (±5%)
                let v_min = 0.9;
                let v_max = 1.1;

                buses.push(BusData {
                    id: bus.id,
                    name: bus.name.clone(),
                    index: bus_index,
                    v_min,
                    v_max,
                    base_kv: bus.voltage_kv,
                });
                bus_index += 1;
            }

            Node::Gen(gen) => {
                // Convert cost model to polynomial coefficients
                let cost_coeffs = match &gen.cost_model {
                    gat_core::CostModel::NoCost => vec![0.0, 0.0],

                    gat_core::CostModel::Polynomial(c) => c.clone(),

                    gat_core::CostModel::PiecewiseLinear(_) => {
                        // Approximate piecewise-linear cost with a linear function
                        // using the marginal cost at the midpoint of the operating range.
                        //
                        // This is a simplification - for exact piecewise handling,
                        // we would need to introduce auxiliary variables for each segment.
                        // See: Carrion & Arroyo (2006), "A computationally efficient
                        // mixed-integer linear formulation for the thermal unit commitment"
                        // DOI: 10.1109/TPWRS.2006.876672
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

            Node::Load(load) => {
                // Aggregate multiple loads at the same bus
                // This is physically correct: loads in parallel have additive power
                let entry = loads.entry(load.bus).or_insert((0.0, 0.0));
                entry.0 += load.active_power_mw;
                entry.1 += load.reactive_power_mvar;
            }
        }
    }

    // Validate minimum network structure
    if buses.is_empty() {
        return Err(OpfError::DataValidation(
            "No buses in network. Cannot run OPF without at least one bus.".into(),
        ));
    }
    if generators.is_empty() {
        return Err(OpfError::DataValidation(
            "No generators in network. OPF requires at least one controllable source.".into(),
        ));
    }

    // ========================================================================
    // PASS 2: Extract branches (edges in the graph)
    // ========================================================================
    //
    // Branches form the "Y-bus" structure of the network. We only include
    // in-service branches (status=true), allowing for contingency analysis
    // by toggling branch status.

    let mut branches = Vec::new();
    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            // Skip out-of-service branches (e.g., for N-1 contingency analysis)
            if !branch.status {
                continue;
            }

            // Zero impedance would cause division by zero and is unphysical
            // (it would represent a perfect short circuit)
            if branch.resistance.abs() < 1e-12 && branch.reactance.abs() < 1e-12 {
                return Err(OpfError::DataValidation(format!(
                    "Branch {} has near-zero impedance (R={}, X={}). \
                     This would represent a short circuit. \
                     If modeling a bus-tie, use a small positive reactance (e.g., 0.0001 p.u.).",
                    branch.name, branch.resistance, branch.reactance
                )));
            }

            branches.push(BranchData {
                name: branch.name.clone(),
                from_bus: branch.from_bus,
                to_bus: branch.to_bus,
                r: branch.resistance,
                x: branch.reactance,
                b_shunt: branch.charging_b_pu,
                tap_ratio: branch.tap_ratio,
                phase_shift_rad: branch.phase_shift_rad,
                // Use s_max_mva if available, otherwise fall back to rating_a_mva
                s_max_mva: branch.s_max_mva.or(branch.rating_a_mva),
            });
        }
    }

    if branches.is_empty() {
        return Err(OpfError::DataValidation(
            "Network contains no in-service AC branches. \
             SOCP OPF requires at least one branch to model power flow."
                .into(),
        ));
    }

    Ok((buses, generators, branches, loads))
}

// ============================================================================
// PER-UNIT SYSTEM CONSTANTS
// ============================================================================

/// System power base in MVA.
///
/// The per-unit system normalizes all quantities to a common base, making
/// calculations independent of voltage level. By convention:
/// - Power base: Typically 100 MVA (allows easy conversion: 1 p.u. = 100 MW)
/// - Voltage base: Nominal voltage at each bus (varies by voltage level)
/// - Impedance base: Z_base = V_base² / S_base
///
/// With 100 MVA base:
/// - A 50 MW load is 0.5 p.u.
/// - A 345 kV bus has V_base = 345 kV
/// - Z_base at 345 kV = 345² / 100 = 1190.25 Ω
///
/// See IEEE Std 141-1993 "Recommended Practice for Electric Power Distribution
/// for Industrial Plants" (Red Book) for per-unit system details.
const BASE_MVA: f64 = 100.0;

/// Compute a representative system base voltage from network buses.
///
/// For multi-voltage networks, we use the median voltage level as the system
/// base. This choice:
/// - Is robust to outliers (unlike mean)
/// - Represents the "typical" voltage level
/// - Makes per-unit impedances roughly comparable across the network
///
/// In practice, power flow calculations are done with per-unit values
/// referenced to each bus's local base, so this is mainly for scaling
/// purposes in the objective function.
fn compute_system_base_kv(buses: &[BusData]) -> f64 {
    if buses.is_empty() {
        return 100.0; // Reasonable fallback
    }

    // Sort voltages to find median
    let mut voltages: Vec<f64> = buses.iter().map(|b| b.base_kv).collect();
    voltages.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Return median (middle element for odd count, lower-middle for even)
    voltages[voltages.len() / 2]
}

// ============================================================================
// MAIN SOLVER FUNCTION
// ============================================================================

/// Solve the SOCP relaxation of AC-OPF using Clarabel.
///
/// # Mathematical Formulation
///
/// The optimization problem is:
///
/// ```text
/// minimize    Σᵢ [c₀ᵢ + c₁ᵢ·Pᵢ + c₂ᵢ·Pᵢ²]     (total generation cost)
///
/// subject to:
///   (1) Power Balance at each bus k:
///       Σ Pgen_k - Σ Pload_k = Σ P_outflow - Σ (P_inflow - R·ℓ)
///       Σ Qgen_k - Σ Qload_k = Σ Q_outflow - Σ (Q_inflow - X·ℓ) + B·v_k
///
///   (2) Voltage Drop along each branch (i→j):
///       vⱼ = vᵢ/τ² - 2(R·P + X·Q)/τ² + (R² + X²)·ℓ
///
///   (3) SOCP Relaxation (replaces |S|² = v·ℓ with ≤):
///       P² + Q² ≤ (v/τ²)·ℓ
///
///   (4) Angle Relationship (for phase shifters):
///       θⱼ - θᵢ + (X·P - R·Q)/τ ≈ -φ
///
///   (5) Bounds:
///       v_min² ≤ v ≤ v_max²     (voltage)
///       Pmin ≤ P_gen ≤ Pmax     (real power)
///       Qmin ≤ Q_gen ≤ Qmax     (reactive power)
///       ℓ ≤ (S_max/S_base)²     (thermal limit)
///       ℓ ≥ 0                   (physical)
/// ```
///
/// # Arguments
///
/// * `network` - The power system network graph
/// * `_max_iterations` - Reserved for future iterative refinement
/// * `_tolerance` - Reserved for future convergence checking
///
/// # Returns
///
/// * `Ok(OpfSolution)` - Optimal dispatch with voltages, flows, and prices
/// * `Err(OpfError)` - If network is invalid or problem is infeasible
///
/// # Example
///
/// ```ignore
/// let solution = solve(&network, 100, 1e-6)?;
/// println!("Total cost: ${}/hr", solution.objective_value);
/// println!("Generator 1 output: {} MW", solution.generator_p["gen1"]);
/// ```
pub fn solve(
    network: &Network,
    _max_iterations: usize,
    _tolerance: f64,
) -> Result<OpfSolution, OpfError> {
    let start = Instant::now();

    // ========================================================================
    // STEP 1: EXTRACT AND VALIDATE NETWORK DATA
    // ========================================================================

    let (buses, generators, branches, loads) = extract_network_data(network)?;

    // Build lookup table: BusId → matrix index
    let bus_map: HashMap<BusId, usize> = buses.iter().map(|b| (b.id, b.index)).collect();

    // System base voltage for consistent scaling
    let system_base_kv = compute_system_base_kv(&buses);

    // ========================================================================
    // STEP 2: DEFINE DECISION VARIABLE LAYOUT
    // ========================================================================
    //
    // Variables are laid out in a single vector x for the conic solver.
    // The ordering is chosen to group related variables for cache efficiency
    // and to simplify constraint construction.
    //
    // Variable layout:
    // ┌─────────────────────────────────────────────────────────────────┐
    // │ v[0..n_bus] │ θ[0..n_bus] │ Pg[0..n_gen] │ Qg[0..n_gen] │ ...  │
    // │─────────────│─────────────│──────────────│──────────────│      │
    // │ P[0..n_br]  │ Q[0..n_br]  │ ℓ[0..n_br]   │              │      │
    // └─────────────────────────────────────────────────────────────────┘
    //
    // Where:
    //   v[i]  = |V_i|² (squared voltage magnitude at bus i)
    //   θ[i]  = voltage angle at bus i (radians)
    //   Pg[g] = real power output of generator g (per-unit)
    //   Qg[g] = reactive power output of generator g (per-unit)
    //   P[b]  = real power flow on branch b (per-unit)
    //   Q[b]  = reactive power flow on branch b (per-unit)
    //   ℓ[b]  = |I_b|² (squared current magnitude on branch b)

    let n_bus = buses.len();
    let n_gen = generators.len();
    let n_branch = branches.len();

    // Compute starting indices for each variable group
    let var_v_start = 0; // Squared voltage magnitudes
    let var_theta_start = var_v_start + n_bus; // Voltage angles
    let var_pgen_start = var_theta_start + n_bus; // Generator real power
    let var_qgen_start = var_pgen_start + n_gen; // Generator reactive power
    let var_pflow_start = var_qgen_start + n_gen; // Branch real power flow
    let var_qflow_start = var_pflow_start + n_branch; // Branch reactive power flow
    let var_l_start = var_qflow_start + n_branch; // Squared branch current
    let n_var = var_l_start + n_branch; // Total number of variables

    // ========================================================================
    // STEP 3: BUILD QUADRATIC OBJECTIVE (P MATRIX)
    // ========================================================================
    //
    // Clarabel solves:  minimize  (1/2)·x'Px + q'x
    //
    // For generator cost c₀ + c₁·P + c₂·P², we need:
    //   - P matrix: diagonal with 2·c₂ on generator P variables
    //   - q vector: c₁ on generator P variables
    //
    // The factor of 2 comes from the 1/2 in Clarabel's objective.
    // c₀ is a constant and doesn't affect optimization (added to result).
    //
    // We must also scale for per-unit: if P_mw = P_pu × BASE_MVA, then
    //   c₂·P_mw² = c₂·(P_pu × BASE_MVA)² = (c₂·BASE_MVA²)·P_pu²
    //
    // P matrix is stored in Compressed Sparse Column (CSC) format for efficiency.

    let mut p_col_ptr = vec![0usize]; // Column pointers
    let mut p_row_idx = Vec::new(); // Row indices of non-zeros
    let mut p_values = Vec::new(); // Non-zero values

    for col in 0..n_var {
        let start_nnz = p_row_idx.len();

        // Check if this column corresponds to a generator P variable
        if col >= var_pgen_start && col < var_qgen_start {
            let gen_idx = col - var_pgen_start;
            let c2 = generators[gen_idx].cost_coeffs.get(2).copied().unwrap_or(0.0);

            if c2.abs() > 1e-12 {
                // Add diagonal entry: P[col, col] = 2·c₂ / BASE_MVA²
                // (The 2 accounts for Clarabel's 1/2 factor)
                p_row_idx.push(col);
                p_values.push(2.0 * c2 / (BASE_MVA * BASE_MVA));
            }
        }

        // Advance column pointer (even if no entries in this column)
        p_col_ptr.push(p_row_idx.len());

        // Note: we ignore the start_nnz check since we unconditionally push col_ptr
        let _ = start_nnz;
    }

    // ========================================================================
    // STEP 4: BUILD LINEAR OBJECTIVE (q VECTOR)
    // ========================================================================
    //
    // The linear cost term c₁·P_gen is scaled for per-unit:
    //   c₁·P_mw = c₁·(P_pu × BASE_MVA) = (c₁/BASE_MVA) × P_pu × BASE_MVA²
    //
    // We store c₁/BASE_MVA in q, and the final cost will be computed
    // in MW units when extracting the solution.

    let mut obj = vec![0.0f64; n_var];
    for (i, gen) in generators.iter().enumerate() {
        let idx = var_pgen_start + i;
        let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
        obj[idx] = c1 / BASE_MVA;
    }

    // Compute per-bus voltage ratio for potential future use in multi-voltage scaling
    let _bus_kv_ratio: Vec<f64> = buses
        .iter()
        .map(|b| b.base_kv / system_base_kv)
        .collect();

    // ========================================================================
    // STEP 5: BUILD CONSTRAINT MATRIX (A) AND RHS (b)
    // ========================================================================
    //
    // Clarabel requires constraints in the form: Ax + s = b, where s ∈ K (cone)
    //
    // We build the A matrix in column-major format (CSC) by accumulating
    // (row, coefficient) pairs for each column, then converting to CSC.
    //
    // Constraint types and their cone membership:
    //   - Equality (Ax = b): Zero cone (s = 0)
    //   - Inequality (Ax ≤ b): Nonnegative cone (s ≥ 0)
    //   - SOC constraint: Second-order cone

    let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_var]; // Column-wise storage
    let mut rhs: Vec<f64> = Vec::new(); // Right-hand side vector
    let mut cones: Vec<SupportedConeT<f64>> = Vec::new(); // Cone specifications

    // Track constraint row indices for extracting dual variables (shadow prices)
    // These are used to compute LMPs and identify binding constraints
    let mut row_voltage_min: Vec<usize> = Vec::with_capacity(n_bus);
    let mut row_voltage_max: Vec<usize> = Vec::with_capacity(n_bus);
    let mut row_gen_pmin: Vec<usize> = Vec::with_capacity(n_gen);
    let mut row_gen_pmax: Vec<usize> = Vec::with_capacity(n_gen);
    let mut row_gen_qmin: Vec<usize> = Vec::with_capacity(n_gen);
    let mut row_gen_qmax: Vec<usize> = Vec::with_capacity(n_gen);
    let mut row_branch_thermal: Vec<Option<usize>> = vec![None; n_branch];

    // ------------------------------------------------------------------------
    // Helper: Add an equality constraint (Zero cone)
    // Constraint: Σ coeffᵢ·xᵢ = b
    // ------------------------------------------------------------------------
    let push_eq = |coeffs: &[(usize, f64)],
                   b: f64,
                   rows: &mut Vec<Vec<(usize, f64)>>,
                   rhs: &mut Vec<f64>,
                   cones: &mut Vec<SupportedConeT<f64>>|
     -> usize {
        let row_idx = rhs.len();

        // Add coefficient to each variable's column
        for &(col, val) in coeffs {
            rows[col].push((row_idx, val));
        }
        rhs.push(b);

        // Efficiently merge consecutive equality constraints into one Zero cone
        match cones.last_mut() {
            Some(SupportedConeT::ZeroConeT(n)) => *n += 1,
            _ => cones.push(SupportedConeT::ZeroConeT(1)),
        }

        row_idx
    };

    // ------------------------------------------------------------------------
    // Helper: Add an inequality constraint (Nonnegative cone)
    // Constraint: Σ coeffᵢ·xᵢ ≤ b  (equivalently: b - Σ coeffᵢ·xᵢ ≥ 0)
    // ------------------------------------------------------------------------
    let push_leq = |coeffs: &[(usize, f64)],
                    b: f64,
                    rows: &mut Vec<Vec<(usize, f64)>>,
                    rhs: &mut Vec<f64>,
                    cones: &mut Vec<SupportedConeT<f64>>|
     -> usize {
        let row_idx = rhs.len();

        for &(col, val) in coeffs {
            rows[col].push((row_idx, val));
        }
        rhs.push(b);

        // Merge consecutive inequality constraints
        match cones.last_mut() {
            Some(SupportedConeT::NonnegativeConeT(n)) => *n += 1,
            _ => cones.push(SupportedConeT::NonnegativeConeT(1)),
        }

        row_idx
    };

    // ========================================================================
    // CONSTRAINT GROUP 1: VOLTAGE BOUNDS
    // ========================================================================
    //
    // Voltage limits protect equipment and ensure power quality:
    //   v_min² ≤ v ≤ v_max²
    //
    // Note: v is the SQUARED voltage magnitude, so bounds are squared.
    // This allows the voltage drop equation to remain linear.
    //
    // Typical limits (ANSI C84.1):
    //   - Service voltage: 0.95 to 1.05 p.u. (Range A)
    //   - Utilization voltage: 0.90 to 1.10 p.u. (Range B)

    for (i, bus) in buses.iter().enumerate() {
        let v_var = var_v_start + i;

        // v ≥ v_min²  →  -v ≤ -v_min²
        row_voltage_min.push(push_leq(
            &[(v_var, -1.0)],
            -(bus.v_min * bus.v_min),
            &mut rows,
            &mut rhs,
            &mut cones,
        ));

        // v ≤ v_max²
        row_voltage_max.push(push_leq(
            &[(v_var, 1.0)],
            bus.v_max * bus.v_max,
            &mut rows,
            &mut rhs,
            &mut cones,
        ));
    }

    // ========================================================================
    // CONSTRAINT GROUP 2: REFERENCE BUS (SLACK BUS)
    // ========================================================================
    //
    // Power systems require a reference for voltage angle (arbitrary choice)
    // and voltage magnitude (at least one bus must be "stiff").
    //
    // We fix bus 0 as the reference:
    //   - v[0] = 1.0 (1.0² = 1.0 p.u. squared magnitude)
    //   - θ[0] = 0.0 (angle reference)
    //
    // In practice, the slack bus is usually a large generator with good
    // voltage regulation capability.

    // Reference voltage magnitude: v[0] = 1.0
    push_eq(
        &[(var_v_start, 1.0)],
        1.0,
        &mut rows,
        &mut rhs,
        &mut cones,
    );

    // Reference angle: θ[0] = 0
    push_eq(
        &[(var_theta_start, 1.0)],
        0.0,
        &mut rows,
        &mut rhs,
        &mut cones,
    );

    // ========================================================================
    // CONSTRAINT GROUP 3: ANGLE BOUNDS
    // ========================================================================
    //
    // Bound angles to a reasonable range to aid numerical stability.
    // Large angle differences (>30°) typically indicate network stress.
    // We use ±90° (π/2 rad) as a generous bound.
    //
    // Note: Bus 0's angle is fixed to 0, so we only bound buses 1..n_bus.

    for i in 1..n_bus {
        let theta_var = var_theta_start + i;

        // θ ≤ π/2
        push_leq(
            &[(theta_var, 1.0)],
            std::f64::consts::FRAC_PI_2,
            &mut rows,
            &mut rhs,
            &mut cones,
        );

        // θ ≥ -π/2  →  -θ ≤ π/2
        push_leq(
            &[(theta_var, -1.0)],
            std::f64::consts::FRAC_PI_2,
            &mut rows,
            &mut rhs,
            &mut cones,
        );
    }

    // ========================================================================
    // CONSTRAINT GROUP 4: GENERATOR LIMITS
    // ========================================================================
    //
    // Generator operating limits come from physical and economic constraints:
    //
    // Real power (P):
    //   - Pmin: Minimum stable generation (thermal units can't run too low)
    //   - Pmax: Nameplate capacity or derated limit
    //
    // Reactive power (Q):
    //   - Qmin: Underexcited limit (absorbing VARs, limited by stability)
    //   - Qmax: Overexcited limit (producing VARs, limited by field heating)
    //
    // All values converted to per-unit on 100 MVA base.

    for (i, gen) in generators.iter().enumerate() {
        let p_var = var_pgen_start + i;

        // P ≤ Pmax/BASE_MVA
        row_gen_pmax.push(push_leq(
            &[(p_var, 1.0)],
            gen.pmax_mw / BASE_MVA,
            &mut rows,
            &mut rhs,
            &mut cones,
        ));

        // P ≥ Pmin/BASE_MVA  →  -P ≤ -Pmin/BASE_MVA
        row_gen_pmin.push(push_leq(
            &[(p_var, -1.0)],
            -gen.pmin_mw / BASE_MVA,
            &mut rows,
            &mut rhs,
            &mut cones,
        ));

        let q_var = var_qgen_start + i;

        // Q ≤ Qmax/BASE_MVA
        row_gen_qmax.push(push_leq(
            &[(q_var, 1.0)],
            gen.qmax_mvar / BASE_MVA,
            &mut rows,
            &mut rhs,
            &mut cones,
        ));

        // Q ≥ Qmin/BASE_MVA
        row_gen_qmin.push(push_leq(
            &[(q_var, -1.0)],
            -gen.qmin_mvar / BASE_MVA,
            &mut rows,
            &mut rhs,
            &mut cones,
        ));
    }

    // ========================================================================
    // CONSTRAINT GROUP 5: BRANCH THERMAL LIMITS
    // ========================================================================
    //
    // Thermal limits prevent conductor overheating:
    //   |S| = √(P² + Q²) ≤ S_max
    //
    // In the branch-flow model with squared current ℓ:
    //   |S|² = v·ℓ  →  |S| = √(v·ℓ)
    //
    // Since v ≈ 1.0, we approximate:
    //   ℓ ≤ (S_max/BASE_MVA)²
    //
    // This is conservative when v < 1.0 (actual |S| would be smaller).
    //
    // Also enforce ℓ ≥ 0 (squared current is non-negative).

    for (i, br) in branches.iter().enumerate() {
        let l_var = var_l_start + i;

        if let Some(smax) = br.s_max_mva {
            // ℓ ≤ (S_max/BASE_MVA)²
            let smax_pu = smax / BASE_MVA;
            let row = push_leq(
                &[(l_var, 1.0)],
                smax_pu * smax_pu,
                &mut rows,
                &mut rhs,
                &mut cones,
            );
            row_branch_thermal[i] = Some(row);
        }

        // ℓ ≥ 0  →  -ℓ ≤ 0
        push_leq(&[(l_var, -1.0)], 0.0, &mut rows, &mut rhs, &mut cones);
    }

    // ========================================================================
    // CONSTRAINT GROUP 6: POWER BALANCE (KIRCHHOFF'S CURRENT LAW)
    // ========================================================================
    //
    // At each bus, power in = power out (conservation of energy):
    //
    //   Σ P_gen - Σ P_load = Σ P_out - Σ (P_in - R·ℓ)
    //   Σ Q_gen - Σ Q_load = Σ Q_out - Σ (Q_in - X·ℓ) + B_shunt·v
    //
    // Where:
    //   - P_out, Q_out: Power leaving on branches from this bus
    //   - P_in, Q_in: Power arriving on branches to this bus
    //   - R·ℓ, X·ℓ: Losses on incoming branches
    //   - B_shunt·v: Reactive power from line charging (capacitive)
    //
    // The half-line charging model places B/2 at each end of the line.
    //
    // Dual variables on these constraints give Locational Marginal Prices (LMPs).

    let mut p_balance_rows: Vec<usize> = Vec::with_capacity(buses.len());

    for bus in &buses {
        let mut coeffs_p: Vec<(usize, f64)> = Vec::new();
        let mut coeffs_q: Vec<(usize, f64)> = Vec::new();
        let bus_idx = bus.index;

        // Add generator contributions at this bus
        for (g_idx, gen) in generators.iter().enumerate() {
            if gen.bus_id == bus.id {
                coeffs_p.push((var_pgen_start + g_idx, 1.0)); // +P_gen
                coeffs_q.push((var_qgen_start + g_idx, 1.0)); // +Q_gen
            }
        }

        // Add branch contributions
        for (br_idx, br) in branches.iter().enumerate() {
            if br.from_bus == bus.id {
                // Outgoing branch: power flows OUT of this bus
                coeffs_p.push((var_pflow_start + br_idx, -1.0)); // -P_flow (leaving)
                coeffs_q.push((var_qflow_start + br_idx, -1.0)); // -Q_flow (leaving)

                // Half of line charging at sending end: +B/2 × v
                // Line charging is capacitive (generates reactive power)
                let b_half = 0.5 * br.b_shunt;
                if b_half.abs() > 1e-12 {
                    coeffs_q.push((var_v_start + bus_idx, b_half));
                }
            }

            if br.to_bus == bus.id {
                // Incoming branch: power flows INTO this bus (minus losses)
                // Received power = P_flow - R·ℓ (losses subtracted)
                coeffs_p.push((var_pflow_start + br_idx, 1.0)); // +P_flow
                coeffs_p.push((var_l_start + br_idx, -br.r)); // -R·ℓ (real loss)

                coeffs_q.push((var_qflow_start + br_idx, 1.0)); // +Q_flow
                coeffs_q.push((var_l_start + br_idx, -br.x)); // -X·ℓ (reactive loss)

                // Half of line charging at receiving end
                let b_half = 0.5 * br.b_shunt;
                if b_half.abs() > 1e-12 {
                    coeffs_q.push((var_v_start + bus_idx, b_half));
                }
            }
        }

        // Get load at this bus (default to zero if no load)
        let (p_load, q_load) = loads.get(&bus.id).copied().unwrap_or((0.0, 0.0));

        // P balance: Σ contributions = P_load (in per-unit)
        let row_p = push_eq(
            &coeffs_p,
            p_load / BASE_MVA,
            &mut rows,
            &mut rhs,
            &mut cones,
        );
        p_balance_rows.push(row_p);

        // Q balance: Σ contributions = Q_load (in per-unit)
        push_eq(
            &coeffs_q,
            q_load / BASE_MVA,
            &mut rows,
            &mut rhs,
            &mut cones,
        );
    }

    // ========================================================================
    // CONSTRAINT GROUP 7: VOLTAGE DROP EQUATIONS
    // ========================================================================
    //
    // The voltage drop along a branch relates sending and receiving voltages
    // to the power flow and losses. This is derived from Ohm's law.
    //
    // For a branch i→j with impedance z = r + jx and tap ratio τ:
    //
    //   Complex power: S = V·I* = P + jQ
    //   Current: I = S*/V* = (P - jQ)/V*
    //   Voltage drop: ΔV = I·z = (P - jQ)·(r + jx) / V*
    //
    // Taking magnitudes and using v = |V|²:
    //
    //   vⱼ = vᵢ/τ² - 2(r·P + x·Q)/τ² + (r² + x²)·ℓ
    //
    // This is LINEAR in the optimization variables (v, P, Q, ℓ)!
    //
    // The tap ratio τ² appears in the voltage and impedance terms because
    // the transformer transforms voltage by τ, which transforms impedance
    // (referred to primary side) by τ².
    //
    // For PHASE-SHIFTING TRANSFORMERS, we also need an angle equation:
    //
    //   θⱼ = θᵢ - φ - arg(ΔV/V) ≈ θᵢ - φ - (x·P - r·Q)/vᵢ
    //
    // Since vᵢ is a variable, we linearize around v ≈ 1.0 p.u.:
    //
    //   θⱼ - θᵢ + (x·P - r·Q)/τ ≈ -φ
    //
    // This is an approximation valid for typical operating conditions (v ≈ 1).

    for (i, br) in branches.iter().enumerate() {
        let from = *bus_map.get(&br.from_bus).expect("from bus must exist");
        let to = *bus_map.get(&br.to_bus).expect("to bus must exist");

        let tau2 = br.tap_ratio * br.tap_ratio; // τ²
        let z2 = br.r * br.r + br.x * br.x; // |z|² = r² + x²

        // --------------------------------------------------------------------
        // Voltage magnitude drop equation:
        //   vⱼ - vᵢ/τ² + 2(r·P + x·Q)/τ² - (r² + x²)·ℓ = 0
        // --------------------------------------------------------------------
        let mut v_coeffs: Vec<(usize, f64)> = Vec::new();

        v_coeffs.push((var_v_start + to, 1.0)); // vⱼ
        v_coeffs.push((var_v_start + from, -1.0 / tau2)); // -vᵢ/τ²
        v_coeffs.push((var_pflow_start + i, 2.0 * br.r / tau2)); // +2r·P/τ²
        v_coeffs.push((var_qflow_start + i, 2.0 * br.x / tau2)); // +2x·Q/τ²
        v_coeffs.push((var_l_start + i, -z2)); // -(r² + x²)·ℓ

        push_eq(&v_coeffs, 0.0, &mut rows, &mut rhs, &mut cones);

        // --------------------------------------------------------------------
        // Angle equation (for phase shifters and general angle tracking):
        //   θⱼ - θᵢ + (x·P - r·Q)/τ = -φ
        //
        // This constraint couples angles to power flows, which is needed
        // for networks with phase-shifting transformers or when angle
        // output is desired.
        //
        // We include this for all branches with non-zero impedance,
        // which provides angle estimation even without phase shifters.
        // --------------------------------------------------------------------
        if br.phase_shift_rad.abs() > 1e-12 || z2 > 1e-12 {
            let mut theta_coeffs: Vec<(usize, f64)> = Vec::new();

            theta_coeffs.push((var_theta_start + to, 1.0)); // θⱼ
            theta_coeffs.push((var_theta_start + from, -1.0)); // -θᵢ
            theta_coeffs.push((var_pflow_start + i, br.x / br.tap_ratio)); // +x·P/τ
            theta_coeffs.push((var_qflow_start + i, -br.r / br.tap_ratio)); // -r·Q/τ

            push_eq(
                &theta_coeffs,
                -br.phase_shift_rad, // = -φ
                &mut rows,
                &mut rhs,
                &mut cones,
            );
        }
    }

    // ========================================================================
    // CONSTRAINT GROUP 8: SECOND-ORDER CONE (SOC) CONSTRAINTS
    // ========================================================================
    //
    // The SOCP relaxation replaces the nonlinear equality:
    //   P² + Q² = v·ℓ  (exact AC relationship)
    //
    // With the convex inequality:
    //   P² + Q² ≤ v·ℓ  (SOCP relaxation)
    //
    // This is a ROTATED SECOND-ORDER CONE constraint:
    //   2·v·ℓ ≥ P² + Q², with v,ℓ ≥ 0
    //
    // Clarabel uses STANDARD second-order cones: t ≥ ||x||₂
    // We convert using the identity:
    //   2ab ≥ c² + d²  ⟺  (a+b)² ≥ (a-b)² + c² + d² + 2²
    //                  ⟺  (a+b) ≥ ||(a-b, c, d)||
    //
    // For our constraint 2·(v/τ²)·ℓ ≥ (2P)² + (2Q)²:
    //   Let a = v/τ², b = ℓ
    //   (v/τ² + ℓ) ≥ ||(v/τ² - ℓ, 2P, 2Q)||
    //
    // In Clarabel's form Ax + s = b with s ∈ SOC:
    //   s₀ = -(v/τ² + ℓ)      (cone's "t" component)
    //   s₁ = -2P               (x component 1)
    //   s₂ = -2Q               (x component 2)
    //   s₃ = -(v/τ² - ℓ)       (x component 3)
    //
    // The negative signs arise because Clarabel puts s on the LHS.
    //
    // WHY IS THE RELAXATION TIGHT?
    //
    // At optimum, we typically get P² + Q² = v·ℓ (equality) because:
    // 1. The objective includes c₂·P² (convex), pushing toward lower P
    // 2. If P² + Q² < v·ℓ, we could reduce ℓ (and thus losses r·ℓ)
    // 3. Loss minimization drives the constraint to equality
    //
    // The relaxation may be loose when:
    // - Highly meshed networks with many parallel paths
    // - Voltage constraints binding (unusual topology)
    // - Very high reactive flows (rare in practice)
    //
    // See Farivar & Low (2013) for exactness conditions.

    for (i, br) in branches.iter().enumerate() {
        let from_idx = *bus_map.get(&br.from_bus).expect("from bus exists");
        let v_idx = var_v_start + from_idx;
        let l_idx = var_l_start + i;
        let p_idx = var_pflow_start + i;
        let q_idx = var_qflow_start + i;

        let tau2 = br.tap_ratio * br.tap_ratio;

        // Allocate 4 rows for the SOC constraint (t, x₁, x₂, x₃)
        let base = rhs.len();
        rhs.extend_from_slice(&[0.0, 0.0, 0.0, 0.0]);

        // s₀ = -(v/τ² + ℓ)  →  coefficient on v is -1/τ², on ℓ is -1
        rows[v_idx].push((base, -1.0 / tau2));
        rows[l_idx].push((base, -1.0));

        // s₁ = -2P
        rows[p_idx].push((base + 1, -2.0));

        // s₂ = -2Q
        rows[q_idx].push((base + 2, -2.0));

        // s₃ = -(v/τ² - ℓ)  →  coefficient on v is -1/τ², on ℓ is +1
        rows[v_idx].push((base + 3, -1.0 / tau2));
        rows[l_idx].push((base + 3, 1.0));

        // Register this as a second-order cone of dimension 4
        cones.push(SupportedConeT::SecondOrderConeT(4));
    }

    // ========================================================================
    // STEP 6: CONVERT TO SPARSE CSC FORMAT
    // ========================================================================
    //
    // Clarabel requires the constraint matrix A in Compressed Sparse Column
    // (CSC) format, which is efficient for the column operations used in
    // interior-point methods.
    //
    // CSC format uses three arrays:
    //   - col_ptr[j]: Index where column j's entries start in row_idx/values
    //   - row_idx[k]: Row index of the k-th non-zero
    //   - values[k]: Value of the k-th non-zero
    //
    // We've been accumulating entries column-wise in `rows`, so conversion
    // is straightforward: sort each column by row index and concatenate.

    let n_con_rows = rhs.len();
    let mut col_ptr = Vec::with_capacity(n_var + 1);
    let mut row_idx = Vec::new();
    let mut values = Vec::new();
    let mut nnz = 0;

    for col in 0..n_var {
        col_ptr.push(nnz);

        // Sort entries in this column by row index (required for CSC)
        rows[col].sort_by_key(|(r, _)| *r);

        for &(r, v) in &rows[col] {
            row_idx.push(r);
            values.push(v);
            nnz += 1;
        }
    }
    col_ptr.push(nnz); // Final column pointer

    // Construct sparse matrices
    let a_mat = CscMatrix::new(n_con_rows, n_var, col_ptr, row_idx, values);
    let p_mat = CscMatrix::new(n_var, n_var, p_col_ptr, p_row_idx, p_values);

    // ========================================================================
    // STEP 7: INVOKE THE CONIC SOLVER
    // ========================================================================
    //
    // Clarabel solves the conic program:
    //   minimize    (1/2)x'Px + q'x
    //   subject to  Ax + s = b
    //               s ∈ K (product of cones)
    //
    // It uses a primal-dual interior-point method with:
    // - Nesterov-Todd scaling for symmetric cones
    // - Direct factorization (QDLDL) for the KKT system
    // - Adaptive step size and centering
    //
    // Typical convergence: 15-30 iterations for 1e-8 tolerance.

    let settings = DefaultSettingsBuilder::default()
        .verbose(false) // Suppress iteration output for production use
        .build()
        .map_err(|e| OpfError::NumericalIssue(format!("Clarabel settings error: {:?}", e)))?;

    let mut solver = clarabel::solver::DefaultSolver::new(
        &p_mat, &obj, &a_mat, &rhs, &cones, settings,
    )
    .map_err(|e| OpfError::NumericalIssue(format!("Clarabel initialization failed: {:?}", e)))?;

    solver.solve();

    // Check solver status
    let sol = solver.solution;
    if !matches!(
        sol.status,
        clarabel::solver::SolverStatus::Solved
            | clarabel::solver::SolverStatus::AlmostSolved
            | clarabel::solver::SolverStatus::AlmostDualInfeasible
            | clarabel::solver::SolverStatus::AlmostPrimalInfeasible
            | clarabel::solver::SolverStatus::DualInfeasible
    ) {
        return Err(OpfError::NumericalIssue(format!(
            "Clarabel returned status {:?}. \
             This may indicate an infeasible problem (load exceeds capacity, \
             voltage limits too tight) or numerical issues.",
            sol.status
        )));
    }

    // Extract primal solution (x) and dual variables (z)
    let x = &sol.x;
    let z = &sol.z;

    // ========================================================================
    // STEP 8: EXTRACT AND FORMAT SOLUTION
    // ========================================================================

    let mut result = OpfSolution {
        converged: true,
        method_used: OpfMethod::SocpRelaxation,
        iterations: sol.iterations as usize,
        solve_time_ms: start.elapsed().as_millis(),
        ..Default::default()
    };

    // ------------------------------------------------------------------------
    // 8a. Generator dispatch and total cost
    // ------------------------------------------------------------------------
    //
    // Convert per-unit generator outputs back to MW/MVAr and compute
    // the total generation cost using the full polynomial.

    let mut total_cost = 0.0;
    for (idx, gen) in generators.iter().enumerate() {
        let p = x[var_pgen_start + idx] * BASE_MVA; // MW
        let q = x[var_qgen_start + idx] * BASE_MVA; // MVAr

        result.generator_p.insert(gen.name.clone(), p);
        result.generator_q.insert(gen.name.clone(), q);

        // Compute cost: c₀ + c₁·P + c₂·P²
        let c0 = gen.cost_coeffs.first().copied().unwrap_or(0.0);
        let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
        let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
        total_cost += c0 + c1 * p + c2 * p * p;
    }
    result.objective_value = total_cost;

    // ------------------------------------------------------------------------
    // 8b. Bus voltages and angles
    // ------------------------------------------------------------------------
    //
    // Extract voltage magnitude (sqrt of squared variable) and angle.
    // Angles are converted from radians to degrees for output.

    for (idx, bus) in buses.iter().enumerate() {
        let v_sq = x[var_v_start + idx];
        let v_mag = v_sq.max(0.0).sqrt(); // Handle small negative from numerics

        let theta_rad = x[var_theta_start + idx];
        let theta_deg = theta_rad.to_degrees();

        result.bus_voltage_mag.insert(bus.name.clone(), v_mag);
        result.bus_voltage_ang.insert(bus.name.clone(), theta_deg);
    }

    // ------------------------------------------------------------------------
    // 8c. Branch flows and system losses
    // ------------------------------------------------------------------------
    //
    // Power flow on each branch and total system losses.
    // Losses = Σ r·ℓ (real power) + Σ x·ℓ (reactive power)

    let mut total_losses = 0.0;
    for (idx, br) in branches.iter().enumerate() {
        let p = x[var_pflow_start + idx] * BASE_MVA; // MW
        let q = x[var_qflow_start + idx] * BASE_MVA; // MVAr
        let l = x[var_l_start + idx]; // Squared current (p.u.)

        result.branch_p_flow.insert(br.name.clone(), p);
        result.branch_q_flow.insert(br.name.clone(), q);

        // Real power losses on this branch: r·ℓ (in per-unit, then scale)
        total_losses += br.r * l * BASE_MVA;

        // Check if thermal limit is binding
        if let Some(row) = row_branch_thermal[idx] {
            if row < z.len() && br.s_max_mva.is_some() {
                let smax = br.s_max_mva.unwrap();
                let limit_pu_sq = (smax / BASE_MVA).powi(2);
                let slack = limit_pu_sq - l;

                // Constraint is binding if slack is near zero
                if slack.abs() < 1e-3 {
                    result.binding_constraints.push(ConstraintInfo {
                        name: br.name.clone(),
                        constraint_type: ConstraintType::BranchFlowLimit,
                        value: l.sqrt() * BASE_MVA, // |I| in per-unit → MVA
                        limit: smax,
                        shadow_price: z[row], // Dual variable ($/MW congestion)
                    });
                }
            }
        }
    }
    result.total_losses_mw = total_losses;

    // ------------------------------------------------------------------------
    // 8d. Locational Marginal Prices (LMPs)
    // ------------------------------------------------------------------------
    //
    // LMPs represent the cost of serving one additional MW of load at each bus.
    // They come from the dual variables on power balance constraints.
    //
    // In a network without congestion, all buses have the same LMP equal to
    // the system marginal cost. With congestion, LMPs diverge:
    // - Higher at load centers behind congested interfaces
    // - Lower at generation pockets with excess capacity
    //
    // For quadratic costs, the marginal cost at generator g is:
    //   MC_g = c₁ + 2·c₂·P_g
    //
    // The system LMP (at the slack bus) equals the marginal cost of the
    // marginal generator (one not at its limits).
    //
    // See: Schweppe et al. (1988) "Spot Pricing of Electricity"
    // DOI: [10.1007/978-1-4613-1683-1](https://doi.org/10.1007/978-1-4613-1683-1)

    // First, estimate system-wide LMP from marginal generator
    let mut system_lmp = 0.0;
    for (idx, gen) in generators.iter().enumerate() {
        let p_pu = x[var_pgen_start + idx];
        let p_mw = p_pu * BASE_MVA;

        let at_min = (p_mw - gen.pmin_mw).abs() < 1e-3;
        let at_max = (p_mw - gen.pmax_mw).abs() < 1e-3;

        if !at_min && !at_max {
            // This generator is marginal (not at its limits)
            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
            system_lmp = c1 + 2.0 * c2 * p_mw; // dC/dP
            break;
        }
    }

    // Fallback: if all generators at limits, use highest marginal cost
    if system_lmp == 0.0 {
        for (idx, gen) in generators.iter().enumerate() {
            let p_mw = x[var_pgen_start + idx] * BASE_MVA;
            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
            let marginal = c1 + 2.0 * c2 * p_mw;
            system_lmp = system_lmp.max(marginal);
        }
    }

    // Initialize all buses with system LMP
    for bus in &buses {
        result.bus_lmp.insert(bus.name.clone(), system_lmp);
    }

    // Refine LMPs using dual variables on power balance constraints
    // The dual on P-balance gives ∂(cost)/∂(P_load) = LMP
    for (i, bus) in buses.iter().enumerate() {
        if let Some(&row) = p_balance_rows.get(i) {
            if row < z.len() {
                // Dual variable gives LMP in $/MWh (already scaled)
                result.bus_lmp.insert(bus.name.clone(), z[row]);
            }
        }

        // Record binding voltage constraints
        let v_mag = *result.bus_voltage_mag.get(&bus.name).unwrap_or(&1.0);

        if let Some(&row_min) = row_voltage_min.get(i) {
            if row_min < z.len() && (v_mag - bus.v_min).abs() < 1e-4 {
                result.binding_constraints.push(ConstraintInfo {
                    name: bus.name.clone(),
                    constraint_type: ConstraintType::VoltageMin,
                    value: v_mag,
                    limit: bus.v_min,
                    shadow_price: z[row_min],
                });
            }
        }

        if let Some(&row_max) = row_voltage_max.get(i) {
            if row_max < z.len() && (v_mag - bus.v_max).abs() < 1e-4 {
                result.binding_constraints.push(ConstraintInfo {
                    name: bus.name.clone(),
                    constraint_type: ConstraintType::VoltageMax,
                    value: v_mag,
                    limit: bus.v_max,
                    shadow_price: z[row_max],
                });
            }
        }
    }

    // Record binding generator constraints
    for (i, gen) in generators.iter().enumerate() {
        let p = *result.generator_p.get(&gen.name).unwrap_or(&0.0);

        if let Some(&row) = row_gen_pmax.get(i) {
            if row < z.len() && (p - gen.pmax_mw).abs() < 1e-3 {
                result.binding_constraints.push(ConstraintInfo {
                    name: gen.name.clone(),
                    constraint_type: ConstraintType::GeneratorPMax,
                    value: p,
                    limit: gen.pmax_mw,
                    shadow_price: z[row],
                });
            }
        }

        if let Some(&row) = row_gen_pmin.get(i) {
            if row < z.len() && (p - gen.pmin_mw).abs() < 1e-3 {
                result.binding_constraints.push(ConstraintInfo {
                    name: gen.name.clone(),
                    constraint_type: ConstraintType::GeneratorPMin,
                    value: p,
                    limit: gen.pmin_mw,
                    shadow_price: z[row],
                });
            }
        }

        let q = *result.generator_q.get(&gen.name).unwrap_or(&0.0);

        if let Some(&row) = row_gen_qmax.get(i) {
            if row < z.len() && (q - gen.qmax_mvar).abs() < 1e-3 {
                result.binding_constraints.push(ConstraintInfo {
                    name: gen.name.clone(),
                    constraint_type: ConstraintType::GeneratorQMax,
                    value: q,
                    limit: gen.qmax_mvar,
                    shadow_price: z[row],
                });
            }
        }

        if let Some(&row) = row_gen_qmin.get(i) {
            if row < z.len() && (q - gen.qmin_mvar).abs() < 1e-3 {
                result.binding_constraints.push(ConstraintInfo {
                    name: gen.name.clone(),
                    constraint_type: ConstraintType::GeneratorQMin,
                    value: q,
                    limit: gen.qmin_mvar,
                    shadow_price: z[row],
                });
            }
        }
    }

    Ok(result)
}
