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

use crate::helpers::{safe_u64_to_usize, ImportDiagnostics, ImportResult};

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

/// Zero-copy view into a parsed DataFrame.
///
/// This struct provides efficient access to DataFrame cells without cloning
/// strings or values. It pre-computes a column name -> index mapping for O(1) lookups.
struct DataFrameView<'a> {
    index: &'a [usize],
    data: &'a [Vec<Value>],
    /// Column name to index mapping for fast lookups
    col_map: HashMap<&'a str, usize>,
}

impl<'a> DataFrameView<'a> {
    /// Create a view over the DataFrame content
    fn new(content: &'a DataFrameContent) -> Self {
        let col_map = content
            .columns
            .iter()
            .enumerate()
            .map(|(i, name)| (name.as_str(), i))
            .collect();
        Self {
            index: &content.index,
            data: &content.data,
            col_map,
        }
    }

    /// Number of rows in the DataFrame
    fn len(&self) -> usize {
        self.data.len()
    }

    /// Get the index value for a row (the pandas index, not array position)
    fn get_index(&self, row: usize) -> usize {
        self.index.get(row).copied().unwrap_or(row)
    }

    /// Get a reference to a cell value by row index and column name
    fn get(&self, row: usize, col: &str) -> Option<&'a Value> {
        let col_idx = self.col_map.get(col)?;
        self.data.get(row)?.get(*col_idx)
    }
}

impl DataFrameJson {
    /// Parse the inner JSON string and return a view over the data.
    ///
    /// This returns the parsed content which can be wrapped in a DataFrameView
    /// for zero-copy access to cells.
    fn parse_content(&self) -> Result<DataFrameContent> {
        serde_json::from_str(&self._object).with_context(|| "parsing DataFrame JSON content")
    }
}

// ============================================================================
// Helper functions for extracting typed values from DataFrameView
// ============================================================================

fn view_get_f64(view: &DataFrameView, row: usize, key: &str) -> Option<f64> {
    view.get(row, key).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::Null => None,
        _ => None,
    })
}

fn view_get_usize(view: &DataFrameView, row: usize, key: &str) -> Option<usize> {
    view.get(row, key).and_then(|v| match v {
        Value::Number(n) => n.as_u64().and_then(|x| safe_u64_to_usize(x).ok()),
        Value::Null => None,
        _ => None,
    })
}

fn view_get_bool(view: &DataFrameView, row: usize, key: &str) -> Option<bool> {
    view.get(row, key).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        Value::Null => None,
        _ => None,
    })
}

