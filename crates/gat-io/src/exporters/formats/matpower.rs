//! MATPOWER .m file exporter
//!
//! Converts in-memory Network representation back to MATPOWER case format.

use anyhow::{Context, Result};
use gat_core::{CostModel, Edge, Network, Node};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::exporters::ExportMetadata;
use crate::importers::matpower_parser::{
    MatpowerBranch, MatpowerBus, MatpowerCase, MatpowerGen, MatpowerGenCost,
};

/// Export a Network to MATPOWER .m format
pub fn export_network_to_matpower(
    network: &Network,
    output_path: impl AsRef<Path>,
    metadata: Option<&ExportMetadata>,
) -> Result<()> {
    let case = network_to_matpower_case(network)?;
    write_matpower_case(&case, output_path, metadata)
}

/// Convert Network to MatpowerCase structure
fn network_to_matpower_case(network: &Network) -> Result<MatpowerCase> {
    let mut case = MatpowerCase::default();
    case.version = "2".to_string();
    case.base_mva = 100.0; // Default base MVA

    // Build bus ID mapping (BusId -> sequential index for MATPOWER)
    let mut bus_id_to_idx: HashMap<usize, usize> = HashMap::new();
    let mut bus_idx = 1; // MATPOWER uses 1-based indexing

    // Collect buses and build mapping
    let mut buses_data: Vec<(usize, MatpowerBus)> = Vec::new();
    let mut loads_by_bus: HashMap<usize, (f64, f64)> = HashMap::new(); // bus_id -> (Pd, Qd)

    for node in network.graph.node_weights() {
        match node {
            Node::Bus(bus) => {
                let bus_id = bus.id.value();
                if !bus_id_to_idx.contains_key(&bus_id) {
                    bus_id_to_idx.insert(bus_id, bus_idx);

                    // Determine bus type (will be updated when we process generators)
                    let bus_type = 1; // PQ bus by default

                    buses_data.push((
                        bus_id,
                        MatpowerBus {
                            bus_i: bus_idx,
                            bus_type,
                            pd: 0.0, // Will be filled from loads
                            qd: 0.0,
                            gs: 0.0,
                            bs: 0.0,
                            area: bus.area_id.unwrap_or(1) as i32,
                            vm: bus.voltage_pu.value(),
                            va: bus.angle_rad.to_degrees().value(),
                            base_kv: bus.base_kv.value(),
                            zone: bus.zone_id.unwrap_or(1) as i32,
                            vmax: bus.vmax_pu.map(|v| v.value()).unwrap_or(1.1),
                            vmin: bus.vmin_pu.map(|v| v.value()).unwrap_or(0.9),
                        },
                    ));
                    bus_idx += 1;
                }
            }
            Node::Load(load) => {
                let bus_id = load.bus.value();
                let entry = loads_by_bus.entry(bus_id).or_insert((0.0, 0.0));
                entry.0 += load.active_power.value();
                entry.1 += load.reactive_power.value();
            }
            Node::Gen(_) => {
                // Generators are processed separately
            }
            Node::Shunt(_) => {
                // Shunts are handled separately (added to bus gs/bs)
            }
        }
    }

    // Update bus loads
    for (bus_id, bus) in &mut buses_data {
        if let Some((pd, qd)) = loads_by_bus.get(bus_id) {
            bus.pd = *pd;
            bus.qd = *qd;
        }
    }

    // Track which buses have generators (for bus type determination)
    let mut gen_buses: HashMap<usize, bool> = HashMap::new(); // bus_id -> is_slack

    // Process generators
    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            if !gen.status {
                continue; // Skip offline generators
            }

            let bus_id = gen.bus.value();
            let matpower_bus_idx = bus_id_to_idx.get(&bus_id).copied().unwrap_or(1);

            // Determine if this is a slack bus (first generator is typically slack)
            let is_slack = gen_buses.is_empty();
            gen_buses.insert(bus_id, is_slack);

            case.gen.push(MatpowerGen {
                gen_bus: matpower_bus_idx,
                pg: gen.active_power.value(),
                qg: gen.reactive_power.value(),
                qmax: gen.qmax.value(),
                qmin: gen.qmin.value(),
                vg: gen.voltage_setpoint.map(|v| v.value()).unwrap_or(1.0),
                mbase: gen.mbase.map(|v| v.value()).unwrap_or(case.base_mva),
                gen_status: if gen.status { 1 } else { 0 },
                pmax: gen.pmax.value(),
                pmin: gen.pmin.value(),
            });

            // Convert cost model
            let gencost = match &gen.cost_model {
                CostModel::NoCost => MatpowerGenCost {
                    model: 1, // Piecewise linear
                    startup: gen.cost_startup.unwrap_or(0.0),
                    shutdown: gen.cost_shutdown.unwrap_or(0.0),
                    ncost: 2,
                    cost: vec![0.0, 0.0, 0.0, 0.0], // Zero cost
                },
                CostModel::Polynomial(coeffs) => {
                    let mut cost_vec = coeffs.clone();
                    // MATPOWER polynomial format: [c2, c1, c0] for c2*P^2 + c1*P + c0
                    // Our format: [c0, c1, c2] for c0 + c1*P + c2*P^2
                    // So we need to reverse
                    cost_vec.reverse();

                    MatpowerGenCost {
                        model: 2, // Polynomial
                        startup: gen.cost_startup.unwrap_or(0.0),
                        shutdown: gen.cost_shutdown.unwrap_or(0.0),
                        ncost: cost_vec.len() as i32,
                        cost: cost_vec,
                    }
                }
                CostModel::PiecewiseLinear(points) => {
                    let mut cost_vec = Vec::new();
                    for (mw, cost) in points {
                        cost_vec.push(*mw);
                        cost_vec.push(*cost);
                    }

                    MatpowerGenCost {
                        model: 1, // Piecewise linear
                        startup: gen.cost_startup.unwrap_or(0.0),
                        shutdown: gen.cost_shutdown.unwrap_or(0.0),
                        ncost: points.len() as i32,
                        cost: cost_vec,
                    }
                }
            };

            case.gencost.push(gencost);
        }
    }

    // Update bus types based on generators
    for (bus_id, bus) in &mut buses_data {
        if let Some(is_slack) = gen_buses.get(bus_id) {
            bus.bus_type = if *is_slack { 3 } else { 2 }; // 3 = slack, 2 = PV
        }
    }

    // Sort buses by index and add to case
    buses_data.sort_by_key(|(_, bus)| bus.bus_i);
    case.bus = buses_data.into_iter().map(|(_, bus)| bus).collect();

    // Process branches
    for edge in network.graph.edge_weights() {
        match edge {
            Edge::Branch(branch) => {
                if !branch.status {
                    continue; // Skip offline branches
                }

                let from_idx = bus_id_to_idx
                    .get(&branch.from_bus.value())
                    .copied()
                    .unwrap_or(1);
                let to_idx = bus_id_to_idx
                    .get(&branch.to_bus.value())
                    .copied()
                    .unwrap_or(1);

                case.branch.push(MatpowerBranch {
                    f_bus: from_idx,
                    t_bus: to_idx,
                    br_r: branch.resistance,
                    br_x: branch.reactance,
                    br_b: branch.charging_b.value(),
                    rate_a: branch.rating_a.map(|v| v.value()).unwrap_or(0.0),
                    rate_b: branch.rating_b.map(|v| v.value()).unwrap_or(0.0),
                    rate_c: branch.rating_c.map(|v| v.value()).unwrap_or(0.0),
                    tap: branch.tap_ratio,
                    shift: branch.phase_shift.to_degrees().value(),
                    br_status: if branch.status { 1 } else { 0 },
                    angmin: branch.angle_min.map(|v| v.to_degrees().value()).unwrap_or(-360.0),
                    angmax: branch.angle_max.map(|v| v.to_degrees().value()).unwrap_or(360.0),
                });
            }
            Edge::Transformer(tx) => {
                // Transformers are represented as branches with tap ratio
                let from_idx = bus_id_to_idx
                    .get(&tx.from_bus.value())
                    .copied()
                    .unwrap_or(1);
                let to_idx = bus_id_to_idx.get(&tx.to_bus.value()).copied().unwrap_or(1);

                case.branch.push(MatpowerBranch {
                    f_bus: from_idx,
                    t_bus: to_idx,
                    br_r: 0.0,   // Ideal transformer
                    br_x: 0.001, // Small reactance to avoid singularity
                    br_b: 0.0,
                    rate_a: 0.0,
                    rate_b: 0.0,
                    rate_c: 0.0,
                    tap: tx.ratio,
                    shift: 0.0,
                    br_status: 1,
                    angmin: -360.0,
                    angmax: 360.0,
                });
            }
        }
    }

    Ok(case)
}

