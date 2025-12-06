//! PowerModels.jl JSON format importer
//!
//! PowerModels.jl is a Julia package for power system optimization.
//! It uses a JSON format with dictionary-based component storage.
//!
//! Reference: <https://lanl-ansi.github.io/PowerModels.jl/stable/network-data/>

use std::{collections::HashMap, fs, path::Path};

use anyhow::{anyhow, Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
    NodeIndex,
};
use serde::Deserialize;
use serde_json::Value;

use crate::helpers::{ImportDiagnostics, ImportResult};

/// Top-level PowerModels.jl JSON structure
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)] // Field names match PowerModels JSON schema
pub struct PowerModelsJson {
    /// Case name
    #[serde(default)]
    pub name: String,
    /// Whether data is in per-unit
    #[serde(default = "default_per_unit")]
    pub per_unit: bool,
    /// Base MVA for per-unit conversion
    #[serde(default = "default_base_mva")]
    pub baseMVA: f64,
    /// Bus data (index -> bus)
    #[serde(default)]
    pub bus: HashMap<String, BusData>,
    /// Generator data (index -> gen)
    #[serde(default)]
    pub gen: HashMap<String, GenData>,
    /// Load data (index -> load)
    #[serde(default)]
    pub load: HashMap<String, LoadData>,
    /// Branch data (index -> branch)
    #[serde(default)]
    pub branch: HashMap<String, BranchData>,
    /// Shunt data (index -> shunt)
    #[serde(default)]
    pub shunt: HashMap<String, ShuntData>,
    /// Extra fields we don't process
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

fn default_per_unit() -> bool {
    true
}

fn default_base_mva() -> f64 {
    100.0
}

/// PowerModels bus data
#[derive(Debug, Deserialize)]
pub struct BusData {
    /// Bus index (1-based)
    #[serde(alias = "bus_i")]
    pub index: i64,
    /// Status (1=active, 0=inactive)
    #[serde(default = "default_status")]
    pub status: i64,
    /// Voltage angle (radians)
    #[serde(default)]
    pub va: f64,
    /// Voltage magnitude (p.u.)
    #[serde(default = "default_vm")]
    pub vm: f64,
    /// Minimum voltage (p.u.)
    #[serde(default = "default_vmin")]
    pub vmin: f64,
    /// Maximum voltage (p.u.)
    #[serde(default = "default_vmax")]
    pub vmax: f64,
    /// Base voltage (kV)
    #[serde(default)]
    pub base_kv: f64,
    /// Bus type (1=PQ, 2=PV, 3=ref, 4=isolated)
    #[serde(default = "default_bus_type")]
    pub bus_type: i64,
    /// Bus name
    #[serde(default)]
    pub name: String,
    /// Area ID
    #[serde(default)]
    pub area: Option<i64>,
    /// Zone ID
    #[serde(default)]
    pub zone: Option<i64>,
}

fn default_status() -> i64 {
    1
}

fn default_vm() -> f64 {
    1.0
}

fn default_vmin() -> f64 {
    0.9
}

fn default_vmax() -> f64 {
    1.1
}

fn default_bus_type() -> i64 {
    1
}

/// PowerModels generator data
#[derive(Debug, Deserialize)]
pub struct GenData {
    /// Generator index (1-based)
    pub index: i64,
    /// Status (1=active, 0=inactive)
    #[serde(default = "default_status")]
    pub gen_status: i64,
    /// Bus this generator is connected to
    pub gen_bus: i64,
    /// Active power output (MW)
    #[serde(default)]
    pub pg: f64,
    /// Reactive power output (MVAr)
    #[serde(default)]
    pub qg: f64,
    /// Minimum active power (MW)
    #[serde(default)]
    pub pmin: f64,
    /// Maximum active power (MW)
    pub pmax: f64,
    /// Minimum reactive power (MVAr)
    #[serde(default)]
    pub qmin: f64,
    /// Maximum reactive power (MVAr)
    #[serde(default)]
    pub qmax: f64,
    /// Voltage setpoint (p.u.)
    #[serde(default = "default_vm")]
    pub vg: f64,
    /// Machine base (MVA)
    #[serde(default = "default_base_mva")]
    pub mbase: f64,
    /// Cost model (1=piecewise linear, 2=polynomial)
    #[serde(default = "default_cost_model")]
    pub model: i64,
    /// Number of cost coefficients
    #[serde(default)]
    pub ncost: i64,
    /// Cost coefficients
    #[serde(default)]
    pub cost: Vec<f64>,
    /// Generator name
    #[serde(default)]
    pub name: String,
}

fn default_cost_model() -> i64 {
    2
}

/// PowerModels load data
#[derive(Debug, Deserialize)]
pub struct LoadData {
    /// Load index (1-based)
    pub index: i64,
    /// Status (1=active, 0=inactive)
    #[serde(default = "default_status")]
    pub status: i64,
    /// Bus this load is connected to
    pub load_bus: i64,
    /// Active power demand (MW)
    #[serde(default)]
    pub pd: f64,
    /// Reactive power demand (MVAr)
    #[serde(default)]
    pub qd: f64,
    /// Load name
    #[serde(default)]
    pub name: String,
}

/// PowerModels branch data
#[derive(Debug, Deserialize)]
pub struct BranchData {
    /// Branch index (1-based)
    pub index: i64,
    /// Status (1=active, 0=inactive)
    #[serde(default = "default_status")]
    pub br_status: i64,
    /// From bus
    pub f_bus: i64,
    /// To bus
    pub t_bus: i64,
    /// Resistance (p.u.)
    #[serde(default)]
    pub br_r: f64,
    /// Reactance (p.u.)
    pub br_x: f64,
    /// Tap ratio (p.u., 0 or 1 for lines)
    #[serde(default = "default_tap")]
    pub tap: f64,
    /// Phase shift (radians)
    #[serde(default)]
    pub shift: f64,
    /// From-side shunt conductance (p.u.)
    #[serde(default)]
    pub g_fr: f64,
    /// From-side shunt susceptance (p.u.)
    #[serde(default)]
    pub b_fr: f64,
    /// To-side shunt conductance (p.u.)
    #[serde(default)]
    pub g_to: f64,
    /// To-side shunt susceptance (p.u.)
    #[serde(default)]
    pub b_to: f64,
    /// Is this a transformer?
    #[serde(default)]
    pub transformer: bool,
    /// Rating A (MVA)
    #[serde(default)]
    pub rate_a: Option<f64>,
    /// Rating B (MVA)
    #[serde(default)]
    pub rate_b: Option<f64>,
    /// Rating C (MVA)
    #[serde(default)]
    pub rate_c: Option<f64>,
    /// Minimum angle difference (radians)
    #[serde(default = "default_angmin")]
    pub angmin: f64,
    /// Maximum angle difference (radians)
    #[serde(default = "default_angmax")]
    pub angmax: f64,
    /// Branch name
    #[serde(default)]
    pub name: String,
}

fn default_tap() -> f64 {
    1.0
}

fn default_angmin() -> f64 {
    -std::f64::consts::PI / 3.0 // -60 degrees
}

fn default_angmax() -> f64 {
    std::f64::consts::PI / 3.0 // +60 degrees
}

/// PowerModels shunt data
#[derive(Debug, Deserialize)]
pub struct ShuntData {
    /// Shunt index (1-based)
    pub index: i64,
    /// Status (1=active, 0=inactive)
    #[serde(default = "default_status")]
    pub status: i64,
    /// Bus this shunt is connected to
    pub shunt_bus: i64,
    /// Shunt conductance (p.u.)
    #[serde(default)]
    pub gs: f64,
    /// Shunt susceptance (p.u.)
    #[serde(default)]
    pub bs: f64,
    /// Shunt name
    #[serde(default)]
    pub name: String,
}

// =============================================================================
// Import Functions
// =============================================================================

/// Parse a PowerModels.jl JSON file from a path.
///
/// Returns a [`gat_core::Network`] and import diagnostics.
pub fn parse_powermodels<P: AsRef<Path>>(path: P) -> Result<ImportResult> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("reading PowerModels file: {:?}", path.as_ref()))?;
    parse_powermodels_string(&content)
}

