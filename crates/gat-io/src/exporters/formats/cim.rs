//! CIM RDF/XML file exporter
//!
//! Serializes a [`Network`] into a minimal CIM RDF document that can be re-imported by the CIM
//! parser.

use crate::exporters::ExportMetadata;
use anyhow::{Context, Result};
use gat_core::{Branch, BranchId, Edge, Network, Node};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn bus_id_string(bus_id: usize) -> String {
    format!("Bus{}", bus_id)
}

fn write_component_header(
    writer: &mut impl Write,
    element: &str,
    rdf_id: &str,
    indentation: &str,
) -> Result<()> {
    writeln!(
        writer,
        "{indentation}<cim:{element} rdf:ID=\"{id}\">",
        indentation = indentation,
        element = element,
        id = rdf_id
    )?;
    Ok(())
}

fn write_component_footer(writer: &mut impl Write, element: &str, indentation: &str) -> Result<()> {
    writeln!(
        writer,
        "{indentation}</cim:{element}>",
        indentation = indentation,
        element = element
    )?;
    Ok(())
}

/// Export a Network to CIM RDF/XML format
pub fn export_network_to_cim(
    network: &Network,
    output_path: impl AsRef<Path>,
    metadata: Option<&ExportMetadata>,
) -> Result<()> {
    let mut buses: Vec<_> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Bus(bus) => Some(bus.clone()),
            _ => None,
        })
        .collect();
    buses.sort_by_key(|bus| bus.id.value());

    let gens: Vec<_> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Gen(gen) if gen.status => Some(gen.clone()),
            _ => None,
        })
        .collect();

    let loads: Vec<_> = network
        .graph
        .node_weights()
        .filter_map(|node| match node {
            Node::Load(load) => Some(load.clone()),
            _ => None,
        })
        .collect();

    let mut branches: Vec<_> = network
        .graph
        .edge_weights()
        .filter_map(|edge| match edge {
            Edge::Branch(branch) => Some(Branch {
                id: branch.id,
                name: branch.name.clone(),
                from_bus: branch.from_bus,
                to_bus: branch.to_bus,
                resistance: branch.resistance,
                reactance: branch.reactance,
                ..Branch::default()
            }),
            Edge::Transformer(tx) => Some(Branch {
                id: BranchId::new(tx.id.value()),
                name: tx.name.clone(),
                from_bus: tx.from_bus,
                to_bus: tx.to_bus,
                resistance: 0.0,
                reactance: 0.0,
                tap_ratio: tx.ratio,
                ..Branch::default()
            }),
        })
        .collect();
    branches.sort_by(|a, b| {
        a.from_bus
            .value()
            .cmp(&b.from_bus.value())
            .then_with(|| a.to_bus.value().cmp(&b.to_bus.value()))
    });

    let file = File::create(output_path.as_ref())
        .with_context(|| format!("creating CIM RDF file: {}", output_path.as_ref().display()))?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "<?xml version=\"1.0\"?>")?;
    writeln!(
        writer,
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
         xmlns:cim=\"http://iec.ch/TC57/2013/CIM-schema-cim16#\">"
    )?;
    write_cim_metadata(&mut writer, metadata)?;

    for bus in &buses {
        let id_str = bus_id_string(bus.id.value());
        write_component_header(&mut writer, "BusbarSection", &id_str, "  ")?;
        writeln!(
            writer,
            "    <cim:IdentifiedObject.name>{}</cim:IdentifiedObject.name>",
            xml_escape(&bus.name)
        )?;
        write_component_footer(&mut writer, "BusbarSection", "  ")?;
    }

    for (idx, branch) in branches.iter().enumerate() {
        let name = if branch.name.is_empty() {
            format!(
                "Branch {}-{}",
                branch.from_bus.value(),
                branch.to_bus.value()
            )
        } else {
            branch.name.clone()
        };
        let id_str = format!("Line{}", idx + 1);
        write_component_header(&mut writer, "ACLineSegment", &id_str, "  ")?;
        writeln!(
            writer,
            "    <cim:IdentifiedObject.name>{}</cim:IdentifiedObject.name>",
            xml_escape(&name)
        )?;
        writeln!(
            writer,
            "    <cim:ACLineSegment.end1 rdf:resource=\"#{}\"/>",
            bus_id_string(branch.from_bus.value())
        )?;
        writeln!(
            writer,
            "    <cim:ACLineSegment.end2 rdf:resource=\"#{}\"/>",
            bus_id_string(branch.to_bus.value())
        )?;
        writeln!(
            writer,
            "    <cim:ACLineSegment.r>{:.6}</cim:ACLineSegment.r>",
            branch.resistance
        )?;
        writeln!(
            writer,
            "    <cim:ACLineSegment.x>{:.6}</cim:ACLineSegment.x>",
            branch.reactance
        )?;
        write_component_footer(&mut writer, "ACLineSegment", "  ")?;
    }

    for (idx, load) in loads.iter().enumerate() {
        let id_str = format!("Load{}", idx + 1);
        write_component_header(&mut writer, "Load", &id_str, "  ")?;
        writeln!(
            writer,
            "    <cim:IdentifiedObject.name>{}</cim:IdentifiedObject.name>",
            xml_escape(&load.name)
        )?;
        writeln!(
            writer,
            "    <cim:Load.p>{:.6}</cim:Load.p>",
            load.active_power_mw
        )?;
        writeln!(
            writer,
            "    <cim:Load.q>{:.6}</cim:Load.q>",
            load.reactive_power_mvar
        )?;
        writeln!(
            writer,
            "    <cim:Load.ConnectivityNode rdf:resource=\"#{}\"/>",
            bus_id_string(load.bus.value())
        )?;
        write_component_footer(&mut writer, "Load", "  ")?;
    }

    for (idx, gen) in gens.iter().enumerate() {
        let id_str = format!("Gen{}", idx + 1);
        write_component_header(&mut writer, "SynchronousMachine", &id_str, "  ")?;
        writeln!(
            writer,
            "    <cim:IdentifiedObject.name>{}</cim:IdentifiedObject.name>",
            xml_escape(&gen.name)
        )?;
        writeln!(
            writer,
            "    <cim:SynchronousMachine.p>{:.6}</cim:SynchronousMachine.p>",
            gen.active_power_mw
        )?;
        writeln!(
            writer,
            "    <cim:SynchronousMachine.q>{:.6}</cim:SynchronousMachine.q>",
            gen.reactive_power_mvar
        )?;
        writeln!(
            writer,
            "    <cim:SynchronousMachine.ConnectivityNode rdf:resource=\"#{}\"/>",
            bus_id_string(gen.bus.value())
        )?;
        write_component_footer(&mut writer, "SynchronousMachine", "  ")?;
    }

    writeln!(writer, "</rdf:RDF>")?;
    writer.flush()?;
    Ok(())
}

fn write_cim_metadata(writer: &mut impl Write, metadata: Option<&ExportMetadata>) -> Result<()> {
    if let Some(meta) = metadata {
        if let Some(desc) = meta.source_description() {
            writeln!(writer, "  <!-- Source: {} -->", desc)?;
        }
        if let Some(ts) = meta.creation_timestamp() {
            writeln!(writer, "  <!-- Arrow dataset created at {} -->", ts)?;
        }
        if let Some(version) = meta.gat_version() {
            writeln!(writer, "  <!-- GAT version {} -->", version)?;
        }
    }
    Ok(())
}
