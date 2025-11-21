use anyhow::{anyhow, Context, Result};
use gat_algo::power_flow;
use gat_core::{solver::SolverKind, BusId, Edge, Gen, GenId, Network, Node};
use gat_io::importers;
use polars::prelude::{DataFrame, NamedFrom, ParquetCompression, ParquetWriter, Series};
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// Import a MATPOWER case and emit dist-specific node/branch tables as Parquet.
///
/// The radial distribution dataset mirrors the Baran/Wu branch-flow formulation for radial
/// networks (doi:10.1109/TPWRD.1989.4303454), so we expose feeder-level node/branch files that
/// tie into the downstream workflows.
pub fn import_matpower_case(matpower: &str, out_dir: &Path, feeder_id: Option<&str>) -> Result<()> {
    fs::create_dir_all(out_dir).with_context(|| {
        format!(
            "failed to create dist output directory '{}'; check permissions",
            out_dir.display()
        )
    })?;

    let temp = tempdir().context("creating temporary folder for MATPOWER import")?;
    let temp_path = temp.path().join("matpower.arrow");
    let network = importers::import_matpower_case(matpower, temp_path.to_str().unwrap())?;

    let nodes = build_node_frame(&network, feeder_id.unwrap_or("default"));
    let branches = build_branch_frame(&network, feeder_id.unwrap_or("default"));
    write_parquet(out_dir.join("dist_nodes.parquet"), nodes)?;
    write_parquet(out_dir.join("dist_branches.parquet"), branches)?;

    println!(
        "Dist import produced '{}' nodes and '{}' branches under {}",
        network.graph.node_count(),
        network.graph.edge_count(),
        out_dir.display()
    );
    Ok(())
}

/// Run a distribution-aware AC power flow and persist its Parquet trace.
///
/// The solver here retains the Newton-Raphson flavor from the canonical DOI:10.1109/TPWRS.2012.2187686
/// while keeping the CLI semantics aligned with the upstream `gat pf ac` command.
pub fn run_power_flow(
    grid_file: &Path,
    out_file: &Path,
    solver_kind: SolverKind,
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    let network = load_network(grid_file)?;
    let solver = solver_kind.build_solver();
    power_flow::ac_power_flow(&network, solver.as_ref(), tol, max_iter, out_file, &[])
        .with_context(|| format!("running dist pf on {}", grid_file.display()))
}

/// Run a simple single-objective AC OPF for hosting/volt-var experiments.
pub fn run_optimal_power_flow(
    grid_file: &Path,
    out_file: &Path,
    solver_kind: SolverKind,
    tol: f64,
    max_iter: u32,
    objective: &str,
) -> Result<()> {
    println!(
        "Dist OPF objective '{}'; recording results for downstream offline workflows",
        objective
    );
    let network = load_network(grid_file)?;
    let solver = solver_kind.build_solver();
    power_flow::ac_optimal_power_flow(&network, solver.as_ref(), tol, max_iter, out_file, &[])
        .with_context(|| format!("running dist opf on {}", grid_file.display()))
}

/// Sweep DER injections at selected buses to approximate hosting capacity boundaries.
///
/// This heuristic mirrors historical hosting-capacity sweeps (doi:10.1109/TDEI.2016.7729825) by
/// deploying incremental injections and checking feasibility via the existing AC OPF.
pub fn hostcap_sweep(
    grid_file: &Path,
    target_buses: &[usize],
    max_injection: f64,
    steps: usize,
    out_dir: &Path,
    solver_kind: SolverKind,
) -> Result<()> {
    if steps == 0 {
        return Err(anyhow!("hostcap steps must be at least 1"));
    }
    fs::create_dir_all(out_dir).with_context(|| {
        format!(
            "failed to create hostcap directory '{}'; check permissions",
            out_dir.display()
        )
    })?;

    let network = load_network(grid_file)?;
    let bus_names = collect_bus_names(&network);
    let mut targets = target_buses.to_vec();
    if targets.is_empty() {
        targets = bus_names.keys().copied().collect();
    }

    let mut summary_bus = Vec::new();
    let mut summary_node = Vec::new();
    let mut summary_step = Vec::new();
    let mut summary_injection = Vec::new();
    let mut summary_success = Vec::new();
    let mut summary_artifact = Vec::new();

    for &bus_id in &targets {
        let node_label = bus_names
            .get(&bus_id)
            .unwrap_or(&"unknown".to_string())
            .clone();
        for step in 0..=steps {
            let injection = (step as f64) * max_injection / (steps as f64);
            let host_network = add_virtual_der(&network, bus_id, injection, step);
            let artifact = out_dir.join(format!("hostcap_bus{}_step{}.parquet", bus_id, step));
            let solver = solver_kind.build_solver();
            let run_result = power_flow::ac_optimal_power_flow(
                &host_network,
                solver.as_ref(),
                1e-6,
                20,
                &artifact,
                &[],
            );
            let success = run_result.is_ok();
            if let Err(err) = run_result {
                eprintln!("hostcap run failed for bus {} step {}: {err}", bus_id, step);
            }
            summary_bus.push(bus_id as i64);
            summary_node.push(node_label.clone());
            summary_step.push(step as i64);
            summary_injection.push(injection);
            summary_success.push(success);
            summary_artifact.push(artifact.display().to_string());
        }
    }

    let detail = DataFrame::new(vec![
        Series::new("bus_id", summary_bus),
        Series::new("node_label", summary_node),
        Series::new("step", summary_step),
        Series::new("injection_mw", summary_injection),
        Series::new("success", summary_success),
        Series::new("artifact", summary_artifact),
    ])?;
    let detail_height = detail.height();
    write_parquet(out_dir.join("hostcap_summary.parquet"), detail)?;
    println!(
        "Hostcap sweep generated {} rows and artifacts in {}",
        detail_height,
        out_dir.display()
    );
    Ok(())
}

