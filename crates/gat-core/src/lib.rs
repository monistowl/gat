//! # gat-core: Power Grid Modeling Core
//!
//! Provides the fundamental data structures and graph-based network models for power system analysis.
//!
//! ## Design Philosophy
//!
//! Networks are modeled as **undirected multigraphs** where:
//! - **Nodes**: Buses (buses), Generators (gen), Loads (load)
//! - **Edges**: Branches (transmission lines and transformers)
//!
//! This graph-based approach enables:
//! - Fast topological queries (connectivity, island detection)
//! - Efficient parallel analysis using rayon
//! - Type-safe element access with newtype IDs
//! - Support for multiple edge types between same nodes (parallel branches)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use gat_core::*;
//!
//! // Create a network
//! let mut network = Network::new();
//!
//! // Add buses
//! let bus1_idx = network.graph.add_node(Node::Bus(Bus {
//!     id: BusId::new(1),
//!     name: "Bus 1".to_string(),
//!     base_kv: Kilovolts(138.0),
//!     voltage_pu: PerUnit(1.0),
//!     angle_rad: Radians(0.0),
//!     ..Bus::default()
//! }));
//!
//! let bus2_idx = network.graph.add_node(Node::Bus(Bus {
//!     id: BusId::new(2),
//!     name: "Bus 2".to_string(),
//!     base_kv: Kilovolts(138.0),
//!     voltage_pu: PerUnit(1.0),
//!     angle_rad: Radians(0.0),
//!     ..Bus::default()
//! }));
//!
//! // Add a generator
//! network.graph.add_node(Node::Gen(Gen::new(
//!     GenId::new(1),
//!     "Gen 1".to_string(),
//!     BusId::new(1),
//! ).with_p_limits(0.0, 100.0)));
//!
//! // Add a load
//! network.graph.add_node(Node::Load(Load {
//!     id: LoadId::new(1),
//!     name: "Load 1".to_string(),
//!     bus: BusId::new(2),
//!     active_power: Megawatts(50.0),
//!     reactive_power: Megavars(10.0),
//! }));
//!
//! // Connect buses with a branch
//! network.graph.add_edge(
//!     bus1_idx,
//!     bus2_idx,
//!     Edge::Branch(Branch {
//!         id: BranchId::new(1),
//!         name: "Line 1-2".to_string(),
//!         from_bus: BusId::new(1),
//!         to_bus: BusId::new(2),
//!         resistance: 0.01,
//!         reactance: 0.1,
//!         ..Branch::default()
//!     }),
//! );
//! ```
//!
//! ## Core Data Structures
//!
//! - [`Network`] - The main network container (petgraph `UnDiGraph<Node, Edge>`)
//! - [`Node`] - Enum for Bus, Gen, Load elements
//! - [`Edge`] - Enum for Branch, Transformer connections
//! - Type-safe IDs: [`BusId`], [`GenId`], [`LoadId`], [`BranchId`], [`TransformerId`]
//!
//! ## ID System
//!
//! Every element has a unique ID (newtype wrapper around `usize`):
//! - **Bus IDs** (1-based in MATPOWER): Bus#1, Bus#2, ...
//! - **Generator IDs**: Gen#1, Gen#2, ...
//! - **Load IDs**: Load#1, Load#2, ...
//! - **Branch IDs**: Branch#1, Branch#2, ...
//!
//! IDs enable:
//! - Type safety: Can't confuse bus IDs with generator IDs
//! - Foreign key validation in Arrow schemas
//! - Consistent roundtrip import/export
//!
//! ## Modules
//!
//! - [`diagnostics`] - Validation and diagnostic reporting
//! - [`graph_utils`] - Topological analysis (connectivity, islands, etc.)
//! - [`solver`] - Power flow and optimization algorithms
//!
//! ## Integration with gat-io
//!
//! The gat-io crate provides importers from various formats (MATPOWER, PSS/E, CIM, pandapower)
//! that construct [`Network`] graphs from external data.

