use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use caseformat::{read_dir, read_zip, Branch as CaseBranch, Bus as CaseBus, Gen as CaseGen};
use gat_core::Network;

use super::matpower_parser::{parse_matpower_file, MatpowerCase, MatpowerGenCost};
use crate::helpers::{
    BranchInput, BusInput, GenInput, ImportDiagnostics, ImportResult, LoadInput, NetworkBuilder,
};
use crate::arrow_manifest::{compute_sha256, SourceInfo};
use crate::exporters::arrow_directory_writer::SystemInfo;
use zip::ZipArchive;

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

fn matpower_metadata(m_file: &Path) -> Result<(Option<SystemInfo>, Option<SourceInfo>)> {
    if !m_file.exists() {
        return Ok((None, None));
    }

    if m_file.is_file() {
        let file_hash = compute_sha256(m_file)?;
        let file_name = m_file
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| m_file.display().to_string());

        let source_info = SourceInfo {
            file: file_name.clone(),
            format: "matpower".to_string(),
            file_hash,
        };

        let ext = m_file
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());

        let system_info = match ext.as_deref() {
            Some("m") => parse_matpower_file(m_file).ok().map(|case| SystemInfo {
                base_mva: case.base_mva,
                base_frequency_hz: 60.0,
                name: m_file
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string()),
                description: Some(format!("Imported MATPOWER case {}", file_name)),
            }),
            Some("case") => case_metadata_from_archive(m_file)
                .ok()
                .flatten()
                .map(|(case_name, base_mva)| SystemInfo {
                    base_mva,
                    base_frequency_hz: 60.0,
                    name: case_name.or_else(|| {
                        m_file
                            .file_stem()
                            .map(|stem| stem.to_string_lossy().to_string())
                    }),
                    description: Some(format!("Imported MATPOWER case {}", file_name)),
                }),
            _ => None,
        };

        return Ok((system_info, Some(source_info)));
    }

    Ok((None, None))
}

fn case_metadata_from_archive(path: &Path) -> Result<Option<(Option<String>, f64)>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.name().to_lowercase().ends_with("case.csv") {
            let mut contents = String::new();
            entry.read_to_string(&mut contents)?;
            let mut lines = contents.lines();
            lines.next(); // skip header
            if let Some(row) = lines.next() {
                let cols: Vec<&str> = row.split(',').collect();
                let case_name = cols.get(0).map(|s| s.to_string());
                let base_mva = cols
                    .get(2)
                    .and_then(|v| v.trim().parse::<f64>().ok())
                    .unwrap_or(100.0);
                return Ok(Some((case_name, base_mva)));
            }
        }
    }
    Ok(None)
}

pub fn import_matpower_case(m_file: &str, output_dir: impl AsRef<Path>) -> Result<Network> {
    println!(
        "Importing MATPOWER from {} to {}",
        m_file,
        output_dir.as_ref().display()
    );
    let path = Path::new(m_file);
    let (system_info, source_info) = matpower_metadata(path)?;
    let network = load_matpower_network(path)?;
    let writer = crate::exporters::ArrowDirectoryWriter::new(output_dir)?;
    writer.write_network(&network, system_info, source_info)?;
    Ok(network)
}

/// Parse a MATPOWER case file and return an ImportResult with diagnostics.
/// This is the new diagnostics-aware entrypoint.
pub fn parse_matpower(m_file: &str) -> Result<ImportResult> {
    let path = Path::new(m_file);
    let mut diag = ImportDiagnostics::new();
    let network = load_matpower_network_with_diagnostics(path, &mut diag)?;
    Ok(ImportResult {
        network,
        diagnostics: diag,
    })
}

