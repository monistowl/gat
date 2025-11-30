//! Generic NetworkBuilder for constructing power network models
//!
//! This module provides a format-agnostic builder pattern for constructing
//! `gat_core::Network` instances. Importers convert their format-specific
//! data into generic input types, and NetworkBuilder handles the common
//! graph construction, ID mapping, and diagnostics tracking.
//!
//! # Example
//! ```ignore
//! let mut builder = NetworkBuilder::new();
//! builder.add_bus(BusInput { id: 1, name: None, voltage_kv: 230.0 });
//! builder.add_load(LoadInput { bus_id: 1, name: None, active_power_mw: 100.0, reactive_power_mvar: 50.0 });
//! let network = builder.build();
//! ```

use std::collections::HashMap;

use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex, Shunt,
    ShuntId,
};

use super::ImportDiagnostics;

/// Generic input data for bus creation
#[derive(Debug, Clone)]
pub struct BusInput {
    pub id: usize,
    pub name: Option<String>,
    pub voltage_kv: f64,
    pub voltage_pu: Option<f64>,
    pub angle_rad: Option<f64>,
    pub vmin_pu: Option<f64>,
    pub vmax_pu: Option<f64>,
    pub area_id: Option<i64>,
    pub zone_id: Option<i64>,
}

/// Generic input data for load creation
#[derive(Debug, Clone)]
pub struct LoadInput {
    pub bus_id: usize,
    pub name: Option<String>,
    pub active_power_mw: f64,
    pub reactive_power_mvar: f64,
}

/// Generic input data for shunt creation (capacitor/reactor)
#[derive(Debug, Clone)]
pub struct ShuntInput {
    pub bus_id: usize,
    pub name: Option<String>,
    /// Shunt conductance in per-unit (MW at 1.0 p.u. voltage)
    pub gs_pu: f64,
    /// Shunt susceptance in per-unit (MVAr at 1.0 p.u. voltage)
    /// Positive = capacitor, Negative = reactor
    pub bs_pu: f64,
}

/// Generic input data for generator creation
#[derive(Debug, Clone)]
pub struct GenInput {
    pub bus_id: usize,
    pub name: Option<String>,
    pub pg: f64,
    pub qg: f64,
    pub pmin: f64,
    pub pmax: f64,
    pub qmin: f64,
    pub qmax: f64,
    pub voltage_setpoint_pu: Option<f64>,
    pub mbase_mva: Option<f64>,
    pub cost_startup: Option<f64>,
    pub cost_shutdown: Option<f64>,
    pub cost_model: gat_core::CostModel,
    pub is_synchronous_condenser: bool,
}

impl Default for GenInput {
    fn default() -> Self {
        Self {
            bus_id: 0,
            name: None,
            pg: 0.0,
            qg: 0.0,
            pmin: 0.0,
            pmax: f64::INFINITY,
            qmin: f64::NEG_INFINITY,
            qmax: f64::INFINITY,
            voltage_setpoint_pu: None,
            mbase_mva: None,
            cost_startup: None,
            cost_shutdown: None,
            cost_model: gat_core::CostModel::NoCost,
            is_synchronous_condenser: false,
        }
    }
}

/// Generic input data for branch creation
#[derive(Debug, Clone)]
pub struct BranchInput {
    pub from_bus: usize,
    pub to_bus: usize,
    pub name: Option<String>,
    pub resistance: f64,
    pub reactance: f64,
    pub charging_b: f64,
    pub tap_ratio: f64,
    pub phase_shift_rad: f64,
    pub rate_mva: Option<f64>,
    pub rating_b_mva: Option<f64>,
    pub rating_c_mva: Option<f64>,
    pub angle_min_rad: Option<f64>,
    pub angle_max_rad: Option<f64>,
    pub element_type: Option<String>,
    pub is_phase_shifter: bool,
}

