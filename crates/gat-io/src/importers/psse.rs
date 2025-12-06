use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
};

use super::arrow::export_network_to_arrow;
use crate::helpers::{ImportDiagnostics, ImportResult};

/// Parse PSS/E RAW file and return network with diagnostics
pub fn parse_psse(raw_file: &str) -> Result<ImportResult> {
    let path = Path::new(raw_file);
    let mut diag = ImportDiagnostics::new();

    let (buses, branches, loads, gens) = parse_psse_raw(path, &mut diag)?;

    diag.stats.buses = buses.len();
    diag.stats.branches = branches.len();
    diag.stats.loads = loads.len();
    diag.stats.generators = gens.len();

    let network = build_network_from_psse(buses, branches, loads, gens, &mut diag)?;

    Ok(ImportResult {
        network,
        diagnostics: diag,
    })
}

/// Legacy function for backwards compatibility - parses and writes Arrow
pub fn import_psse_raw(raw_file: &str, output_file: &str) -> Result<Network> {
    println!("Importing PSSE RAW from {} to {}", raw_file, output_file);

    let result = parse_psse(raw_file)?;

    println!(
        "Parsed {} buses, {} branches, {} loads, {} generators",
        result.diagnostics.stats.buses,
        result.diagnostics.stats.branches,
        result.diagnostics.stats.loads,
        result.diagnostics.stats.generators
    );

    export_network_to_arrow(&result.network, output_file)?;
    Ok(result.network)
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
    charging_b: f64,
    rate_a: Option<f64>,
    tap_ratio: f64,
    phase_shift_rad: f64,
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

/// PSS/E RAW file sections
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum PsseSection {
    Header,
    Bus,
    Load,
    FixedShunt,
    Generator,
    Branch,
    Transformer,
    Other, // Area, Zone, Owner, etc. - we skip these
}

/// Detect PSS/E version from header line
/// v33+ format: "0, 100.00, 33, ..." where field 3 is version
/// Old format: no version field
fn detect_psse_version(first_line: &str) -> u32 {
    let parts: Vec<&str> = first_line.split(',').map(|s| s.trim()).collect();
    if parts.len() >= 3 {
        // Try to parse the third field as version number
        if let Ok(version) = parts[2]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .parse::<u32>()
        {
            if version >= 29 && version <= 40 {
                return version;
            }
        }
    }
    0 // Old format (pre-v29)
}

/// Check if line is a v33+ section terminator "0 / END OF ... DATA"
/// Returns the next section to transition to
fn check_v33_section_marker(line: &str) -> Option<PsseSection> {
    let trimmed = line.trim();

    // v33+ section markers start with "0" followed by "/" and section info
    if !trimmed.starts_with('0') {
        return None;
    }

    // Get the comment part after "/"
    let comment = line.split('/').nth(1)?.to_ascii_uppercase();

    // Parse "END OF X DATA, BEGIN Y DATA" or just "END OF X DATA"
    if comment.contains("BEGIN LOAD DATA") {
        Some(PsseSection::Load)
    } else if comment.contains("BEGIN FIXED SHUNT DATA") {
        Some(PsseSection::FixedShunt)
    } else if comment.contains("BEGIN GENERATOR DATA") {
        Some(PsseSection::Generator)
    } else if comment.contains("BEGIN BRANCH DATA") {
        Some(PsseSection::Branch)
    } else if comment.contains("BEGIN TRANSFORMER DATA") {
        Some(PsseSection::Transformer)
    } else if comment.contains("END OF") {
        // Any other "END OF X DATA" transitions to Other (skip)
        Some(PsseSection::Other)
    } else {
        None
    }
}

fn parse_psse_raw(path: &Path, diag: &mut ImportDiagnostics) -> Result<PsseRawTables> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("reading PSSE RAW '{}'; ensure file exists", path.display()))?;

    let mut lines = contents.lines().peekable();
    let mut buses = Vec::new();
    let mut branches = Vec::new();
    let mut loads = Vec::new();
    let mut gens = Vec::new();

    // Read first line to detect version
    let first_line = lines.next().unwrap_or("");
    let version = detect_psse_version(first_line);

    if version >= 29 {
        // v33+ format: sequential sections with "0 / END OF X, BEGIN Y" markers
        // Skip header lines (typically 2-3 lines before bus data starts)
        let mut header_lines = 1; // Already consumed first line
        while let Some(line) = lines.peek() {
            let trimmed = line.trim();
            // Bus data lines start with a number (bus ID)
            if trimmed
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                // Check if it looks like a bus line (has quoted name in col 1)
                if trimmed.contains('\'') || trimmed.contains('"') {
                    break;
                }
            }
            lines.next();
            header_lines += 1;
            if header_lines > 5 {
                break; // Safety limit
            }
        }

        // Now parse sections
        let mut section = PsseSection::Bus;
        // For transformer parsing, we need to collect multiple lines
        let mut transformer_lines: Vec<String> = Vec::new();
        let mut transformer_start_line: usize = 0;
        let mut line_num = header_lines;

        for raw_line in lines {
            line_num += 1;

            // Check for section transition markers
            if let Some(next_section) = check_v33_section_marker(raw_line) {
                // If we were in transformer section, flush any pending transformer
                if section == PsseSection::Transformer && !transformer_lines.is_empty() {
                    if let Some(xfmr) = parse_psse_transformer_v33(&transformer_lines) {
                        branches.push(xfmr);
                    }
                    transformer_lines.clear();
                }
                section = next_section;
                continue;
            }

            let line = raw_line.split('/').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }

            match section {
                PsseSection::Bus => {
                    if let Some(bus) = parse_psse_bus_line_v33(line) {
                        buses.push(bus);
                    } else {
                        diag.add_error_at_line("parse", "Malformed bus record", line_num);
                    }
                }
                PsseSection::Load => {
                    // Note: parse_psse_load_line_v33 returns None for out-of-service loads (intentional skip)
                    if let Some(load) = parse_psse_load_line_v33(line) {
                        loads.push(load);
                    }
                }
                PsseSection::Generator => {
                    if let Some(gen) = parse_psse_gen_line_v33(line) {
                        gens.push(gen);
                    } else {
                        diag.add_error_at_line("parse", "Malformed generator record", line_num);
                    }
                }
                PsseSection::Branch => {
                    if let Some(branch) = parse_psse_branch_line_v33(line) {
                        branches.push(branch);
                    } else {
                        diag.add_error_at_line("parse", "Malformed branch record", line_num);
                    }
                }
                PsseSection::Transformer => {
                    // Transformer records are multi-line (4 lines for 2-winding, 5 for 3-winding)
                    // Collect lines until we have a complete record
                    if transformer_lines.is_empty() {
                        transformer_start_line = line_num;
                    }
                    transformer_lines.push(line.to_string());

                    // Check if we have a complete 2-winding transformer (4 lines)
                    // Line 1 starts with bus numbers, line 4 starts with winding voltage
                    if transformer_lines.len() >= 4 {
                        // Check if line 1 has K=0 (2-winding) by parsing third field
                        let first_cols: Vec<&str> =
                            transformer_lines[0].split(',').map(|s| s.trim()).collect();
                        let k = first_cols
                            .get(2)
                            .and_then(|s| s.parse::<i32>().ok())
                            .unwrap_or(0);

                        if k == 0 {
                            // 2-winding transformer: 4 lines
                            if let Some(xfmr) = parse_psse_transformer_v33(&transformer_lines) {
                                branches.push(xfmr);
                            } else {
                                diag.add_error_at_line(
                                    "parse",
                                    "Malformed 2-winding transformer record",
                                    transformer_start_line,
                                );
                            }
                            transformer_lines.clear();
                        } else if transformer_lines.len() >= 5 {
                            // 3-winding transformer: 5 lines (we skip these)
                            diag.add_warning_at_line(
                                "parse",
                                "Skipped 3-winding transformer (not supported)",
                                transformer_start_line,
                            );
                            transformer_lines.clear();
                        }
                    }
                }
                _ => {} // Skip other sections
            }
        }

        // Flush any remaining transformer
        if !transformer_lines.is_empty() {
            if let Some(xfmr) = parse_psse_transformer_v33(&transformer_lines) {
                branches.push(xfmr);
            }
        }
    } else {
        // Old format with "BUS DATA FOLLOWS" style markers
        let mut section = PsseSection::Header;

        for raw_line in lines {
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
                    section = PsseSection::Header;
                    continue;
                }
                "BRANCH DATA FOLLOWS" => {
                    section = PsseSection::Branch;
                    continue;
                }
                "END OF BRANCH DATA" => {
                    section = PsseSection::Header;
                    continue;
                }
                "LOAD DATA FOLLOWS" => {
                    section = PsseSection::Load;
                    continue;
                }
                "END OF LOAD DATA" => {
                    section = PsseSection::Header;
                    continue;
                }
                "GENERATOR DATA FOLLOWS" => {
                    section = PsseSection::Generator;
                    continue;
                }
                "END OF GENERATOR DATA" => {
                    section = PsseSection::Header;
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
                _ => {}
            }
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
    let charging_b = columns
        .get(5)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let rate_a = columns
        .get(6)
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|val| *val > 0.0);
    let status = columns
        .get(13)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    Some(PsseBranch {
        from,
        to,
        resistance,
        reactance,
        charging_b,
        rate_a,
        tap_ratio: 1.0,
        phase_shift_rad: 0.0,
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

// ============================================================================
// v33+ format parsers
// Column positions differ from old format
// ============================================================================

/// Parse v33+ bus line: id, 'name', base_kv, type, area, zone, owner, vm, va
fn parse_psse_bus_line_v33(line: &str) -> Option<PsseBus> {
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

/// Parse v33+ load line: bus, 'id', status, area, zone, pd, qd, ...
fn parse_psse_load_line_v33(line: &str) -> Option<PsseLoad> {
    let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if columns.len() < 7 {
        return None;
    }

    let bus = columns[0].parse::<usize>().ok()?;
    // col[2] is status (1=in service)
    let status = columns
        .get(2)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);
    if status == 0 {
        return None; // Skip out-of-service loads
    }
    // col[5] = PL (constant power P)
    // col[6] = QL (constant power Q)
    let pd = columns
        .get(5)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let qd = columns
        .get(6)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Some(PsseLoad { bus, pd, qd })
}

/// Parse v33+ generator line: bus, 'id', pg, qg, qt, qb, vs, ireg, mbase, ..., stat(col 14)
fn parse_psse_gen_line_v33(line: &str) -> Option<PsseGen> {
    let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if columns.len() < 5 {
        return None;
    }

    let bus = columns[0].parse::<usize>().ok()?;
    // col[2] = PG, col[3] = QG
    let pg = columns
        .get(2)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let qg = columns
        .get(3)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    // col[14] = STAT (machine status)
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

/// Parse v33+ branch line: from, to, 'ckt', r, x, b, ratea, rateb, ratec, ..., st(col 13)
fn parse_psse_branch_line_v33(line: &str) -> Option<PsseBranch> {
    let columns: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if columns.len() < 6 {
        return None;
    }

    let from = columns[0].parse::<usize>().ok()?;
    let to = columns[1].parse::<usize>().ok()?;
    // col[2] = circuit ID (string), col[3] = R, col[4] = X, col[5] = B
    let resistance = columns
        .get(3)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let reactance = columns
        .get(4)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let charging_b = columns
        .get(5)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    // col[6] = RATEA, col[7] = RATEB, col[8] = RATEC
    let rate_a = columns
        .get(6)
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|val| *val > 0.0);
    // col[13] = ST (branch status)
    let status = columns
        .get(13)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    Some(PsseBranch {
        from,
        to,
        resistance,
        reactance,
        charging_b,
        rate_a,
        tap_ratio: 1.0,
        phase_shift_rad: 0.0,
        in_service: status != 0,
    })
}

/// Parse v33+ 2-winding transformer (4 lines)
/// Line 1: I, J, K, CKT, CW, CZ, CM, MAG1, MAG2, NMETR, NAME, STAT, ...
/// Line 2: R1-2, X1-2, SBASE1-2
/// Line 3: WINDV1, NOMV1, ANG1, RATA1, RATB1, RATC1, ...
/// Line 4: WINDV2, NOMV2
fn parse_psse_transformer_v33(lines: &[String]) -> Option<PsseBranch> {
    if lines.len() < 4 {
        return None;
    }

    // Line 1: bus connections and status
    let line1_cols: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();
    if line1_cols.len() < 12 {
        return None;
    }

    let from = line1_cols[0].parse::<usize>().ok()?;
    let to = line1_cols[1].parse::<usize>().ok()?;
    // col[2] = K (tertiary bus, 0 for 2-winding)
    // col[11] = STAT (status: 0=out, 1=in service, 2/3/4 = special)
    let status = line1_cols
        .get(11)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    // Line 2: impedance data (R, X on system base)
    let line2_cols: Vec<&str> = lines[1].split(',').map(|s| s.trim()).collect();
    if line2_cols.len() < 3 {
        return None;
    }

    let resistance = line2_cols[0].parse::<f64>().unwrap_or(0.0);
    let reactance = line2_cols[1].parse::<f64>().unwrap_or(0.0);
    // col[2] = SBASE1-2 (winding MVA base, typically 100)

    // Line 3: winding 1 data (tap ratio and angle)
    let line3_cols: Vec<&str> = lines[2].split(',').map(|s| s.trim()).collect();
    if line3_cols.len() < 6 {
        return None;
    }

    // WINDV1 = off-nominal tap ratio (pu on winding 1 base)
    let tap_ratio = line3_cols[0].parse::<f64>().unwrap_or(1.0);
    // col[1] = NOMV1 (nominal voltage, 0.0 means use bus base kV)
    // col[2] = ANG1 (phase shift angle in degrees)
    let phase_shift_deg = line3_cols
        .get(2)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    // col[3] = RATA1 (MVA rating)
    let rate_a = line3_cols
        .get(3)
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|val| *val > 0.0);

    // Line 4: winding 2 data (we mostly ignore this for simple model)
    // WINDV2 = tap ratio on winding 2 (typically 1.0)

    Some(PsseBranch {
        from,
        to,
        resistance,
        reactance,
        charging_b: 0.0, // Transformers typically have no line charging
        rate_a,
        tap_ratio,
        phase_shift_rad: phase_shift_deg.to_radians(),
        in_service: status != 0,
    })
}

fn build_network_from_psse(
    buses: Vec<PsseBus>,
    branches: Vec<PsseBranch>,
    loads: Vec<PsseLoad>,
    gens: Vec<PsseGen>,
    _diag: &mut ImportDiagnostics,
) -> Result<Network> {
    let mut network = Network::new();
    let mut bus_index_map: HashMap<usize, NodeIndex> = HashMap::new();

    for bus in buses {
        let id = BusId::new(bus.id);
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id,
            name: bus.name,
            base_kv: gat_core::Kilovolts(bus.voltage_kv),
            ..Bus::default()
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
            active_power: gat_core::Megawatts(pd),
            reactive_power: gat_core::Megavars(qd),
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
            active_power: gat_core::Megawatts(gen.pg),
            reactive_power: gat_core::Megavars(gen.qg),
            pmin: gat_core::Megawatts(0.0),
            pmax: gat_core::Megawatts(f64::INFINITY),
            qmin: gat_core::Megavars(f64::NEG_INFINITY),
            qmax: gat_core::Megavars(f64::INFINITY),
            cost_model: gat_core::CostModel::NoCost,
            is_synchronous_condenser: false,
            ..Gen::default()
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
            tap_ratio: branch.tap_ratio,
            phase_shift: gat_core::Radians(branch.phase_shift_rad),
            charging_b: gat_core::PerUnit(branch.charging_b),
            s_max: branch.rate_a.map(gat_core::MegavoltAmperes),
            status: branch.in_service,
            rating_a: branch.rate_a.map(gat_core::MegavoltAmperes),
            ..Branch::default()
        };

        network
            .graph
            .add_edge(from_idx, to_idx, Edge::Branch(branch_record));
    }

    Ok(network)
}