/// Parse PowerModels.jl JSON from a string.
pub fn parse_powermodels_string(content: &str) -> Result<ImportResult> {
    let pm: PowerModelsJson =
        serde_json::from_str(content).with_context(|| "parsing PowerModels JSON")?;

    let mut network = Network::new();
    let mut diagnostics = ImportDiagnostics::new();
    let mut bus_idx_map: HashMap<i64, NodeIndex> = HashMap::new();

    // Import buses
    for (_, bus_data) in &pm.bus {
        match import_bus(bus_data) {
            Ok(bus) => {
                let node_idx = network.graph.add_node(Node::Bus(bus.clone()));
                bus_idx_map.insert(bus_data.index, node_idx);
            }
            Err(e) => diagnostics.add_warning("import", &format!("Bus {}: {}", bus_data.index, e)),
        }
    }

    // Import generators
    for (_, gen_data) in &pm.gen {
        match import_gen(gen_data, &bus_idx_map) {
            Ok(gen) => {
                network.graph.add_node(Node::Gen(gen));
            }
            Err(e) => diagnostics.add_warning("import", &format!("Gen {}: {}", gen_data.index, e)),
        }
    }

    // Import loads
    for (_, load_data) in &pm.load {
        match import_load(load_data, &bus_idx_map) {
            Ok(load) => {
                network.graph.add_node(Node::Load(load));
            }
            Err(e) => {
                diagnostics.add_warning("import", &format!("Load {}: {}", load_data.index, e))
            }
        }
    }

    // Import branches
    for (_, branch_data) in &pm.branch {
        match import_branch(branch_data, &bus_idx_map) {
            Ok((branch, from_idx, to_idx)) => {
                network
                    .graph
                    .add_edge(from_idx, to_idx, Edge::Branch(branch));
            }
            Err(e) => {
                diagnostics.add_warning("import", &format!("Branch {}: {}", branch_data.index, e))
            }
        }
    }

    // TODO: Import shunts (add to bus or as separate elements)

    Ok(ImportResult {
        network,
        diagnostics,
    })
}

