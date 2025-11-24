/// PFDelta dataset loader for power flow benchmark instances
///
/// PFDelta (https://github.com/MOSSLab-MIT/pfdelta) is a comprehensive benchmark
/// containing 859,800 solved power flow instances across IEEE standard test cases
/// (14/30/57/118-bus and GOC 500/2000-bus) with N, N-1, and N-2 contingencies.
///
/// This module provides utilities to load PFDelta JSON instances and convert them
/// to GAT's Network representation for AC OPF solving and reliability analysis.

use anyhow::{Result, anyhow, Context};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use gat_core::{Network, Node, Edge, Bus, Gen, Load, Branch, BusId, GenId, LoadId, BranchId};

/// PFDelta test case metadata
#[derive(Debug, Clone)]
pub struct PFDeltaTestCase {
    /// Case name (e.g., "case57")
    pub case_name: String,
    /// Contingency type: "n", "n-1", or "n-2"
    pub contingency_type: String,
    /// Path to the JSON file
    pub file_path: String,
    /// Whether this is a near-infeasible case
    pub is_near_infeasible: bool,
}

/// Load a single PFDelta JSON test case and convert to GAT Network
pub fn load_pfdelta_case(json_path: &Path) -> Result<Network> {
    let json_content = fs::read_to_string(json_path)
        .with_context(|| format!("reading PFDelta JSON: {}", json_path.display()))?;

    let data: Value = serde_json::from_str(&json_content)
        .with_context(|| format!("parsing PFDelta JSON: {}", json_path.display()))?;

    convert_pfdelta_to_network(&data)
}

/// Convert PFDelta JSON structure to GAT Network
fn convert_pfdelta_to_network(data: &Value) -> Result<Network> {
    let mut network = Network::new();

    // Extract buses
    let buses = data["bus"]
        .as_object()
        .ok_or_else(|| anyhow!("No 'bus' field in PFDelta JSON"))?;

    // Create a mapping from bus index to NodeIndex
    let mut bus_node_map: HashMap<usize, gat_core::NodeIndex> = HashMap::new();

    for (bus_idx_str, bus_data) in buses {
        let bus_idx: usize = bus_idx_str
            .parse()
            .with_context(|| format!("Invalid bus index: {}", bus_idx_str))?;

        let bus_name = format!("bus_{}", bus_idx);
        let voltage_kv = bus_data["vn"]
            .as_f64()
            .unwrap_or(100.0); // Default to 100 kV

        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(bus_idx),
            name: bus_name,
            voltage_kv,
        }));

        bus_node_map.insert(bus_idx, node_idx);
    }

    // Extract generators
    let gen_data = data["gen"].as_object();
    if let Some(generators) = gen_data {
        for (gen_idx_str, gen) in generators {
            let gen_idx: usize = gen_idx_str
                .parse()
                .with_context(|| format!("Invalid gen index: {}", gen_idx_str))?;

            let bus_id = gen["bus"]
                .as_u64()
                .unwrap_or(0) as usize;

            let pg = gen["pg"].as_f64().unwrap_or(0.0);  // Active power (MW)
            let qg = gen["qg"].as_f64().unwrap_or(0.0);  // Reactive power (MVAr)

            let gen_name = format!("gen_{}", gen_idx);

            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_idx),
                name: gen_name,
                bus: BusId::new(bus_id),
                active_power_mw: pg,
                reactive_power_mvar: qg,
            }));
        }
    }

    // Extract loads
    let load_data = data["load"].as_object();
    if let Some(loads) = load_data {
        for (load_idx_str, load) in loads {
            let load_idx: usize = load_idx_str
                .parse()
                .with_context(|| format!("Invalid load index: {}", load_idx_str))?;

            let bus_id = load["bus"]
                .as_u64()
                .unwrap_or(0) as usize;

            let pd = load["pd"].as_f64().unwrap_or(0.0);  // Active power (MW)
            let qd = load["qd"].as_f64().unwrap_or(0.0);  // Reactive power (MVAr)

            let load_name = format!("load_{}", load_idx);

            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(load_idx),
                name: load_name,
                bus: BusId::new(bus_id),
                active_power_mw: pd,
                reactive_power_mvar: qd,
            }));
        }
    }

    // Extract branches (transmission lines)
    let branch_data = data["branch"].as_object();
    if let Some(branches) = branch_data {
        for (branch_idx_str, branch) in branches {
            let branch_idx: usize = branch_idx_str
                .parse()
                .with_context(|| format!("Invalid branch index: {}", branch_idx_str))?;

            let from_bus_id = branch["fbus"]
                .as_u64()
                .unwrap_or(0) as usize;

            let to_bus_id = branch["tbus"]
                .as_u64()
                .unwrap_or(0) as usize;

            let r = branch["r"].as_f64().unwrap_or(0.0);      // Resistance (p.u.)
            let x = branch["x"].as_f64().unwrap_or(0.01);     // Reactance (p.u.)

            let branch_name = format!("br_{}_{}", from_bus_id, to_bus_id);

            // Find node indices for the buses
            if let (Some(&from_idx), Some(&to_idx)) =
                (bus_node_map.get(&from_bus_id), bus_node_map.get(&to_bus_id))
            {
                network.graph.add_edge(
                    from_idx,
                    to_idx,
                    Edge::Branch(Branch {
                        id: BranchId::new(branch_idx),
                        name: branch_name,
                        from_bus: BusId::new(from_bus_id),
                        to_bus: BusId::new(to_bus_id),
                        resistance: r,
                        reactance: x,
                    }),
                );
            }
        }
    }

    Ok(network)
}

