//! PSS/E RAW file exporter
//!
//! Converts an in-memory [`Network`] back to the legacy PSS/E RAW format.

use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use crate::exporters::ExportMetadata;
use anyhow::{Context, Result};
use gat_core::{Branch, BranchId, Bus, Edge, Gen, Load, Network, Node};

fn quote_name(input: &str) -> String {
    let safe = input.replace('\'', "''");
    format!("'{}'", if safe.is_empty() { "component" } else { &safe })
}

/// Export a Network to PSS/E RAW format
pub fn export_network_to_psse(
    network: &Network,
    output_path: impl AsRef<Path>,
    metadata: Option<&ExportMetadata>,
) -> Result<()> {
    let mut buses: Vec<Bus> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Bus(bus) => Some(bus.clone()),
            _ => None,
        })
        .collect();
    buses.sort_by_key(|bus| bus.id.value());

    let mut loads_by_bus: HashMap<usize, Vec<Load>> = HashMap::new();
    let mut gens: Vec<Gen> = Vec::new();

    for node in network.graph.node_weights() {
        match node {
            Node::Load(load) => {
                loads_by_bus
                    .entry(load.bus.value())
                    .or_default()
                    .push(load.clone());
            }
            Node::Gen(gen) => {
                if gen.status {
                    gens.push(gen.clone());
                }
            }
            _ => {}
        }
    }

    let mut branches: Vec<Branch> = Vec::new();
    for edge in network.graph.edge_weights() {
        match edge {
            Edge::Branch(branch) => branches.push(branch.clone()),
            Edge::Transformer(tx) => {
                branches.push(Branch {
                    id: BranchId::new(tx.id.value()),
                    name: tx.name.clone(),
                    from_bus: tx.from_bus,
                    to_bus: tx.to_bus,
                    resistance: 0.0,
                    reactance: 0.0,
                    tap_ratio: tx.ratio,
                    ..Branch::default()
                });
            }
        }
    }

    let mut load_entries: Vec<(usize, Vec<Load>)> = loads_by_bus.into_iter().collect();
    load_entries.sort_by_key(|(bus_id, _)| *bus_id);

    branches.sort_by(|a, b| {
        a.from_bus
            .value()
            .cmp(&b.from_bus.value())
            .then_with(|| a.to_bus.value().cmp(&b.to_bus.value()))
    });

    let output = File::create(output_path.as_ref()).with_context(|| {
        format!(
            "creating PSS/E RAW file: {}",
            output_path.as_ref().display()
        )
    })?;
    let mut writer = BufWriter::new(output);

    writeln!(writer, "0, 100.00 / GAT PSS/E export")?;
    write_psse_metadata(&mut writer, metadata)?;

    writeln!(writer, "BUS DATA FOLLOWS")?;
    for bus in &buses {
        let vm = bus.voltage_pu;
        let va = bus.angle_rad.to_degrees();
        writeln!(
            writer,
            "{id},{name},{base_kv:.6},1,0,0,1,1,{vm:.6},{va:.6},0.0,1,0,1,1,0,0,0,0",
            id = bus.id.value(),
            name = quote_name(&bus.name),
            base_kv = bus.voltage_kv,
            vm = vm,
            va = va
        )?;
    }
    writeln!(writer, "END OF BUS DATA")?;

    writeln!(writer, "GENERATOR DATA FOLLOWS")?;
    for gen in &gens {
        let status = if gen.status { 1 } else { 0 };
        writeln!(
            writer,
            "{bus},{name},{pg:.6},{qg:.6},0,0,0,0,0,0,0,0,0,0,{status}",
            bus = gen.bus.value(),
            name = quote_name(&gen.name),
            pg = gen.active_power_mw,
            qg = gen.reactive_power_mvar,
            status = status
        )?;
    }
    writeln!(writer, "END OF GENERATOR DATA")?;

    writeln!(writer, "LOAD DATA FOLLOWS")?;
    for (bus_id, loads) in load_entries {
        for load in loads {
            writeln!(
                writer,
                "{bus},{name},0,{p:.6},{q:.6},0,0,0,0,0,0,1,0,1,1",
                bus = bus_id,
                name = quote_name(&load.name),
                p = load.active_power_mw,
                q = load.reactive_power_mvar,
            )?;
        }
    }
    writeln!(writer, "END OF LOAD DATA")?;

    writeln!(writer, "BRANCH DATA FOLLOWS")?;
    for branch in &branches {
        let status = if branch.status { 1 } else { 0 };
        let rate_a = branch
            .rating_a_mva
            .unwrap_or(branch.s_max_mva.unwrap_or(0.0));
        writeln!(
            writer,
            "{from},{to},1,{r:.6},{x:.6},{b:.6},{rate_a:.6},0,0,{tap:.6},{shift:.6},0,0,{status}",
            from = branch.from_bus.value(),
            to = branch.to_bus.value(),
            r = branch.resistance,
            x = branch.reactance,
            b = branch.charging_b_pu,
            rate_a = rate_a,
            tap = branch.tap_ratio,
            shift = branch.phase_shift_rad.to_degrees(),
            status = status
        )?;
    }
    writeln!(writer, "END OF BRANCH DATA")?;

    writer.flush()?;

    Ok(())
}

fn write_psse_metadata(writer: &mut impl Write, metadata: Option<&ExportMetadata>) -> Result<()> {
    if let Some(meta) = metadata {
        if let Some(desc) = meta.source_description() {
            writeln!(writer, "0, 100.00 / Source: {}", desc)?;
        }
        if let Some(ts) = meta.creation_timestamp() {
            writeln!(writer, "0, 100.00 / Arrow dataset created at {}", ts)?;
        }
        if let Some(version) = meta.gat_version() {
            writeln!(writer, "0, 100.00 / GAT version {}", version)?;
        }
    }
    Ok(())
}