use petgraph::{prelude::*, Undirected};
use serde::{Deserialize, Serialize};

pub mod diagnostics;
pub mod error;
pub mod graph_utils;
pub mod solver;
pub mod units;

pub use diagnostics::{DiagnosticIssue, Diagnostics, ImportDiagnostics, ImportStats, Severity};
pub use error::{GatError, GatResult};
pub use graph_utils::*;
pub use petgraph::graph::NodeIndex;
pub use solver::*;
pub use units::{
    AdmittancePu, CurrentPu, Degrees, ImpedancePu, Kiloamperes, Kilovolts, Megavars,
    MegavoltAmperes, Megawatts, PerUnit, Radians,
};

// Newtype wrappers for IDs for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BusId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BranchId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GenId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LoadId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransformerId(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShuntId(usize);

impl BusId {
    #[inline]
    pub fn new(value: usize) -> Self {
        BusId(value)
    }
    #[inline]
    pub fn value(&self) -> usize {
        self.0
    }
}

impl BranchId {
    #[inline]
    pub fn new(value: usize) -> Self {
        BranchId(value)
    }
    #[inline]
    pub fn value(&self) -> usize {
        self.0
    }
}

impl GenId {
    #[inline]
    pub fn new(value: usize) -> Self {
        GenId(value)
    }
    #[inline]
    pub fn value(&self) -> usize {
        self.0
    }
}

impl LoadId {
    #[inline]
    pub fn new(value: usize) -> Self {
        LoadId(value)
    }
    #[inline]
    pub fn value(&self) -> usize {
        self.0
    }
}

impl TransformerId {
    #[inline]
    pub fn new(value: usize) -> Self {
        TransformerId(value)
    }
    #[inline]
    pub fn value(&self) -> usize {
        self.0
    }
}

impl ShuntId {
    #[inline]
    pub fn new(value: usize) -> Self {
        ShuntId(value)
    }
    #[inline]
    pub fn value(&self) -> usize {
        self.0
    }
}

// Basic component structs
#[derive(Debug, Clone)]
pub struct Bus {
    pub id: BusId,
    pub name: String,
    /// Base voltage in kilovolts (for per-unit conversions)
    pub base_kv: Kilovolts,
    /// Voltage magnitude in per-unit
    pub voltage_pu: PerUnit,
    /// Voltage angle in radians
    pub angle_rad: Radians,
    /// Minimum voltage limit in per-unit
    pub vmin_pu: Option<PerUnit>,
    /// Maximum voltage limit in per-unit
    pub vmax_pu: Option<PerUnit>,
    pub area_id: Option<i64>,
    pub zone_id: Option<i64>,
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            id: BusId(0),
            name: String::new(),
            base_kv: Kilovolts(0.0),
            voltage_pu: PerUnit(1.0),
            angle_rad: Radians(0.0),
            vmin_pu: None,
            vmax_pu: None,
            area_id: None,
            zone_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub id: BranchId,
    pub name: String,
    pub from_bus: BusId,
    pub to_bus: BusId,
    /// Series resistance (per-unit)
    pub resistance: f64,
    /// Series reactance (per-unit)
    pub reactance: f64,
    /// Multiplicative tap magnitude applied from from_bus to to_bus
    pub tap_ratio: f64,
    /// Phase shift applied from from_bus to to_bus
    pub phase_shift: Radians,
    /// Total line charging susceptance (per-unit, split half/half)
    pub charging_b: PerUnit,
    /// Symmetric thermal limit
    pub s_max: Option<MegavoltAmperes>,
    /// Normal rating (Rate A)
    pub rating_a: Option<MegavoltAmperes>,
    /// Emergency rating (Rate B)
    pub rating_b: Option<MegavoltAmperes>,
    /// Short-term rating (Rate C)
    pub rating_c: Option<MegavoltAmperes>,
    /// Operational status flag
    pub status: bool,
    /// Minimum angle difference limit
    pub angle_min: Option<Radians>,
    /// Maximum angle difference limit
    pub angle_max: Option<Radians>,
    /// Element type: line or transformer
    pub element_type: String,
    /// Phase-shifting transformer flag (allows negative reactance)
    pub is_phase_shifter: bool,
}

