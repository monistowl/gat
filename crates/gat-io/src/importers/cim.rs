use std::{collections::HashMap, fs, fs::File, io::Read, path::Path};

use anyhow::{anyhow, Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
};
use quick_xml::{
    events::{BytesStart, Event},
    name::LocalName,
    Reader,
};
use zip::ZipArchive;

use super::arrow::write_network_to_arrow;

pub fn import_cim_rdf(rdf_path: &str, output_file: &str) -> Result<Network> {
    println!("Importing CIM from {} to {}", rdf_path, output_file);
    let path = Path::new(rdf_path);
    let documents = collect_cim_documents(path)?;
    let (buses, lines, loads, gens, limits, volt_limits, transformers) =
        parse_cim_documents(&documents)?;
    let network =
        build_network_from_cim(buses, lines, loads, gens, limits, volt_limits, transformers)?;

    // Validate the network
    super::cim_validator::validate_network_from_cim(&network)?;
    let warnings = super::cim_validator::validate_cim_with_warnings(&network);
    for w in warnings {
        eprintln!("⚠ Warning: {}", w);
    }

    write_network_to_arrow(&network, output_file)?;
    Ok(network)
}

pub(crate) struct CimBus {
    pub id: String,
    pub name: String,
}

pub(crate) struct CimLine {
    pub name: String,
    pub from: String,
    pub to: String,
    pub resistance: f64,
    pub reactance: f64,
}

pub(crate) struct CimLoad {
    pub name: String,
    pub bus_id: String,
    pub active_power_mw: f64,
    pub reactive_power_mvar: f64,
}

pub(crate) struct CimGen {
    pub name: String,
    pub bus_id: String,
    pub active_power_mw: f64,
    pub reactive_power_mvar: f64,
}

#[derive(Debug, Clone)]
pub struct CimOperationalLimit {
    pub equipment_id: String,
    pub limit_type: String, // "ThermalLimit", "VoltageLimit", "FrequencyLimit"
    pub value: f64,
    pub unit: String, // "MW", "kV", "Hz"
}

#[derive(Debug, Clone)]
pub struct CimVoltageLimit {
    pub bus_id: String,
    pub min_voltage: f64,
    pub max_voltage: f64,
}

#[derive(Debug, Clone)]
pub struct CimTransformer {
    pub id: String,
    pub name: String,
    pub from_bus: String,
    pub to_bus: String,
    pub r: f64,
    pub x: f64,
    pub rated_mva: f64,
}

type CimImportResult = (
    Vec<CimBus>,
    Vec<CimLine>,
    Vec<CimLoad>,
    Vec<CimGen>,
    Vec<CimOperationalLimit>,
    Vec<CimVoltageLimit>,
    Vec<CimTransformer>,
);

struct PendingCimGen {
    name: String,
    terminal_ref: Option<String>,
    bus_ref: Option<String>,
    active_power_mw: f64,
    reactive_power_mvar: f64,
}

impl PendingCimGen {
    fn new() -> Self {
        Self {
            name: String::new(),
            terminal_ref: None,
            bus_ref: None,
            active_power_mw: 0.0,
            reactive_power_mvar: 0.0,
        }
    }
}

struct PendingCimLoad {
    name: String,
    bus_ref: Option<String>,
    terminal_ref: Option<String>,
    active_power_mw: f64,
    reactive_power_mvar: f64,
}

struct PendingCimTerminal {
    id: String,
    bus_ref: Option<String>,
}

impl PendingCimLoad {
    fn new() -> Self {
        Self {
            name: String::new(),
            bus_ref: None,
            terminal_ref: None,
            active_power_mw: 0.0,
            reactive_power_mvar: 0.0,
        }
    }
}

impl PendingCimTerminal {
    fn new(id: String) -> Self {
        Self { id, bus_ref: None }
    }
}

