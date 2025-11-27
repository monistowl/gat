//! Network inspection and diagnostic commands.
//!
//! Deep-dive analysis tools for understanding network structure,
//! identifying anomalies, and debugging import issues.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use gat_cli::cli::InspectCommands;
use gat_core::{Edge, Network, Node};
use gat_io::importers::{load_grid_from_arrow, Format};
use serde::Serialize;

pub fn handle(command: &InspectCommands) -> Result<()> {
    match command {
        InspectCommands::Summary { input } => handle_summary(input),
        InspectCommands::Generators { input, bus, format } => {
            handle_generators(input, *bus, format)
        }
        InspectCommands::Branches {
            input,
            rating_lt,
            format,
        } => handle_branches(input, *rating_lt, format),
        InspectCommands::PowerBalance { input } => handle_power_balance(input),
        InspectCommands::Json { input, pretty } => handle_json(input, *pretty),
    }
}

/// Load a network from Arrow directory or auto-detect format
fn load_network(input: &str) -> Result<Network> {
    let path = Path::new(input);
    if !path.exists() {
        bail!("Input '{}' does not exist", input);
    }

    // Try Arrow directory first
    if path.is_dir() && path.join("manifest.json").exists() {
        return load_grid_from_arrow(input).context("loading Arrow directory");
    }

    // Try auto-detect format
    if let Some((format, _confidence)) = Format::detect(path) {
        let result = format.parse(input)?;
        return Ok(result.network);
    }

    bail!(
        "Unable to determine format for '{}'. Use Arrow directory or supported format.",
        input
    );
}

/// Summary statistics for the network
fn handle_summary(input: &str) -> Result<()> {
    let network = load_network(input)?;

    let mut bus_count = 0;
    let mut gen_count = 0;
    let mut load_count = 0;
    let mut shunt_count = 0;
    let mut branch_count = 0;

    let mut total_gen_pmax = 0.0;
    let mut total_gen_pmin = 0.0;
    let mut total_load_p = 0.0;
    let mut total_load_q = 0.0;

    // Count nodes
    for node in network.graph.node_weights() {
        match node {
            Node::Bus(_) => bus_count += 1,
            Node::Gen(g) => {
                gen_count += 1;
                total_gen_pmax += g.pmax_mw;
                total_gen_pmin += g.pmin_mw;
            }
            Node::Load(l) => {
                load_count += 1;
                total_load_p += l.active_power_mw;
                total_load_q += l.reactive_power_mvar;
            }
            Node::Shunt(_) => shunt_count += 1,
        }
    }

    // Count edges
    for edge in network.graph.edge_weights() {
        if matches!(edge, Edge::Branch(_)) {
            branch_count += 1;
        }
    }

    // Count generators per bus
    let mut gen_bus_counts: HashMap<usize, usize> = HashMap::new();
    for node in network.graph.node_weights() {
        if let Node::Gen(g) = node {
            *gen_bus_counts.entry(g.bus.value()).or_insert(0) += 1;
        }
    }

    println!("Network Summary");
    println!("===============");
    println!();
    println!("Components:");
    println!("  Buses:      {}", bus_count);
    println!("  Branches:   {}", branch_count);
    println!("  Generators: {}", gen_count);
    println!("  Loads:      {}", load_count);
    println!("  Shunts:     {}", shunt_count);
    println!();
    println!("Generation:");
    println!("  Total Pmax: {:.2} MW", total_gen_pmax);
    println!("  Total Pmin: {:.2} MW", total_gen_pmin);
    println!();
    println!("Load:");
    println!("  Total P:    {:.2} MW", total_load_p);
    println!("  Total Q:    {:.2} MVAr", total_load_q);
    println!();
    println!("Reserve Margin:");
    let reserve = if total_load_p > 0.0 {
        (total_gen_pmax - total_load_p) / total_load_p * 100.0
    } else {
        f64::INFINITY
    };
    println!("  (Pmax - Load) / Load = {:.1}%", reserve);

    // Distribution analysis
    let unique_gen_buses = gen_bus_counts.len();
    if gen_count > 1 && unique_gen_buses == 1 {
        let bus_id = gen_bus_counts.keys().next().unwrap();
        println!();
        println!(
            "WARNING: All {} generators at bus {} - likely mapping bug!",
            gen_count, bus_id
        );
    }

    Ok(())
}

/// Generator information for JSON output
#[derive(Serialize)]
struct GeneratorInfo {
    id: usize,
    name: String,
    bus: usize,
    pmin_mw: f64,
    pmax_mw: f64,
    qmin_mvar: f64,
    qmax_mvar: f64,
    pg_mw: f64,
    qg_mvar: f64,
}

