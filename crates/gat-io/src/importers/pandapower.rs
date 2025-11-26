//! Pandapower JSON network format importer
//!
//! Pandapower uses a JSON format that wraps serialized pandas DataFrames.
//! Each component table (bus, load, gen, line, trafo) is a DataFrame in JSON format.
//!
//! Reference: <https://pandapower.readthedocs.io/en/latest/file_io.html>

use std::{collections::HashMap, fs, path::Path};

use anyhow::{anyhow, Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
};
use serde::Deserialize;
use serde_json::Value;

use crate::helpers::{ImportDiagnostics, ImportResult};

/// Top-level pandapower JSON structure
#[derive(Debug, Deserialize)]
struct PandapowerJson {
    _module: String,
    _class: String,
    _object: PandapowerNet,
}

/// The pandapower network container
#[derive(Debug, Deserialize)]
struct PandapowerNet {
    bus: Option<DataFrameJson>,
    load: Option<DataFrameJson>,
    gen: Option<DataFrameJson>,
    ext_grid: Option<DataFrameJson>,
    line: Option<DataFrameJson>,
    trafo: Option<DataFrameJson>,
    shunt: Option<DataFrameJson>,
    // We skip DC, 3-phase, and result tables for now
    #[serde(flatten)]
    _extra: HashMap<String, Value>,
}

/// A serialized pandas DataFrame in JSON format
#[derive(Debug, Deserialize)]
struct DataFrameJson {
    _module: String,
    _class: String,
    /// The actual DataFrame content as a JSON string
    _object: String,
    orient: Option<String>,
    #[serde(flatten)]
    _extra: HashMap<String, Value>,
}

/// Parsed DataFrame content (split orientation)
#[derive(Debug, Deserialize)]
struct DataFrameContent {
    columns: Vec<String>,
    index: Vec<usize>,
    data: Vec<Vec<Value>>,
}

impl DataFrameJson {
    /// Parse the inner JSON string into rows with named columns
    fn parse_rows(&self) -> Result<Vec<HashMap<String, Value>>> {
        let content: DataFrameContent = serde_json::from_str(&self._object)
            .with_context(|| "parsing DataFrame JSON content")?;

        let mut rows = Vec::with_capacity(content.data.len());
        for (i, row_data) in content.data.iter().enumerate() {
            let mut row = HashMap::new();
            // Add the index as a special column
            row.insert(
                "_index".to_string(),
                Value::Number(content.index.get(i).copied().unwrap_or(i).into()),
            );
            for (j, col_name) in content.columns.iter().enumerate() {
                if let Some(val) = row_data.get(j) {
                    row.insert(col_name.clone(), val.clone());
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }
}

// ============================================================================
// Helper functions for extracting typed values from JSON
// ============================================================================

fn get_f64(row: &HashMap<String, Value>, key: &str) -> Option<f64> {
    row.get(key).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::Null => None,
        _ => None,
    })
}

fn get_usize(row: &HashMap<String, Value>, key: &str) -> Option<usize> {
    row.get(key).and_then(|v| match v {
        Value::Number(n) => n.as_u64().map(|x| x as usize),
        Value::Null => None,
        _ => None,
    })
}

fn get_bool(row: &HashMap<String, Value>, key: &str) -> Option<bool> {
    row.get(key).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        Value::Null => None,
        _ => None,
    })
}

fn get_string(row: &HashMap<String, Value>, key: &str) -> Option<String> {
    row.get(key).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Null => None,
        _ => None,
    })
}

// ============================================================================
// Main parsing functions
// ============================================================================

/// Parse a pandapower JSON file and return an ImportResult with diagnostics.
pub fn parse_pandapower(json_file: &str) -> Result<ImportResult> {
    let path = Path::new(json_file);
    let mut diag = ImportDiagnostics::new();
    let network = load_pandapower_network_with_diagnostics(path, &mut diag)?;
    Ok(ImportResult {
        network,
        diagnostics: diag,
    })
}

/// Load a pandapower JSON file and return a Network (without diagnostics).
pub fn load_pandapower_network(json_file: &Path) -> Result<Network> {
    let mut diag = ImportDiagnostics::new();
    load_pandapower_network_with_diagnostics(json_file, &mut diag)
}

/// Load pandapower network with diagnostics tracking.
fn load_pandapower_network_with_diagnostics(
    json_file: &Path,
    diag: &mut ImportDiagnostics,
) -> Result<Network> {
    let content = fs::read_to_string(json_file)
        .with_context(|| format!("reading pandapower JSON file '{}'", json_file.display()))?;

    let pp: PandapowerJson = serde_json::from_str(&content)
        .with_context(|| format!("parsing pandapower JSON from '{}'", json_file.display()))?;

    build_network_from_pandapower(&pp._object, diag)
}