/// List available PFDelta test cases in a directory
pub fn list_pfdelta_cases(pfdelta_root: &Path) -> Result<Vec<PFDeltaTestCase>> {
    let mut cases = Vec::new();

    // Expected structure: pfdelta_root/case{14,30,57,118,500,2000}/{n,n-1,n-2}/raw/
    for entry in fs::read_dir(pfdelta_root)
        .with_context(|| format!("reading PFDelta directory: {}", pfdelta_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let case_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Check for contingency subdirectories
            for cont_type in &["n", "n-1", "n-2"] {
                let raw_dir = path.join(cont_type).join("raw");
                if raw_dir.exists() {
                    // List JSON files in raw directory
                    if let Ok(files) = fs::read_dir(&raw_dir) {
                        for file in files.flatten() {
                            let file_path = file.path();
                            if file_path.extension().map_or(false, |ext| ext == "json") {
                                cases.push(PFDeltaTestCase {
                                    case_name: case_name.clone(),
                                    contingency_type: cont_type.to_string(),
                                    file_path: file_path.display().to_string(),
                                    is_near_infeasible: false,
                                });
                            }
                        }
                    }
                }

                // Also check for near-infeasible cases
                let nose_dir = path.join(cont_type).join("nose");
                if nose_dir.exists() {
                    if let Ok(files) = fs::read_dir(&nose_dir) {
                        for file in files.flatten() {
                            let file_path = file.path();
                            if file_path.extension().map_or(false, |ext| ext == "json") {
                                cases.push(PFDeltaTestCase {
                                    case_name: case_name.clone(),
                                    contingency_type: format!("{}_nose", cont_type),
                                    file_path: file_path.display().to_string(),
                                    is_near_infeasible: true,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(cases)
}

/// Batch load multiple PFDelta test cases
pub fn load_pfdelta_batch(
    test_cases: &[PFDeltaTestCase],
    max_count: Option<usize>,
) -> Result<Vec<(PFDeltaTestCase, Network)>> {
    let limit = max_count.unwrap_or(test_cases.len());
    let mut loaded = Vec::new();

    for (i, test_case) in test_cases.iter().take(limit).enumerate() {
        match load_pfdelta_case(Path::new(&test_case.file_path)) {
            Ok(network) => {
                loaded.push((test_case.clone(), network));
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to load test case {} ({}): {}",
                    i, test_case.file_path, e
                );
            }
        }
    }

    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pfdelta_json_parsing() {
        // Create a minimal valid PFDelta JSON structure
        let json_str = r#"{
            "bus": {
                "1": {"vn": 100.0},
                "2": {"vn": 100.0}
            },
            "gen": {
                "1": {"bus": 1, "pg": 100.0, "qg": 50.0}
            },
            "load": {
                "1": {"bus": 2, "pd": 80.0, "qd": 40.0}
            },
            "branch": {
                "1": {"fbus": 1, "tbus": 2, "r": 0.01, "x": 0.05}
            }
        }"#;

        let data: Value = serde_json::from_str(json_str).unwrap();
        let network = convert_pfdelta_to_network(&data).unwrap();

        // Should have 4 nodes: 2 buses + 1 gen + 1 load
        assert_eq!(network.graph.node_count(), 4);
        // Should have 1 branch edge
        assert_eq!(network.graph.edge_count(), 1);
    }
}
