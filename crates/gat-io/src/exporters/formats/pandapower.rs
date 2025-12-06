//! pandapower JSON exporter
//!
//! Serializes GAT networks into the subset of the pandapower JSON schema required by the importer.

use crate::exporters::ExportMetadata;
use anyhow::Result;
use gat_core::{Branch, Bus, Edge, Network, Node};
use serde::Serialize;
use serde_json::{json, Number, Value};
use std::{collections::HashMap, fs, path::Path};

#[derive(Serialize)]
struct DataFrameObject {
    columns: Vec<String>,
    index: Vec<usize>,
    data: Vec<Vec<Value>>,
}

fn f64_value(value: f64) -> Value {
    Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn opt_f64_value(value: Option<f64>) -> Value {
    value.map(f64_value).unwrap_or(Value::Null)
}

fn make_dataframe(
    columns: &[&str],
    index: Vec<usize>,
    rows: Vec<Vec<Value>>,
    dtype: Value,
) -> Result<Value> {
    let object = DataFrameObject {
        columns: columns.iter().map(|s| s.to_string()).collect(),
        index,
        data: rows,
    };
    let obj_str = serde_json::to_string(&object)?;
    Ok(json!({
        "_module": "pandas.core.frame",
        "_class": "DataFrame",
        "_object": obj_str,
        "orient": "split",
        "dtype": dtype,
        "is_multiindex": false,
        "is_multicolumn": false,
    }))
}

fn branch_current_limit(branch: &Branch, bus_voltage_kv: f64) -> Value {
    let rating = branch
        .rating_a
        .or(branch.s_max)
        .map(|v| v.value())
        .unwrap_or(0.0);
    if rating <= 0.0 || bus_voltage_kv <= 0.0 {
        return Value::Null;
    }
    let current = rating / (3f64.sqrt() * bus_voltage_kv);
    f64_value(current)
}

/// Export a Network to pandapower-style JSON
pub fn export_network_to_pandapower(
    network: &Network,
    output_path: impl AsRef<Path>,
    metadata: Option<&ExportMetadata>,
) -> Result<()> {
    let mut buses: Vec<_> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Bus(bus) => Some(bus.clone()),
            _ => None,
        })
        .collect();
    buses.sort_by_key(|bus| bus.id.value());

    let bus_map: HashMap<usize, Bus> = buses
        .iter()
        .map(|bus| (bus.id.value(), bus.clone()))
        .collect();

    let gens: Vec<_> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Gen(gen) if gen.status => Some(gen.clone()),
            _ => None,
        })
        .collect();

    let loads: Vec<_> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Load(load) => Some(load.clone()),
            _ => None,
        })
        .collect();

    let mut lines = Vec::new();
    let mut trafos = Vec::new();
    for edge in network.graph.edge_weights() {
        match edge {
            Edge::Branch(branch) => lines.push(branch.clone()),
            Edge::Transformer(tx) => trafos.push(tx.clone()),
        }
    }

    let bus_rows: Vec<Vec<Value>> = buses
        .iter()
        .map(|bus| {
            vec![
                Value::String(bus.name.clone()),
                f64_value(bus.base_kv.value()),
                Value::Bool(true),
            ]
        })
        .collect();

    let bus_dtype = json!({
        "name": "object",
        "vn_kv": "float64",
        "in_service": "bool",
    });

    let bus_table = make_dataframe(
        &["name", "vn_kv", "in_service"],
        buses.iter().map(|bus| bus.id.value()).collect(),
        bus_rows,
        bus_dtype,
    )?;

    let load_rows: Vec<Vec<Value>> = loads
        .iter()
        .map(|load| {
            vec![
                Value::String(load.name.clone()),
                Value::Number(Number::from(load.bus.value())),
                f64_value(load.active_power.value()),
                f64_value(load.reactive_power.value()),
                Value::Bool(true),
            ]
        })
        .collect();
    let load_dtype = json!({
        "name": "object",
        "bus": "uint32",
        "p_mw": "float64",
        "q_mvar": "float64",
        "in_service": "bool",
    });
    let load_table = make_dataframe(
        &["name", "bus", "p_mw", "q_mvar", "in_service"],
        loads.iter().map(|load| load.id.value()).collect(),
        load_rows,
        load_dtype,
    )?;

    let gen_rows: Vec<Vec<Value>> = gens
        .iter()
        .map(|gen| {
            vec![
                Value::String(gen.name.clone()),
                Value::Number(Number::from(gen.bus.value())),
                f64_value(gen.active_power.value()),
                opt_f64_value(Some(gen.pmax.value())),
                opt_f64_value(Some(gen.pmin.value())),
                opt_f64_value(Some(gen.qmax.value())),
                opt_f64_value(Some(gen.qmin.value())),
                Value::Bool(true),
            ]
        })
        .collect();
    let gen_dtype = json!({
        "name": "object",
        "bus": "uint32",
        "p_mw": "float64",
        "max_p_mw": "float64",
        "min_p_mw": "float64",
        "max_q_mvar": "float64",
        "min_q_mvar": "float64",
        "in_service": "bool",
    });
    let gen_table = make_dataframe(
        &[
            "name",
            "bus",
            "p_mw",
            "max_p_mw",
            "min_p_mw",
            "max_q_mvar",
            "min_q_mvar",
            "in_service",
        ],
        gens.iter().map(|gen| gen.id.value()).collect(),
        gen_rows,
        gen_dtype,
    )?;

    let line_rows: Vec<Vec<Value>> = lines
        .iter()
        .map(|branch| {
            let from_voltage = bus_map
                .get(&branch.from_bus.value())
                .map(|bus| bus.base_kv.value())
                .unwrap_or(1.0);
            vec![
                Value::String(branch.name.clone()),
                Value::Number(Number::from(branch.from_bus.value())),
                Value::Number(Number::from(branch.to_bus.value())),
                f64_value(1.0),
                f64_value(branch.resistance),
                f64_value(branch.reactance),
                f64_value(0.0),
                branch_current_limit(branch, from_voltage),
                f64_value(1.0),
                Value::Bool(branch.status),
            ]
        })
        .collect();
    let line_dtype = json!({
        "name": "object",
        "from_bus": "uint32",
        "to_bus": "uint32",
        "length_km": "float64",
        "r_ohm_per_km": "float64",
        "x_ohm_per_km": "float64",
        "c_nf_per_km": "float64",
        "max_i_ka": "float64",
        "parallel": "float64",
        "in_service": "bool",
    });
    let line_table = make_dataframe(
        &[
            "name",
            "from_bus",
            "to_bus",
            "length_km",
            "r_ohm_per_km",
            "x_ohm_per_km",
            "c_nf_per_km",
            "max_i_ka",
            "parallel",
            "in_service",
        ],
        lines.iter().map(|branch| branch.id.value()).collect(),
        line_rows,
        line_dtype,
    )?;

    let trafo_rows: Vec<Vec<Value>> = trafos
        .iter()
        .map(|tx| {
            vec![
                Value::String(tx.name.clone()),
                Value::Number(Number::from(tx.from_bus.value())),
                Value::Number(Number::from(tx.to_bus.value())),
                f64_value(100.0),
                f64_value(
                    bus_map
                        .get(&tx.from_bus.value())
                        .map(|bus| bus.base_kv.value())
                        .unwrap_or(1.0),
                ),
                f64_value(
                    bus_map
                        .get(&tx.to_bus.value())
                        .map(|bus| bus.base_kv.value())
                        .unwrap_or(1.0),
                ),
                f64_value(0.0),
                f64_value(0.0),
                f64_value(0.0),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Bool(true),
            ]
        })
        .collect();
    let trafo_dtype = json!({
        "name": "object",
        "hv_bus": "uint32",
        "lv_bus": "uint32",
        "sn_mva": "float64",
        "vn_hv_kv": "float64",
        "vn_lv_kv": "float64",
        "vk_percent": "float64",
        "vkr_percent": "float64",
        "shift_degree": "float64",
        "tap_pos": "float64",
        "tap_neutral": "float64",
        "tap_step_percent": "float64",
        "in_service": "bool",
    });
    let trafo_table = make_dataframe(
        &[
            "name",
            "hv_bus",
            "lv_bus",
            "sn_mva",
            "vn_hv_kv",
            "vn_lv_kv",
            "vk_percent",
            "vkr_percent",
            "shift_degree",
            "tap_pos",
            "tap_neutral",
            "tap_step_percent",
            "in_service",
        ],
        trafos.iter().map(|tx| tx.id.value()).collect(),
        trafo_rows,
        trafo_dtype,
    )?;

    let ext_grid_table = make_dataframe(
        &[
            "name",
            "bus",
            "max_p_mw",
            "min_p_mw",
            "max_q_mvar",
            "min_q_mvar",
            "in_service",
        ],
        Vec::new(),
        Vec::new(),
        json!({
            "name": "object",
            "bus": "uint32",
            "max_p_mw": "float64",
            "min_p_mw": "float64",
            "max_q_mvar": "float64",
            "min_q_mvar": "float64",
            "in_service": "bool",
        }),
    )?;

    let meta_value = metadata.and_then(|meta| {
        let mut map = serde_json::Map::new();
        if let Some(source) = &meta.source {
            map.insert(
                "source".to_string(),
                json!({
                    "file": source.file,
                    "format": source.format,
                    "hash": source.file_hash,
                }),
            );
        }
        if let Some(ts) = meta.creation_timestamp() {
            map.insert("created_at".to_string(), Value::String(ts));
        }
        if let Some(version) = meta.gat_version() {
            map.insert(
                "gat_version".to_string(),
                Value::String(version.to_string()),
            );
        }
        if map.is_empty() {
            None
        } else {
            Some(Value::Object(map))
        }
    });

    let mut payload = json!({
        "_module": "pandapower.auxiliary",
        "_class": "pandapowerNet",
        "_object": {
            "bus": bus_table,
            "load": load_table,
            "gen": gen_table,
            "line": line_table,
            "trafo": trafo_table,
            "ext_grid": ext_grid_table,
        }
    });

    if let Some(meta_value) = meta_value {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("_meta".to_string(), meta_value);
        }
    }

    let output = serde_json::to_string_pretty(&payload)?;
    fs::write(output_path, output)?;
    Ok(())
}
