//! Network inspection and diagnostic commands.
//!
//! Deep-dive analysis tools for understanding network structure,
//! identifying anomalies, and debugging import issues.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use gat_cli::cli::InspectCommands;
use gat_cli::common::{OutputFormat, write_json, write_jsonl, write_csv_from_json};
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
        InspectCommands::Thermal {
            input,
            threshold,
            format,
        } => handle_thermal(input, *threshold, format),
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
                total_gen_pmax += g.pmax.value();
                total_gen_pmin += g.pmin.value();
            }
            Node::Load(l) => {
                load_count += 1;
                total_load_p += l.active_power.value();
                total_load_q += l.reactive_power.value();
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

fn handle_generators(input: &str, bus_filter: Option<usize>, format: &OutputFormat) -> Result<()> {
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
                pmin_mw: g.pmin.value(),
                pmax_mw: g.pmax.value(),
                qmin_mvar: g.qmin.value(),
                qmax_mvar: g.qmax.value(),
                pg_mw: g.active_power.value(),
                qg_mvar: g.reactive_power.value(),
            });
        }
    }

    // Sort by bus then id
    generators.sort_by(|a, b| a.bus.cmp(&b.bus).then(a.id.cmp(&b.id)));

    match format {
        OutputFormat::Table => {
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
        OutputFormat::Json => {
            // Convert to serde_json::Value for consistency with writer functions
            let json_data: Vec<serde_json::Value> = generators
                .iter()
                .map(|g| serde_json::to_value(g).unwrap())
                .collect();
            write_json(&json_data, &mut std::io::stdout(), true)?;
        }
        OutputFormat::Csv => {
            let json_data: Vec<serde_json::Value> = generators
                .iter()
                .map(|g| serde_json::to_value(g).unwrap())
                .collect();
            write_csv_from_json(&json_data, &mut std::io::stdout())?;
        }
        OutputFormat::Jsonl => {
            let json_data: Vec<serde_json::Value> = generators
                .iter()
                .map(|g| serde_json::to_value(g).unwrap())
                .collect();
            write_jsonl(&json_data, &mut std::io::stdout())?;
        }
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

fn handle_branches(input: &str, rating_filter: Option<f64>, format: &OutputFormat) -> Result<()> {
    let network = load_network(input)?;

    let mut branches: Vec<BranchInfo> = Vec::new();

    for edge in network.graph.edge_weights() {
        if let Edge::Branch(b) = edge {
            if let Some(max_rating) = rating_filter {
                if let Some(rating) = b.s_max {
                    if rating.value() >= max_rating {
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
                b_pu: b.charging_b.value(),
                rating_mva: b.s_max.map(|v| v.value()),
                tap_ratio: b.tap_ratio,
            });
        }
    }

    // Sort by from_bus, then to_bus
    branches.sort_by(|a, b| a.from_bus.cmp(&b.from_bus).then(a.to_bus.cmp(&b.to_bus)));

    match format {
        OutputFormat::Table => {
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
        OutputFormat::Json => {
            let json_data: Vec<serde_json::Value> = branches
                .iter()
                .map(|b| serde_json::to_value(b).unwrap())
                .collect();
            write_json(&json_data, &mut std::io::stdout(), true)?;
        }
        OutputFormat::Csv => {
            let json_data: Vec<serde_json::Value> = branches
                .iter()
                .map(|b| serde_json::to_value(b).unwrap())
                .collect();
            write_csv_from_json(&json_data, &mut std::io::stdout())?;
        }
        OutputFormat::Jsonl => {
            let json_data: Vec<serde_json::Value> = branches
                .iter()
                .map(|b| serde_json::to_value(b).unwrap())
                .collect();
            write_jsonl(&json_data, &mut std::io::stdout())?;
        }
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
                total_gen_pmax += g.pmax.value();
                total_gen_pmin += g.pmin.value();
                total_gen_qmax += g.qmax.value();
                total_gen_qmin += g.qmin.value();
                gen_count += 1;
            }
            Node::Load(l) => {
                total_load_p += l.active_power.value();
                total_load_q += l.reactive_power.value();
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
                    voltage_kv: b.base_kv.value(),
                    vmin_pu: b.vmin_pu.map(|v| v.value()),
                    vmax_pu: b.vmax_pu.map(|v| v.value()),
                });
            }
            Node::Gen(g) => {
                generators.push(GeneratorInfo {
                    id: g.id.value(),
                    name: g.name.clone(),
                    bus: g.bus.value(),
                    pmin_mw: g.pmin.value(),
                    pmax_mw: g.pmax.value(),
                    qmin_mvar: g.qmin.value(),
                    qmax_mvar: g.qmax.value(),
                    pg_mw: g.active_power.value(),
                    qg_mvar: g.reactive_power.value(),
                });
            }
            Node::Load(l) => {
                loads.push(LoadJson {
                    id: l.id.value(),
                    name: l.name.clone(),
                    bus: l.bus.value(),
                    p_mw: l.active_power.value(),
                    q_mvar: l.reactive_power.value(),
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
                b_pu: b.charging_b.value(),
                rating_mva: b.s_max.map(|v| v.value()),
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

/// Branch thermal loading information
#[derive(Debug, Serialize)]
struct ThermalInfo {
    id: usize,
    name: String,
    from_bus: usize,
    to_bus: usize,
    rating_mva: f64,
    // Note: without OPF solution, we can't compute actual flow
    // This command shows branches by rating and flags potential bottlenecks
}

/// Handle the `gat inspect thermal` command
/// Shows branches sorted by thermal rating to identify potential bottlenecks
pub fn handle_thermal(input: &str, threshold_mva: Option<f64>, format: &OutputFormat) -> Result<()> {
    let network = load_network(input)?;

    let mut branches: Vec<(usize, String, usize, usize, f64)> = Vec::new();

    for edge in network.graph.edge_weights() {
        if let Edge::Branch(b) = edge {
            if let Some(rating) = b.s_max {
                let rating_val = rating.value();
                // Filter by threshold if provided
                if let Some(thresh) = threshold_mva {
                    if rating_val > thresh {
                        continue;
                    }
                }
                branches.push((
                    b.id.value(),
                    b.name.clone(),
                    b.from_bus.value(),
                    b.to_bus.value(),
                    rating_val,
                ));
            }
        }
    }

    // Sort by rating (ascending) - lowest ratings are most likely bottlenecks
    branches.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap());

    match format {
        OutputFormat::Table => {
            println!("Thermal Limit Analysis");
            println!("======================");
            println!();

            if branches.is_empty() {
                println!("No branches with thermal ratings found.");
                return Ok(());
            }

            // Statistics
            let ratings: Vec<f64> = branches.iter().map(|(_, _, _, _, r)| *r).collect();
            let min_rating = ratings.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_rating = ratings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let avg_rating: f64 = ratings.iter().sum::<f64>() / ratings.len() as f64;

            println!("Rating Statistics:");
            println!("  Min:  {:.1} MVA", min_rating);
            println!("  Max:  {:.1} MVA", max_rating);
            println!("  Avg:  {:.1} MVA", avg_rating);
            println!();

            // Show lowest-rated branches (potential bottlenecks)
            let show_count = branches.len().min(20);
            println!(
                "Lowest-rated branches (top {} potential bottlenecks):",
                show_count
            );
            println!();
            println!(
                "{:>6} {:>6} {:>6} {:>20} {:>12}",
                "ID", "From", "To", "Name", "Rating (MVA)"
            );
            println!("{}", "-".repeat(60));

            for (id, name, from, to, rating) in branches.iter().take(show_count) {
                let status = if *rating < avg_rating * 0.5 {
                    " ⚠ LOW"
                } else {
                    ""
                };
                println!(
                    "{:>6} {:>6} {:>6} {:>20} {:>12.1}{}",
                    id,
                    from,
                    to,
                    truncate(name, 20),
                    rating,
                    status
                );
            }

            println!();
            println!("Total: {} branches with thermal limits", branches.len());

            // Identify potential critical paths
            let low_rated: Vec<_> = branches
                .iter()
                .filter(|(_, _, _, _, r)| *r < avg_rating * 0.5)
                .collect();
            if !low_rated.is_empty() {
                println!();
                println!(
                    "⚠ {} branches have ratings < 50% of average ({:.1} MVA)",
                    low_rated.len(),
                    avg_rating * 0.5
                );
                println!("  These may become thermal bottlenecks under high load.");
            }
        }
        OutputFormat::Json => {
            let thermal_info: Vec<ThermalInfo> = branches
                .iter()
                .map(|(id, name, from, to, rating)| ThermalInfo {
                    id: *id,
                    name: name.clone(),
                    from_bus: *from,
                    to_bus: *to,
                    rating_mva: *rating,
                })
                .collect();
            let json_data: Vec<serde_json::Value> = thermal_info
                .iter()
                .map(|t| serde_json::to_value(t).unwrap())
                .collect();
            write_json(&json_data, &mut std::io::stdout(), true)?;
        }
        OutputFormat::Csv => {
            let thermal_info: Vec<ThermalInfo> = branches
                .iter()
                .map(|(id, name, from, to, rating)| ThermalInfo {
                    id: *id,
                    name: name.clone(),
                    from_bus: *from,
                    to_bus: *to,
                    rating_mva: *rating,
                })
                .collect();
            let json_data: Vec<serde_json::Value> = thermal_info
                .iter()
                .map(|t| serde_json::to_value(t).unwrap())
                .collect();
            write_csv_from_json(&json_data, &mut std::io::stdout())?;
        }
        OutputFormat::Jsonl => {
            let thermal_info: Vec<ThermalInfo> = branches
                .iter()
                .map(|(id, name, from, to, rating)| ThermalInfo {
                    id: *id,
                    name: name.clone(),
                    from_bus: *from,
                    to_bus: *to,
                    rating_mva: *rating,
                })
                .collect();
            let json_data: Vec<serde_json::Value> = thermal_info
                .iter()
                .map(|t| serde_json::to_value(t).unwrap())
                .collect();
            write_jsonl(&json_data, &mut std::io::stdout())?;
        }
    }

    Ok(())
}