impl Default for BranchInput {
    fn default() -> Self {
        Self {
            from_bus: 0,
            to_bus: 0,
            name: None,
            resistance: 0.0,
            reactance: 0.0,
            charging_b: 0.0,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            rate_mva: None,
            rating_b_mva: None,
            rating_c_mva: None,
            angle_min_rad: None,
            angle_max_rad: None,
            element_type: None,
            is_phase_shifter: false,
        }
    }
}

/// Result of adding an element to the network
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddResult {
    /// Element was successfully added
    Added,
    /// Element was skipped (e.g., references unknown bus)
    Skipped,
}

/// Builder for constructing Network from generic inputs.
///
/// Handles the common operations of network construction:
/// - Bus ID to NodeIndex mapping
/// - Sequential ID assignment for loads, generators, branches
/// - Diagnostics tracking (optional)
/// - Orphan detection and error recovery
pub struct NetworkBuilder<'a> {
    network: Network,
    bus_map: HashMap<usize, NodeIndex>,
    diag: Option<&'a mut ImportDiagnostics>,
    next_load_id: usize,
    next_gen_id: usize,
    next_branch_id: usize,
    next_shunt_id: usize,
}

impl<'a> NetworkBuilder<'a> {
    /// Create a new NetworkBuilder without diagnostics
    pub fn new() -> Self {
        Self {
            network: Network::new(),
            bus_map: HashMap::new(),
            diag: None,
            next_load_id: 0,
            next_gen_id: 0,
            next_branch_id: 0,
            next_shunt_id: 0,
        }
    }