/// Build a gat Network from parsed pandapower data.
fn build_network_from_pandapower(
    pp: &PandapowerNet,
    diag: &mut ImportDiagnostics,
) -> Result<Network> {
    let mut network = Network::new();
    let mut bus_index_map: HashMap<usize, NodeIndex> = HashMap::new();

    // =========================================================================
    // 1. Parse buses
    // =========================================================================
    if let Some(bus_df) = &pp.bus {
        let rows = bus_df.parse_rows()?;
        for row in &rows {
            let idx = get_usize(row, "_index").unwrap_or(0);
            let name = get_string(row, "name").unwrap_or_else(|| format!("Bus {}", idx));
            let vn_kv = get_f64(row, "vn_kv").unwrap_or(1.0);
            let in_service = get_bool(row, "in_service").unwrap_or(true);

            if !in_service {
                diag.stats.skipped_lines += 1;
                continue;
            }

            let bus_id = BusId::new(idx);
            let node_idx = network.graph.add_node(Node::Bus(Bus {
                id: bus_id,
                name,
                voltage_kv: vn_kv,
            }));
            bus_index_map.insert(idx, node_idx);
            diag.stats.buses += 1;
        }
    }

    // =========================================================================
    // 2. Parse loads
    // =========================================================================
    let mut load_id = 0usize;
    if let Some(load_df) = &pp.load {
        let rows = load_df.parse_rows()?;
        for row in &rows {
            let idx = get_usize(row, "_index").unwrap_or(load_id);
            let bus = get_usize(row, "bus").ok_or_else(|| anyhow!("load missing 'bus' field"))?;
            let p_mw = get_f64(row, "p_mw").unwrap_or(0.0);
            let q_mvar = get_f64(row, "q_mvar").unwrap_or(0.0);
            let in_service = get_bool(row, "in_service").unwrap_or(true);
            let name = get_string(row, "name").unwrap_or_else(|| format!("Load {}", idx));

            if !in_service {
                diag.stats.skipped_lines += 1;
                continue;
            }

            if !bus_index_map.contains_key(&bus) {
                diag.add_warning(
                    "orphan_load",
                    &format!("load {} references unknown bus {}", idx, bus),
                );
                continue;
            }

            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(load_id),
                name,
                bus: BusId::new(bus),
                active_power_mw: p_mw,
                reactive_power_mvar: q_mvar,
            }));
            load_id += 1;
            diag.stats.loads += 1;
        }
    }

    // =========================================================================
    // 3. Parse generators (gen table + ext_grid as slack)
    // =========================================================================
    let mut gen_id = 0usize;

    // ext_grid entries are slack/reference buses - model as generators
    if let Some(ext_grid_df) = &pp.ext_grid {
        let rows = ext_grid_df.parse_rows()?;
        for row in &rows {
            let idx = get_usize(row, "_index").unwrap_or(gen_id);
            let bus = get_usize(row, "bus").ok_or_else(|| anyhow!("ext_grid missing 'bus'"))?;
            let in_service = get_bool(row, "in_service").unwrap_or(true);
            let name = get_string(row, "name").unwrap_or_else(|| format!("Slack {}", idx));

            if !in_service {
                diag.stats.skipped_lines += 1;
                continue;
            }

            if !bus_index_map.contains_key(&bus) {
                diag.add_warning(
                    "orphan_ext_grid",
                    &format!("ext_grid {} references unknown bus {}", idx, bus),
                );
                continue;
            }

            // ext_grid P/Q limits
            let max_p = get_f64(row, "max_p_mw");
            let min_p = get_f64(row, "min_p_mw");
            let max_q = get_f64(row, "max_q_mvar");
            let min_q = get_f64(row, "min_q_mvar");

            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_id),
                name,
                bus: BusId::new(bus),
                active_power_mw: 0.0, // Slack determines P from power balance
                reactive_power_mvar: 0.0,
                pmin_mw: min_p.unwrap_or(f64::NEG_INFINITY),
                pmax_mw: max_p.unwrap_or(f64::INFINITY),
                qmin_mvar: min_q.unwrap_or(f64::NEG_INFINITY),
                qmax_mvar: max_q.unwrap_or(f64::INFINITY),
                cost_model: gat_core::CostModel::NoCost,
                is_synchronous_condenser: false,
            }));
            gen_id += 1;
            diag.stats.generators += 1;
        }
    }

    // Regular generators
    if let Some(gen_df) = &pp.gen {
        let rows = gen_df.parse_rows()?;
        for row in &rows {
            let idx = get_usize(row, "_index").unwrap_or(gen_id);
            let bus = get_usize(row, "bus").ok_or_else(|| anyhow!("gen missing 'bus' field"))?;
            let p_mw = get_f64(row, "p_mw").unwrap_or(0.0);
            let in_service = get_bool(row, "in_service").unwrap_or(true);
            let name = get_string(row, "name").unwrap_or_else(|| format!("Gen {}@{}", idx, bus));

            if !in_service {
                diag.stats.skipped_lines += 1;
                continue;
            }

            if !bus_index_map.contains_key(&bus) {
                diag.add_warning(
                    "orphan_generator",
                    &format!("generator {} references unknown bus {}", idx, bus),
                );
                continue;
            }

            let min_q = get_f64(row, "min_q_mvar");
            let max_q = get_f64(row, "max_q_mvar");
            let min_p = get_f64(row, "min_p_mw");
            let max_p = get_f64(row, "max_p_mw");

            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_id),
                name,
                bus: BusId::new(bus),
                active_power_mw: p_mw,
                reactive_power_mvar: 0.0, // Q is determined by power flow
                pmin_mw: min_p.unwrap_or(0.0),
                pmax_mw: max_p.unwrap_or(f64::INFINITY),
                qmin_mvar: min_q.unwrap_or(f64::NEG_INFINITY),
                qmax_mvar: max_q.unwrap_or(f64::INFINITY),
                cost_model: gat_core::CostModel::NoCost,
                is_synchronous_condenser: false,
            }));
            gen_id += 1;
            diag.stats.generators += 1;
        }
    }

    // =========================================================================
    // 4. Parse lines (branches)
    // =========================================================================
    let mut branch_id = 0usize;
    if let Some(line_df) = &pp.line {
        let rows = line_df.parse_rows()?;
        for row in &rows {
            let idx = get_usize(row, "_index").unwrap_or(branch_id);
            let from_bus =
                get_usize(row, "from_bus").ok_or_else(|| anyhow!("line missing 'from_bus'"))?;
            let to_bus =
                get_usize(row, "to_bus").ok_or_else(|| anyhow!("line missing 'to_bus'"))?;
            let in_service = get_bool(row, "in_service").unwrap_or(true);

            if !in_service {
                diag.stats.skipped_lines += 1;
                continue;
            }

            let from_idx = match bus_index_map.get(&from_bus) {
                Some(idx) => *idx,
                None => {
                    diag.add_warning(
                        "orphan_line",
                        &format!("line {} references unknown from_bus {}", idx, from_bus),
                    );
                    continue;
                }
            };
            let to_idx = match bus_index_map.get(&to_bus) {
                Some(idx) => *idx,
                None => {
                    diag.add_warning(
                        "orphan_line",
                        &format!("line {} references unknown to_bus {}", idx, to_bus),
                    );
                    continue;
                }
            };

            // Pandapower stores per-km values, need to multiply by length
            let length_km = get_f64(row, "length_km").unwrap_or(1.0);
            let r_ohm_per_km = get_f64(row, "r_ohm_per_km").unwrap_or(0.0);
            let x_ohm_per_km = get_f64(row, "x_ohm_per_km").unwrap_or(0.0);
            let c_nf_per_km = get_f64(row, "c_nf_per_km").unwrap_or(0.0);
            let max_i_ka = get_f64(row, "max_i_ka");
            let parallel = get_usize(row, "parallel").unwrap_or(1) as f64;

            // Convert to per-unit (requires base voltage from bus)
            // For now, store as raw ohms - we'd need Sbase and Vbase for proper pu conversion
            let r_total = r_ohm_per_km * length_km / parallel;
            let x_total = x_ohm_per_km * length_km / parallel;

            // Charging susceptance: B = 2*pi*f*C, f=50/60Hz typically
            // c_nf_per_km is in nanofarads, convert to charging parameter
            // B_total = omega * C * length = 2*pi*50 * (c_nf * 1e-9) * length_km
            let omega = 2.0 * std::f64::consts::PI * 50.0; // Assuming 50 Hz
            let b_total = omega * (c_nf_per_km * 1e-9) * length_km * parallel;

            let name = get_string(row, "name")
                .unwrap_or_else(|| format!("Line {}-{}", from_bus, to_bus));

            // Calculate MVA rating from current rating if available
            // S = sqrt(3) * V * I for 3-phase
            // We'd need bus voltage for this - approximate with 1.0 pu
            let rating_mva = max_i_ka.map(|i_ka| {
                // Rough approximation - would need actual bus voltage
                let v_kv = 1.0; // Placeholder
                (3.0_f64).sqrt() * v_kv * i_ka * 1000.0 / 1000.0 // Convert to MVA
            });

            let branch = Branch {
                id: BranchId::new(branch_id),
                name,
                from_bus: BusId::new(from_bus),
                to_bus: BusId::new(to_bus),
                resistance: r_total,
                reactance: x_total,
                tap_ratio: 1.0,
                phase_shift_rad: 0.0,
                charging_b_pu: b_total,
                s_max_mva: rating_mva,
                status: true,
                rating_a_mva: rating_mva,
                is_phase_shifter: false,
                ..Branch::default()
            };

            network
                .graph
                .add_edge(from_idx, to_idx, Edge::Branch(branch));
            branch_id += 1;
            diag.stats.branches += 1;
        }
    }

    // =========================================================================
    // 5. Parse transformers (trafo)
    // =========================================================================
    if let Some(trafo_df) = &pp.trafo {
        let rows = trafo_df.parse_rows()?;
        for row in &rows {
            let idx = get_usize(row, "_index").unwrap_or(branch_id);
            let hv_bus =
                get_usize(row, "hv_bus").ok_or_else(|| anyhow!("trafo missing 'hv_bus'"))?;
            let lv_bus =
                get_usize(row, "lv_bus").ok_or_else(|| anyhow!("trafo missing 'lv_bus'"))?;
            let in_service = get_bool(row, "in_service").unwrap_or(true);

            if !in_service {
                diag.stats.skipped_lines += 1;
                continue;
            }

            let hv_idx = match bus_index_map.get(&hv_bus) {
                Some(idx) => *idx,
                None => {
                    diag.add_warning(
                        "orphan_trafo",
                        &format!("trafo {} references unknown hv_bus {}", idx, hv_bus),
                    );
                    continue;
                }
            };
            let lv_idx = match bus_index_map.get(&lv_bus) {
                Some(idx) => *idx,
                None => {
                    diag.add_warning(
                        "orphan_trafo",
                        &format!("trafo {} references unknown lv_bus {}", idx, lv_bus),
                    );
                    continue;
                }
            };

            let sn_mva = get_f64(row, "sn_mva").unwrap_or(100.0);
            let vn_hv_kv = get_f64(row, "vn_hv_kv").unwrap_or(1.0);
            let vn_lv_kv = get_f64(row, "vn_lv_kv").unwrap_or(1.0);
            let vk_percent = get_f64(row, "vk_percent").unwrap_or(0.0);
            let vkr_percent = get_f64(row, "vkr_percent").unwrap_or(0.0);
            let shift_degree = get_f64(row, "shift_degree").unwrap_or(0.0);
            let tap_pos = get_f64(row, "tap_pos");
            let tap_neutral = get_f64(row, "tap_neutral");
            let tap_step_percent = get_f64(row, "tap_step_percent");

            // Calculate tap ratio
            let tap_ratio = if let (Some(pos), Some(neutral), Some(step)) =
                (tap_pos, tap_neutral, tap_step_percent)
            {
                1.0 + (pos - neutral) * step / 100.0
            } else {
                vn_hv_kv / vn_lv_kv // Nominal turns ratio
            };

            // Convert vk (short-circuit voltage) to impedance
            // vk% = Z_pu * 100, so Z_pu = vk% / 100
            // vkr% gives the resistive part
            let z_pu = vk_percent / 100.0;
            let r_pu = vkr_percent / 100.0;
            let x_pu = (z_pu * z_pu - r_pu * r_pu).sqrt();

            let name = get_string(row, "name")
                .unwrap_or_else(|| format!("Trafo {}-{}", hv_bus, lv_bus));

            let branch = Branch {
                id: BranchId::new(branch_id),
                name,
                from_bus: BusId::new(hv_bus),
                to_bus: BusId::new(lv_bus),
                resistance: r_pu,
                reactance: x_pu,
                tap_ratio,
                phase_shift_rad: shift_degree.to_radians(),
                charging_b_pu: 0.0,
                s_max_mva: Some(sn_mva),
                status: true,
                rating_a_mva: Some(sn_mva),
                is_phase_shifter: shift_degree.abs() > 1e-6,
                ..Branch::default()
            };

            network
                .graph
                .add_edge(hv_idx, lv_idx, Edge::Branch(branch));
            branch_id += 1;
            diag.stats.branches += 1;
        }
    }

    Ok(network)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_pandapower_case14() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test_data/adjacent/pandapower/case14.json");

        if !path.exists() {
            eprintln!("Skipping test - sample file not found at {:?}", path);
            return;
        }

        let result = parse_pandapower(path.to_str().unwrap()).expect("should parse");
        let diag = &result.diagnostics;

        // IEEE 14-bus should have:
        // - 14 buses
        // - 11 loads
        // - 5 generators (1 ext_grid + 4 gen)
        // - 20 branches (15 lines + 5 trafos)
        assert_eq!(diag.stats.buses, 14, "expected 14 buses");
        assert_eq!(diag.stats.loads, 11, "expected 11 loads");
        assert_eq!(diag.stats.generators, 5, "expected 5 generators");
        assert_eq!(diag.stats.branches, 20, "expected 20 branches");
        assert_eq!(diag.warning_count(), 0, "expected no warnings");
    }
}
