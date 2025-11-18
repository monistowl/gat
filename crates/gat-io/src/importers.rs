use std::{collections::HashMap, convert::TryFrom, fs, fs::File, io::Read, path::Path};

use anyhow::{anyhow, Context, Result};
use caseformat::{read_dir, read_zip, Branch as CaseBranch, Bus as CaseBus, Gen as CaseGen};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
    Transformer, TransformerId,
};
use polars::prelude::{
    DataFrame, IpcReader, IpcWriter, NamedFrom, PolarsResult, SerReader, SerWriter, Series,
};
use quick_xml::{
    events::{BytesStart, Event},
    name::LocalName,
    Reader,
};
use zip::ZipArchive;

pub fn import_psse_raw(raw_file: &str, output_file: &str) -> Result<Network> {
    println!("Importing PSSE RAW from {} to {}", raw_file, output_file);
    let path = Path::new(raw_file);
    let (buses, branches, loads, gens) = parse_psse_raw(path)?;
    let network = build_network_from_psse(buses, branches, loads, gens)?;
    write_network_to_arrow(&network, output_file)?;
    Ok(network)
}

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

pub fn import_cim_rdf(rdf_path: &str, output_file: &str) -> Result<Network> {
    println!("Importing CIM from {} to {}", rdf_path, output_file);
    let path = Path::new(rdf_path);
    let documents = collect_cim_documents(path)?;
    let (buses, lines, loads, gens) = parse_cim_documents(&documents)?;
    let network = build_network_from_cim(buses, lines, loads, gens)?;
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

type PsseRawTables = (
    Vec<PsseBus>,
    Vec<PsseBranch>,
    Vec<PsseLoad>,
    Vec<PsseGen>,
);

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

    for (branch_id, branch) in branches
        .into_iter()
        .filter(|b| b.in_service)
        .enumerate()
    {
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

struct CimBus {
    id: String,
    name: String,
}

struct CimLine {
    name: String,
    from: String,
    to: String,
    resistance: f64,
    reactance: f64,
}

struct CimLoad {
    name: String,
    bus_id: String,
    active_power_mw: f64,
    reactive_power_mvar: f64,
}

struct CimGen {
    name: String,
    bus_id: String,
    active_power_mw: f64,
    reactive_power_mvar: f64,
}

type CimImportResult = (Vec<CimBus>, Vec<CimLine>, Vec<CimLoad>, Vec<CimGen>);

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
        Ok(vec![fs::read_to_string(path)?])
    }
}

fn parse_cim_documents(docs: &[String]) -> Result<CimImportResult> {
    let mut buses = Vec::new();
    let mut lines = Vec::new();
    let mut loads = Vec::new();
    let mut gens = Vec::new();
    let mut terminal_bus_map: HashMap<String, String> = HashMap::new();

    for doc in docs {
        let mut reader = Reader::from_str(doc);
        reader.trim_text(true);
        let mut current_bus: Option<CimBus> = None;
        let mut current_line: Option<CimLine> = None;
        let mut current_load: Option<PendingCimLoad> = None;
        let mut current_terminal: Option<PendingCimTerminal> = None;
        let mut current_gen: Option<PendingCimGen> = None;
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
        Ok((buses, lines, loads, gens))
    }
}

fn build_network_from_cim(
    buses: Vec<CimBus>,
    lines: Vec<CimLine>,
    loads: Vec<CimLoad>,
    gens: Vec<CimGen>,
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
        };

        network
            .graph
            .add_edge(*from_idx, *to_idx, Edge::Branch(branch));
    }

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

fn write_network_to_arrow(network: &Network, output_file: &str) -> Result<()> {
    let mut df = network_to_dataframe(network).context("building DataFrame for network export")?;
    let mut file = File::create(output_file).with_context(|| {
        format!(
            "creating Arrow output '{}'; ensure directory exists",
            output_file
        )
    })?;
    IpcWriter::new(&mut file)
        .finish(&mut df)
        .context("writing Arrow output file")?;
    Ok(())
}