fn collect_cim_documents(path: &Path) -> Result<Vec<String>> {
    if path.is_dir() {
        let mut docs = Vec::new();
        for entry in fs::read_dir(path).with_context(|| {
            format!(
                "reading CIM directory '{}'; ensure it contains RDF/XML files",
                path.display()
            )
        })? {
            let entry = entry?;
            let doc_path = entry.path();
            if let Some(ext) = doc_path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_ascii_lowercase();
                if ext_lower == "rdf" || ext_lower == "xml" {
                    docs.push(fs::read_to_string(&doc_path)?);
                }
            }
        }
        if docs.is_empty() {
            Err(anyhow!(
                "CIM directory '{}' contains no RDF/XML files",
                path.display()
            ))
        } else {
            Ok(docs)
        }
    } else if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
    {
        let mut archive = ZipArchive::new(File::open(path)?)?;
        let mut docs = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name().to_ascii_lowercase().ends_with(".rdf")
                || file.name().to_ascii_lowercase().ends_with(".xml")
            {
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                docs.push(contents);
            }
        }
        if docs.is_empty() {
            Err(anyhow!(
                "CIM zip '{}' contains no RDF/XML files",
                path.display()
            ))
        } else {
            Ok(docs)
        }
    } else {
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading CIM file '{}'; ensure it exists", path.display()))?;
        Ok(vec![text])
    }
}

