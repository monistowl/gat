//! OPFData/GridOpt dataset loader for AC-OPF with topology perturbations.
//!
//! OPFData provides 300k+ solved AC-OPF instances per grid with:
//! - Load perturbations (FullTop)
//! - Topology perturbations (N-1 line/gen/transformer outages)
//!
//! Reference: https://arxiv.org/abs/2406.07234
//!
//! Format: JSON files containing dict of sample_id -> sample data
//! Each sample has: grid (nodes, edges), solution, metadata (objective)

use anyhow::{Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// Reference OPF solution from OPFData
#[derive(Debug, Clone, Default)]
pub struct OpfDataSolution {
    pub vm: HashMap<usize, f64>,
    pub va: HashMap<usize, f64>,
    pub pgen: HashMap<usize, f64>,
    pub qgen: HashMap<usize, f64>,
    pub objective: f64,
}

/// Complete OPFData instance
#[derive(Debug)]
pub struct OpfDataInstance {
    pub sample_id: String,
    pub file_path: PathBuf,
    pub network: Network,
    pub solution: OpfDataSolution,
}

/// Metadata about available OPFData samples in a directory
#[derive(Debug, Clone)]
pub struct OpfDataSampleRef {
    pub sample_id: String,
    pub file_path: PathBuf,
}

/// List all sample references in an OPFData directory
///
/// Expected structure:
/// ```text
/// opfdata_root/
///   pglib_opf_case118_ieee/
///     group_0/
///       merged_1.json
///       merged_2.json
/// ```
pub fn list_sample_refs(root: &Path) -> Result<Vec<OpfDataSampleRef>> {
    let mut refs = Vec::new();

    // Walk all JSON files in the directory structure
    visit_opfdata_files(root, &mut refs)?;

    Ok(refs)
}

fn visit_opfdata_files(dir: &Path, refs: &mut Vec<OpfDataSampleRef>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("reading OPFData directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            visit_opfdata_files(&path, refs)?;
        } else if path.extension().map_or(false, |e| e == "json") {
            // Parse JSON file to discover sample IDs
            if let Ok(file) = File::open(&path) {
                let reader = BufReader::new(file);
                if let Ok(data) = serde_json::from_reader::<_, Value>(reader) {
                    if let Some(obj) = data.as_object() {
                        for key in obj.keys() {
                            refs.push(OpfDataSampleRef {
                                sample_id: key.clone(),
                                file_path: path.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Load a specific sample from an OPFData JSON file
pub fn load_opfdata_instance(file_path: &Path, sample_id: &str) -> Result<OpfDataInstance> {
    let file = File::open(file_path)
        .with_context(|| format!("opening OPFData file: {}", file_path.display()))?;
    let reader = BufReader::new(file);
    let data: Value = serde_json::from_reader(reader)
        .with_context(|| format!("parsing OPFData JSON: {}", file_path.display()))?;

    let sample = data.get(sample_id).ok_or_else(|| {
        anyhow::anyhow!("sample {} not found in {}", sample_id, file_path.display())
    })?;

    let network = build_network_from_opfdata(sample)?;
    let solution = build_solution_from_opfdata(sample)?;

    Ok(OpfDataInstance {
        sample_id: sample_id.to_string(),
        file_path: file_path.to_path_buf(),
        network,
        solution,
    })
}

/// Build a Network from OPFData sample JSON
fn build_network_from_opfdata(sample: &Value) -> Result<Network> {
    let mut network = Network::new();
    let mut bus_index_map: HashMap<usize, NodeIndex> = HashMap::new();

    let grid = sample
        .get("grid")
        .ok_or_else(|| anyhow::anyhow!("missing 'grid' field"))?;
    let nodes = grid
        .get("nodes")
        .ok_or_else(|| anyhow::anyhow!("missing 'nodes' field"))?;

    // Parse buses - format: [[base_kv, type, vmin, vmax], ...]
    let bus_data = nodes
        .get("bus")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("missing 'bus' array"))?;

    for (bus_idx, bus_row) in bus_data.iter().enumerate() {
        let bus_array = bus_row
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("bus row not array"))?;
        let base_kv = bus_array.get(0).and_then(|v| v.as_f64()).unwrap_or(138.0);
        // type: 1=PQ, 2=PV, 3=Slack
        // vmin, vmax at indices 2, 3

        let bus_id = BusId::new(bus_idx + 1); // 1-indexed
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: bus_id,
            name: format!("Bus {}", bus_idx + 1),
            voltage_kv: base_kv,
        }));
        bus_index_map.insert(bus_idx, node_idx);
    }

    // Parse generators - format: [[mbase, pg, qg, pmax, pmin, qmin, qmax, status, ...], ...]
    // We also need generator-to-bus mapping from context
    let context = grid.get("context").and_then(|v| v.as_object());
    let gen_bus_map: Vec<usize> = context
        .and_then(|c| c.get("gen_bus"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64().map(|u| u as usize))
                .collect()
        })
        .unwrap_or_default();

    let gen_data = nodes.get("generator").and_then(|v| v.as_array());

    if let Some(gens) = gen_data {
        for (gen_idx, gen_row) in gens.iter().enumerate() {
            let gen_array = gen_row
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("gen row not array"))?;

            // [mbase, pg, qg, pmax, pmin, qmin, qmax, status, ...]
            let pg = gen_array.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0; // p.u. to MW
            let qg = gen_array.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0;
            let pmax = gen_array
                .get(3)
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::INFINITY)
                * 100.0;
            let pmin = gen_array.get(4).and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0;
            let qmin = gen_array
                .get(5)
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::NEG_INFINITY)
                * 100.0;
            let qmax = gen_array
                .get(6)
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::INFINITY)
                * 100.0;
            let status = gen_array.get(7).and_then(|v| v.as_f64()).unwrap_or(1.0);

            if status < 0.5 {
                continue; // Generator offline
            }

            // Get bus from context or default to first bus
            let gen_bus_idx = gen_bus_map.get(gen_idx).copied().unwrap_or(0);
            let bus_id = BusId::new(gen_bus_idx + 1);

            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_idx),
                name: format!("Gen {}@{}", gen_idx, gen_bus_idx + 1),
                bus: bus_id,
                active_power_mw: pg,
                reactive_power_mvar: qg,
                pmin_mw: pmin,
                pmax_mw: pmax,
                qmin_mvar: qmin,
                qmax_mvar: qmax,
                cost_model: gat_core::CostModel::NoCost,
            }));
        }
    }

    // Add loads - from context load_p and load_q arrays
    if let Some(ctx) = context {
        let load_p: Vec<f64> = ctx
            .get("load_p")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();
        let load_q: Vec<f64> = ctx
            .get("load_q")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();

        for (bus_idx, (p, q)) in load_p.iter().zip(load_q.iter()).enumerate() {
            if *p != 0.0 || *q != 0.0 {
                network.graph.add_node(Node::Load(Load {
                    id: LoadId::new(bus_idx),
                    name: format!("Load {}", bus_idx + 1),
                    bus: BusId::new(bus_idx + 1),
                    active_power_mw: p * 100.0, // p.u. to MW
                    reactive_power_mvar: q * 100.0,
                }));
            }
        }
    }

    // Parse edges - ac_line and transformer
    let edges = grid
        .get("edges")
        .ok_or_else(|| anyhow::anyhow!("missing 'edges' field"))?;

    // AC lines
    let mut branch_id = 0usize;
    if let Some(ac_line) = edges.get("ac_line").and_then(|v| v.as_object()) {
        let senders: Vec<usize> = ac_line
            .get("senders")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|u| u as usize))
                    .collect()
            })
            .unwrap_or_default();
        let receivers: Vec<usize> = ac_line
            .get("receivers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|u| u as usize))
                    .collect()
            })
            .unwrap_or_default();
        let features: Vec<Vec<f64>> = ac_line
            .get("features")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|row| {
                        row.as_array()
                            .map(|r| r.iter().filter_map(|v| v.as_f64()).collect())
                    })
                    .collect()
            })
            .unwrap_or_default();

        for ((&from_bus, &to_bus), feat) in
            senders.iter().zip(receivers.iter()).zip(features.iter())
        {
            // Features: [angmin, angmax, r, r, x, b, rate_a, rate_b, rate_c]
            let resistance = feat.get(2).copied().unwrap_or(0.01);
            let reactance = feat.get(4).copied().unwrap_or(0.1);

            if let (Some(&from_idx), Some(&to_idx)) =
                (bus_index_map.get(&from_bus), bus_index_map.get(&to_bus))
            {
                let branch = Branch {
                    id: BranchId::new(branch_id),
                    name: format!("Line {}-{}", from_bus + 1, to_bus + 1),
                    from_bus: BusId::new(from_bus + 1),
                    to_bus: BusId::new(to_bus + 1),
                    resistance,
                    reactance,
                };
                network
                    .graph
                    .add_edge(from_idx, to_idx, Edge::Branch(branch));
                branch_id += 1;
            }
        }
    }

    // Transformers
    if let Some(transformer) = edges.get("transformer").and_then(|v| v.as_object()) {
        let senders: Vec<usize> = transformer
            .get("senders")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|u| u as usize))
                    .collect()
            })
            .unwrap_or_default();
        let receivers: Vec<usize> = transformer
            .get("receivers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|u| u as usize))
                    .collect()
            })
            .unwrap_or_default();
        let features: Vec<Vec<f64>> = transformer
            .get("features")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|row| {
                        row.as_array()
                            .map(|r| r.iter().filter_map(|v| v.as_f64()).collect())
                    })
                    .collect()
            })
            .unwrap_or_default();

        for ((&from_bus, &to_bus), feat) in
            senders.iter().zip(receivers.iter()).zip(features.iter())
        {
            // Features: [angmin, angmax, r, x, rate_a, rate_b, rate_c, tap, shift, g, b]
            let resistance = feat.get(2).copied().unwrap_or(0.0);
            let reactance = feat.get(3).copied().unwrap_or(0.1);

            if let (Some(&from_idx), Some(&to_idx)) =
                (bus_index_map.get(&from_bus), bus_index_map.get(&to_bus))
            {
                let branch = Branch {
                    id: BranchId::new(branch_id),
                    name: format!("Transformer {}-{}", from_bus + 1, to_bus + 1),
                    from_bus: BusId::new(from_bus + 1),
                    to_bus: BusId::new(to_bus + 1),
                    resistance,
                    reactance,
                };
                network
                    .graph
                    .add_edge(from_idx, to_idx, Edge::Branch(branch));
                branch_id += 1;
            }
        }
    }

    Ok(network)
}