/// Load a PowerModels.jl network from a path (convenience function).
pub fn load_powermodels_network<P: AsRef<Path>>(path: P) -> Result<Network> {
    let result = parse_powermodels(path)?;
    if result.diagnostics.has_errors() {
        return Err(anyhow!(
            "PowerModels import had {} errors",
            result.diagnostics.error_count()
        ));
    }
    Ok(result.network)
}

// =============================================================================
// Internal Import Helpers
// =============================================================================

fn import_bus(data: &BusData) -> Result<Bus> {
    let bus_id = data.index as usize;
    let name = if data.name.is_empty() {
        format!("Bus_{}", bus_id)
    } else {
        data.name.clone()
    };

    Ok(Bus {
        id: BusId::new(bus_id),
        name,
        base_kv: gat_core::Kilovolts(if data.base_kv > 0.0 {
            data.base_kv
        } else {
            // Default to 138 kV if not specified
            138.0
        }),
        voltage_pu: gat_core::PerUnit(data.vm),
        angle_rad: gat_core::Radians(data.va),
        vmax_pu: Some(gat_core::PerUnit(data.vmax)),
        vmin_pu: Some(gat_core::PerUnit(data.vmin)),
        area_id: data.area,
        zone_id: data.zone,
    })
}

fn import_gen(data: &GenData, bus_map: &HashMap<i64, NodeIndex>) -> Result<Gen> {
    // Verify bus exists
    if !bus_map.contains_key(&data.gen_bus) {
        return Err(anyhow!("generator bus {} not found", data.gen_bus));
    }

    let gen_id = data.index as usize;
    let name = if data.name.is_empty() {
        format!("Gen_{}", gen_id)
    } else {
        data.name.clone()
    };

    // Convert cost coefficients based on model type
    // Model 2 = polynomial: cost = c0 + c1*P + c2*P^2
    let cost_model = if data.model == 2 && !data.cost.is_empty() {
        // PowerModels stores coefficients as [cn, cn-1, ..., c1, c0]
        let n = data.cost.len();
        let c0 = if n >= 1 { data.cost[n - 1] } else { 0.0 };
        let c1 = if n >= 2 { data.cost[n - 2] } else { 0.0 };
        let c2 = if n >= 3 { data.cost[n - 3] } else { 0.0 };
        CostModel::Polynomial(vec![c0, c1, c2])
    } else if data.model == 1 && !data.cost.is_empty() {
        // Piecewise linear: pairs of (mw, cost)
        let points: Vec<(f64, f64)> = data
            .cost
            .chunks(2)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    Some((chunk[0], chunk[1]))
                } else {
                    None
                }
            })
            .collect();
        CostModel::PiecewiseLinear(points)
    } else {
        CostModel::NoCost
    };

    Ok(Gen {
        id: GenId::new(gen_id),
        name,
        bus: BusId::new(data.gen_bus as usize),
        active_power: gat_core::Megawatts(data.pg),
        reactive_power: gat_core::Megavars(data.qg),
        pmax: gat_core::Megawatts(data.pmax),
        pmin: gat_core::Megawatts(data.pmin),
        qmax: gat_core::Megavars(data.qmax),
        qmin: gat_core::Megavars(data.qmin),
        voltage_setpoint: Some(gat_core::PerUnit(data.vg)),
        mbase: Some(gat_core::MegavoltAmperes(data.mbase)),
        status: data.gen_status == 1,
        cost_model,
        ..Gen::default()
    })
}

