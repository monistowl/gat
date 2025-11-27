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
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, ImportDiagnostics, Load, LoadId, Network, Node,
    NodeIndex, Shunt, ShuntId,
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
        } else if path.extension().is_some_and(|e| e == "json") {
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
///
/// Returns the instance along with import diagnostics that track any
/// parsing warnings (e.g., missing bus mappings that defaulted to bus 0).
pub fn load_opfdata_instance(
    file_path: &Path,
    sample_id: &str,
) -> Result<(OpfDataInstance, ImportDiagnostics)> {
    let mut diagnostics = ImportDiagnostics::new();

    let file = File::open(file_path)
        .with_context(|| format!("opening OPFData file: {}", file_path.display()))?;
    let reader = BufReader::new(file);
    let data: Value = serde_json::from_reader(reader)
        .with_context(|| format!("parsing OPFData JSON: {}", file_path.display()))?;

    let sample = data.get(sample_id).ok_or_else(|| {
        anyhow::anyhow!("sample {} not found in {}", sample_id, file_path.display())
    })?;

    let network = build_network_from_opfdata(sample, &mut diagnostics)?;
    let solution = build_solution_from_opfdata(sample)?;

    let instance = OpfDataInstance {
        sample_id: sample_id.to_string(),
        file_path: file_path.to_path_buf(),
        network,
        solution,
    };

    Ok((instance, diagnostics))
}

/// Build a Network from OPFData sample JSON
fn build_network_from_opfdata(
    sample: &Value,
    diagnostics: &mut ImportDiagnostics,
) -> Result<Network> {
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
        // Bus format: [base_kv, type, vmin, vmax]
        let base_kv = match bus_array.first().and_then(|v| v.as_f64()) {
            Some(kv) => kv,
            None => {
                diagnostics.add_warning(
                    "parse",
                    &format!("Bus {} missing base_kv, defaulting to 138.0 kV", bus_idx + 1),
                );
                138.0
            }
        };
        // type: 1=PQ, 2=PV, 3=Slack (at index 1)
        // vmin, vmax at indices 2, 3
        let vmin = bus_array.get(2).and_then(|v| v.as_f64());
        let vmax = bus_array.get(3).and_then(|v| v.as_f64());

        let bus_id = BusId::new(bus_idx + 1); // 1-indexed
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: bus_id,
            name: format!("Bus {}", bus_idx + 1),
            voltage_kv: base_kv,
            vmin_pu: vmin,
            vmax_pu: vmax,
            ..Bus::default()
        }));
        bus_index_map.insert(bus_idx, node_idx);
    }

    // Parse edges first - needed for generator/load/shunt bus mappings
    let edges = grid
        .get("edges")
        .ok_or_else(|| anyhow::anyhow!("missing 'edges' field"))?;

    // Parse generators - format: [[mbase, pg, qg, pmax, pmin, qmin, qmax, status, ...], ...]
    // OPFData column [3] contains pmax (larger capacity), column [4] contains pmin (smaller minimum)
    // Generator-to-bus mapping is in edges.generator_link (like load_link, shunt_link)
    let gen_link = edges.get("generator_link").and_then(|v| v.as_object());
    let gen_bus_map: Vec<usize> = match gen_link.and_then(|l| l.get("receivers")).and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_u64().map(|u| u as usize))
            .collect(),
        None => {
            diagnostics.add_warning(
                "parse",
                "Missing edges.generator_link.receivers - generator bus mappings will default to bus 0",
            );
            Vec::new()
        }
    };

    let gen_data = nodes.get("generator").and_then(|v| v.as_array());

    if let Some(gens) = gen_data {
        for (gen_idx, gen_row) in gens.iter().enumerate() {
            let gen_array = gen_row
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("gen row not array"))?;

            // [mbase, pg, qg, pmax, pmin, qmin, qmax, status, ...]
            // Column [3] = pmax (larger capacity), Column [4] = pmin (minimum output)
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

            // Cost coefficients at indices 8, 9, 10: [c2, c1, c0]
            // OPFData stores coefficients for p.u. power: cost = c2*p_pu^2 + c1*p_pu + c0
            // With Sbase=100, p_pu = P_mw/100, so:
            // cost = c2*(P/100)^2 + c1*(P/100) + c0
            //      = (c2/10000)*P^2 + (c1/100)*P + c0
            // For solver expecting cost(P_mw): c2_mw = c2/10000, c1_mw = c1/100, c0_mw = c0
            let c2 = gen_array.get(8).and_then(|v| v.as_f64()).unwrap_or(0.0) / 10000.0;
            let c1 = gen_array.get(9).and_then(|v| v.as_f64()).unwrap_or(0.0) / 100.0;
            let c0 = gen_array.get(10).and_then(|v| v.as_f64()).unwrap_or(0.0);

            if status < 0.5 {
                continue; // Generator offline
            }

            // Handle synchronous condensers (pmax=0 means reactive-only device)
            // When pmax=0, set pmin=pmax=0 to avoid infeasible constraints
            let (pmin_final, pmax_final, is_condenser) = if pmax <= 0.0 {
                (0.0, 0.0, true)
            } else {
                // Ensure pmin <= pmax (some data has inconsistent values)
                (pmin.min(pmax), pmax, false)
            };

            // Get bus from generator_link mapping - warn if missing (likely a mapping bug)
            let gen_bus_idx = match gen_bus_map.get(gen_idx).copied() {
                Some(idx) => idx,
                None => {
                    diagnostics.add_warning(
                        "parse",
                        &format!(
                            "Generator {} has no bus mapping in generator_link.receivers, defaulting to bus 0",
                            gen_idx
                        ),
                    );
                    0
                }
            };
            let bus_id = BusId::new(gen_bus_idx + 1);

            // Build cost model: polynomial [c0, c1, c2]
            let cost_model = if c0 == 0.0 && c1 == 0.0 && c2 == 0.0 {
                gat_core::CostModel::NoCost
            } else {
                gat_core::CostModel::Polynomial(vec![c0, c1, c2])
            };

            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_idx),
                name: format!("Gen {}@{}", gen_idx, gen_bus_idx + 1),
                bus: bus_id,
                active_power_mw: if is_condenser { 0.0 } else { pg },
                reactive_power_mvar: qg,
                pmin_mw: pmin_final,
                pmax_mw: pmax_final,
                qmin_mvar: qmin,
                qmax_mvar: qmax,
                cost_model,
                is_synchronous_condenser: is_condenser,
                ..Gen::default()
            }));
        }
    }

    // Add loads - from nodes.load with load_link for bus mapping
    // Format: nodes.load = [[pd_pu, qd_pu], ...]
    // load_link.senders = load indices, load_link.receivers = bus indices
    let load_data = nodes.get("load").and_then(|v| v.as_array());
    let load_link = edges.get("load_link").and_then(|v| v.as_object());

    if let Some(loads) = load_data {
        let load_bus_map: Vec<usize> = match load_link
            .and_then(|link| link.get("receivers"))
            .and_then(|v| v.as_array())
        {
            Some(arr) => arr
                .iter()
                .filter_map(|v| v.as_u64().map(|u| u as usize))
                .collect(),
            None => {
                if !loads.is_empty() {
                    diagnostics.add_warning(
                        "parse",
                        "Missing edges.load_link.receivers - load bus mappings will default to bus 0",
                    );
                }
                Vec::new()
            }
        };

        for (load_idx, load_row) in loads.iter().enumerate() {
            let load_array = load_row
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("load row not array"))?;

            let pd_pu = load_array.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let qd_pu = load_array.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);

            if pd_pu != 0.0 || qd_pu != 0.0 {
                let bus_idx = match load_bus_map.get(load_idx).copied() {
                    Some(idx) => idx,
                    None => {
                        diagnostics.add_warning(
                            "parse",
                            &format!(
                                "Load {} has no bus mapping in load_link.receivers, defaulting to bus 0",
                                load_idx
                            ),
                        );
                        0
                    }
                };
                network.graph.add_node(Node::Load(Load {
                    id: LoadId::new(load_idx),
                    name: format!("Load {}@{}", load_idx, bus_idx + 1),
                    bus: BusId::new(bus_idx + 1),
                    active_power_mw: pd_pu * 100.0, // p.u. to MW
                    reactive_power_mvar: qd_pu * 100.0,
                }));
            }
        }
    }

    // Add shunts - from nodes.shunt with shunt_link for bus mapping
    // Format: nodes.shunt = [[bs_pu, gs_pu], ...]  (susceptance first, then conductance)
    let shunt_data = nodes.get("shunt").and_then(|v| v.as_array());
    let shunt_link = edges.get("shunt_link").and_then(|v| v.as_object());

    if let Some(shunts) = shunt_data {
        let shunt_bus_map: Vec<usize> = match shunt_link
            .and_then(|link| link.get("receivers"))
            .and_then(|v| v.as_array())
        {
            Some(arr) => arr
                .iter()
                .filter_map(|v| v.as_u64().map(|u| u as usize))
                .collect(),
            None => {
                if !shunts.is_empty() {
                    diagnostics.add_warning(
                        "parse",
                        "Missing edges.shunt_link.receivers - shunt bus mappings will default to bus 0",
                    );
                }
                Vec::new()
            }
        };

        for (shunt_idx, shunt_row) in shunts.iter().enumerate() {
            let shunt_array = shunt_row.as_array();
            if let Some(arr) = shunt_array {
                // OPFData format: [bs_pu, gs_pu]
                let bs_pu = arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                let gs_pu = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);

                if bs_pu != 0.0 || gs_pu != 0.0 {
                    let bus_idx = match shunt_bus_map.get(shunt_idx).copied() {
                        Some(idx) => idx,
                        None => {
                            diagnostics.add_warning(
                                "parse",
                                &format!(
                                    "Shunt {} has no bus mapping in shunt_link.receivers, defaulting to bus 0",
                                    shunt_idx
                                ),
                            );
                            0
                        }
                    };
                    network.graph.add_node(Node::Shunt(Shunt {
                        id: ShuntId::new(shunt_idx),
                        name: format!("Shunt {}@{}", shunt_idx, bus_idx + 1),
                        bus: BusId::new(bus_idx + 1),
                        gs_pu,
                        bs_pu,
                        status: true,
                    }));
                }
            }
        }
    }

    // Parse edges - ac_line and transformer (edges already fetched above)

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
            // OPFData AC line features: [angmin, angmax, b/2, b/2, r, x, rate_a, rate_b, rate_c]
            // Indices 4,5 are series impedance r and x - verified by X/R ratio analysis
            // Indices 2,3 appear to be B/2 at each end of the π-model (they're always equal)
            let resistance = feat.get(4).copied().unwrap_or(0.01);
            let reactance = feat.get(5).copied().unwrap_or(0.1);
            // Total line charging = B/2 + B/2 from each end
            let b_total = feat.get(2).copied().unwrap_or(0.0) + feat.get(3).copied().unwrap_or(0.0);

            if let (Some(&from_idx), Some(&to_idx)) =
                (bus_index_map.get(&from_bus), bus_index_map.get(&to_bus))
            {
                // Rating is in p.u., convert to MVA (Sbase = 100)
                let rating_a_mva = feat.get(6).map(|r| r * 100.0);
                // DEBUG: Temporarily disable flow limits to test if they cause infeasibility
                let branch = Branch {
                    id: BranchId::new(branch_id),
                    name: format!("Line {}-{}", from_bus + 1, to_bus + 1),
                    from_bus: BusId::new(from_bus + 1),
                    to_bus: BusId::new(to_bus + 1),
                    resistance,
                    reactance,
                    charging_b_pu: b_total,
                    s_max_mva: None, // DEBUG: disabled
                    rating_a_mva: rating_a_mva,
                    ..Branch::default()
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
            // Rating is in p.u., convert to MVA (Sbase = 100)
            let rating_a_mva = feat.get(4).map(|r| r * 100.0);
            let tap = feat.get(7).copied().filter(|t| *t != 0.0).unwrap_or(1.0);
            let shift_rad = feat
                .get(8)
                .copied()
                .map(|deg| deg.to_radians())
                .unwrap_or(0.0);
            let charging_b = feat.get(10).copied().unwrap_or(0.0);

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
                    tap_ratio: tap,
                    phase_shift_rad: shift_rad,
                    charging_b_pu: charging_b,
                    s_max_mva: rating_a_mva,
                    rating_a_mva: rating_a_mva,
                    ..Branch::default()
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
                        let vm = arr.first().and_then(|v| v.as_f64()).unwrap_or(1.0);
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
                        let pg = arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) * 100.0; // p.u. to MW
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

        let (instance, diagnostics) = load_opfdata_instance(path, &first_key).unwrap();

        assert!(!instance.sample_id.is_empty());
        assert!(instance.network.graph.node_count() > 0);
        assert!(instance.network.graph.edge_count() > 0);
        assert!(instance.solution.objective > 0.0);
        // Good data should have no warnings about missing bus mappings
        assert_eq!(
            diagnostics.warning_count(),
            0,
            "Unexpected warnings: {:?}",
            diagnostics.issues
        );
    }
}
