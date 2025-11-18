use std::{collections::HashMap, fs::File, path::Path};

use anyhow::{anyhow, Context, Result};
use caseformat::{read_dir, read_zip, Branch as CaseBranch, Bus as CaseBus, Gen as CaseGen};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
};

use super::arrow::write_network_to_arrow;

pub fn import_matpower_case(m_file: &str, output_file: &str) -> Result<Network> {
    println!("Importing MATPOWER from {} to {}", m_file, output_file);
    let path = Path::new(m_file);
    let (_case, buses, gens, branches, _gencost, _dcline, _readme, _license) = if path.is_dir() {
        let dir_path = path.to_path_buf();
        read_dir(&dir_path).with_context(|| {
            format!(
                "reading MATPOWER directory '{}'; expected case data",
                m_file
            )
        })?
    } else {
        let file = File::open(path)
            .with_context(|| format!("opening MATPOWER case file '{}'; expected zip", m_file))?;
        read_zip(file)
            .with_context(|| format!("reading MATPOWER zip '{}'; failed to parse", m_file))?
    };

    let network = build_network_from_case(buses, branches, gens)?;
    write_network_to_arrow(&network, output_file)?;
    Ok(network)
}

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