/// Load MATPOWER network with diagnostics tracking.
fn load_matpower_network_with_diagnostics(
    m_file: &Path,
    diag: &mut ImportDiagnostics,
) -> Result<Network> {
    // If it's a single .m file, use our parser
    if m_file.is_file() {
        if let Some(ext) = m_file.extension() {
            if ext == "m" {
                let case = parse_matpower_file(m_file)?;
                return build_network_from_matpower_case_with_diagnostics(&case, diag);
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
        return build_network_from_case_with_diagnostics(buses, branches, gens, diag);
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
            return build_network_from_matpower_case_with_diagnostics(&case, diag);
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
        return build_network_from_case_with_diagnostics(buses, branches, gens, diag);
    }

    Err(anyhow!(
        "MATPOWER path '{}' is neither a file nor a directory",
        m_file.display()
    ))
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
    build_network_from_matpower_case_impl(case, None)
}

/// Internal implementation shared by with/without diagnostics variants
fn build_network_from_matpower_case_impl(
    case: &MatpowerCase,
    diag: Option<&mut ImportDiagnostics>,
) -> Result<Network> {
    // Pre-allocate with capacity hints based on known data sizes
    let bus_capacity = case.bus.len();
    let mut builder = match diag {
        Some(d) => NetworkBuilder::with_diagnostics_and_capacity(d, bus_capacity),
        None => NetworkBuilder::with_capacity(bus_capacity),
    };

    // Add buses
    for bus in &case.bus {
        builder.add_bus(BusInput {
            id: bus.bus_i,
            name: None,
            voltage_kv: bus.base_kv,
            voltage_pu: Some(bus.vm),
            angle_rad: Some(bus.va.to_radians()),
            vmin_pu: Some(bus.vmin),
            vmax_pu: Some(bus.vmax),
            area_id: Some(bus.area as i64),
            zone_id: Some(bus.zone as i64),
        });
    }

    // Add loads (embedded in MATPOWER bus data)
    for bus in &case.bus {
        if bus.pd != 0.0 || bus.qd != 0.0 {
            builder.add_load(LoadInput {
                bus_id: bus.bus_i,
                name: Some(format!("Load {}", bus.bus_i)),
                active_power_mw: bus.pd,
                reactive_power_mvar: bus.qd,
            });
        }
    }

    // Add generators
    let mut skipped_gens = 0usize;
    for (i, gen) in case.gen.iter().enumerate() {
        if gen.gen_status == 0 {
            skipped_gens += 1;
            continue;
        }

        // Synchronous condenser detection:
        // 1. Pmax <= 0 (can only absorb power or provide reactive support)
        // 2. Negative active power setpoint (absorbing P)
        // 3. Negative Pmin with Pmax near zero (typical syncon with small motor load)
        let is_syncon = gen.pmax <= 0.0 || gen.pg < 0.0 || (gen.pmin < 0.0 && gen.pmax <= 0.1);

        // Get cost data if available
        let gencost = case.gencost.get(i);
        let cost_model = gencost_to_cost_model(gencost);
        let cost_startup = gencost.map(|gc| gc.startup);
        let cost_shutdown = gencost.map(|gc| gc.shutdown);

        builder.add_gen(GenInput {
            bus_id: gen.gen_bus,
            name: None,
            pg: gen.pg,
            qg: gen.qg,
            pmin: gen.pmin,
            pmax: gen.pmax,
            qmin: gen.qmin,
            qmax: gen.qmax,
            voltage_setpoint_pu: Some(gen.vg),
            mbase_mva: Some(gen.mbase),
            cost_startup,
            cost_shutdown,
            cost_model,
            is_synchronous_condenser: is_syncon,
        });
    }
    if skipped_gens > 0 {
        builder.record_skipped(skipped_gens);
    }

    // Add branches
    let mut skipped_branches = 0usize;
    for br in &case.branch {
        if br.br_status == 0 {
            skipped_branches += 1;
            continue;
        }

        // Phase-shifter detection: non-zero phase shift OR negative reactance OR negative resistance
        let is_phase_shifter = br.shift.abs() > 1e-6 || br.br_x < 0.0 || br.br_r < 0.0;
        
        let tap_ratio = if br.tap == 0.0 { 1.0 } else { br.tap };
        let phase_shift_rad = br.shift.to_radians();
        
        // Determine element type
        let element_type = if tap_ratio != 1.0 || phase_shift_rad.abs() > 1e-9 {
            Some("transformer".to_string())
        } else {
            Some("line".to_string())
        };

        builder.add_branch(BranchInput {
            from_bus: br.f_bus,
            to_bus: br.t_bus,
            name: None,
            resistance: br.br_r,
            reactance: br.br_x,
            charging_b: br.br_b,
            tap_ratio,
            phase_shift_rad,
            rate_mva: (br.rate_a > 0.0).then_some(br.rate_a),
            rating_b_mva: (br.rate_b > 0.0).then_some(br.rate_b),
            rating_c_mva: (br.rate_c > 0.0).then_some(br.rate_c),
            angle_min_rad: Some(br.angmin.to_radians()),
            angle_max_rad: Some(br.angmax.to_radians()),
            element_type,
            is_phase_shifter,
        });
    }
    if skipped_branches > 0 {
        builder.record_skipped(skipped_branches);
    }

    Ok(builder.build())
}

/// Build network from MATPOWER case with diagnostics tracking
fn build_network_from_matpower_case_with_diagnostics(
    case: &MatpowerCase,
    diag: &mut ImportDiagnostics,
) -> Result<Network> {
    build_network_from_matpower_case_impl(case, Some(diag))
}

/// Build network from caseformat structs
fn build_network_from_case(
    case_buses: Vec<CaseBus>,
    case_branches: Vec<CaseBranch>,
    case_gens: Vec<CaseGen>,
) -> Result<Network> {
    build_network_from_case_impl(case_buses, case_branches, case_gens, None)
}

/// Build network from caseformat structs with diagnostics tracking
fn build_network_from_case_with_diagnostics(
    case_buses: Vec<CaseBus>,
    case_branches: Vec<CaseBranch>,
    case_gens: Vec<CaseGen>,
    diag: &mut ImportDiagnostics,
) -> Result<Network> {
    build_network_from_case_impl(case_buses, case_branches, case_gens, Some(diag))
}

/// Internal implementation shared by with/without diagnostics variants
fn build_network_from_case_impl(
    case_buses: Vec<CaseBus>,
    case_branches: Vec<CaseBranch>,
    case_gens: Vec<CaseGen>,
    diag: Option<&mut ImportDiagnostics>,
) -> Result<Network> {
    // Pre-allocate with capacity hints based on known data sizes
    let bus_capacity = case_buses.len();
    let mut builder = match diag {
        Some(d) => NetworkBuilder::with_diagnostics_and_capacity(d, bus_capacity),
        None => NetworkBuilder::with_capacity(bus_capacity),
    };

    // Add buses
    for case_bus in &case_buses {
        builder.add_bus(BusInput {
            id: case_bus.bus_i,
            name: None,
            voltage_kv: case_bus.base_kv,
            voltage_pu: Some(case_bus.vm),
            angle_rad: Some(case_bus.va.to_radians()),
            vmin_pu: Some(case_bus.vmin),
            vmax_pu: Some(case_bus.vmax),
            area_id: None,
            zone_id: Some(case_bus.zone as i64),
        });
    }

    // Add loads (embedded in caseformat bus data)
    for case_bus in &case_buses {
        if case_bus.pd != 0.0 || case_bus.qd != 0.0 {
            builder.add_load(LoadInput {
                bus_id: case_bus.bus_i,
                name: Some(format!("Load {}", case_bus.bus_i)),
                active_power_mw: case_bus.pd,
                reactive_power_mvar: case_bus.qd,
            });
        }
    }

    // Add generators
    let mut skipped_gens = 0usize;
    for case_gen in case_gens {
        if case_gen.gen_status == 0 {
            skipped_gens += 1;
            continue;
        }

        builder.add_gen(GenInput {
            bus_id: case_gen.gen_bus,
            name: None,
            pg: case_gen.pg,
            qg: case_gen.qg,
            pmin: case_gen.pmin,
            pmax: case_gen.pmax,
            qmin: case_gen.qmin,
            qmax: case_gen.qmax,
            voltage_setpoint_pu: Some(case_gen.vg),
            mbase_mva: Some(case_gen.mbase),
            cost_startup: None,
            cost_shutdown: None,
            cost_model: gat_core::CostModel::NoCost,
            is_synchronous_condenser: false,
        });
    }
    if skipped_gens > 0 {
        builder.record_skipped(skipped_gens);
    }

    // Add branches
    let mut skipped_branches = 0usize;
    for case_branch in case_branches {
        if !case_branch.is_on() {
            skipped_branches += 1;
            continue;
        }

        let tap_ratio = if case_branch.tap == 0.0 {
            1.0
        } else {
            case_branch.tap
        };
        let phase_shift_rad = case_branch.shift.to_radians();
        
        // Determine element type
        let element_type = if tap_ratio != 1.0 || phase_shift_rad.abs() > 1e-9 {
            Some("transformer".to_string())
        } else {
            Some("line".to_string())
        };

        builder.add_branch(BranchInput {
            from_bus: case_branch.f_bus,
            to_bus: case_branch.t_bus,
            name: None,
            resistance: case_branch.br_r,
            reactance: case_branch.br_x,
            charging_b: case_branch.br_b,
            tap_ratio,
            phase_shift_rad,
            rate_mva: (case_branch.rate_a > 0.0).then_some(case_branch.rate_a),
            rating_b_mva: (case_branch.rate_b > 0.0).then_some(case_branch.rate_b),
            rating_c_mva: (case_branch.rate_c > 0.0).then_some(case_branch.rate_c),
            angle_min_rad: case_branch.angmin.map(|v| v.to_radians()),
            angle_max_rad: case_branch.angmax.map(|v| v.to_radians()),
            element_type,
            is_phase_shifter: false,
        });
    }
    if skipped_branches > 0 {
        builder.record_skipped(skipped_branches);
    }

    Ok(builder.build())
}