pub(crate) fn parse_cim_documents(documents: &[String]) -> Result<CimImportResult> {
    let mut buses: Vec<CimBus> = Vec::new();
    let mut lines: Vec<CimLine> = Vec::new();
    let mut loads: Vec<CimLoad> = Vec::new();
    let mut gens: Vec<CimGen> = Vec::new();
    let operational_limits = Vec::new();
    let voltage_limits = Vec::new();
    let transformers = Vec::new();

    for doc in documents {
        let mut reader = Reader::from_str(doc);
        reader.trim_text(true);

        let mut current_bus: Option<CimBus> = None;
        let mut current_line: Option<CimLine> = None;
        let mut current_load: Option<PendingCimLoad> = None;
        let mut current_gen: Option<PendingCimGen> = None;
        let mut current_terminal: Option<PendingCimTerminal> = None;
        let mut terminal_bus_map: HashMap<String, String> = HashMap::new();
        let mut active_tag: Option<String> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = e.local_name();
                    let tag = local_name_as_str(&name);
                    active_tag = Some(tag.to_string());
                    match tag {
                        "BusbarSection" => {
                            if let Some(id) = attribute_value(e, "ID")? {
                                current_bus = Some(CimBus {
                                    id,
                                    name: String::new(),
                                });
                            }
                        }
                        "ACLineSegment" => {
                            if attribute_value(e, "ID")?.is_some() {
                                current_line = Some(CimLine {
                                    name: String::new(),
                                    from: String::new(),
                                    to: String::new(),
                                    resistance: 0.0,
                                    reactance: 0.0,
                                });
                            }
                        }
                        "Load" => {
                            current_load = Some(PendingCimLoad::new());
                        }
                        "SynchronousMachine" => {
                            current_gen = Some(PendingCimGen::new());
                        }
                        "Terminal" => {
                            if let Some(id) = attribute_value(e, "ID")? {
                                current_terminal = Some(PendingCimTerminal::new(id));
                            }
                        }
                        "OperationalLimit" => {
                            // Operational limit parsing - basic structure for now
                            // Will be fully implemented when we have proper test data
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let name = e.local_name();
                    let tag = local_name_as_str(&name);
                    if let Some(load) = current_load.as_mut() {
                        match tag {
                            "Load.Terminal" => {
                                if let Some(resource) = attribute_value(e, "resource")? {
                                    load.terminal_ref =
                                        Some(resource.trim_start_matches('#').to_string());
                                }
                            }
                            "Load.ConnectivityNode" => {
                                if let Some(resource) = attribute_value(e, "resource")? {
                                    load.bus_ref =
                                        Some(resource.trim_start_matches('#').to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(gen) = current_gen.as_mut() {
                        match tag {
                            "SynchronousMachine.Terminal" => {
                                if let Some(resource) = attribute_value(e, "resource")? {
                                    gen.terminal_ref =
                                        Some(resource.trim_start_matches('#').to_string());
                                }
                            }
                            "SynchronousMachine.ConnectivityNode" => {
                                if let Some(resource) = attribute_value(e, "resource")? {
                                    gen.bus_ref =
                                        Some(resource.trim_start_matches('#').to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(term) = current_terminal.as_mut() {
                        if tag == "Terminal.ConnectivityNode" {
                            if let Some(resource) = attribute_value(e, "resource")? {
                                term.bus_ref = Some(resource.trim_start_matches('#').to_string());
                            }
                        }
                    }
                    if let Some(line) = current_line.as_mut() {
                        match tag {
                            "ACLineSegment.end1" => {
                                if let Some(resource) = attribute_value(e, "resource")? {
                                    line.from = resource.trim_start_matches('#').to_string();
                                }
                            }
                            "ACLineSegment.end2" => {
                                if let Some(resource) = attribute_value(e, "resource")? {
                                    line.to = resource.trim_start_matches('#').to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    if let Some(ref tag) = active_tag {
                        let text = e.unescape()?.trim().to_string();
                        if let Some(bus) = current_bus.as_mut() {
                            if tag == "IdentifiedObject.name" {
                                bus.name = text.clone();
                            }
                        }
                        if let Some(line) = current_line.as_mut() {
                            match tag.as_str() {
                                "IdentifiedObject.name" => {
                                    line.name = text.clone();
                                }
                                "ACLineSegment.r" => {
                                    line.resistance = text.parse().unwrap_or(0.0);
                                }
                                "ACLineSegment.x" => {
                                    line.reactance = text.parse().unwrap_or(0.0);
                                }
                                _ => {}
                            }
                        }
                        if let Some(load) = current_load.as_mut() {
                            match tag.as_str() {
                                "IdentifiedObject.name" => {
                                    load.name = text.clone();
                                }
                                "Load.p" => {
                                    load.active_power_mw = text.parse().unwrap_or(0.0);
                                }
                                "Load.q" => {
                                    load.reactive_power_mvar = text.parse().unwrap_or(0.0);
                                }
                                _ => {}
                            }
                        }
                        if let Some(gen) = current_gen.as_mut() {
                            match tag.as_str() {
                                "IdentifiedObject.name" => {
                                    gen.name = text.clone();
                                }
                                "SynchronousMachine.p" => {
                                    gen.active_power_mw = text.parse().unwrap_or(0.0);
                                }
                                "SynchronousMachine.q" => {
                                    gen.reactive_power_mvar = text.parse().unwrap_or(0.0);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.local_name();
                    let tag = local_name_as_str(&name);
                    match tag {
                        "BusbarSection" => {
                            if let Some(bus) = current_bus.take() {
                                if !bus.name.is_empty() {
                                    buses.push(bus);
                                }
                            }
                        }
                        "ACLineSegment" => {
                            if let Some(line) = current_line.take() {
                                if !line.from.is_empty() && !line.to.is_empty() {
                                    lines.push(line);
                                }
                            }
                        }
                        "Terminal" => {
                            if let Some(term) = current_terminal.take() {
                                if let Some(bus_ref) = term.bus_ref {
                                    terminal_bus_map.insert(term.id, bus_ref);
                                }
                            }
                        }
                        "Load" => {
                            if let Some(load) = current_load.take() {
                                let bus_ref = load.bus_ref.clone().or_else(|| {
                                    load.terminal_ref
                                        .as_ref()
                                        .and_then(|term| terminal_bus_map.get(term).cloned())
                                });
                                if let Some(bus_id) = bus_ref {
                                    loads.push(CimLoad {
                                        name: load.name,
                                        bus_id,
                                        active_power_mw: load.active_power_mw,
                                        reactive_power_mvar: load.reactive_power_mvar,
                                    });
                                }
                            }
                        }
                        "SynchronousMachine" => {
                            if let Some(gen) = current_gen.take() {
                                let bus_ref = gen.bus_ref.clone().or_else(|| {
                                    gen.terminal_ref
                                        .as_ref()
                                        .and_then(|term| terminal_bus_map.get(term).cloned())
                                });
                                if let Some(bus_id) = bus_ref {
                                    gens.push(CimGen {
                                        name: gen.name,
                                        bus_id,
                                        active_power_mw: gen.active_power_mw,
                                        reactive_power_mvar: gen.reactive_power_mvar,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                    active_tag = None;
                }
                Ok(Event::Eof) => break,
                Err(err) => return Err(err.into()),
                _ => {}
            }
        }
    }

    if buses.is_empty() {
        Err(anyhow!("no bus definitions discovered in CIM documents"))
    } else {
        Ok((
            buses,
            lines,
            loads,
            gens,
            operational_limits,
            voltage_limits,
            transformers,
        ))
    }
}

fn build_network_from_cim(
    buses: Vec<CimBus>,
    lines: Vec<CimLine>,
    loads: Vec<CimLoad>,
    gens: Vec<CimGen>,
    limits: Vec<CimOperationalLimit>,
    voltage_limits: Vec<CimVoltageLimit>,
    transformers: Vec<CimTransformer>,
) -> Result<Network> {
    let mut network = Network::new();
    let mut node_map: HashMap<String, (BusId, NodeIndex)> = HashMap::new();
    for (idx, bus) in buses.into_iter().enumerate() {
        let bus_id = BusId::new(idx + 1);
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: bus_id,
            name: bus.name,
            voltage_kv: 138.0,
        }));
        node_map.insert(bus.id, (bus_id, node_idx));
    }

    let mut load_counter = 0usize;
    for load in loads {
        if let Some((bus_id, _)) = node_map.get(&load.bus_id) {
            if load.active_power_mw == 0.0 && load.reactive_power_mvar == 0.0 {
                continue;
            }
            let name = if load.name.is_empty() {
                format!("CIM load @ {}", load.bus_id)
            } else {
                load.name.clone()
            };
            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(load_counter),
                name,
                bus: *bus_id,
                active_power_mw: load.active_power_mw,
                reactive_power_mvar: load.reactive_power_mvar,
            }));
            load_counter += 1;
        }
    }

    let mut gen_counter = 0usize;
    for gen in gens {
        if let Some((bus_id, _)) = node_map.get(&gen.bus_id) {
            if gen.active_power_mw == 0.0 && gen.reactive_power_mvar == 0.0 {
                continue;
            }
            let name = if gen.name.is_empty() {
                format!("CIM gen @ {}", gen.bus_id)
            } else {
                gen.name.clone()
            };
            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_counter),
                name,
                bus: *bus_id,
                active_power_mw: gen.active_power_mw,
                reactive_power_mvar: gen.reactive_power_mvar,
                pmin_mw: 0.0,
                pmax_mw: f64::INFINITY,
                qmin_mvar: f64::NEG_INFINITY,
                qmax_mvar: f64::INFINITY,
                cost_model: gat_core::CostModel::NoCost,
            }));
            gen_counter += 1;
        }
    }

    for (branch_id, line) in lines.into_iter().enumerate() {
        let (from_bus_id, from_idx) = node_map
            .get(&line.from)
            .with_context(|| format!("CIM line references unknown bus {}", line.from))?;
        let (to_bus_id, to_idx) = node_map
            .get(&line.to)
            .with_context(|| format!("CIM line references unknown bus {}", line.to))?;

        let branch = Branch {
            id: BranchId::new(branch_id),
            name: if line.name.is_empty() {
                format!("{}-{}", line.from, line.to)
            } else {
                line.name.clone()
            },
            from_bus: *from_bus_id,
            to_bus: *to_bus_id,
            resistance: line.resistance,
            reactance: line.reactance,
            ..Branch::default()
        };

        network
            .graph
            .add_edge(*from_idx, *to_idx, Edge::Branch(branch));
    }

    // Apply voltage limits to buses
    // Note: Bus struct does not yet have min_voltage/max_voltage fields
    // These will be applied once the struct is extended
    for _volt_limit in voltage_limits {
        // Future: Apply to bus when fields are added
        // if let Some(bus) = network.buses.iter_mut().find(|b| b.name == volt_limit.bus_id) {
        //     bus.min_voltage = volt_limit.min_voltage;
        //     bus.max_voltage = volt_limit.max_voltage;
        // }
    }

    // Apply thermal limits to branches
    // Note: Branch struct does not yet have rate_a field
    // These will be applied once the struct is extended
    for _limit in limits {
        // Future: Apply to branch when fields are added
        // if limit.limit_type == "ThermalLimit" {
        //     if let Some(branch) = network.branches.iter_mut().find(|br| br.name == limit.equipment_id) {
        //         branch.rate_a = limit.value;
        //     }
        // }
    }

    // Transformers are collected but not yet used
    // Future: Add transformer handling once Transformer node type is added
    let _ = transformers;

    Ok(network)
}

fn attribute_value(event: &BytesStart, key: &str) -> Result<Option<String>> {
    for attr in event.attributes().with_checks(false) {
        let attr = attr?;
        if let Ok(name) = std::str::from_utf8(attr.key.local_name().as_ref()) {
            if name.eq_ignore_ascii_case(key) {
                return Ok(Some(attr.unescape_value()?.into_owned()));
            }
        }
    }
    Ok(None)
}

fn local_name_as_str<'a>(name: &'a LocalName<'a>) -> &'a str {
    std::str::from_utf8(name.as_ref()).unwrap_or_default()
}