pub fn load_grid_from_arrow(grid_file: &str) -> Result<Network> {
    let file = File::open(grid_file)
        .with_context(|| format!("opening Arrow dataset '{}'; ensure it exists", grid_file))?;
    let reader = IpcReader::new(file);
    let df = reader
        .finish()
        .context("reading Arrow IPC dataset for grid import")?;
    dataframe_to_network(&df).context("converting Arrow dataset into Network graph")
}

fn network_to_dataframe(network: &Network) -> PolarsResult<DataFrame> {
    let mut element_type: Vec<String> = Vec::new();
    let mut element_id: Vec<i64> = Vec::new();
    let mut element_name: Vec<String> = Vec::new();
    let mut voltage_kv: Vec<Option<f64>> = Vec::new();
    let mut from_bus: Vec<Option<i64>> = Vec::new();
    let mut to_bus: Vec<Option<i64>> = Vec::new();
    let mut resistance: Vec<Option<f64>> = Vec::new();
    let mut reactance: Vec<Option<f64>> = Vec::new();
    let mut active_power: Vec<Option<f64>> = Vec::new();
    let mut reactive_power: Vec<Option<f64>> = Vec::new();

    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Bus(bus) => {
                element_type.push("bus".to_string());
                element_id.push(bus.id.value() as i64);
                element_name.push(bus.name.clone());
                voltage_kv.push(Some(bus.voltage_kv));
                from_bus.push(None);
                to_bus.push(None);
                resistance.push(None);
                reactance.push(None);
                active_power.push(None);
                reactive_power.push(None);
            }
            Node::Gen(gen) => {
                element_type.push("gen".to_string());
                element_id.push(gen.id.value() as i64);
                element_name.push(gen.name.clone());
                voltage_kv.push(None);
                from_bus.push(Some(gen.bus.value() as i64));
                to_bus.push(None);
                resistance.push(None);
                reactance.push(None);
                active_power.push(Some(gen.active_power_mw));
                reactive_power.push(Some(gen.reactive_power_mvar));
            }
            Node::Load(load) => {
                element_type.push("load".to_string());
                element_id.push(load.id.value() as i64);
                element_name.push(load.name.clone());
                voltage_kv.push(None);
                from_bus.push(Some(load.bus.value() as i64));
                to_bus.push(None);
                resistance.push(None);
                reactance.push(None);
                active_power.push(Some(load.active_power_mw));
                reactive_power.push(Some(load.reactive_power_mvar));
            }
        }
    }

    for edge_idx in network.graph.edge_indices() {
        let edge = &network.graph[edge_idx];
        match edge {
            Edge::Branch(branch) => {
                element_type.push("branch".to_string());
                element_id.push(branch.id.value() as i64);
                element_name.push(branch.name.clone());
                voltage_kv.push(None);
                from_bus.push(Some(branch.from_bus.value() as i64));
                to_bus.push(Some(branch.to_bus.value() as i64));
                resistance.push(Some(branch.resistance));
                reactance.push(Some(branch.reactance));
                active_power.push(None);
                reactive_power.push(None);
            }
            Edge::Transformer(tx) => {
                element_type.push("transformer".to_string());
                element_id.push(tx.id.value() as i64);
                element_name.push(tx.name.clone());
                voltage_kv.push(None);
                from_bus.push(Some(tx.from_bus.value() as i64));
                to_bus.push(Some(tx.to_bus.value() as i64));
                resistance.push(None);
                reactance.push(None);
                active_power.push(None);
                reactive_power.push(None);
            }
        }
    }

    DataFrame::new(vec![
        Series::new("type", element_type),
        Series::new("id", element_id),
        Series::new("name", element_name),
        Series::new("voltage_kv", voltage_kv),
        Series::new("from_bus", from_bus),
        Series::new("to_bus", to_bus),
        Series::new("resistance", resistance),
        Series::new("reactance", reactance),
        Series::new("active_power_mw", active_power),
        Series::new("reactive_power_mvar", reactive_power),
    ])
}