    /// Create a new NetworkBuilder with pre-allocated capacity
    ///
    /// Use this when you know the approximate number of buses to avoid
    /// HashMap reallocations during construction.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            network: Network::new(),
            bus_map: HashMap::with_capacity(capacity),
            diag: None,
            next_load_id: 0,
            next_gen_id: 0,
            next_branch_id: 0,
            next_shunt_id: 0,
        }
    }

    /// Create a new NetworkBuilder with diagnostics tracking
    pub fn with_diagnostics(diag: &'a mut ImportDiagnostics) -> Self {
        Self {
            network: Network::new(),
            bus_map: HashMap::new(),
            diag: Some(diag),
            next_load_id: 0,
            next_gen_id: 0,
            next_branch_id: 0,
            next_shunt_id: 0,
        }
    }

    /// Create a new NetworkBuilder with diagnostics and pre-allocated capacity
    ///
    /// Combines diagnostics tracking with capacity hints for optimal performance.
    pub fn with_diagnostics_and_capacity(diag: &'a mut ImportDiagnostics, capacity: usize) -> Self {
        Self {
            network: Network::new(),
            bus_map: HashMap::with_capacity(capacity),
            diag: Some(diag),
            next_load_id: 0,
            next_gen_id: 0,
            next_branch_id: 0,
            next_shunt_id: 0,
        }
    }

    /// Add a bus to the network
    pub fn add_bus(&mut self, input: BusInput) -> NodeIndex {
        let bus_id = BusId::new(input.id);
        let name = input.name.unwrap_or_else(|| format!("Bus {}", input.id));

        let node_idx = self.network.graph.add_node(Node::Bus(Bus {
            id: bus_id,
            name,
            voltage_kv: input.voltage_kv,
            voltage_pu: input.voltage_pu.unwrap_or(1.0),
            angle_rad: input.angle_rad.unwrap_or(0.0),
            vmin_pu: input.vmin_pu,
            vmax_pu: input.vmax_pu,
            area_id: input.area_id,
            zone_id: input.zone_id,
        }));

        self.bus_map.insert(input.id, node_idx);

        if let Some(ref mut diag) = self.diag {
            diag.stats.buses += 1;
        }

        node_idx
    }

    /// Check if a bus exists in the network
    pub fn has_bus(&self, bus_id: usize) -> bool {
        self.bus_map.contains_key(&bus_id)
    }

    /// Get the NodeIndex for a bus ID
    pub fn get_bus_index(&self, bus_id: usize) -> Option<NodeIndex> {
        self.bus_map.get(&bus_id).copied()
    }

    /// Add a load to the network
    ///
    /// Returns `AddResult::Skipped` if the referenced bus doesn't exist.
    pub fn add_load(&mut self, input: LoadInput) -> AddResult {
        if !self.bus_map.contains_key(&input.bus_id) {
            if let Some(ref mut diag) = self.diag {
                diag.add_warning(
                    "orphan_load",
                    &format!("load references unknown bus {}", input.bus_id),
                );
            }
            return AddResult::Skipped;
        }

        let name = input
            .name
            .unwrap_or_else(|| format!("Load {}", input.bus_id));

        self.network.graph.add_node(Node::Load(Load {
            id: LoadId::new(self.next_load_id),
            name,
            bus: BusId::new(input.bus_id),
            active_power_mw: input.active_power_mw,
            reactive_power_mvar: input.reactive_power_mvar,
        }));

        self.next_load_id += 1;

        if let Some(ref mut diag) = self.diag {
            diag.stats.loads += 1;
        }

        AddResult::Added
    }

    /// Add a shunt (capacitor/reactor) to the network
    ///
    /// Returns `AddResult::Skipped` if the referenced bus doesn't exist.
    pub fn add_shunt(&mut self, input: ShuntInput) -> AddResult {
        if !self.bus_map.contains_key(&input.bus_id) {
            if let Some(ref mut diag) = self.diag {
                diag.add_warning(
                    "orphan_shunt",
                    &format!("shunt references unknown bus {}", input.bus_id),
                );
            }
            return AddResult::Skipped;
        }

        let name = input
            .name
            .unwrap_or_else(|| format!("Shunt {}", input.bus_id));

        self.network.graph.add_node(Node::Shunt(Shunt {
            id: ShuntId::new(self.next_shunt_id),
            name,
            bus: BusId::new(input.bus_id),
            gs_pu: input.gs_pu,
            bs_pu: input.bs_pu,
            status: true,
        }));

        self.next_shunt_id += 1;

        // Note: diagnostics doesn't track shunts separately yet, but we could add it
        AddResult::Added
    }

    /// Add a generator to the network
    ///
    /// Returns `AddResult::Skipped` if the referenced bus doesn't exist.
    pub fn add_gen(&mut self, input: GenInput) -> AddResult {
        if !self.bus_map.contains_key(&input.bus_id) {
            if let Some(ref mut diag) = self.diag {
                diag.add_warning(
                    "orphan_generator",
                    &format!("generator references unknown bus {}", input.bus_id),
                );
            }
            return AddResult::Skipped;
        }

        let name = input
            .name
            .unwrap_or_else(|| format!("Gen {}@{}", self.next_gen_id, input.bus_id));

        self.network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(self.next_gen_id),
            name,
            bus: BusId::new(input.bus_id),
            active_power_mw: input.pg,
            reactive_power_mvar: input.qg,
            pmin_mw: input.pmin,
            pmax_mw: input.pmax,
            qmin_mvar: input.qmin,
            qmax_mvar: input.qmax,
            voltage_setpoint_pu: input.voltage_setpoint_pu,
            mbase_mva: input.mbase_mva,
            cost_startup: input.cost_startup,
            cost_shutdown: input.cost_shutdown,
            cost_model: input.cost_model,
            is_synchronous_condenser: input.is_synchronous_condenser,
            ..Gen::default()
        }));

        self.next_gen_id += 1;

        if let Some(ref mut diag) = self.diag {
            diag.stats.generators += 1;
        }

        AddResult::Added
    }

    /// Add a branch to the network
    ///
    /// Returns `AddResult::Skipped` if either referenced bus doesn't exist.
    pub fn add_branch(&mut self, input: BranchInput) -> AddResult {
        let from_idx = match self.bus_map.get(&input.from_bus) {
            Some(idx) => *idx,
            None => {
                if let Some(ref mut diag) = self.diag {
                    diag.add_warning(
                        "orphan_branch",
                        &format!("branch references unknown from bus {}", input.from_bus),
                    );
                }
                return AddResult::Skipped;
            }
        };

        let to_idx = match self.bus_map.get(&input.to_bus) {
            Some(idx) => *idx,
            None => {
                if let Some(ref mut diag) = self.diag {
                    diag.add_warning(
                        "orphan_branch",
                        &format!("branch references unknown to bus {}", input.to_bus),
                    );
                }
                return AddResult::Skipped;
            }
        };

        let name = input
            .name
            .unwrap_or_else(|| format!("Branch {}-{}", input.from_bus, input.to_bus));

        let element_type = input.element_type.unwrap_or_else(|| {
            if input.tap_ratio != 1.0 || input.phase_shift_rad.abs() > 1e-9 {
                "transformer".to_string()
            } else {
                "line".to_string()
            }
        });

        let branch = Branch {
            id: BranchId::new(self.next_branch_id),
            name,
            from_bus: BusId::new(input.from_bus),
            to_bus: BusId::new(input.to_bus),
            resistance: input.resistance,
            reactance: input.reactance,
            tap_ratio: input.tap_ratio,
            phase_shift_rad: input.phase_shift_rad,
            charging_b_pu: input.charging_b,
            s_max_mva: input.rate_mva,
            status: true,
            rating_a_mva: input.rate_mva,
            rating_b_mva: input.rating_b_mva,
            rating_c_mva: input.rating_c_mva,
            angle_min_rad: input.angle_min_rad,
            angle_max_rad: input.angle_max_rad,
            element_type,
            is_phase_shifter: input.is_phase_shifter,
        };

        self.network
            .graph
            .add_edge(from_idx, to_idx, Edge::Branch(branch));
        self.next_branch_id += 1;

        if let Some(ref mut diag) = self.diag {
            diag.stats.branches += 1;
        }

        AddResult::Added
    }

    /// Record skipped elements in diagnostics
    pub fn record_skipped(&mut self, count: usize) {
        if let Some(ref mut diag) = self.diag {
            diag.stats.skipped_lines += count;
        }
    }

    /// Consume the builder and return the constructed network
    pub fn build(self) -> Network {
        self.network
    }

    /// Get current counts for testing/debugging
    pub fn counts(&self) -> (usize, usize, usize, usize) {
        (
            self.bus_map.len(),
            self.next_load_id,
            self.next_gen_id,
            self.next_branch_id,
        )
    }
}

