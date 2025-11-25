use std::{collections::HashMap, fs::File, path::Path};

use anyhow::{anyhow, Context, Result};
use caseformat::{read_dir, read_zip, Branch as CaseBranch, Bus as CaseBus, Gen as CaseGen};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
};

use super::arrow::write_network_to_arrow;
use super::matpower_parser::{parse_matpower_file, MatpowerCase};

/// Load a MATPOWER case file and return a Network (without writing to disk)
///
/// Supports:
/// - Single .m file (MATPOWER format)
/// - Directory containing CSV files (caseformat)
/// - Zip archive containing CSV files (caseformat)
/// - Directory containing .m files
pub fn load_matpower_network(m_file: &Path) -> Result<Network> {
    // If it's a single .m file, use our parser
    if m_file.is_file() {
        if let Some(ext) = m_file.extension() {
            if ext == "m" {
                let case = parse_matpower_file(m_file)?;
                return build_network_from_matpower_case(&case);
            }
        }
        // Try as zip archive
        let file = File::open(m_file).with_context(|| {
            format!(
                "opening MATPOWER case file '{}'; expected zip archive",
                m_file.display()
            )
        })?;
        let (_case, buses, gens, branches, _gencost, _dcline, _readme, _license) =
            read_zip(file).with_context(|| {
                format!(
                    "reading MATPOWER zip '{}'; failed to parse",
                    m_file.display()
                )
            })?;
        return build_network_from_case(buses, branches, gens);
    }

    // Directory - check if it has .m files or CSV files
    if m_file.is_dir() {
        // Check for .m files first
        let m_files: Vec<_> = std::fs::read_dir(m_file)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "m").unwrap_or(false))
            .collect();

        if !m_files.is_empty() {
            // Find the main case file (usually named case.m or the directory name.m)
            let case_file = m_files
                .iter()
                .find(|e| e.path().file_stem().map(|s| s == "case").unwrap_or(false))
                .or_else(|| m_files.first())
                .map(|e| e.path())
                .ok_or_else(|| anyhow!("no .m files found in directory"))?;

            let case = parse_matpower_file(&case_file)?;
            return build_network_from_matpower_case(&case);
        }

        // Try caseformat CSV directory
        let dir_path = m_file.to_path_buf();
        let (_case, buses, gens, branches, _gencost, _dcline, _readme, _license) =
            read_dir(&dir_path).with_context(|| {
                format!(
                    "reading MATPOWER directory '{}'; expected case data with CSV or .m files",
                    m_file.display()
                )
            })?;
        return build_network_from_case(buses, branches, gens);
    }

    Err(anyhow!(
        "MATPOWER path '{}' is neither a file nor a directory",
        m_file.display()
    ))
}

pub fn import_matpower_case(m_file: &str, output_file: &str) -> Result<Network> {
    println!("Importing MATPOWER from {} to {}", m_file, output_file);
    let path = Path::new(m_file);
    let network = load_matpower_network(path)?;
    write_network_to_arrow(&network, output_file)?;
    Ok(network)
}