impl Default for Branch {
    fn default() -> Self {
        Self {
            id: BranchId(0),
            name: String::new(),
            from_bus: BusId(0),
            to_bus: BusId(0),
            resistance: 0.0,
            reactance: 0.0,
            tap_ratio: 1.0,
            phase_shift: Radians(0.0),
            charging_b: PerUnit(0.0),
            s_max: None,
            rating_a: None,
            rating_b: None,
            rating_c: None,
            status: true,
            angle_min: None,
            angle_max: None,
            element_type: "line".to_string(),
            is_phase_shifter: false,
        }
    }
}

impl Branch {
    /// Construct a branch from legacy impedance fields, filling new parameters with defaults.
    pub fn new(
        id: BranchId,
        name: String,
        from_bus: BusId,
        to_bus: BusId,
        resistance: f64,
        reactance: f64,
    ) -> Self {
        Self {
            id,
            name,
            from_bus,
            to_bus,
            resistance,
            reactance,
            ..Self::default()
        }
    }

    /// Attach a symmetric thermal limit in MVA.
    pub fn with_s_max(mut self, s_max_mva: Option<f64>) -> Self {
        self.s_max = s_max_mva.map(MegavoltAmperes);
        self
    }

    /// Mark branch as phase-shifting transformer (allows negative reactance).
    pub fn as_phase_shifter(mut self) -> Self {
        self.is_phase_shifter = true;
        self
    }
}

/// Generator cost model for OPF optimization
#[derive(Debug, Clone, Default)]
pub enum CostModel {
    /// No cost function specified
    #[default]
    NoCost,
    /// Polynomial cost: `cost = sum(coeffs[i] * P^i)` where `coeffs[0]` is constant term.
    /// For quadratic: `coeffs = [c0, c1, c2]` means `cost = c0 + c1*P + c2*P^2`.
    Polynomial(Vec<f64>),
    /// Piecewise linear cost: Vec<(mw, $/hr)> breakpoints
    PiecewiseLinear(Vec<(f64, f64)>),
}

impl CostModel {
    /// Create quadratic cost: c0 + c1*P + c2*P^2
    pub fn quadratic(c0: f64, c1: f64, c2: f64) -> Self {
        CostModel::Polynomial(vec![c0, c1, c2])
    }

    /// Create linear cost: c0 + c1*P (marginal cost c1 in $/MWh)
    pub fn linear(c0: f64, c1: f64) -> Self {
        CostModel::Polynomial(vec![c0, c1])
    }

    /// Evaluate cost at given power output ($/hr)
    pub fn evaluate(&self, p_mw: f64) -> f64 {
        match self {
            CostModel::NoCost => 0.0,
            CostModel::Polynomial(coeffs) => coeffs
                .iter()
                .enumerate()
                .map(|(i, c)| c * p_mw.powi(i as i32))
                .sum(),
            CostModel::PiecewiseLinear(points) => {
                if points.is_empty() {
                    return 0.0;
                }
                if p_mw <= points[0].0 {
                    return points[0].1;
                }
                if p_mw >= points.last().unwrap().0 {
                    return points.last().unwrap().1;
                }
                for i in 0..points.len() - 1 {
                    if p_mw >= points[i].0 && p_mw <= points[i + 1].0 {
                        let t = (p_mw - points[i].0) / (points[i + 1].0 - points[i].0);
                        return points[i].1 + t * (points[i + 1].1 - points[i].1);
                    }
                }
                0.0
            }
        }
    }

