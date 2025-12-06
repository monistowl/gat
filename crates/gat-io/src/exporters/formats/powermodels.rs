//! PowerModels.jl JSON exporter
//!
//! Serializes GAT networks into the PowerModels.jl JSON format used by the Julia
//! PowerModels package for power system optimization.
//!
//! Reference: <https://lanl-ansi.github.io/PowerModels.jl/stable/network-data/>

use crate::exporters::ExportMetadata;
use anyhow::Result;
use gat_core::{Branch, Bus, CostModel, Edge, Gen, Load, Network, Node};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::{fs, path::Path};

/// PowerModels bus export data
#[derive(Serialize)]
struct BusExport {
    index: usize,
    bus_type: i32,
    vm: f64,
    va: f64,
    vmin: f64,
    vmax: f64,
    base_kv: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    area: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zone: Option<i64>,
    name: String,
    status: i32,
}

/// PowerModels generator export data
#[derive(Serialize)]
struct GenExport {
    index: usize,
    gen_bus: usize,
    gen_status: i32,
    pg: f64,
    qg: f64,
    pmin: f64,
    pmax: f64,
    qmin: f64,
    qmax: f64,
    vg: f64,
    mbase: f64,
    model: i32,
    ncost: i32,
    cost: Vec<f64>,
    name: String,
}

/// PowerModels load export data
#[derive(Serialize)]
struct LoadExport {
    index: usize,
    load_bus: usize,
    status: i32,
    pd: f64,
    qd: f64,
    name: String,
}

/// PowerModels branch export data
#[derive(Serialize)]
struct BranchExport {
    index: usize,
    f_bus: usize,
    t_bus: usize,
    br_status: i32,
    br_r: f64,
    br_x: f64,
    tap: f64,
    shift: f64,
    b_fr: f64,
    b_to: f64,
    g_fr: f64,
    g_to: f64,
    transformer: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_a: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_b: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_c: Option<f64>,
    angmin: f64,
    angmax: f64,
    name: String,
}

/// Export a Network to PowerModels.jl JSON format
///
/// # Arguments
///
/// * `network` - The Network to export
/// * `output_path` - Path to write the JSON file
/// * `metadata` - Optional metadata to include
///
/// # Example
///
/// ```no_run
/// use gat_io::exporters::formats::export_network_to_powermodels;
/// use gat_io::importers::load_grid_from_arrow;
///
/// # fn main() -> anyhow::Result<()> {
/// let network = load_grid_from_arrow("grid.arrow")?;
/// export_network_to_powermodels(&network, "output.json", None)?;
/// # Ok(())
/// # }
/// ```
pub fn export_network_to_powermodels(
    network: &Network,
    output_path: impl AsRef<Path>,
    metadata: Option<&ExportMetadata>,
) -> Result<()> {
    let json = export_network_to_powermodels_string(network, metadata)?;
    fs::write(output_path, json)?;
    Ok(())
}