/// Build network from our MATPOWER parser output
fn build_network_from_matpower_case(case: &MatpowerCase) -> Result<Network> {
    let mut network = Network::new();
    let mut bus_index_map: HashMap<usize, NodeIndex> = HashMap::new();

    // Add buses
    for bus in &case.bus {
        let bus_id = BusId::new(bus.bus_i);
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: bus_id,
            name: format!("Bus {}", bus.bus_i),
            voltage_kv: bus.base_kv,
        }));
        bus_index_map.insert(bus.bus_i, node_idx);
    }

    // Add loads
    let mut load_id = 0usize;
    for bus in &case.bus {
        if bus.pd != 0.0 || bus.qd != 0.0 {
            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(load_id),
                name: format!("Load {}", bus.bus_i),
                bus: BusId::new(bus.bus_i),
                active_power_mw: bus.pd,
                reactive_power_mvar: bus.qd,
            }));
            load_id += 1;
        }
    }

    // Add generators
    let mut gen_id = 0usize;
    for gen in &case.gen {
        if gen.gen_status == 0 {
            continue;
        }
        if !bus_index_map.contains_key(&gen.gen_bus) {
            return Err(anyhow!("generator references unknown bus {}", gen.gen_bus));
        }
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(gen_id),
            name: format!("Gen {}@{}", gen_id, gen.gen_bus),
            bus: BusId::new(gen.gen_bus),
            active_power_mw: gen.pg,
            reactive_power_mvar: gen.qg,
        }));
        gen_id += 1;
    }

    // Add branches
    let mut branch_id = 0usize;
    for br in &case.branch {
        if br.br_status == 0 {
            continue;
        }

        let from_idx = *bus_index_map
            .get(&br.f_bus)
            .with_context(|| format!("branch references unknown from bus {}", br.f_bus))?;
        let to_idx = *bus_index_map
            .get(&br.t_bus)
            .with_context(|| format!("branch references unknown to bus {}", br.t_bus))?;

        let branch = Branch {
            id: BranchId::new(branch_id),
            name: format!("Branch {}-{}", br.f_bus, br.t_bus),
            from_bus: BusId::new(br.f_bus),
            to_bus: BusId::new(br.t_bus),
            resistance: br.br_r,
            reactance: br.br_x,
        };

        network
            .graph
            .add_edge(from_idx, to_idx, Edge::Branch(branch));

        branch_id += 1;
    }

    Ok(network)
}

/// Build network from caseformat structs
fn build_network_from_case(
    case_buses: Vec<CaseBus>,
    case_branches: Vec<CaseBranch>,
    case_gens: Vec<CaseGen>,
) -> Result<Network> {
    let mut network = Network::new();
    let mut bus_index_map: HashMap<usize, NodeIndex> = HashMap::new();
    for case_bus in &case_buses {
        let bus_id = BusId::new(case_bus.bus_i);
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: bus_id,
            name: format!("Bus {}", case_bus.bus_i),
            voltage_kv: case_bus.base_kv,
        }));
        bus_index_map.insert(case_bus.bus_i, node_idx);
    }

    let mut load_id = 0usize;
    for case_bus in &case_buses {
        if case_bus.pd != 0.0 || case_bus.qd != 0.0 {
            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(load_id),
                name: format!("Load {}", case_bus.bus_i),
                bus: BusId::new(case_bus.bus_i),
                active_power_mw: case_bus.pd,
                reactive_power_mvar: case_bus.qd,
            }));
            load_id += 1;
        }
    }

    let mut gen_id = 0usize;
    for case_gen in case_gens {
        if case_gen.gen_status == 0 {
            continue;
        }
        if !bus_index_map.contains_key(&case_gen.gen_bus) {
            return Err(anyhow!(
                "generator references unknown bus {}",
                case_gen.gen_bus
            ));
        }
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(gen_id),
            name: format!("Gen {}@{}", gen_id, case_gen.gen_bus),
            bus: BusId::new(case_gen.gen_bus),
            active_power_mw: case_gen.pg,
            reactive_power_mvar: case_gen.qg,
        }));
        gen_id += 1;
    }

    let mut branch_id = 0usize;
    for case_branch in case_branches {
        if !case_branch.is_on() {
            continue;
        }

        let from_idx = *bus_index_map
            .get(&case_branch.f_bus)
            .with_context(|| format!("branch references unknown from bus {}", case_branch.f_bus))?;
        let to_idx = *bus_index_map
            .get(&case_branch.t_bus)
            .with_context(|| format!("branch references unknown to bus {}", case_branch.t_bus))?;

        let branch = Branch {
            id: BranchId::new(branch_id),
            name: format!("Branch {}-{}", case_branch.f_bus, case_branch.t_bus),
            from_bus: BusId::new(case_branch.f_bus),
            to_bus: BusId::new(case_branch.t_bus),
            resistance: case_branch.br_r,
            reactance: case_branch.br_x,
        };

        network
            .graph
            .add_edge(from_idx, to_idx, Edge::Branch(branch));

        branch_id += 1;
    }

    Ok(network)
}