/// Build solution from OPFData sample JSON
fn build_solution_from_opfdata(sample: &Value) -> Result<OpfDataSolution> {
    let mut solution = OpfDataSolution::default();

    // Get objective from metadata
    if let Some(metadata) = sample.get("metadata") {
        solution.objective = metadata
            .get("objective")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
    }

    // Get solution node values
    if let Some(sol) = sample.get("solution") {
        if let Some(nodes) = sol.get("nodes") {
            // Bus solution: [[vm, va], ...]
            if let Some(bus_sol) = nodes.get("bus").and_then(|v| v.as_array()) {
                for (idx, bus_row) in bus_sol.iter().enumerate() {
                    if let Some(arr) = bus_row.as_array() {
                        let vm = arr.get(0).and_then(|v| v.as_f64()).unwrap_or(1.0);
                        let va = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                        solution.vm.insert(idx + 1, vm);
                        solution.va.insert(idx + 1, va);
                    }
                }
            }

            // Generator solution: [[pg, qg], ...]
            if let Some(gen_sol) = nodes.get("generator").and_then(|v| v.as_array()) {
                for (idx, gen_row) in gen_sol.iter().enumerate() {
                    if let Some(arr) = gen_row.as_array() {
                        let pg = arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0; // p.u. to MW
                        let qg = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0;
                        solution.pgen.insert(idx, pg);
                        solution.qgen.insert(idx, qg);
                    }
                }
            }
        }
    }

    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_opfdata_sample() {
        // This test will only pass if OPFData is downloaded
        let path = Path::new(
            "/tmp/opfdata_download/dataset_release_1/pglib_opf_case118_ieee/group_0/merged_1.json",
        );
        if !path.exists() {
            eprintln!("Skipping test: OPFData not available at {:?}", path);
            return;
        }

        // Load first sample
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let data: Value = serde_json::from_reader(reader).unwrap();
        let first_key = data.as_object().unwrap().keys().next().unwrap().clone();

        let instance = load_opfdata_instance(path, &first_key).unwrap();

        assert!(!instance.sample_id.is_empty());
        assert!(instance.network.graph.node_count() > 0);
        assert!(instance.network.graph.edge_count() > 0);
        assert!(instance.solution.objective > 0.0);
    }
}