/// Export a Network to PowerModels.jl JSON format as a string
pub fn export_network_to_powermodels_string(
    network: &Network,
    metadata: Option<&ExportMetadata>,
) -> Result<String> {
    // Collect buses
    let mut buses: Vec<Bus> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Bus(bus) => Some(bus.clone()),
            _ => None,
        })
        .collect();
    buses.sort_by_key(|bus| bus.id.value());

    // Collect generators
    let gens: Vec<Gen> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Gen(gen) => Some(gen.clone()),
            _ => None,
        })
        .collect();

    // Collect loads
    let loads: Vec<Load> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Load(load) => Some(load.clone()),
            _ => None,
        })
        .collect();

    // Collect branches
    let branches: Vec<Branch> = network
        .graph
        .edge_weights()
        .filter_map(|edge| match edge {
            Edge::Branch(branch) => Some(branch.clone()),
            _ => None,
        })
        .collect();

    // Build bus dictionary
    let bus_dict: HashMap<String, BusExport> = buses
        .iter()
        .map(|bus| {
            let bus_type = determine_bus_type(bus, &gens);
            (
                bus.id.value().to_string(),
                BusExport {
                    index: bus.id.value(),
                    bus_type,
                    vm: bus.voltage_pu.value(),
                    va: bus.angle_rad.value(),
                    vmin: bus.vmin_pu.map(|v| v.value()).unwrap_or(0.9),
                    vmax: bus.vmax_pu.map(|v| v.value()).unwrap_or(1.1),
                    base_kv: bus.base_kv.value(),
                    area: bus.area_id,
                    zone: bus.zone_id,
                    name: bus.name.clone(),
                    status: 1,
                },
            )
        })
        .collect();

    // Build generator dictionary
    let gen_dict: HashMap<String, GenExport> = gens
        .iter()
        .map(|gen| {
            let (model, ncost, cost) = export_cost_model(&gen.cost_model);
            (
                gen.id.value().to_string(),
                GenExport {
                    index: gen.id.value(),
                    gen_bus: gen.bus.value(),
                    gen_status: if gen.status { 1 } else { 0 },
                    pg: gen.active_power.value(),
                    qg: gen.reactive_power.value(),
                    pmin: gen.pmin.value(),
                    pmax: gen.pmax.value(),
                    qmin: gen.qmin.value(),
                    qmax: gen.qmax.value(),
                    vg: gen.voltage_setpoint.map(|v| v.value()).unwrap_or(1.0),
                    mbase: gen.mbase.map(|v| v.value()).unwrap_or(100.0),
                    model,
                    ncost,
                    cost,
                    name: gen.name.clone(),
                },
            )
        })
        .collect();

    // Build load dictionary
    let load_dict: HashMap<String, LoadExport> = loads
        .iter()
        .map(|load| {
            (
                load.id.value().to_string(),
                LoadExport {
                    index: load.id.value(),
                    load_bus: load.bus.value(),
                    status: 1,
                    pd: load.active_power.value(),
                    qd: load.reactive_power.value(),
                    name: load.name.clone(),
                },
            )
        })
        .collect();

    // Build branch dictionary
    let branch_dict: HashMap<String, BranchExport> = branches
        .iter()
        .map(|branch| {
            // Split charging susceptance half/half
            let b_half = branch.charging_b.value() / 2.0;
            let is_transformer = branch.element_type == "transformer"
                || branch.tap_ratio != 1.0
                || branch.phase_shift.value() != 0.0;

            (
                branch.id.value().to_string(),
                BranchExport {
                    index: branch.id.value(),
                    f_bus: branch.from_bus.value(),
                    t_bus: branch.to_bus.value(),
                    br_status: if branch.status { 1 } else { 0 },
                    br_r: branch.resistance,
                    br_x: branch.reactance,
                    tap: branch.tap_ratio,
                    shift: branch.phase_shift.value(), // PowerModels uses radians
                    b_fr: b_half,
                    b_to: b_half,
                    g_fr: 0.0,
                    g_to: 0.0,
                    transformer: is_transformer,
                    rate_a: branch.rating_a.map(|v| v.value()).or(branch.s_max.map(|v| v.value())),
                    rate_b: branch.rating_b.map(|v| v.value()),
                    rate_c: branch.rating_c.map(|v| v.value()),
                    angmin: branch.angle_min.map(|v| v.value()).unwrap_or(-std::f64::consts::PI / 3.0),
                    angmax: branch.angle_max.map(|v| v.value()).unwrap_or(std::f64::consts::PI / 3.0),
                    name: branch.name.clone(),
                },
            )
        })
        .collect();

    // Build root object
    let case_name = metadata
        .and_then(|m| m.source.as_ref())
        .map(|s| s.file.clone())
        .unwrap_or_else(|| "network".to_string());

    let mut root = json!({
        "name": case_name,
        "per_unit": true,
        "baseMVA": 100.0,
        "bus": bus_dict,
        "gen": gen_dict,
        "load": load_dict,
        "branch": branch_dict,
        "shunt": {},
    });

    // Add metadata if provided
    if let Some(meta) = metadata {
        let mut meta_obj = serde_json::Map::new();
        if let Some(source) = &meta.source {
            meta_obj.insert(
                "source".to_string(),
                json!({
                    "file": source.file,
                    "format": source.format,
                    "hash": source.file_hash,
                }),
            );
        }
        if let Some(ts) = meta.creation_timestamp() {
            meta_obj.insert("created_at".to_string(), Value::String(ts));
        }
        if let Some(version) = meta.gat_version() {
            meta_obj.insert(
                "gat_version".to_string(),
                Value::String(version.to_string()),
            );
        }
        if !meta_obj.is_empty() {
            if let Some(obj) = root.as_object_mut() {
                obj.insert("_meta".to_string(), Value::Object(meta_obj));
            }
        }
    }

    Ok(serde_json::to_string_pretty(&root)?)
}