impl Default for NetworkBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_network_construction() {
        let mut builder = NetworkBuilder::new();

        // Add buses
        builder.add_bus(BusInput {
            id: 1,
            name: Some("Bus A".to_string()),
            voltage_kv: 230.0,
            voltage_pu: None,
            angle_rad: None,
            vmin_pu: None,
            vmax_pu: None,
            area_id: None,
            zone_id: None,
        });
        builder.add_bus(BusInput {
            id: 2,
            name: None,
            voltage_kv: 115.0,
            voltage_pu: None,
            angle_rad: None,
            vmin_pu: None,
            vmax_pu: None,
            area_id: None,
            zone_id: None,
        });

        // Add load
        let result = builder.add_load(LoadInput {
            bus_id: 1,
            name: None,
            active_power_mw: 100.0,
            reactive_power_mvar: 50.0,
        });
        assert_eq!(result, AddResult::Added);

        // Add generator
        let result = builder.add_gen(GenInput {
            bus_id: 2,
            name: Some("Gen 1".to_string()),
            pg: 150.0,
            ..Default::default()
        });
        assert_eq!(result, AddResult::Added);

        // Add branch
        let result = builder.add_branch(BranchInput {
            from_bus: 1,
            to_bus: 2,
            resistance: 0.01,
            reactance: 0.1,
            ..Default::default()
        });
        assert_eq!(result, AddResult::Added);