fn import_load(data: &LoadData, bus_map: &HashMap<i64, NodeIndex>) -> Result<Load> {
    // Verify bus exists
    if !bus_map.contains_key(&data.load_bus) {
        return Err(anyhow!("load bus {} not found", data.load_bus));
    }

    let load_id = data.index as usize;
    let name = if data.name.is_empty() {
        format!("Load_{}", load_id)
    } else {
        data.name.clone()
    };

    Ok(Load {
        id: LoadId::new(load_id),
        name,
        bus: BusId::new(data.load_bus as usize),
        active_power: gat_core::Megawatts(data.pd),
        reactive_power: gat_core::Megavars(data.qd),
    })
}

fn import_branch(
    data: &BranchData,
    bus_map: &HashMap<i64, NodeIndex>,
) -> Result<(Branch, NodeIndex, NodeIndex)> {
    // Verify buses exist
    let from_idx = bus_map
        .get(&data.f_bus)
        .ok_or_else(|| anyhow!("from bus {} not found", data.f_bus))?;
    let to_idx = bus_map
        .get(&data.t_bus)
        .ok_or_else(|| anyhow!("to bus {} not found", data.t_bus))?;

    let branch_id = data.index as usize;
    let name = if data.name.is_empty() {
        format!("Branch_{}", branch_id)
    } else {
        data.name.clone()
    };

    // Total charging susceptance = b_fr + b_to (both sides combined)
    let charging_b = data.b_fr + data.b_to;

    // Tap ratio: PowerModels uses 0 or 1 for lines, actual ratio for transformers
    let tap = if data.tap == 0.0 { 1.0 } else { data.tap };

    Ok((
        Branch {
            id: BranchId::new(branch_id),
            name,
            from_bus: BusId::new(data.f_bus as usize),
            to_bus: BusId::new(data.t_bus as usize),
            resistance: data.br_r,
            reactance: data.br_x,
            charging_b: gat_core::PerUnit(charging_b),
            tap_ratio: tap,
            phase_shift: gat_core::Radians(data.shift), // Already in radians from PowerModels
            rating_a: data.rate_a.map(gat_core::MegavoltAmperes),
            rating_b: data.rate_b.map(gat_core::MegavoltAmperes),
            rating_c: data.rate_c.map(gat_core::MegavoltAmperes),
            status: data.br_status == 1,
            angle_min: Some(gat_core::Radians(data.angmin)),
            angle_max: Some(gat_core::Radians(data.angmax)),
            element_type: if data.transformer {
                "transformer".to_string()
            } else {
                "line".to_string()
            },
            ..Branch::default()
        },
        *from_idx,
        *to_idx,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_powermodels() {
        let json = r#"{
            "name": "test_case",
            "baseMVA": 100.0,
            "per_unit": true,
            "bus": {
                "1": {"index": 1, "bus_type": 3, "vm": 1.0, "va": 0.0, "vmin": 0.9, "vmax": 1.1},
                "2": {"index": 2, "bus_type": 1, "vm": 1.0, "va": 0.0, "vmin": 0.9, "vmax": 1.1}
            },
            "gen": {
                "1": {"index": 1, "gen_bus": 1, "pg": 100.0, "qg": 50.0, "pmax": 200.0, "pmin": 0.0, "qmax": 100.0, "qmin": -100.0, "gen_status": 1}
            },
            "load": {
                "1": {"index": 1, "load_bus": 2, "pd": 100.0, "qd": 50.0, "status": 1}
            },
            "branch": {
                "1": {"index": 1, "f_bus": 1, "t_bus": 2, "br_r": 0.01, "br_x": 0.1, "br_status": 1}
            }
        }"#;

        let result = parse_powermodels_string(json).expect("should parse");
        let network = result.network;

        // Check bus count (2 buses + 1 gen + 1 load = 4 nodes)
        assert_eq!(network.graph.node_count(), 4);

        // Check branch count
        assert_eq!(network.graph.edge_count(), 1);

        // Verify no errors
        assert!(!result.diagnostics.has_errors());
    }

    #[test]
    fn test_parse_with_cost_coefficients() {
        let json = r#"{
            "baseMVA": 100.0,
            "bus": {
                "1": {"index": 1, "bus_type": 3, "vm": 1.0}
            },
            "gen": {
                "1": {
                    "index": 1,
                    "gen_bus": 1,
                    "pg": 100.0,
                    "pmax": 200.0,
                    "gen_status": 1,
                    "model": 2,
                    "ncost": 3,
                    "cost": [0.01, 10.0, 100.0]
                }
            }
        }"#;

        let result = parse_powermodels_string(json).expect("should parse");
        let network = result.network;

        // Find the generator and check cost model
        for node_idx in network.graph.node_indices() {
            if let Node::Gen(gen) = &network.graph[node_idx] {
                // PowerModels stores [c2, c1, c0] = [0.01, 10.0, 100.0]
                // We convert to [c0, c1, c2] = [100.0, 10.0, 0.01]
                match &gen.cost_model {
                    CostModel::Polynomial(coeffs) => {
                        assert_eq!(coeffs.len(), 3);
                        assert!((coeffs[0] - 100.0).abs() < 1e-10); // c0
                        assert!((coeffs[1] - 10.0).abs() < 1e-10); // c1
                        assert!((coeffs[2] - 0.01).abs() < 1e-10); // c2
                    }
                    _ => panic!("Expected polynomial cost model"),
                }
            }
        }
    }

    #[test]
    fn test_parse_transformer_branch() {
        let json = r#"{
            "baseMVA": 100.0,
            "bus": {
                "1": {"index": 1, "bus_type": 3, "vm": 1.0, "base_kv": 230.0},
                "2": {"index": 2, "bus_type": 1, "vm": 1.0, "base_kv": 115.0}
            },
            "branch": {
                "1": {
                    "index": 1,
                    "f_bus": 1,
                    "t_bus": 2,
                    "br_r": 0.001,
                    "br_x": 0.05,
                    "tap": 0.95,
                    "shift": 0.0,
                    "transformer": true,
                    "rate_a": 100.0,
                    "br_status": 1
                }
            }
        }"#;

        let result = parse_powermodels_string(json).expect("should parse");
        let network = result.network;

        // Find the branch and check transformer flag
        for edge_idx in network.graph.edge_indices() {
            if let Edge::Branch(branch) = &network.graph[edge_idx] {
                assert_eq!(branch.tap_ratio, 0.95);
                assert_eq!(branch.element_type, "transformer");
                assert_eq!(branch.rating_a.map(|v| v.value()), Some(100.0));
            }
        }
    }

    #[test]
    fn test_bus_not_found_warning() {
        let json = r#"{
            "baseMVA": 100.0,
            "bus": {
                "1": {"index": 1, "bus_type": 3, "vm": 1.0}
            },
            "gen": {
                "1": {"index": 1, "gen_bus": 999, "pg": 100.0, "pmax": 200.0, "gen_status": 1}
            }
        }"#;

        let result = parse_powermodels_string(json).expect("should parse");

        // Should have warning about missing bus
        assert!(result.diagnostics.warning_count() > 0);
    }
}