fn handle_generators(input: &str, bus_filter: Option<usize>, format: &str) -> Result<()> {
    let network = load_network(input)?;

    let mut generators: Vec<GeneratorInfo> = Vec::new();

    for node in network.graph.node_weights() {
        if let Node::Gen(g) = node {
            if let Some(filter_bus) = bus_filter {
                if g.bus.value() != filter_bus {
                    continue;
                }
            }
            generators.push(GeneratorInfo {
                id: g.id.value(),
                name: g.name.clone(),
                bus: g.bus.value(),
                pmin_mw: g.pmin_mw,
                pmax_mw: g.pmax_mw,
                qmin_mvar: g.qmin_mvar,
                qmax_mvar: g.qmax_mvar,
                pg_mw: g.active_power_mw,
                qg_mvar: g.reactive_power_mvar,
            });
        }
    }

    // Sort by bus then id
    generators.sort_by(|a, b| a.bus.cmp(&b.bus).then(a.id.cmp(&b.id)));

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&generators)?);
    } else {
        // Table format
        println!(
            "{:>6} {:>6} {:>20} {:>10} {:>10} {:>10} {:>10}",
            "ID", "Bus", "Name", "Pmin", "Pmax", "Qmin", "Qmax"
        );
        println!("{}", "-".repeat(80));
        for g in &generators {
            println!(
                "{:>6} {:>6} {:>20} {:>10.2} {:>10.2} {:>10.2} {:>10.2}",
                g.id,
                g.bus,
                truncate(&g.name, 20),
                g.pmin_mw,
                g.pmax_mw,
                g.qmin_mvar,
                g.qmax_mvar
            );
        }
        println!();
        println!("Total: {} generators", generators.len());
    }

    Ok(())
}

/// Branch information for JSON output
#[derive(Serialize)]
struct BranchInfo {
    id: usize,
    name: String,
    from_bus: usize,
    to_bus: usize,
    r_pu: f64,
    x_pu: f64,
    b_pu: f64,
    rating_mva: Option<f64>,
    tap_ratio: f64,
}

fn handle_branches(input: &str, rating_filter: Option<f64>, format: &str) -> Result<()> {
    let network = load_network(input)?;

    let mut branches: Vec<BranchInfo> = Vec::new();

    for edge in network.graph.edge_weights() {
        if let Edge::Branch(b) = edge {
            if let Some(max_rating) = rating_filter {
                if let Some(rating) = b.s_max_mva {
                    if rating >= max_rating {
                        continue;
                    }
                }
            }
            branches.push(BranchInfo {
                id: b.id.value(),
                name: b.name.clone(),
                from_bus: b.from_bus.value(),
                to_bus: b.to_bus.value(),
                r_pu: b.resistance,
                x_pu: b.reactance,
                b_pu: b.charging_b_pu,
                rating_mva: b.s_max_mva,
                tap_ratio: b.tap_ratio,
            });
        }
    }

    // Sort by from_bus, then to_bus
    branches.sort_by(|a, b| a.from_bus.cmp(&b.from_bus).then(a.to_bus.cmp(&b.to_bus)));

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&branches)?);
    } else {
        // Table format
        println!(
            "{:>6} {:>6} {:>6} {:>20} {:>10} {:>10} {:>10}",
            "ID", "From", "To", "Name", "R (pu)", "X (pu)", "Rating"
        );
        println!("{}", "-".repeat(80));
        for b in &branches {
            let rating_str = b
                .rating_mva
                .map(|r| format!("{:.1}", r))
                .unwrap_or_else(|| "-".to_string());
            println!(
                "{:>6} {:>6} {:>6} {:>20} {:>10.5} {:>10.5} {:>10}",
                b.id,
                b.from_bus,
                b.to_bus,
                truncate(&b.name, 20),
                b.r_pu,
                b.x_pu,
                rating_str
            );
        }
        println!();
        println!("Total: {} branches", branches.len());
    }

    Ok(())
}