/// Write MatpowerCase to .m file
fn write_matpower_case(
    case: &MatpowerCase,
    output_path: impl AsRef<Path>,
    metadata: Option<&ExportMetadata>,
) -> Result<()> {
    let path = output_path.as_ref();
    let mut file =
        File::create(path).with_context(|| format!("creating output file: {}", path.display()))?;

    write_matpower_metadata(&mut file, metadata)?;

    // Write header
    writeln!(file, "function mpc = case")?;
    writeln!(file, "%% MATPOWER Case Format : Version {}", case.version)?;
    writeln!(file, "%% Generated by GAT (Grid Analysis Toolkit)")?;
    writeln!(file)?;

    // Write baseMVA
    writeln!(file, "mpc.version = '{}';", case.version)?;
    writeln!(file, "mpc.baseMVA = {};", case.base_mva)?;
    writeln!(file)?;

    // Write bus data
    writeln!(file, "%% bus data")?;
    writeln!(
        file,
        "%%\tbus_i\ttype\tPd\tQd\tGs\tBs\tarea\tVm\tVa\tbaseKV\tzone\tVmax\tVmin"
    )?;
    writeln!(file, "mpc.bus = [")?;
    for bus in &case.bus {
        writeln!(
            file,
            "\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}\t{}\t{:.6}\t{:.6};",
            bus.bus_i,
            bus.bus_type,
            bus.pd,
            bus.qd,
            bus.gs,
            bus.bs,
            bus.area,
            bus.vm,
            bus.va,
            bus.base_kv,
            bus.zone,
            bus.vmax,
            bus.vmin
        )?;
    }
    writeln!(file, "];")?;
    writeln!(file)?;

    // Write generator data
    writeln!(file, "%% generator data")?;
    writeln!(
        file,
        "%%\tbus\tPg\tQg\tQmax\tQmin\tVg\tmBase\tstatus\tPmax\tPmin"
    )?;
    writeln!(file, "mpc.gen = [")?;
    for gen in &case.gen {
        writeln!(
            file,
            "\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{:.6}\t{:.6};",
            gen.gen_bus,
            gen.pg,
            gen.qg,
            gen.qmax,
            gen.qmin,
            gen.vg,
            gen.mbase,
            gen.gen_status,
            gen.pmax,
            gen.pmin
        )?;
    }
    writeln!(file, "];")?;
    writeln!(file)?;

    // Write branch data
    writeln!(file, "%% branch data")?;
    writeln!(
        file,
        "%%\tfbus\ttbus\tr\tx\tb\trateA\trateB\trateC\tratio\tangle\tstatus\tangmin\tangmax"
    )?;
    writeln!(file, "mpc.branch = [")?;
    for branch in &case.branch {
        writeln!(
            file,
            "\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{:.6}\t{:.6};",
            branch.f_bus,
            branch.t_bus,
            branch.br_r,
            branch.br_x,
            branch.br_b,
            branch.rate_a,
            branch.rate_b,
            branch.rate_c,
            branch.tap,
            branch.shift,
            branch.br_status,
            branch.angmin,
            branch.angmax
        )?;
    }
    writeln!(file, "];")?;
    writeln!(file)?;

    // Write generator cost data if present
    if !case.gencost.is_empty() {
        writeln!(file, "%% generator cost data")?;
        writeln!(file, "%%\tmodel\tstartup\tshutdown\tn\tc(n-1)\t...\tc0")?;
        writeln!(file, "mpc.gencost = [")?;
        for gencost in &case.gencost {
            write!(
                file,
                "\t{}\t{:.6}\t{:.6}\t{}",
                gencost.model, gencost.startup, gencost.shutdown, gencost.ncost
            )?;
            for cost_val in &gencost.cost {
                write!(file, "\t{:.6}", cost_val)?;
            }
            writeln!(file, ";")?;
        }
        writeln!(file, "];")?;
    }

    file.flush()
        .with_context(|| format!("flushing output file: {}", path.display()))?;

    Ok(())
}