    /// Get marginal cost at given power ($/MWh, derivative of cost function)
    pub fn marginal_cost(&self, p_mw: f64) -> f64 {
        match self {
            CostModel::NoCost => 0.0,
            CostModel::Polynomial(coeffs) => {
                // d/dP[sum(c_i * P^i)] = sum(i * c_i * P^(i-1))
                coeffs
                    .iter()
                    .enumerate()
                    .skip(1)
                    .map(|(i, c)| (i as f64) * c * p_mw.powi(i as i32 - 1))
                    .sum()
            }
            CostModel::PiecewiseLinear(points) => {
                if points.len() < 2 {
                    return 0.0;
                }
                for i in 0..points.len() - 1 {
                    if p_mw >= points[i].0 && p_mw <= points[i + 1].0 {
                        return (points[i + 1].1 - points[i].1) / (points[i + 1].0 - points[i].0);
                    }
                }
                // Return last segment's slope if beyond range
                let n = points.len();
                (points[n - 1].1 - points[n - 2].1) / (points[n - 1].0 - points[n - 2].0)
            }
        }
    }

    /// Check if this cost model has actual cost data
    pub fn has_cost(&self) -> bool {
        !matches!(self, CostModel::NoCost)
    }
}

#[derive(Debug, Clone)]
pub struct Gen {
    pub id: GenId,
    pub name: String,
    pub bus: BusId,
    /// Active power output (MW)
    pub active_power: Megawatts,
    /// Reactive power output (Mvar)
    pub reactive_power: Megavars,
    /// Minimum active power output
    pub pmin: Megawatts,
    /// Maximum active power output
    pub pmax: Megawatts,
    /// Minimum reactive power output
    pub qmin: Megavars,
    /// Maximum reactive power output
    pub qmax: Megavars,
    /// In-service status
    pub status: bool,
    /// Voltage setpoint (per-unit)
    pub voltage_setpoint: Option<PerUnit>,
    /// Machine MVA base
    pub mbase: Option<MegavoltAmperes>,
    /// Startup cost ($)
    pub cost_startup: Option<f64>,
    /// Shutdown cost ($)
    pub cost_shutdown: Option<f64>,
    /// Cost function for OPF
    pub cost_model: CostModel,
    /// Synchronous condenser flag (allows negative Pg for reactive-only devices)
    pub is_synchronous_condenser: bool,
}

impl Default for Gen {
    fn default() -> Self {
        Self {
            id: GenId(0),
            name: String::new(),
            bus: BusId(0),
            active_power: Megawatts(0.0),
            reactive_power: Megavars(0.0),
            pmin: Megawatts(0.0),
            pmax: Megawatts(f64::INFINITY),
            qmin: Megavars(f64::NEG_INFINITY),
            qmax: Megavars(f64::INFINITY),
            status: true,
            voltage_setpoint: None,
            mbase: None,
            cost_startup: None,
            cost_shutdown: None,
            cost_model: CostModel::NoCost,
            is_synchronous_condenser: false,
        }
    }
}

impl Gen {
    /// Create a new generator with default limits (no constraints)
    pub fn new(id: GenId, name: String, bus: BusId) -> Self {
        Self {
            id,
            name,
            bus,
            active_power: Megawatts(0.0),
            reactive_power: Megavars(0.0),
            pmin: Megawatts(0.0),
            pmax: Megawatts(f64::INFINITY),
            qmin: Megavars(f64::NEG_INFINITY),
            qmax: Megavars(f64::INFINITY),
            status: true,
            voltage_setpoint: None,
            mbase: None,
            cost_startup: None,
            cost_shutdown: None,
            cost_model: CostModel::NoCost,
            is_synchronous_condenser: false,
        }
    }

    /// Set active power limits (in MW)
    pub fn with_p_limits(mut self, pmin: f64, pmax: f64) -> Self {
        self.pmin = Megawatts(pmin);
        self.pmax = Megawatts(pmax);
        self
    }

    /// Set reactive power limits (in Mvar)
    pub fn with_q_limits(mut self, qmin: f64, qmax: f64) -> Self {
        self.qmin = Megavars(qmin);
        self.qmax = Megavars(qmax);
        self
    }