fn write_parquet(path: PathBuf, mut df: DataFrame) -> Result<()> {
    let mut file = File::create(&path).with_context(|| {
        format!(
            "creating Parquet output '{}'; ensure path exists",
            path.display()
        )
    })?;
    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(&mut df)
        .with_context(|| format!("writing Parquet table {}", path.display()))?;
    Ok(())
}

fn load_network(grid_file: &Path) -> Result<Network> {
    let grid_str = grid_file
        .to_str()
        .ok_or_else(|| anyhow!("grid path contains invalid UTF-8: {}", grid_file.display()))?;
    importers::load_grid_from_arrow(grid_str)
        .with_context(|| format!("loading grid arrow {}", grid_file.display()))
}

fn build_node_frame(network: &Network, feeder: &str) -> DataFrame {
    let mut load_map: HashMap<BusId, f64> = HashMap::new();
    let mut load_map_q: HashMap<BusId, f64> = HashMap::new();
    let mut gens: HashMap<BusId, usize> = HashMap::new();

    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Load(load) => {
                *load_map.entry(load.bus).or_insert(0.0) += load.active_power_mw;
                *load_map_q.entry(load.bus).or_insert(0.0) += load.reactive_power_mvar;
            }
            Node::Gen(gen) => {
                *gens.entry(gen.bus).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    let mut ids = Vec::new();
    let mut phases = Vec::new();
    let mut types = Vec::new();
    let mut v_min = Vec::new();
    let mut v_max = Vec::new();
    let mut load_p = Vec::new();
    let mut load_q = Vec::new();
    let mut feeders = Vec::new();

    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            ids.push(bus.id.value() as i64);
            phases.push("ABC".to_string());
            let node_type = if gens.get(&bus.id).is_some() {
                "source"
            } else {
                "load"
            };
            types.push(node_type.to_string());
            v_min.push(0.95);
            v_max.push(1.05);
            load_p.push(*load_map.get(&bus.id).unwrap_or(&0.0));
            load_q.push(*load_map_q.get(&bus.id).unwrap_or(&0.0));
            feeders.push(feeder.to_string());
        }
    }

    DataFrame::new(vec![
        Series::new("node_id", ids),
        Series::new("phase", phases),
        Series::new("node_type", types),
        Series::new("v_min", v_min),
        Series::new("v_max", v_max),
        Series::new("load_p_mw", load_p),
        Series::new("load_q_mvar", load_q),
        Series::new("feeder_id", feeders),
    ])
    .expect("dist pointer frame should always construct")
}

fn build_branch_frame(network: &Network, _feeder: &str) -> DataFrame {
    let mut ids = Vec::new();
    let mut from_nodes = Vec::new();
    let mut to_nodes = Vec::new();
    let mut r = Vec::new();
    let mut x = Vec::new();
    let mut b = Vec::new();
    let mut tap = Vec::new();
    let mut status = Vec::new();
    let mut thermal = Vec::new();

    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            ids.push(branch.id.value() as i64);
            from_nodes.push(branch.from_bus.value() as i64);
            to_nodes.push(branch.to_bus.value() as i64);
            r.push(branch.resistance);
            x.push(branch.reactance);
            b.push(0.0);
            tap.push(1.0);
            status.push("closed".to_string());
            thermal.push(1e6);
        }
    }

    DataFrame::new(vec![
        Series::new("branch_id", ids),
        Series::new("from_node", from_nodes),
        Series::new("to_node", to_nodes),
        Series::new("r", r),
        Series::new("x", x),
        Series::new("b", b),
        Series::new("tap", tap),
        Series::new("status", status),
        Series::new("thermal_limit", thermal),
    ])
    .expect("dist branch frame should always construct")
}

fn collect_bus_names(network: &Network) -> HashMap<usize, String> {
    let mut map = HashMap::new();
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            map.insert(bus.id.value(), bus.name.clone());
        }
    }
    map
}

fn add_virtual_der(network: &Network, bus_id: usize, injection: f64, step: usize) -> Network {
    let mut clone = Network {
        graph: network.graph.clone(),
    };
    let gen_id = GenId::new(clone.graph.node_count());
    let der = Gen {
        id: gen_id,
        name: format!("hostcap_der_{}_{}", bus_id, step),
        bus: BusId::new(bus_id),
        active_power_mw: injection,
        reactive_power_mvar: 0.0,
    };
    clone.graph.add_node(Node::Gen(der));
    clone
}
