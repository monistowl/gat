use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
};

use super::arrow::write_network_to_arrow;

pub fn import_psse_raw(raw_file: &str, output_file: &str) -> Result<Network> {
    println!("Importing PSSE RAW from {} to {}", raw_file, output_file);
    let path = Path::new(raw_file);
    let (buses, branches, loads, gens) = parse_psse_raw(path)?;
    let network = build_network_from_psse(buses, branches, loads, gens)?;
    write_network_to_arrow(&network, output_file)?;
    Ok(network)
}

struct PsseBus {
    id: usize,
    name: String,
    voltage_kv: f64,
}

struct PsseBranch {
    from: usize,
    to: usize,
    resistance: f64,
    reactance: f64,
    in_service: bool,
}

struct PsseLoad {
    bus: usize,
    pd: f64,
    qd: f64,
}

struct PsseGen {
    bus: usize,
    pg: f64,
    qg: f64,
    status: i32,
}

type PsseRawTables = (Vec<PsseBus>, Vec<PsseBranch>, Vec<PsseLoad>, Vec<PsseGen>);

#[derive(PartialEq, Eq)]
enum PsseSection {
    None,
    Bus,
    Branch,
    Load,
    Generator,
}

fn parse_psse_raw(path: &Path) -> Result<PsseRawTables> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("reading PSSE RAW '{}'; ensure file exists", path.display()))?;
    let mut section = PsseSection::None;
    let mut buses = Vec::new();
    let mut branches = Vec::new();
    let mut loads = Vec::new();
    let mut gens = Vec::new();

    for raw_line in contents.lines() {
        let line = raw_line.split('/').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        match line.to_ascii_uppercase().as_str() {
            "BUS DATA FOLLOWS" => {
                section = PsseSection::Bus;
                continue;
            }
            "END OF BUS DATA" => {
                section = PsseSection::None;
                continue;
            }
            "BRANCH DATA FOLLOWS" => {
                section = PsseSection::Branch;
                continue;
            }
            "END OF BRANCH DATA" => {
                section = PsseSection::None;
                continue;
            }
            "LOAD DATA FOLLOWS" => {
                section = PsseSection::Load;
                continue;
            }
            "END OF LOAD DATA" => {
                section = PsseSection::None;
                continue;
            }
            "GENERATOR DATA FOLLOWS" => {
                section = PsseSection::Generator;
                continue;
            }
            "END OF GENERATOR DATA" => {
                section = PsseSection::None;
                continue;
            }
            _ => {}
        }

        match section {
            PsseSection::Bus => {
                if let Some(bus) = parse_psse_bus_line(line) {
                    buses.push(bus);
                }
            }
            PsseSection::Branch => {
                if let Some(branch) = parse_psse_branch_line(line) {
                    branches.push(branch);
                }
            }
            PsseSection::Load => {
                if let Some(load) = parse_psse_load_line(line) {
                    loads.push(load);
                }
            }
            PsseSection::Generator => {
                if let Some(gen) = parse_psse_gen_line(line) {
                    gens.push(gen);
                }
            }
            PsseSection::None => {}
        }
    }

    Ok((buses, branches, loads, gens))
}

fn parse_psse_bus_line(line: &str) -> Option<PsseBus> {
    let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if columns.len() < 3 {
        return None;
    }

    let id = columns[0].parse::<usize>().ok()?;
    let name = columns[1]
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();
    let voltage_kv = columns[2].parse::<f64>().unwrap_or(0.0);

    Some(PsseBus {
        id,
        name,
        voltage_kv,
    })
}

fn parse_psse_branch_line(line: &str) -> Option<PsseBranch> {
    let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if columns.len() < 5 {
        return None;
    }

    let from = columns[0].parse::<usize>().ok()?;
    let to = columns[1].parse::<usize>().ok()?;
    let resistance = columns[3].parse::<f64>().unwrap_or(0.0);
    let reactance = columns[4].parse::<f64>().unwrap_or(0.0);
    let status = columns
        .get(10)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    Some(PsseBranch {
        from,
        to,
        resistance,
        reactance,
        in_service: status != 0,
    })
}

fn parse_psse_load_line(line: &str) -> Option<PsseLoad> {
    let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if columns.len() < 4 {
        return None;
    }

    let bus = columns[0].parse::<usize>().ok()?;
    let pd = columns
        .get(2)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let qd = columns
        .get(3)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Some(PsseLoad { bus, pd, qd })
}

fn parse_psse_gen_line(line: &str) -> Option<PsseGen> {
    let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if columns.len() < 5 {
        return None;
    }

    let bus = columns[0].parse::<usize>().ok()?;
    let pg = columns
        .get(2)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let qg = columns
        .get(3)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let status = columns
        .get(14)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    Some(PsseGen {
        bus,
        pg,
        qg,
        status,
    })
}

fn build_network_from_psse(
    buses: Vec<PsseBus>,
    branches: Vec<PsseBranch>,
    loads: Vec<PsseLoad>,
    gens: Vec<PsseGen>,
) -> Result<Network> {
    let mut network = Network::new();
    let mut bus_index_map: HashMap<usize, NodeIndex> = HashMap::new();

    for bus in buses {
        let id = BusId::new(bus.id);
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id,
            name: bus.name,
            voltage_kv: bus.voltage_kv,
        }));
        bus_index_map.insert(bus.id, node_idx);
    }

    let mut load_map: HashMap<usize, (f64, f64)> = HashMap::new();
    for load in loads {
        if !bus_index_map.contains_key(&load.bus) {
            continue;
        }
        let entry = load_map.entry(load.bus).or_insert((0.0, 0.0));
        entry.0 += load.pd;
        entry.1 += load.qd;
    }
    let mut load_id = 0usize;
    for (bus_idx, (pd, qd)) in load_map {
        if pd == 0.0 && qd == 0.0 {
            continue;
        }
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(load_id),
            name: format!("PSSE load @ bus {}", bus_idx),
            bus: BusId::new(bus_idx),
            active_power_mw: pd,
            reactive_power_mvar: qd,
        }));
        load_id += 1;
    }

    let mut gen_id = 0usize;
    for gen in gens.into_iter().filter(|g| g.status != 0) {
        if !bus_index_map.contains_key(&gen.bus) {
            continue;
        }
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(gen_id),
            name: format!("PSSE gen @ bus {}", gen.bus),
            bus: BusId::new(gen.bus),
            active_power_mw: gen.pg,
            reactive_power_mvar: gen.qg,
        }));
        gen_id += 1;
    }

    for (branch_id, branch) in branches.into_iter().filter(|b| b.in_service).enumerate() {
        let from_idx = *bus_index_map
            .get(&branch.from)
            .with_context(|| format!("PSSE branch references unknown bus {}", branch.from))?;
        let to_idx = *bus_index_map
            .get(&branch.to)
            .with_context(|| format!("PSSE branch references unknown bus {}", branch.to))?;

        let branch_record = Branch {
            id: BranchId::new(branch_id),
            name: format!("Branch {}-{}", branch.from, branch.to),
            from_bus: BusId::new(branch.from),
            to_bus: BusId::new(branch.to),
            resistance: branch.resistance,
            reactance: branch.reactance,
        };

        network
            .graph
            .add_edge(from_idx, to_idx, Edge::Branch(branch_record));
    }

    Ok(network)
}