    /// Set cost model
    pub fn with_cost(mut self, cost: CostModel) -> Self {
        self.cost_model = cost;
        self
    }

    /// Mark generator as synchronous condenser (allows negative Pg)
    pub fn as_synchronous_condenser(mut self) -> Self {
        self.is_synchronous_condenser = true;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Load {
    pub id: LoadId,
    pub name: String,
    pub bus: BusId,
    /// Active power demand (MW)
    pub active_power: Megawatts,
    /// Reactive power demand (Mvar)
    pub reactive_power: Megavars,
}

#[derive(Debug, Clone)]
pub struct Transformer {
    pub id: TransformerId,
    pub name: String,
    pub from_bus: BusId,
    pub to_bus: BusId,
    pub ratio: f64,
}

/// Shunt element (capacitor or reactor) connected to a bus
///
/// Shunts inject reactive power (capacitors: +Q, reactors: -Q) to control
/// voltage and provide reactive power support. The Y-bus includes shunt
/// admittance as diagonal elements: Y_ii += gs + j*bs
#[derive(Debug, Clone)]
pub struct Shunt {
    pub id: ShuntId,
    pub name: String,
    /// Bus this shunt is connected to
    pub bus: BusId,
    /// Shunt conductance in per-unit (typically 0 for capacitors/reactors)
    pub gs_pu: f64,
    /// Shunt susceptance in per-unit (positive = capacitor, negative = reactor)
    pub bs_pu: f64,
    /// In-service status
    pub status: bool,
}

impl Default for Shunt {
    fn default() -> Self {
        Self {
            id: ShuntId(0),
            name: String::new(),
            bus: BusId(0),
            gs_pu: 0.0,
            bs_pu: 0.0,
            status: true,
        }
    }
}

// Enum to represent different types of nodes in the graph
#[derive(Debug, Clone)]
pub enum Node {
    Bus(Bus),
    Gen(Gen),
    Load(Load),
    Shunt(Shunt),
}

// Enum to represent different types of edges in the graph
#[derive(Debug, Clone)]
pub enum Edge {
    Branch(Branch),
    Transformer(Transformer),
}

/// The core power network graph
#[derive(Debug, Default)]
pub struct Network {
    pub graph: Graph<Node, Edge, Undirected>,
}

// The physical grid is represented as a graph where buses, generators, and loads are nodes,
// while branches and transformers are edges. This mirrors the standard approach to power
// system modeling and keeps topology explicit for algorithms such as DC power flow and
// contingency screening (see doi:10.1109/TPWRS.2012.2187686).

impl Network {
    pub fn new() -> Self {
        Self {
            graph: Graph::new_undirected(),
        }
    }

    /// Compute basic statistics about the network
    pub fn stats(&self) -> NetworkStats {
        let mut stats = NetworkStats::default();

        for node in self.graph.node_weights() {
            match node {
                Node::Bus(_) => stats.num_buses += 1,
                Node::Gen(g) => {
                    stats.num_gens += 1;
                    stats.total_gen_capacity_mw += g.pmax.value();
                    stats.total_gen_pmin_mw += g.pmin.value();
                }
                Node::Load(l) => {
                    stats.num_loads += 1;
                    stats.total_load_mw += l.active_power.value();
                    stats.total_load_mvar += l.reactive_power.value();
                }
                Node::Shunt(_) => stats.num_shunts += 1,
            }
        }

        stats.num_branches = self.graph.edge_count();
        stats
    }


    /// Validate network data for common issues that cause solver failures.
    ///
    /// Populates the provided `Diagnostics` with any warnings/errors found.
    /// This is the preferred validation method.
    pub fn validate_into(&self, diag: &mut Diagnostics) {
        let stats = self.stats();

        // Check for empty network
        if stats.num_buses == 0 {
            diag.add_error("structure", "Network has no buses");
            return; // Can't check further
        }

        // Check for zero load (likely parser bug)
        if stats.total_load_mw.abs() < 1e-9 && stats.num_loads > 0 {
            diag.add_error(
                "structure",
                &format!(
                    "Total load is 0 MW but {} loads exist - likely parser bug",
                    stats.num_loads
                ),
            );
        } else if stats.total_load_mw.abs() < 1e-9 {
            diag.add_warning("structure", "Network has no loads");
        }

        // Check for no generators
        if stats.num_gens == 0 {
            diag.add_error("structure", "Network has no generators");
        }

        // Check gen capacity vs load
        if stats.total_gen_capacity_mw < stats.total_load_mw {
            diag.add_warning(
                "capacity",
                &format!(
                    "Total generation capacity ({:.1} MW) is less than total load ({:.1} MW)",
                    stats.total_gen_capacity_mw, stats.total_load_mw
                ),
            );
        }

        // Check for branches
        if stats.num_branches == 0 && stats.num_buses > 1 {
            diag.add_error("structure", "Network has multiple buses but no branches");
        }
    }

    /// Get total active power generation (MW)
    pub fn total_generation_mw(&self) -> f64 {
        self.graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Gen(g) if g.status => Some(g.active_power.value()),
                _ => None,
            })
            .sum()
    }