fn dataframe_to_network(df: &DataFrame) -> Result<Network> {
    let type_col = df
        .column("type")
        .context("missing 'type' column in grid arrow file")?
        .utf8()
        .context("'type' column must be utf8")?;
    let id_col = df
        .column("id")
        .context("missing 'id' column in grid arrow file")?
        .i64()
        .context("'id' column must be integers")?;
    let name_col = df
        .column("name")
        .context("missing 'name' column in grid arrow file")?
        .utf8()
        .context("'name' column must be utf8")?;
    let voltage_col = df
        .column("voltage_kv")
        .context("missing 'voltage_kv' column in grid arrow file")?
        .f64()
        .context("'voltage_kv' column must be float64")?;
    let from_col = df
        .column("from_bus")
        .context("missing 'from_bus' column in grid arrow file")?
        .i64()
        .context("'from_bus' column must be integers")?;
    let to_col = df
        .column("to_bus")
        .context("missing 'to_bus' column in grid arrow file")?
        .i64()
        .context("'to_bus' column must be integers")?;
    let resistance_col = df
        .column("resistance")
        .context("missing 'resistance' column in grid arrow file")?
        .f64()
        .context("'resistance' column must be float64")?;
    let reactance_col = df
        .column("reactance")
        .context("missing 'reactance' column in grid arrow file")?
        .f64()
        .context("'reactance' column must be float64")?;
    let active_power_col = df
        .column("active_power_mw")
        .ok()
        .and_then(|series| series.f64().ok());
    let reactive_power_col = df
        .column("reactive_power_mvar")
        .ok()
        .and_then(|series| series.f64().ok());

    let mut network = Network::new();
    let mut bus_index_map: HashMap<i64, NodeIndex> = HashMap::new();

    for row in 0..df.height() {
        if type_col.get(row) == Some("bus") {
            let id_value = id_col
                .get(row)
                .context("grid row missing id while reconstructing buses")?;
            let name = name_col
                .get(row)
                .context("grid row missing name while reconstructing buses")?;
            let voltage = voltage_col
                .get(row)
                .context("grid row missing voltage for bus reconstruction")?;
            let node_index = network.graph.add_node(Node::Bus(Bus {
                id: BusId::new(usize::try_from(id_value).context("bus id must be non-negative")?),
                name: name.to_string(),
                voltage_kv: voltage,
            }));
            bus_index_map.insert(id_value, node_index);
        }
    }

    for row in 0..df.height() {
        match type_col.get(row) {
            Some("gen") => {
                let name = name_col
                    .get(row)
                    .context("grid row missing name for generator")?;
                let bus_value = from_col
                    .get(row)
                    .context("generator row missing bus reference")?;
                let bus_id = usize::try_from(bus_value).context("bus id must be non-negative")?;
                if !bus_index_map.contains_key(&bus_value) {
                    return Err(anyhow!("generator references unknown bus {}", bus_value));
                }
                let id_value = id_col
                    .get(row)
                    .context("grid row missing id for generator")?;
                let active_power = active_power_col
                    .as_ref()
                    .and_then(|col| col.get(row))
                    .unwrap_or(0.0);
                let reactive_power = reactive_power_col
                    .as_ref()
                    .and_then(|col| col.get(row))
                    .unwrap_or(0.0);

                network.graph.add_node(Node::Gen(Gen {
                    id: GenId::new(
                        usize::try_from(id_value).context("gen id must be non-negative")?,
                    ),
                    name: name.to_string(),
                    bus: BusId::new(bus_id),
                    active_power_mw: active_power,
                    reactive_power_mvar: reactive_power,
                }));
            }
            Some("load") => {
                let name = name_col
                    .get(row)
                    .context("grid row missing name for load")?;
                let bus_value = from_col
                    .get(row)
                    .context("load row missing bus reference")?;
                let bus_id = usize::try_from(bus_value).context("bus id must be non-negative")?;
                if !bus_index_map.contains_key(&bus_value) {
                    return Err(anyhow!("load references unknown bus {}", bus_value));
                }
                let active_power = active_power_col
                    .as_ref()
                    .and_then(|col| col.get(row))
                    .unwrap_or(0.0);
                let reactive_power = reactive_power_col
                    .as_ref()
                    .and_then(|col| col.get(row))
                    .unwrap_or(0.0);
                let id_value = id_col.get(row).context("grid row missing id for load")?;
                let load_id = usize::try_from(id_value).context("load id must be non-negative")?;

                network.graph.add_node(Node::Load(Load {
                    id: LoadId::new(load_id),
                    name: name.to_string(),
                    bus: BusId::new(bus_id),
                    active_power_mw: active_power,
                    reactive_power_mvar: reactive_power,
                }));
            }
            _ => {}
        }
    }

    let mut branch_counter = 0usize;
    let mut transformer_counter = 0usize;
    for row in 0..df.height() {
        match type_col.get(row) {
            Some("branch") => {
                let name = name_col
                    .get(row)
                    .context("grid row missing name for branch")?;
                let from_bus = from_col.get(row).context("branch row missing from_bus")?;
                let to_bus = to_col.get(row).context("branch row missing to_bus")?;
                let resistance = resistance_col.get(row).unwrap_or(0.0);
                let reactance = reactance_col.get(row).unwrap_or(0.0);
                let from_idx = bus_index_map
                    .get(&from_bus)
                    .with_context(|| format!("branch references unknown from bus {}", from_bus))?;
                let to_idx = bus_index_map
                    .get(&to_bus)
                    .with_context(|| format!("branch references unknown to bus {}", to_bus))?;

                let branch = Branch {
                    id: BranchId::new(branch_counter),
                    name: name.to_string(),
                    from_bus: BusId::new(
                        usize::try_from(from_bus).context("bus id must be non-negative")?,
                    ),
                    to_bus: BusId::new(
                        usize::try_from(to_bus).context("bus id must be non-negative")?,
                    ),
                    resistance,
                    reactance,
                };

                network
                    .graph
                    .add_edge(*from_idx, *to_idx, Edge::Branch(branch));
                branch_counter += 1;
            }
            Some("transformer") => {
                let name = name_col
                    .get(row)
                    .context("grid row missing name for transformer")?;
                let from_bus = from_col
                    .get(row)
                    .context("transformer row missing from_bus")?;
                let to_bus = to_col.get(row).context("transformer row missing to_bus")?;
                let from_idx = bus_index_map.get(&from_bus).with_context(|| {
                    format!("transformer references unknown from bus {}", from_bus)
                })?;
                let to_idx = bus_index_map
                    .get(&to_bus)
                    .with_context(|| format!("transformer references unknown to bus {}", to_bus))?;

                let transformer = Transformer {
                    id: TransformerId::new(transformer_counter),
                    name: name.to_string(),
                    from_bus: BusId::new(
                        usize::try_from(from_bus).context("bus id must be non-negative")?,
                    ),
                    to_bus: BusId::new(
                        usize::try_from(to_bus).context("bus id must be non-negative")?,
                    ),
                    ratio: 1.0,
                };

                network
                    .graph
                    .add_edge(*from_idx, *to_idx, Edge::Transformer(transformer));
                transformer_counter += 1;
            }
            _ => continue,
        }
    }

    Ok(network)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::Node;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    #[test]
    fn import_matpower_case_sample() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should exist");
        let case_path = repo_root.join("test_data/matpower/ieee14.case");
        assert!(case_path.exists());

        let temp_dir = tempdir().expect("tmp dir");
        let output_path = temp_dir.path().join("grid.arrow");

        let network =
            import_matpower_case(case_path.to_str().unwrap(), output_path.to_str().unwrap())
                .expect("import should succeed");

        assert!(output_path.exists(), "arrow output file created");

        let loaded_network = load_grid_from_arrow(output_path.to_str().unwrap())
            .expect("loading arrow dataset should succeed");

        assert_eq!(
            loaded_network.graph.node_count(),
            network.graph.node_count()
        );
        assert_eq!(
            loaded_network.graph.edge_count(),
            network.graph.edge_count()
        );

        assert_eq!(network.graph.edge_count(), 20);
        let bus_count = network
            .graph
            .node_indices()
            .filter(|idx| matches!(network.graph[*idx], Node::Bus(_)))
            .count();
        assert_eq!(bus_count, 14);
        let generator_count = network
            .graph
            .node_indices()
            .filter(|idx| matches!(network.graph[*idx], Node::Gen(_)))
            .count();
        assert_eq!(generator_count, 5);
        let load_count = network
            .graph
            .node_indices()
            .filter(|idx| matches!(network.graph[*idx], Node::Load(_)))
            .count();
        assert!(load_count >= 1);
    }

    #[test]
    fn import_psse_raw_sample() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should exist");
        let raw_path = repo_root.join("test_data/psse/sample.raw");
        assert!(raw_path.exists());

        let temp_dir = tempdir().expect("tmp dir");
        let output_path = temp_dir.path().join("psse.arrow");

        let network = import_psse_raw(raw_path.to_str().unwrap(), output_path.to_str().unwrap())
            .expect("import should succeed");

        assert!(output_path.exists(), "arrow output file created");

        let loaded_network = load_grid_from_arrow(output_path.to_str().unwrap())
            .expect("loading arrow dataset should succeed");

        assert_eq!(
            loaded_network.graph.node_count(),
            network.graph.node_count()
        );
        assert_eq!(
            loaded_network.graph.edge_count(),
            network.graph.edge_count()
        );
        assert_eq!(network.graph.node_count(), 2);
        assert_eq!(network.graph.edge_count(), 1);
    }

    #[test]
    fn import_cim_rdf_sample() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should exist");
        let cim_path = repo_root.join("test_data/cim/simple.rdf");
        assert!(cim_path.exists());

        let temp_dir = tempdir().expect("tmp dir");
        let output_path = temp_dir.path().join("cim.arrow");

        let network = import_cim_rdf(cim_path.to_str().unwrap(), output_path.to_str().unwrap())
            .expect("import should succeed");

        assert!(output_path.exists(), "arrow output file created");

        let loaded_network = load_grid_from_arrow(output_path.to_str().unwrap())
            .expect("loading arrow dataset should succeed");

        assert_eq!(
            loaded_network.graph.node_count(),
            network.graph.node_count()
        );
        assert_eq!(
            loaded_network.graph.edge_count(),
            network.graph.edge_count()
        );
    }

    #[test]
    fn import_cim_rdf_zip_sample() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should exist");
        let cim_path = repo_root.join("test_data/cim/simple.rdf");
        assert!(cim_path.exists());

        let temp_dir = tempdir().expect("tmp dir");
        let zip_path = temp_dir.path().join("cim.zip");
        let mut zip_file = File::create(&zip_path).expect("zip file");
        let mut writer = ZipWriter::new(&mut zip_file);
        writer
            .start_file(
                "network.rdf",
                FileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .expect("start file");
        let contents = std::fs::read_to_string(cim_path).expect("read sample");
        writer
            .write_all(contents.as_bytes())
            .expect("write contents");
        writer.finish().expect("finish zip");

        let output_path = temp_dir.path().join("cim_zip.arrow");
        let network = import_cim_rdf(zip_path.to_str().unwrap(), output_path.to_str().unwrap())
            .expect("import should succeed");

        assert!(output_path.exists(), "arrow output file created");
        assert_eq!(network.graph.node_count(), 2);
        assert_eq!(network.graph.edge_count(), 1);
    }
}