fn view_get_string(view: &DataFrameView, row: usize, key: &str) -> Option<String> {
    view.get(row, key).and_then(|v| match v {
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

    // =========================================================================
    // 1. Parse buses
    // =========================================================================
    // Use reasonable default capacity to reduce HashMap reallocations
    // (typical power networks have 10-10000 buses)
    let mut bus_index_map: HashMap<usize, NodeIndex> = HashMap::with_capacity(256);

    if let Some(bus_df) = &pp.bus {
        let content = bus_df.parse_content()?;
        let view = DataFrameView::new(&content);
        for row in 0..view.len() {
            let idx = view.get_index(row);
            let name =
                view_get_string(&view, row, "name").unwrap_or_else(|| format!("Bus {}", idx));
            let vn_kv = view_get_f64(&view, row, "vn_kv").unwrap_or(1.0);
            let in_service = view_get_bool(&view, row, "in_service").unwrap_or(true);

            if !in_service {
                diag.stats.skipped_lines += 1;
                continue;
            }

            let bus_id = BusId::new(idx);
            let node_idx = network.graph.add_node(Node::Bus(Bus {
                id: bus_id,
                name,
                base_kv: gat_core::Kilovolts(vn_kv),
                ..Default::default()
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
        let content = load_df.parse_content()?;
        let view = DataFrameView::new(&content);
        for row in 0..view.len() {
            let idx = view.get_index(row);
            let bus = view_get_usize(&view, row, "bus")
                .ok_or_else(|| anyhow!("load missing 'bus' field"))?;
            let p_mw = view_get_f64(&view, row, "p_mw").unwrap_or(0.0);
            let q_mvar = view_get_f64(&view, row, "q_mvar").unwrap_or(0.0);
            let in_service = view_get_bool(&view, row, "in_service").unwrap_or(true);
            let name =
                view_get_string(&view, row, "name").unwrap_or_else(|| format!("Load {}", idx));

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
                active_power: gat_core::Megawatts(p_mw),
                reactive_power: gat_core::Megavars(q_mvar),
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
        let content = ext_grid_df.parse_content()?;
        let view = DataFrameView::new(&content);
        for row in 0..view.len() {
            let idx = view.get_index(row);
            let bus = view_get_usize(&view, row, "bus")
                .ok_or_else(|| anyhow!("ext_grid missing 'bus'"))?;
            let in_service = view_get_bool(&view, row, "in_service").unwrap_or(true);
            let name =
                view_get_string(&view, row, "name").unwrap_or_else(|| format!("Slack {}", idx));

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
            let max_p = view_get_f64(&view, row, "max_p_mw");
            let min_p = view_get_f64(&view, row, "min_p_mw");
            let max_q = view_get_f64(&view, row, "max_q_mvar");
            let min_q = view_get_f64(&view, row, "min_q_mvar");

            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_id),
                name,
                bus: BusId::new(bus),
                active_power: gat_core::Megawatts(0.0), // Slack determines P from power balance
                reactive_power: gat_core::Megavars(0.0),
                pmin: gat_core::Megawatts(min_p.unwrap_or(f64::NEG_INFINITY)),
                pmax: gat_core::Megawatts(max_p.unwrap_or(f64::INFINITY)),
                qmin: gat_core::Megavars(min_q.unwrap_or(f64::NEG_INFINITY)),
                qmax: gat_core::Megavars(max_q.unwrap_or(f64::INFINITY)),
                cost_model: gat_core::CostModel::NoCost,
                is_synchronous_condenser: false,
                ..Gen::default()
            }));
            gen_id += 1;
            diag.stats.generators += 1;
        }
    }

    // Regular generators
    if let Some(gen_df) = &pp.gen {
        let content = gen_df.parse_content()?;
        let view = DataFrameView::new(&content);
        for row in 0..view.len() {
            let idx = view.get_index(row);
            let bus = view_get_usize(&view, row, "bus")
                .ok_or_else(|| anyhow!("gen missing 'bus' field"))?;
            let p_mw = view_get_f64(&view, row, "p_mw").unwrap_or(0.0);
            let in_service = view_get_bool(&view, row, "in_service").unwrap_or(true);
            let name = view_get_string(&view, row, "name")
                .unwrap_or_else(|| format!("Gen {}@{}", idx, bus));

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

            let min_q = view_get_f64(&view, row, "min_q_mvar");
            let max_q = view_get_f64(&view, row, "max_q_mvar");
            let min_p = view_get_f64(&view, row, "min_p_mw");
            let max_p = view_get_f64(&view, row, "max_p_mw");

            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_id),
                name,
                bus: BusId::new(bus),
                active_power: gat_core::Megawatts(p_mw),
                reactive_power: gat_core::Megavars(0.0), // Q is determined by power flow
                pmin: gat_core::Megawatts(min_p.unwrap_or(0.0)),
                pmax: gat_core::Megawatts(max_p.unwrap_or(f64::INFINITY)),
                qmin: gat_core::Megavars(min_q.unwrap_or(f64::NEG_INFINITY)),
                qmax: gat_core::Megavars(max_q.unwrap_or(f64::INFINITY)),
                cost_model: gat_core::CostModel::NoCost,
                is_synchronous_condenser: false,
                ..Gen::default()
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
        let content = line_df.parse_content()?;
        let view = DataFrameView::new(&content);
        for row in 0..view.len() {
            let idx = view.get_index(row);
            let from_bus = view_get_usize(&view, row, "from_bus")
                .ok_or_else(|| anyhow!("line missing 'from_bus'"))?;
            let to_bus = view_get_usize(&view, row, "to_bus")
                .ok_or_else(|| anyhow!("line missing 'to_bus'"))?;
            let in_service = view_get_bool(&view, row, "in_service").unwrap_or(true);

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
            let length_km = view_get_f64(&view, row, "length_km").unwrap_or(1.0);
            let r_ohm_per_km = view_get_f64(&view, row, "r_ohm_per_km").unwrap_or(0.0);
            let x_ohm_per_km = view_get_f64(&view, row, "x_ohm_per_km").unwrap_or(0.0);
            let c_nf_per_km = view_get_f64(&view, row, "c_nf_per_km").unwrap_or(0.0);
            let max_i_ka = view_get_f64(&view, row, "max_i_ka");
            let parallel = view_get_usize(&view, row, "parallel").unwrap_or(1) as f64;

            // Convert to per-unit (requires base voltage from bus)
            // For now, store as raw ohms - we'd need Sbase and Vbase for proper pu conversion
            let r_total = r_ohm_per_km * length_km / parallel;
            let x_total = x_ohm_per_km * length_km / parallel;

            // Charging susceptance: B = 2*pi*f*C, f=50/60Hz typically
            // c_nf_per_km is in nanofarads, convert to charging parameter
            // B_total = omega * C * length = 2*pi*50 * (c_nf * 1e-9) * length_km
            let omega = 2.0 * std::f64::consts::PI * 50.0; // Assuming 50 Hz
            let b_total = omega * (c_nf_per_km * 1e-9) * length_km * parallel;

            let name = view_get_string(&view, row, "name")
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
                phase_shift: gat_core::Radians(0.0),
                charging_b: gat_core::PerUnit(b_total),
                s_max: rating_mva.map(gat_core::MegavoltAmperes),
                status: true,
                rating_a: rating_mva.map(gat_core::MegavoltAmperes),
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
        let content = trafo_df.parse_content()?;
        let view = DataFrameView::new(&content);
        for row in 0..view.len() {
            let idx = view.get_index(row);
            let hv_bus = view_get_usize(&view, row, "hv_bus")
                .ok_or_else(|| anyhow!("trafo missing 'hv_bus'"))?;
            let lv_bus = view_get_usize(&view, row, "lv_bus")
                .ok_or_else(|| anyhow!("trafo missing 'lv_bus'"))?;
            let in_service = view_get_bool(&view, row, "in_service").unwrap_or(true);

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

            let sn_mva = view_get_f64(&view, row, "sn_mva").unwrap_or(100.0);
            let vn_hv_kv = view_get_f64(&view, row, "vn_hv_kv").unwrap_or(1.0);
            let vn_lv_kv = view_get_f64(&view, row, "vn_lv_kv").unwrap_or(1.0);
            let vk_percent = view_get_f64(&view, row, "vk_percent").unwrap_or(0.0);
            let vkr_percent = view_get_f64(&view, row, "vkr_percent").unwrap_or(0.0);
            let shift_degree = view_get_f64(&view, row, "shift_degree").unwrap_or(0.0);
            let tap_pos = view_get_f64(&view, row, "tap_pos");
            let tap_neutral = view_get_f64(&view, row, "tap_neutral");
            let tap_step_percent = view_get_f64(&view, row, "tap_step_percent");

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

            let name = view_get_string(&view, row, "name")
                .unwrap_or_else(|| format!("Trafo {}-{}", hv_bus, lv_bus));

            let branch = Branch {
                id: BranchId::new(branch_id),
                name,
                from_bus: BusId::new(hv_bus),
                to_bus: BusId::new(lv_bus),
                resistance: r_pu,
                reactance: x_pu,
                tap_ratio,
                phase_shift: gat_core::Radians(shift_degree.to_radians()),
                charging_b: gat_core::PerUnit(0.0),
                s_max: Some(gat_core::MegavoltAmperes(sn_mva)),
                status: true,
                rating_a: Some(gat_core::MegavoltAmperes(sn_mva)),
                is_phase_shifter: shift_degree.abs() > 1e-6,
                ..Branch::default()
            };

            network.graph.add_edge(hv_idx, lv_idx, Edge::Branch(branch));
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

        let result =
            parse_pandapower(path.to_str().unwrap()).expect("should parse pandapower sample");
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