    /// Get total active power load (MW)
    pub fn total_load_mw(&self) -> f64 {
        self.graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Load(l) => Some(l.active_power.value()),
                _ => None,
            })
            .sum()
    }

    /// Get total generation capacity (MW)
    pub fn total_capacity_mw(&self) -> f64 {
        self.graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Gen(g) if g.status => Some(g.pmax.value()),
                _ => None,
            })
            .filter(|v| v.is_finite())
            .sum()
    }

    /// Get reserve margin (generation capacity - load) / load
    pub fn reserve_margin(&self) -> f64 {
        let load = self.total_load_mw();
        if load.abs() < 1e-9 {
            return f64::INFINITY;
        }
        let capacity = self.total_capacity_mw();
        (capacity - load) / load
    }

    /// Find generators at a specific bus
    pub fn generators_at_bus(&self, bus_id: BusId) -> Vec<&Gen> {
        self.graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Gen(g) if g.bus == bus_id => Some(g),
                _ => None,
            })
            .collect()
    }

    /// Find loads at a specific bus
    pub fn loads_at_bus(&self, bus_id: BusId) -> Vec<&Load> {
        self.graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Load(l) if l.bus == bus_id => Some(l),
                _ => None,
            })
            .collect()
    }

    /// Get all buses as a vector
    pub fn buses(&self) -> Vec<&Bus> {
        self.graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Bus(b) => Some(b),
                _ => None,
            })
            .collect()
    }

    /// Get all generators as a vector
    pub fn generators(&self) -> Vec<&Gen> {
        self.graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Gen(g) => Some(g),
                _ => None,
            })
            .collect()
    }

    /// Get all branches as a vector
    pub fn branches(&self) -> Vec<&Branch> {
        self.graph
            .edge_weights()
            .filter_map(|e| match e {
                Edge::Branch(b) => Some(b),
                _ => None,
            })
            .collect()
    }
}

/// Statistics about a network's size and capacity
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub num_buses: usize,
    pub num_gens: usize,
    pub num_loads: usize,
    pub num_shunts: usize,
    pub num_branches: usize,
    pub total_load_mw: f64,
    pub total_load_mvar: f64,
    pub total_gen_capacity_mw: f64,
    pub total_gen_pmin_mw: f64,
}

impl std::fmt::Display for NetworkStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} buses, {} branches, {} gens ({:.0} MW), {} loads ({:.0} MW)",
            self.num_buses,
            self.num_branches,
            self.num_gens,
            self.total_gen_capacity_mw,
            self.num_loads,
            self.total_load_mw
        )
    }
}


impl Node {
    /// Returns a human-readable label for the node (bus/gen/load/shunt name).
    pub fn label(&self) -> &str {
        match self {
            Node::Bus(bus) => &bus.name,
            Node::Gen(gen) => &gen.name,
            Node::Load(load) => &load.name,
            Node::Shunt(shunt) => &shunt.name,
        }
    }
}