fn handle_power_balance(input: &str) -> Result<()> {
    let network = load_network(input)?;

    let mut total_gen_pmax = 0.0;
    let mut total_gen_pmin = 0.0;
    let mut total_gen_qmax = 0.0;
    let mut total_gen_qmin = 0.0;
    let mut total_load_p = 0.0;
    let mut total_load_q = 0.0;
    let mut total_shunt_g = 0.0;
    let mut total_shunt_b = 0.0;
    let mut gen_count = 0;
    let mut load_count = 0;

    for node in network.graph.node_weights() {
        match node {
            Node::Gen(g) => {
                total_gen_pmax += g.pmax_mw;
                total_gen_pmin += g.pmin_mw;
                total_gen_qmax += g.qmax_mvar;
                total_gen_qmin += g.qmin_mvar;
                gen_count += 1;
            }
            Node::Load(l) => {
                total_load_p += l.active_power_mw;
                total_load_q += l.reactive_power_mvar;
                load_count += 1;
            }
            Node::Shunt(s) => {
                total_shunt_g += s.gs_pu;
                total_shunt_b += s.bs_pu;
            }
            _ => {}
        }
    }

    println!("Power Balance Analysis");
    println!("======================");
    println!();
    println!("Generation Capacity ({} units):", gen_count);
    println!(
        "  P range: [{:.2}, {:.2}] MW",
        total_gen_pmin, total_gen_pmax
    );
    println!(
        "  Q range: [{:.2}, {:.2}] MVAr",
        total_gen_qmin, total_gen_qmax
    );
    println!();
    println!("Load ({} units):", load_count);
    println!("  P total: {:.2} MW", total_load_p);
    println!("  Q total: {:.2} MVAr", total_load_q);
    println!();
    println!("Shunt Elements:");
    println!("  G total: {:.4} pu", total_shunt_g);
    println!("  B total: {:.4} pu", total_shunt_b);
    println!();
    println!("Feasibility Check:");
    if total_gen_pmax < total_load_p {
        println!(
            "  INFEASIBLE: Gen Pmax ({:.2}) < Load P ({:.2})",
            total_gen_pmax, total_load_p
        );
    } else if total_gen_pmin > total_load_p {
        println!(
            "  WARNING: Gen Pmin ({:.2}) > Load P ({:.2}) - over-generation",
            total_gen_pmin, total_load_p
        );
    } else {
        println!("  OK: Load within generation limits");
        let headroom = total_gen_pmax - total_load_p;
        let reserve_pct = headroom / total_load_p * 100.0;
        println!("  Headroom: {:.2} MW ({:.1}%)", headroom, reserve_pct);
    }

    Ok(())
}

/// Network data for JSON export
#[derive(Serialize)]
struct NetworkJson {
    buses: Vec<BusJson>,
    generators: Vec<GeneratorInfo>,
    loads: Vec<LoadJson>,
    branches: Vec<BranchInfo>,
}

#[derive(Serialize)]
struct BusJson {
    id: usize,
    name: String,
    voltage_kv: f64,
    vmin_pu: Option<f64>,
    vmax_pu: Option<f64>,
}

#[derive(Serialize)]
struct LoadJson {
    id: usize,
    name: String,
    bus: usize,
    p_mw: f64,
    q_mvar: f64,
}

fn handle_json(input: &str, pretty: bool) -> Result<()> {
    let network = load_network(input)?;

    let mut buses = Vec::new();
    let mut generators = Vec::new();
    let mut loads = Vec::new();
    let mut branches = Vec::new();

    for node in network.graph.node_weights() {
        match node {
            Node::Bus(b) => {
                buses.push(BusJson {
                    id: b.id.value(),
                    name: b.name.clone(),
                    voltage_kv: b.voltage_kv,
                    vmin_pu: b.vmin_pu,
                    vmax_pu: b.vmax_pu,
                });
            }
            Node::Gen(g) => {
                generators.push(GeneratorInfo {
                    id: g.id.value(),
                    name: g.name.clone(),
                    bus: g.bus.value(),
                    pmin_mw: g.pmin_mw,
                    pmax_mw: g.pmax_mw,
                    qmin_mvar: g.qmin_mvar,
                    qmax_mvar: g.qmax_mvar,
                    pg_mw: g.active_power_mw,
                    qg_mvar: g.reactive_power_mvar,
                });
            }
            Node::Load(l) => {
                loads.push(LoadJson {
                    id: l.id.value(),
                    name: l.name.clone(),
                    bus: l.bus.value(),
                    p_mw: l.active_power_mw,
                    q_mvar: l.reactive_power_mvar,
                });
            }
            Node::Shunt(_) => {} // Skip shunts for now
        }
    }

    for edge in network.graph.edge_weights() {
        if let Edge::Branch(b) = edge {
            branches.push(BranchInfo {
                id: b.id.value(),
                name: b.name.clone(),
                from_bus: b.from_bus.value(),
                to_bus: b.to_bus.value(),
                r_pu: b.resistance,
                x_pu: b.reactance,
                b_pu: b.charging_b_pu,
                rating_mva: b.s_max_mva,
                tap_ratio: b.tap_ratio,
            });
        }
    }

    let output = NetworkJson {
        buses,
        generators,
        loads,
        branches,
    };

    let json = if pretty {
        serde_json::to_string_pretty(&output)?
    } else {
        serde_json::to_string(&output)?
    };
    println!("{}", json);

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