fn write_matpower_metadata(writer: &mut File, metadata: Option<&ExportMetadata>) -> Result<()> {
    if let Some(meta) = metadata {
        if let Some(desc) = meta.source_description() {
            writeln!(writer, "%% Source: {}", desc)?;
        }
        if let Some(ts) = meta.creation_timestamp() {
            writeln!(writer, "%% Arrow dataset created at {}", ts)?;
        }
        if let Some(version) = meta.gat_version() {
            writeln!(writer, "%% Generated by GAT {}", version)?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Branch, BranchId, Bus, BusId, Gen, GenId, Load, LoadId};
    use tempfile::NamedTempFile;

    #[test]
    fn test_export_simple_network() -> Result<()> {
        let mut network = Network::new();

        // Add two buses
        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            voltage_pu: gat_core::PerUnit(1.0),
            angle_rad: gat_core::Radians(0.0),
            vmin_pu: Some(gat_core::PerUnit(0.95)),
            vmax_pu: Some(gat_core::PerUnit(1.05)),
            area_id: Some(1),
            zone_id: Some(1),
        }));

        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            base_kv: gat_core::Kilovolts(138.0),
            voltage_pu: gat_core::PerUnit(1.0),
            angle_rad: gat_core::Radians(0.0),
            vmin_pu: Some(gat_core::PerUnit(0.95)),
            vmax_pu: Some(gat_core::PerUnit(1.05)),
            area_id: Some(1),
            zone_id: Some(1),
        }));

        // Add a generator
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen 1".to_string(),
            bus: BusId::new(1),
            active_power: gat_core::Megawatts(100.0),
            reactive_power: gat_core::Megavars(50.0),
            pmin: gat_core::Megawatts(0.0),
            pmax: gat_core::Megawatts(200.0),
            qmin: gat_core::Megavars(-50.0),
            qmax: gat_core::Megavars(100.0),
            status: true,
            voltage_setpoint: Some(gat_core::PerUnit(1.05)),
            cost_model: CostModel::quadratic(0.0, 20.0, 0.01),
            ..Gen::default()
        }));

        // Add a load
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load 1".to_string(),
            bus: BusId::new(2),
            active_power: gat_core::Megawatts(80.0),
            reactive_power: gat_core::Megavars(40.0),
        }));

        // Add a branch
        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Line 1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                charging_b: gat_core::PerUnit(0.02),
                tap_ratio: 1.0,
                phase_shift: gat_core::Radians(0.0),
                status: true,
                rating_a: Some(gat_core::MegavoltAmperes(250.0)),
                ..Branch::default()
            }),
        );

        // Export to temporary file
        let temp_file = NamedTempFile::new()?;
        export_network_to_matpower(&network, temp_file.path(), None)?;

        // Read back and verify basic structure
        let content = std::fs::read_to_string(temp_file.path())?;
        assert!(content.contains("mpc.baseMVA"));
        assert!(content.contains("mpc.bus"));
        assert!(content.contains("mpc.gen"));
        assert!(content.contains("mpc.branch"));
        assert!(content.contains("mpc.gencost"));

        Ok(())
    }
}
