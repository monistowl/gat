use std::{collections::HashMap, fs::File, path::Path};

use anyhow::{anyhow, Context, Result};
use caseformat::{read_dir, read_zip, Branch as CaseBranch, Bus as CaseBus, Gen as CaseGen};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
};

use super::arrow::write_network_to_arrow;
use super::matpower_parser::{parse_matpower_file, MatpowerCase, MatpowerGenCost};

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

/// Convert MATPOWER gencost to CostModel
fn gencost_to_cost_model(gencost: Option<&MatpowerGenCost>) -> gat_core::CostModel {
    match gencost {
        None => gat_core::CostModel::NoCost,
        Some(gc) => match gc.model {
            2 => {
                // Polynomial cost: cost = c_n*P^n + ... + c_1*P + c_0
                // MATPOWER stores highest degree first: [c_n, ..., c_1, c_0]
                // CostModel expects lowest degree first: [c_0, c_1, ..., c_n]
                let coeffs: Vec<f64> = gc.cost.iter().rev().copied().collect();
                gat_core::CostModel::Polynomial(coeffs)
            }
            1 => {
                // Piecewise linear: pairs of (MW, $/hr)
                // gc.cost = [p1, c1, p2, c2, ...]
                let points: Vec<(f64, f64)> = gc
                    .cost
                    .chunks(2)
                    .filter_map(|chunk| {
                        if chunk.len() == 2 {
                            Some((chunk[0], chunk[1]))
                        } else {
                            None
                        }
                    })
                    .collect();
                gat_core::CostModel::PiecewiseLinear(points)
            }
            _ => gat_core::CostModel::NoCost,
        },
    }
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
    for (i, gen) in case.gen.iter().enumerate() {
        if gen.gen_status == 0 {
            continue;
        }
        if !bus_index_map.contains_key(&gen.gen_bus) {
            return Err(anyhow!("generator references unknown bus {}", gen.gen_bus));
        }
        // Synchronous condenser detection:
        // 1. Pmax <= 0 (can only absorb power or provide reactive support)
        // 2. Negative active power setpoint (absorbing P)
        // 3. Negative Pmin with Pmax near zero (typical syncon with small motor load)
        // Note: Some syncons have Qmax=Qmin=0 for units without Q capability
        let is_syncon = gen.pmax <= 0.0 || gen.pg < 0.0 || (gen.pmin < 0.0 && gen.pmax <= 0.1);
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(gen_id),
            name: format!("Gen {}@{}", gen_id, gen.gen_bus),
            bus: BusId::new(gen.gen_bus),
            active_power_mw: gen.pg,
            reactive_power_mvar: gen.qg,
            pmin_mw: gen.pmin,
            pmax_mw: gen.pmax,
            qmin_mvar: gen.qmin,
            qmax_mvar: gen.qmax,
            cost_model: gencost_to_cost_model(case.gencost.get(i)),
            is_synchronous_condenser: is_syncon,
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

        // Phase-shifter detection: non-zero phase shift OR negative reactance OR negative resistance
        let is_phase_shifter = br.shift.abs() > 1e-6 || br.br_x < 0.0 || br.br_r < 0.0;

        let branch = Branch {
            id: BranchId::new(branch_id),
            name: format!("Branch {}-{}", br.f_bus, br.t_bus),
            from_bus: BusId::new(br.f_bus),
            to_bus: BusId::new(br.t_bus),
            resistance: br.br_r,
            reactance: br.br_x,
            tap_ratio: if br.tap == 0.0 { 1.0 } else { br.tap },
            phase_shift_rad: br.shift.to_radians(),
            charging_b_pu: br.br_b,
            s_max_mva: (br.rate_a > 0.0).then_some(br.rate_a),
            status: br.br_status != 0,
            rating_a_mva: (br.rate_a > 0.0).then_some(br.rate_a),
            is_phase_shifter,
            ..Branch::default()
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
            pmin_mw: case_gen.pmin,
            pmax_mw: case_gen.pmax,
            qmin_mvar: case_gen.qmin,
            qmax_mvar: case_gen.qmax,
            cost_model: gat_core::CostModel::NoCost,
            is_synchronous_condenser: false,
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
            tap_ratio: if case_branch.tap == 0.0 {
                1.0
            } else {
                case_branch.tap
            },
            phase_shift_rad: case_branch.shift.to_radians(),
            charging_b_pu: case_branch.br_b,
            s_max_mva: (case_branch.rate_a > 0.0).then_some(case_branch.rate_a),
            status: case_branch.is_on(),
            rating_a_mva: (case_branch.rate_a > 0.0).then_some(case_branch.rate_a),
            ..Branch::default()
        };

        network
            .graph
            .add_edge(from_idx, to_idx, Edge::Branch(branch));

        branch_id += 1;
    }

    Ok(network)
}