        let network = builder.build();
        assert_eq!(network.graph.node_count(), 4); // 2 buses + 1 load + 1 gen
        assert_eq!(network.graph.edge_count(), 1);
    }

    #[test]
    fn test_orphan_detection() {
        let mut builder = NetworkBuilder::new();

        // Add one bus
        builder.add_bus(BusInput {
            id: 1,
            name: None,
            voltage_kv: 230.0,
            voltage_pu: None,
            angle_rad: None,
            vmin_pu: None,
            vmax_pu: None,
            area_id: None,
            zone_id: None,
        });

        // Try to add load on non-existent bus
        let result = builder.add_load(LoadInput {
            bus_id: 999,
            name: None,
            active_power_mw: 100.0,
            reactive_power_mvar: 50.0,
        });
        assert_eq!(result, AddResult::Skipped);

        // Try to add gen on non-existent bus
        let result = builder.add_gen(GenInput {
            bus_id: 999,
            ..Default::default()
        });
        assert_eq!(result, AddResult::Skipped);

        // Try to add branch with missing from_bus
        let result = builder.add_branch(BranchInput {
            from_bus: 999,
            to_bus: 1,
            ..Default::default()
        });
        assert_eq!(result, AddResult::Skipped);

        // Try to add branch with missing to_bus
        let result = builder.add_branch(BranchInput {
            from_bus: 1,
            to_bus: 999,
            ..Default::default()
        });
        assert_eq!(result, AddResult::Skipped);
    }

    #[test]
    fn test_with_diagnostics() {
        let mut diag = ImportDiagnostics::new();
        let mut builder = NetworkBuilder::with_diagnostics(&mut diag);

        builder.add_bus(BusInput {
            id: 1,
            name: None,
            voltage_kv: 230.0,
            voltage_pu: None,
            angle_rad: None,
            vmin_pu: None,
            vmax_pu: None,
            area_id: None,
            zone_id: None,
        });
        builder.add_load(LoadInput {
            bus_id: 1,
            name: None,
            active_power_mw: 50.0,
            reactive_power_mvar: 25.0,
        });
        builder.add_gen(GenInput {
            bus_id: 1,
            ..Default::default()
        });

        // This should generate a warning
        builder.add_load(LoadInput {
            bus_id: 999,
            name: None,
            active_power_mw: 50.0,
            reactive_power_mvar: 25.0,
        });

        builder.record_skipped(3);

        let _network = builder.build();

        assert_eq!(diag.stats.buses, 1);
        assert_eq!(diag.stats.loads, 1);
        assert_eq!(diag.stats.generators, 1);
        assert_eq!(diag.stats.skipped_lines, 3);
        assert!(!diag.issues.is_empty());
    }

    #[test]
    fn test_sequential_ids() {
        let mut builder = NetworkBuilder::new();

        builder.add_bus(BusInput {
            id: 100,
            name: None,
            voltage_kv: 230.0,
            voltage_pu: None,
            angle_rad: None,
            vmin_pu: None,
            vmax_pu: None,
            area_id: None,
            zone_id: None,
        });
        builder.add_bus(BusInput {
            id: 200,
            name: None,
            voltage_kv: 115.0,
            voltage_pu: None,
            angle_rad: None,
            vmin_pu: None,
            vmax_pu: None,
            area_id: None,
            zone_id: None,
        });

        // Add multiple loads - IDs should be 0, 1, 2
        for _ in 0..3 {
            builder.add_load(LoadInput {
                bus_id: 100,
                name: None,
                active_power_mw: 10.0,
                reactive_power_mvar: 5.0,
            });
        }

        let (buses, loads, gens, branches) = builder.counts();
        assert_eq!(buses, 2);
        assert_eq!(loads, 3);
        assert_eq!(gens, 0);
        assert_eq!(branches, 0);
    }
}