impl Edge {
    /// Returns a human-readable label for the edge (branch/transformer name).
    pub fn label(&self) -> &str {
        match self {
            Edge::Branch(branch) => &branch.name,
            Edge::Transformer(tx) => &tx.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_creation() {
        let mut network = Network::new();

        let bus1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId(0),
            name: "Bus 1".to_string(),
            base_kv: Kilovolts(138.0),
            ..Bus::default()
        }));
        let bus2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId(1),
            name: "Bus 2".to_string(),
            base_kv: Kilovolts(138.0),
            ..Bus::default()
        }));

        let _branch = network.graph.add_edge(
            bus1,
            bus2,
            Edge::Branch(Branch {
                id: BranchId(0),
                name: "Branch 1-2".to_string(),
                from_bus: BusId(0),
                to_bus: BusId(1),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        assert_eq!(network.graph.node_count(), 2);
        assert_eq!(network.graph.edge_count(), 1);

        if let Node::Bus(b) = network.graph[bus1].clone() {
            assert_eq!(b.name, "Bus 1");
        } else {
            panic!("Expected Bus node");
        }
    }

    #[test]
    fn test_network_validation_empty() {
        let network = Network::new();
        let mut diag = Diagnostics::new();
        network.validate_into(&mut diag);
        assert!(diag.has_errors());
        assert!(diag.errors().any(|i| i.message.contains("no buses")));
    }

    #[test]
    fn test_network_validation_no_load() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus {
            id: BusId(0),
            name: "Bus 1".to_string(),
            base_kv: Kilovolts(138.0),
            ..Bus::default()
        }));
        network.graph.add_node(Node::Gen(Gen::new(
            GenId::new(0),
            "Gen 1".to_string(),
            BusId(0),
        )));

        let mut diag = Diagnostics::new();
        network.validate_into(&mut diag);
        // Should warn about no loads
        assert!(diag.warnings().any(|i| i.message.contains("no loads")));
    }

    #[test]
    fn test_network_stats() {
        let mut network = Network::new();
        let bus1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId(0),
            name: "Bus 1".to_string(),
            base_kv: Kilovolts(138.0),
            ..Bus::default()
        }));
        let bus2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId(1),
            name: "Bus 2".to_string(),
            base_kv: Kilovolts(138.0),
            ..Bus::default()
        }));
        let mut gen = Gen::new(GenId::new(0), "Gen 1".to_string(), BusId(0));
        gen.pmax = Megawatts(100.0);
        network.graph.add_node(Node::Gen(gen));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(0),
            name: "Load 1".to_string(),
            bus: BusId(1),
            active_power: Megawatts(50.0),
            reactive_power: Megavars(10.0),
        }));
        network.graph.add_edge(
            bus1,
            bus2,
            Edge::Branch(Branch {
                id: BranchId(0),
                name: "Branch 1-2".to_string(),
                from_bus: BusId(0),
                to_bus: BusId(1),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        let stats = network.stats();
        assert_eq!(stats.num_buses, 2);
        assert_eq!(stats.num_gens, 1);
        assert_eq!(stats.num_loads, 1);
        assert_eq!(stats.num_branches, 1);
        assert!((stats.total_load_mw - 50.0).abs() < 0.01);
        assert!((stats.total_gen_capacity_mw - 100.0).abs() < 0.01);

        // Valid network should have no errors
        let mut diag = Diagnostics::new();
        network.validate_into(&mut diag);
        assert!(!diag.has_errors());
    }

    #[test]
    fn test_synchronous_condenser_flag() {
        // Test that synchronous condenser flag can be set
        let gen = Gen::new(GenId::new(1), "SynCon1".to_string(), BusId::new(1))
            .with_p_limits(-10.0, 0.0) // Consumes up to 10 MW
            .with_q_limits(-100.0, 100.0) // Provides reactive power
            .as_synchronous_condenser();

        assert!(gen.is_synchronous_condenser);
        assert_eq!(gen.pmin.value(), -10.0);
        assert_eq!(gen.pmax.value(), 0.0);
    }

    #[test]
    fn test_total_generation() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus::default()));
        let mut gen1 = Gen::new(GenId::new(1), "Gen1".into(), BusId::new(1));
        gen1.active_power = Megawatts(50.0);
        let mut gen2 = Gen::new(GenId::new(2), "Gen2".into(), BusId::new(1));
        gen2.active_power = Megawatts(30.0);
        network.graph.add_node(Node::Gen(gen1));
        network.graph.add_node(Node::Gen(gen2));

        assert!((network.total_generation_mw() - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_total_load() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus::default()));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load1".into(),
            bus: BusId::new(1),
            active_power: Megawatts(100.0),
            reactive_power: Megavars(20.0),
        }));

        assert!((network.total_load_mw() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_total_capacity() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus::default()));
        let gen1 = Gen::new(GenId::new(1), "Gen1".into(), BusId::new(1))
            .with_p_limits(0.0, 100.0);
        let gen2 = Gen::new(GenId::new(2), "Gen2".into(), BusId::new(1))
            .with_p_limits(0.0, 50.0);
        network.graph.add_node(Node::Gen(gen1));
        network.graph.add_node(Node::Gen(gen2));

        assert!((network.total_capacity_mw() - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_reserve_margin() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus::default()));
        let gen = Gen::new(GenId::new(1), "Gen1".into(), BusId::new(1))
            .with_p_limits(0.0, 150.0);
        network.graph.add_node(Node::Gen(gen));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load1".into(),
            bus: BusId::new(1),
            active_power: Megawatts(100.0),
            reactive_power: Megavars(20.0),
        }));

        // Reserve margin = (capacity - load) / load = (150 - 100) / 100 = 0.5
        assert!((network.reserve_margin() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_generators_at_bus() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus::default()));
        let gen1 = Gen::new(GenId::new(1), "Gen1".into(), BusId::new(1));
        let gen2 = Gen::new(GenId::new(2), "Gen2".into(), BusId::new(1));
        let gen3 = Gen::new(GenId::new(3), "Gen3".into(), BusId::new(2));
        network.graph.add_node(Node::Gen(gen1));
        network.graph.add_node(Node::Gen(gen2));
        network.graph.add_node(Node::Gen(gen3));

        let gens_at_bus1 = network.generators_at_bus(BusId::new(1));
        assert_eq!(gens_at_bus1.len(), 2);

        let gens_at_bus2 = network.generators_at_bus(BusId::new(2));
        assert_eq!(gens_at_bus2.len(), 1);
    }

    #[test]
    fn test_loads_at_bus() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus::default()));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load1".into(),
            bus: BusId::new(1),
            active_power: Megawatts(50.0),
            reactive_power: Megavars(10.0),
        }));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(2),
            name: "Load2".into(),
            bus: BusId::new(2),
            active_power: Megawatts(30.0),
            reactive_power: Megavars(5.0),
        }));

        let loads_at_bus1 = network.loads_at_bus(BusId::new(1));
        assert_eq!(loads_at_bus1.len(), 1);

        let loads_at_bus2 = network.loads_at_bus(BusId::new(2));
        assert_eq!(loads_at_bus2.len(), 1);
    }

    #[test]
    fn test_buses_generators_branches_accessors() {
        let mut network = Network::new();
        let bus1 = network.graph.add_node(Node::Bus(Bus::default()));
        let bus2 = network.graph.add_node(Node::Bus(Bus::default()));
        network.graph.add_node(Node::Gen(Gen::new(GenId::new(1), "Gen1".into(), BusId::new(1))));
        network.graph.add_edge(
            bus1,
            bus2,
            Edge::Branch(Branch::default()),
        );

        assert_eq!(network.buses().len(), 2);
        assert_eq!(network.generators().len(), 1);
        assert_eq!(network.branches().len(), 1);
    }
}
