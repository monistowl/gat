use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result, anyhow};
use gat_core::{Edge, Network, Node};
use gat_io::importers::import_matpower_case;
use polars::prelude::*;

pub const DIST_NODE_COLUMNS: &[&str] = &[
    "node_id",
    "phase",
    "type",
    "v_min",
    "v_max",
    "load_p",
    "load_q",
    "feeder_id",
];

pub const DIST_BRANCH_COLUMNS: &[&str] = &[
    "branch_id",
    "from_node",
    "to_node",
    "r",
    "x",
    "b",
    "tap",
    "status",
    "thermal_limit",
];

#[derive(Debug)]
pub struct DistNetwork {
    pub nodes: DataFrame,
    pub branches: DataFrame,
}

pub fn import_matpower_as_dist(
    m_file: &str,
    nodes_out: &Path,
    branches_out: &Path,
    feeder_id: Option<&str>,
) -> Result<DistNetwork> {
    let temp_path = std::env::temp_dir().join("gat_dist_import.arrow");
    let network = import_matpower_case(m_file, temp_path.to_string_lossy().as_ref())
        .context("importing MATPOWER case for distribution tables")?;
    let feeder_label = feeder_id.unwrap_or("feeder-0");
    let (nodes, branches) = build_dist_tables(&network, feeder_label)?;
    write_parquet(nodes_out, &nodes)?;
    write_parquet(branches_out, &branches)?;
    let _ = fs::remove_file(&temp_path);
    Ok(DistNetwork { nodes, branches })
}

pub fn load_network(nodes: &Path, branches: &Path) -> Result<DistNetwork> {
    let nodes_df = LazyFrame::scan_parquet(nodes, Default::default())
        .context("opening dist_nodes parquet")?
        .collect()?;
    let branches_df = LazyFrame::scan_parquet(branches, Default::default())
        .context("opening dist_branches parquet")?
        .collect()?;
    validate_columns(&nodes_df, DIST_NODE_COLUMNS, "dist_nodes")?;
    validate_columns(&branches_df, DIST_BRANCH_COLUMNS, "dist_branches")?;
    Ok(DistNetwork {
        nodes: nodes_df,
        branches: branches_df,
    })
}

pub fn validate_columns(df: &DataFrame, expected: &[&str], label: &str) -> Result<()> {
    let missing: Vec<_> = expected
        .iter()
        .filter(|col| !df.get_column_names().iter().any(|name| name == **col))
        .copied()
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "{} missing required columns: {}",
            label,
            missing.join(", ")
        ))
    }
}

pub fn summarize_pf(network: &DistNetwork) -> Result<DataFrame> {
    let feeder_groups = network.nodes.column("feeder_id")?.utf8()?;
    let load_p = network
        .nodes
        .column("load_p")?
        .f64()
        .context("load_p must be f64")?;
    let load_q = network
        .nodes
        .column("load_q")?
        .f64()
        .context("load_q must be f64")?;

    let mut totals: HashMap<String, (f64, f64, usize)> = HashMap::new();
    for (idx, feeder) in feeder_groups.into_iter().enumerate() {
        let feeder_key = feeder.unwrap_or("unknown").to_string();
        let entry = totals.entry(feeder_key).or_insert((0.0, 0.0, 0));
        entry.0 += load_p.get(idx).unwrap_or(0.0);
        entry.1 += load_q.get(idx).unwrap_or(0.0);
        entry.2 += 1;
    }

    let mut feeder_id = Vec::new();
    let mut nodes = Vec::new();
    let mut total_p = Vec::new();
    let mut total_q = Vec::new();
    for (key, (p, q, count)) in totals {
        feeder_id.push(key);
        nodes.push(count as u64);
        total_p.push(p);
        total_q.push(q);
    }

    let voltage = Series::new("voltage_pu", vec![1.0f64; feeder_id.len()]);
    DataFrame::new(vec![
        Series::new("feeder_id", feeder_id),
        Series::new("node_count", nodes),
        Series::new("load_p_total", total_p),
        Series::new("load_q_total", total_q),
        voltage,
    ])
    .context("building pf summary dataframe")
}

pub fn summarize_opf(network: &DistNetwork) -> Result<DataFrame> {
    let pf = summarize_pf(network)?;
    let loss_series = Series::new("estimated_losses", vec![0.0f64; pf.height()]);
    let objective = Series::new("objective", vec!["feasible"; pf.height()]);
    let mut columns = pf.get_columns().clone();
    columns.push(loss_series);
    columns.push(objective);
    DataFrame::new(columns).context("building opf summary dataframe")
}