/// Determine bus type: 1=PQ, 2=PV, 3=ref (slack)
fn determine_bus_type(bus: &Bus, gens: &[Gen]) -> i32 {
    // Check if this bus has a generator
    let has_gen = gens.iter().any(|g| g.bus == bus.id && g.status);

    if has_gen {
        // First generator bus with the largest capacity is typically the slack
        // For now, assume PV type for generator buses
        2 // PV bus
    } else {
        1 // PQ bus (load bus)
    }
}

/// Convert CostModel to PowerModels format (model, ncost, cost coefficients)
fn export_cost_model(cost_model: &CostModel) -> (i32, i32, Vec<f64>) {
    match cost_model {
        CostModel::NoCost => (2, 0, vec![]), // Polynomial with no cost
        CostModel::Polynomial(coeffs) => {
            // CostModel stores [c0, c1, c2, ...]
            // PowerModels expects [cn, cn-1, ..., c1, c0]
            let mut reversed: Vec<f64> = coeffs.iter().rev().cloned().collect();
            // Trim leading zeros for cleaner output
            while reversed.len() > 1 && reversed[0] == 0.0 {
                reversed.remove(0);
            }
            let ncost = reversed.len() as i32;
            (2, ncost, reversed) // model=2 for polynomial
        }
        CostModel::PiecewiseLinear(points) => {
            // Points are (mw, cost) pairs
            // PowerModels expects flattened [mw1, cost1, mw2, cost2, ...]
            let flattened: Vec<f64> = points
                .iter()
                .flat_map(|(mw, cost)| vec![*mw, *cost])
                .collect();
            let ncost = points.len() as i32;
            (1, ncost, flattened) // model=1 for piecewise linear
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{BranchId, BusId, GenId, LoadId};

    fn create_test_network() -> Network {
        let mut network = Network::new();

        // Add buses
        let bus1 = Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            voltage_pu: gat_core::PerUnit(1.0),
            angle_rad: gat_core::Radians(0.0),
            vmax_pu: Some(gat_core::PerUnit(1.1)),
            vmin_pu: Some(gat_core::PerUnit(0.9)),
            area_id: Some(1),
            zone_id: Some(1),
        };
        let bus2 = Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            voltage_pu: gat_core::PerUnit(1.0),
            angle_rad: gat_core::Radians(0.0),
            vmax_pu: Some(gat_core::PerUnit(1.1)),
            vmin_pu: Some(gat_core::PerUnit(0.9)),
            area_id: Some(1),
            zone_id: Some(1),
        };

        let b1_idx = network.graph.add_node(Node::Bus(bus1));
        let b2_idx = network.graph.add_node(Node::Bus(bus2));

        // Add generator at bus 1
        let gen = Gen {
            id: GenId::new(1),
            name: "Gen 1".to_string(),
            bus: BusId::new(1),
            active_power: gat_core::Megawatts(100.0),
            reactive_power: gat_core::Megavars(50.0),
            pmin: gat_core::Megawatts(10.0),
            pmax: gat_core::Megawatts(200.0),
            qmin: gat_core::Megavars(-100.0),
            qmax: gat_core::Megavars(100.0),
            status: true,
            voltage_setpoint: Some(gat_core::PerUnit(1.0)),
            mbase: Some(gat_core::MegavoltAmperes(100.0)),
            cost_model: CostModel::Polynomial(vec![100.0, 10.0, 0.01]), // c0=100, c1=10, c2=0.01
            ..Gen::default()
        };
        network.graph.add_node(Node::Gen(gen));

        // Add load at bus 2
        let load = Load {
            id: LoadId::new(1),
            name: "Load 1".to_string(),
            bus: BusId::new(2),
            active_power: gat_core::Megawatts(90.0),
            reactive_power: gat_core::Megavars(40.0),
        };
        network.graph.add_node(Node::Load(load));

        // Add branch
        let branch = Branch {
            id: BranchId::new(1),
            name: "Branch 1-2".to_string(),
            from_bus: BusId::new(1),
            to_bus: BusId::new(2),
            resistance: 0.01,
            reactance: 0.1,
            charging_b: gat_core::PerUnit(0.02),
            tap_ratio: 1.0,
            phase_shift: gat_core::Radians(0.0),
            status: true,
            rating_a: Some(gat_core::MegavoltAmperes(100.0)),
            element_type: "line".to_string(),
            ..Branch::default()
        };
        network.graph.add_edge(b1_idx, b2_idx, Edge::Branch(branch));

        network
    }

    #[test]
    fn test_export_basic_network() {
        let network = create_test_network();
        let json_str = export_network_to_powermodels_string(&network, None).expect("export failed");

        // Parse back and verify structure
        let json: serde_json::Value = serde_json::from_str(&json_str).expect("parse failed");

        assert!(json["baseMVA"].as_f64().is_some());
        assert!(json["bus"].is_object());
        assert!(json["gen"].is_object());
        assert!(json["load"].is_object());
        assert!(json["branch"].is_object());
    }

    #[test]
    fn test_export_cost_coefficients() {
        let network = create_test_network();
        let json_str = export_network_to_powermodels_string(&network, None).expect("export failed");
        let json: serde_json::Value = serde_json::from_str(&json_str).expect("parse failed");

        // Check generator cost model
        let gen = &json["gen"]["1"];
        assert_eq!(gen["model"].as_i64().unwrap(), 2); // Polynomial
        assert_eq!(gen["ncost"].as_i64().unwrap(), 3); // 3 coefficients

        // Cost should be [c2, c1, c0] = [0.01, 10.0, 100.0]
        let cost = gen["cost"].as_array().unwrap();
        assert_eq!(cost.len(), 3);
        assert!((cost[0].as_f64().unwrap() - 0.01).abs() < 1e-10);
        assert!((cost[1].as_f64().unwrap() - 10.0).abs() < 1e-10);
        assert!((cost[2].as_f64().unwrap() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_export_branch_parameters() {
        let network = create_test_network();
        let json_str = export_network_to_powermodels_string(&network, None).expect("export failed");
        let json: serde_json::Value = serde_json::from_str(&json_str).expect("parse failed");

        let branch = &json["branch"]["1"];
        assert_eq!(branch["f_bus"].as_i64().unwrap(), 1);
        assert_eq!(branch["t_bus"].as_i64().unwrap(), 2);
        assert!((branch["br_r"].as_f64().unwrap() - 0.01).abs() < 1e-10);
        assert!((branch["br_x"].as_f64().unwrap() - 0.1).abs() < 1e-10);
        assert_eq!(branch["transformer"].as_bool().unwrap(), false);
    }

    #[test]
    fn test_roundtrip_basic() {
        use crate::importers::parse_powermodels_string;

        let original = create_test_network();

        // Export
        let json_str =
            export_network_to_powermodels_string(&original, None).expect("export failed");

        // Import back
        let result = parse_powermodels_string(&json_str).expect("import failed");
        let imported = result.network;

        // Verify counts match
        let orig_buses: Vec<_> = original
            .graph
            .node_weights()
            .filter(|n| matches!(n, Node::Bus(_)))
            .collect();
        let imp_buses: Vec<_> = imported
            .graph
            .node_weights()
            .filter(|n| matches!(n, Node::Bus(_)))
            .collect();
        assert_eq!(orig_buses.len(), imp_buses.len());

        let orig_branches: Vec<_> = original
            .graph
            .edge_weights()
            .filter(|e| matches!(e, Edge::Branch(_)))
            .collect();
        let imp_branches: Vec<_> = imported
            .graph
            .edge_weights()
            .filter(|e| matches!(e, Edge::Branch(_)))
            .collect();
        assert_eq!(orig_branches.len(), imp_branches.len());
    }
}