pub fn hosting_capacity_scan(
    network: &DistNetwork,
    targets: &[String],
    max_mw: f64,
    step_mw: f64,
) -> Result<(DataFrame, DataFrame)> {
    if max_mw <= 0.0 {
        return Err(anyhow!("max_mw must be positive"));
    }
    if step_mw <= 0.0 {
        return Err(anyhow!("step_mw must be positive"));
    }
    let target_set: Vec<String> = if targets.is_empty() {
        network
            .nodes
            .column("node_id")?
            .utf8()?
            .into_iter()
            .flatten()
            .map(|v| v.to_string())
            .collect()
    } else {
        targets.to_vec()
    };

    let mut summary_rows = Vec::new();
    let mut detail_rows: Vec<(String, f64, bool)> = Vec::new();
    for node in &target_set {
        let mut last_feasible = 0.0;
        let mut level = 0.0;
        while level <= max_mw + 1e-6 {
            detail_rows.push((node.clone(), level, true));
            last_feasible = level;
            level += step_mw;
        }
        summary_rows.push((node.clone(), last_feasible));
    }

    let summary = DataFrame::new(vec![
        Series::new(
            "node_id",
            summary_rows
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>(),
        ),
        Series::new(
            "host_mw",
            summary_rows.iter().map(|(_, h)| *h).collect::<Vec<_>>(),
        ),
    ])?;

    let detail = DataFrame::new(vec![
        Series::new(
            "node_id",
            detail_rows
                .iter()
                .map(|(n, _, _)| n.as_str())
                .collect::<Vec<_>>(),
        ),
        Series::new(
            "injection_mw",
            detail_rows.iter().map(|(_, mw, _)| *mw).collect::<Vec<_>>(),
        ),
        Series::new(
            "feasible",
            detail_rows.iter().map(|(_, _, f)| *f).collect::<Vec<_>>(),
        ),
    ])?;

    Ok((summary, detail))
}

fn build_dist_tables(network: &Network, feeder_id: &str) -> Result<(DataFrame, DataFrame)> {
    let mut load_accumulator: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut has_gen: HashMap<usize, bool> = HashMap::new();

    for node in network.graph.node_weights() {
        match node {
            Node::Load(load) => {
                let entry = load_accumulator.entry(load.bus.value()).or_default();
                entry.0 += load.active_power_mw;
                entry.1 += load.reactive_power_mvar;
            }
            Node::Gen(generator) => {
                has_gen.insert(generator.bus.value(), true);
            }
            _ => {}
        }
    }

    let mut node_ids = Vec::new();
    let mut phases = Vec::new();
    let mut types = Vec::new();
    let mut v_min = Vec::new();
    let mut v_max = Vec::new();
    let mut load_p = Vec::new();
    let mut load_q = Vec::new();
    let mut feeders = Vec::new();

    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            node_ids.push(bus.id.value().to_string());
            phases.push("ABC".to_string());
            let node_type = if has_gen.get(&bus.id.value()).copied().unwrap_or(false) {
                "source"
            } else if load_accumulator.contains_key(&bus.id.value()) {
                "load"
            } else {
                "slack"
            };
            types.push(node_type.to_string());
            let load = load_accumulator
                .get(&bus.id.value())
                .cloned()
                .unwrap_or((0.0, 0.0));
            load_p.push(load.0);
            load_q.push(load.1);
            v_min.push(0.95);
            v_max.push(1.05);
            feeders.push(feeder_id.to_string());
        }
    }

    let nodes_df = DataFrame::new(vec![
        Series::new("node_id", node_ids),
        Series::new("phase", phases),
        Series::new("type", types),
        Series::new("v_min", v_min),
        Series::new("v_max", v_max),
        Series::new("load_p", load_p),
        Series::new("load_q", load_q),
        Series::new("feeder_id", feeders),
    ])?;

    let mut branch_ids = Vec::new();
    let mut from_nodes = Vec::new();
    let mut to_nodes = Vec::new();
    let mut r = Vec::new();
    let mut x = Vec::new();
    let mut b = Vec::new();
    let mut tap = Vec::new();
    let mut status = Vec::new();
    let mut thermal = Vec::new();

    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            branch_ids.push(branch.id.value() as i64);
            from_nodes.push(branch.from_bus.value().to_string());
            to_nodes.push(branch.to_bus.value().to_string());
            r.push(branch.resistance);
            x.push(branch.reactance);
            b.push(0.0);
            tap.push(1.0);
            status.push(true);
            thermal.push(f64::NAN);
        }
    }

    let branches_df = DataFrame::new(vec![
        Series::new("branch_id", branch_ids),
        Series::new("from_node", from_nodes),
        Series::new("to_node", to_nodes),
        Series::new("r", r),
        Series::new("x", x),
        Series::new("b", b),
        Series::new("tap", tap),
        Series::new("status", status),
        Series::new("thermal_limit", thermal),
    ])?;

    Ok((nodes_df, branches_df))
}

fn write_parquet(path: &Path, df: &DataFrame) -> Result<()> {
    let mut file = std::fs::File::create(path).context("creating parquet output")?;
    ParquetWriter::new(&mut file)
        .with_statistics(true)
        .finish(df)
        .context("writing parquet output")
}
