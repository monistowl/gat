//! MATPOWER .m file parser
//!
//! Parses MATPOWER case files in .m (MATLAB) format.
//! Supports the standard MATPOWER case format with mpc.bus, mpc.gen, mpc.branch, etc.

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;

use crate::helpers::{safe_f64_to_i32, safe_f64_to_usize};

/// Parsed MATPOWER case data
#[derive(Debug, Default)]
pub struct MatpowerCase {
    pub version: String,
    pub base_mva: f64,
    pub bus: Vec<MatpowerBus>,
    pub gen: Vec<MatpowerGen>,
    pub branch: Vec<MatpowerBranch>,
    pub gencost: Vec<MatpowerGenCost>,
}

/// MATPOWER bus data (columns from mpc.bus matrix)
#[derive(Debug, Clone)]
pub struct MatpowerBus {
    pub bus_i: usize,
    pub bus_type: i32,
    pub pd: f64,
    pub qd: f64,
    pub gs: f64,
    pub bs: f64,
    pub area: i32,
    pub vm: f64,
    pub va: f64,
    pub base_kv: f64,
    pub zone: i32,
    pub vmax: f64,
    pub vmin: f64,
}

/// MATPOWER generator data (columns from mpc.gen matrix)
#[derive(Debug, Clone)]
pub struct MatpowerGen {
    pub gen_bus: usize,
    pub pg: f64,
    pub qg: f64,
    pub qmax: f64,
    pub qmin: f64,
    pub vg: f64,
    pub mbase: f64,
    pub gen_status: i32,
    pub pmax: f64,
    pub pmin: f64,
}

/// MATPOWER branch data (columns from mpc.branch matrix)
#[derive(Debug, Clone)]
pub struct MatpowerBranch {
    pub f_bus: usize,
    pub t_bus: usize,
    pub br_r: f64,
    pub br_x: f64,
    pub br_b: f64,
    pub rate_a: f64,
    pub rate_b: f64,
    pub rate_c: f64,
    pub tap: f64,
    pub shift: f64,
    pub br_status: i32,
    pub angmin: f64,
    pub angmax: f64,
}

/// MATPOWER generator cost data
#[derive(Debug, Clone)]
pub struct MatpowerGenCost {
    pub model: i32,
    pub startup: f64,
    pub shutdown: f64,
    pub ncost: i32,
    pub cost: Vec<f64>,
}

/// Parse a MATPOWER .m file
pub fn parse_matpower_file(path: &Path) -> Result<MatpowerCase> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading MATPOWER file: {}", path.display()))?;
    parse_matpower_string(&content)
}

/// Parse MATPOWER content from a string (single-pass implementation)
pub fn parse_matpower_string(content: &str) -> Result<MatpowerCase> {
    let mut case = MatpowerCase::default();
    case.base_mva = 100.0; // Default

    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }

        // Handle scalar/string assignments
        if trimmed.starts_with("mpc.version") && trimmed.contains('=') {
            case.version = extract_inline_string(trimmed);
        } else if trimmed.starts_with("mpc.baseMVA") && trimmed.contains('=') {
            if let Some(v) = extract_inline_scalar(trimmed) {
                case.base_mva = v;
            }
        }
        // Handle matrix sections - parse inline from iterator
        // Note: mpc.gencost must be checked BEFORE mpc.gen (prefix collision)
        else if trimmed.starts_with("mpc.bus") && trimmed.contains('[') {
            case.bus = parse_bus_section(trimmed, &mut lines)?;
        } else if trimmed.starts_with("mpc.gencost") && trimmed.contains('[') {
            case.gencost = parse_gencost_section(trimmed, &mut lines)?;
        } else if trimmed.starts_with("mpc.gen") && trimmed.contains('[') {
            case.gen = parse_gen_section(trimmed, &mut lines)?;
        } else if trimmed.starts_with("mpc.branch") && trimmed.contains('[') {
            case.branch = parse_branch_section(trimmed, &mut lines)?;
        }
    }

    if case.bus.is_empty() {
        return Err(anyhow!("mpc.bus matrix not found"));
    }

    Ok(case)
}

/// Extract string value from a single line (e.g., "mpc.version = '2';")
fn extract_inline_string(line: &str) -> String {
    line.split('=')
        .nth(1)
        .map(|v| {
            v.trim()
                .trim_matches(|c| c == '\'' || c == '"' || c == ';')
                .to_string()
        })
        .unwrap_or_default()
}

/// Extract scalar value from a single line (e.g., "mpc.baseMVA = 100.0;")
fn extract_inline_scalar(line: &str) -> Option<f64> {
    line.split('=')
        .nth(1)
        .and_then(|v| v.trim().trim_end_matches(';').parse().ok())
}

/// Parse a row of numeric values from MATPOWER format
fn parse_row(line: &str) -> Vec<f64> {
    line.split(|c: char| c.is_whitespace() || c == ';' || c == '\t')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

/// Check if a line signals end of matrix section
fn is_matrix_end(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "];" || trimmed.ends_with("];")
}

/// Parse bus section from iterator (single-pass)
fn parse_bus_section<'a>(
    header: &str,
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> Result<Vec<MatpowerBus>> {
    let mut buses = Vec::new();
    let mut row_idx = 0;

    // Check if data starts on the header line (after '[')
    if let Some(after_bracket) = header.split('[').nth(1) {
        let data_part = after_bracket.trim_end_matches("];").trim();
        if !data_part.is_empty() && !data_part.starts_with('%') {
            let values = parse_row(data_part);
            if values.len() >= 13 {
                buses.push(parse_bus_row(&values, row_idx)?);
                row_idx += 1;
            }
        }
        // If header line ends the matrix, we're done
        if header.contains("];") {
            return Ok(buses);
        }
    }

    // Continue consuming lines until we hit '];'
    while let Some(line) = lines.next() {
        if is_matrix_end(line) {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }
        let values = parse_row(trimmed);
        if values.len() >= 13 {
            buses.push(parse_bus_row(&values, row_idx)?);
            row_idx += 1;
        }
    }
    Ok(buses)
}

fn parse_bus_row(values: &[f64], row_idx: usize) -> Result<MatpowerBus> {
    Ok(MatpowerBus {
        bus_i: safe_f64_to_usize(values[0])
            .with_context(|| format!("invalid bus_i at row {}", row_idx))?,
        bus_type: safe_f64_to_i32(values[1])
            .with_context(|| format!("invalid bus_type at row {}", row_idx))?,
        pd: values[2],
        qd: values[3],
        gs: values[4],
        bs: values[5],
        area: safe_f64_to_i32(values[6])
            .with_context(|| format!("invalid area at row {}", row_idx))?,
        vm: values[7],
        va: values[8],
        base_kv: values[9],
        zone: safe_f64_to_i32(values[10])
            .with_context(|| format!("invalid zone at row {}", row_idx))?,
        vmax: values[11],
        vmin: values[12],
    })
}

/// Parse gen section from iterator (single-pass)
fn parse_gen_section<'a>(
    header: &str,
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> Result<Vec<MatpowerGen>> {
    let mut gens = Vec::new();
    let mut row_idx = 0;

    // Check if data starts on the header line
    if let Some(after_bracket) = header.split('[').nth(1) {
        let data_part = after_bracket.trim_end_matches("];").trim();
        if !data_part.is_empty() && !data_part.starts_with('%') {
            let values = parse_row(data_part);
            if values.len() >= 10 {
                gens.push(parse_gen_row(&values, row_idx)?);
                row_idx += 1;
            }
        }
        if header.contains("];") {
            return Ok(gens);
        }
    }

    while let Some(line) = lines.next() {
        if is_matrix_end(line) {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }
        let values = parse_row(trimmed);
        if values.len() >= 10 {
            gens.push(parse_gen_row(&values, row_idx)?);
            row_idx += 1;
        }
    }
    Ok(gens)
}

fn parse_gen_row(values: &[f64], row_idx: usize) -> Result<MatpowerGen> {
    Ok(MatpowerGen {
        gen_bus: safe_f64_to_usize(values[0])
            .with_context(|| format!("invalid gen_bus at row {}", row_idx))?,
        pg: values[1],
        qg: values[2],
        qmax: values[3],
        qmin: values[4],
        vg: values[5],
        mbase: values[6],
        gen_status: safe_f64_to_i32(values[7])
            .with_context(|| format!("invalid gen_status at row {}", row_idx))?,
        pmax: values[8],
        pmin: values[9],
    })
}

/// Parse branch section from iterator (single-pass)
fn parse_branch_section<'a>(
    header: &str,
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> Result<Vec<MatpowerBranch>> {
    let mut branches = Vec::new();
    let mut row_idx = 0;

    // Check if data starts on the header line
    if let Some(after_bracket) = header.split('[').nth(1) {
        let data_part = after_bracket.trim_end_matches("];").trim();
        if !data_part.is_empty() && !data_part.starts_with('%') {
            let values = parse_row(data_part);
            if values.len() >= 13 {
                branches.push(parse_branch_row(&values, row_idx)?);
                row_idx += 1;
            }
        }
        if header.contains("];") {
            return Ok(branches);
        }
    }

    while let Some(line) = lines.next() {
        if is_matrix_end(line) {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }
        let values = parse_row(trimmed);
        if values.len() >= 13 {
            branches.push(parse_branch_row(&values, row_idx)?);
            row_idx += 1;
        }
    }
    Ok(branches)
}

fn parse_branch_row(values: &[f64], row_idx: usize) -> Result<MatpowerBranch> {
    Ok(MatpowerBranch {
        f_bus: safe_f64_to_usize(values[0])
            .with_context(|| format!("invalid f_bus at row {}", row_idx))?,
        t_bus: safe_f64_to_usize(values[1])
            .with_context(|| format!("invalid t_bus at row {}", row_idx))?,
        br_r: values[2],
        br_x: values[3],
        br_b: values[4],
        rate_a: values[5],
        rate_b: values[6],
        rate_c: values[7],
        tap: values[8],
        shift: values[9],
        br_status: safe_f64_to_i32(values[10])
            .with_context(|| format!("invalid br_status at row {}", row_idx))?,
        angmin: values[11],
        angmax: values[12],
    })
}

/// Parse gencost section from iterator (single-pass)
fn parse_gencost_section<'a>(
    header: &str,
    lines: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> Result<Vec<MatpowerGenCost>> {
    let mut gencosts = Vec::new();
    let mut row_idx = 0;

    // Check if data starts on the header line
    if let Some(after_bracket) = header.split('[').nth(1) {
        let data_part = after_bracket.trim_end_matches("];").trim();
        if !data_part.is_empty() && !data_part.starts_with('%') {
            let values = parse_row(data_part);
            if values.len() >= 4 {
                gencosts.push(parse_gencost_row(&values, row_idx)?);
                row_idx += 1;
            }
        }
        if header.contains("];") {
            return Ok(gencosts);
        }
    }

    while let Some(line) = lines.next() {
        if is_matrix_end(line) {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }
        let values = parse_row(trimmed);
        if values.len() >= 4 {
            gencosts.push(parse_gencost_row(&values, row_idx)?);
            row_idx += 1;
        }
    }
    Ok(gencosts)
}

fn parse_gencost_row(values: &[f64], row_idx: usize) -> Result<MatpowerGenCost> {
    Ok(MatpowerGenCost {
        model: safe_f64_to_i32(values[0])
            .with_context(|| format!("invalid cost model at row {}", row_idx))?,
        startup: values[1],
        shutdown: values[2],
        ncost: safe_f64_to_i32(values[3])
            .with_context(|| format!("invalid ncost at row {}", row_idx))?,
        cost: values[4..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_case5() {
        let content = r#"
function mpc = case5
mpc.version = '2';
mpc.baseMVA = 100.0;

%% bus data
mpc.bus = [
    1   2   0.0     0.0     0.0   0.0   1   1.0   0.0   230.0   1   1.1   0.9;
    2   1   300.0   98.61   0.0   0.0   1   1.0   0.0   230.0   1   1.1   0.9;
    3   2   300.0   98.61   0.0   0.0   1   1.0   0.0   230.0   1   1.1   0.9;
    4   3   400.0   131.47  0.0   0.0   1   1.0   0.0   230.0   1   1.1   0.9;
    5   2   0.0     0.0     0.0   0.0   1   1.0   0.0   230.0   1   1.1   0.9;
];

%% generator data
mpc.gen = [
    1   20.0   0.0   30.0   -30.0   1.0   100.0   1   40.0    0.0;
    3   260.0  0.0   390.0  -390.0  1.0   100.0   1   520.0   0.0;
    4   100.0  0.0   150.0  -150.0  1.0   100.0   1   200.0   0.0;
];

%% branch data
mpc.branch = [
    1   2   0.00281   0.0281   0.00712   400.0   400.0   400.0   0.0   0.0   1   -30.0   30.0;
    1   4   0.00304   0.0304   0.00658   426     426     426     0.0   0.0   1   -30.0   30.0;
    2   3   0.00108   0.0108   0.01852   426     426     426     0.0   0.0   1   -30.0   30.0;
];

%% generator cost data
mpc.gencost = [
    2   0.0   0.0   3   0.0   14.0   0.0;
    2   0.0   0.0   3   0.0   30.0   0.0;
    2   0.0   0.0   3   0.0   40.0   0.0;
];
"#;

        let case = parse_matpower_string(content).expect("parse matpower string");

        assert_eq!(case.version, "2");
        assert_eq!(case.base_mva, 100.0);
        assert_eq!(case.bus.len(), 5);
        assert_eq!(case.gen.len(), 3);
        assert_eq!(case.branch.len(), 3);
        assert_eq!(case.gencost.len(), 3);

        // Check first bus
        assert_eq!(case.bus[0].bus_i, 1);
        assert_eq!(case.bus[0].bus_type, 2);
        assert_eq!(case.bus[0].base_kv, 230.0);

        // Check first gen
        assert_eq!(case.gen[0].gen_bus, 1);
        assert_eq!(case.gen[0].pg, 20.0);
        assert_eq!(case.gen[0].pmax, 40.0);

        // Check first branch
        assert_eq!(case.branch[0].f_bus, 1);
        assert_eq!(case.branch[0].t_bus, 2);
        assert_eq!(case.branch[0].br_r, 0.00281);
    }

    #[test]
    fn test_reject_negative_bus_id() {
        let content = r#"
mpc.version = '2';
mpc.baseMVA = 100.0;
mpc.bus = [
    -1   2   0.0     0.0     0.0   0.0   1   1.0   0.0   230.0   1   1.1   0.9;
];
"#;
        let result = parse_matpower_string(content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid bus_i") || err.contains("negative"));
    }

    #[test]
    fn test_reject_overflow_bus_id() {
        let content = r#"
mpc.version = '2';
mpc.baseMVA = 100.0;
mpc.bus = [
    9999999999999999999999.0   2   0.0     0.0     0.0   0.0   1   1.0   0.0   230.0   1   1.1   0.9;
];
"#;
        let result = parse_matpower_string(content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid bus_i") || err.contains("exceeds"));
    }

    #[test]
    fn test_reject_negative_branch_bus() {
        let content = r#"
mpc.version = '2';
mpc.baseMVA = 100.0;
mpc.bus = [
    1   2   0.0   0.0   0.0   0.0   1   1.0   0.0   230.0   1   1.1   0.9;
];
mpc.branch = [
    1   -2   0.00281   0.0281   0.00712   400.0   400.0   400.0   0.0   0.0   1   -30.0   30.0;
];
"#;
        let result = parse_matpower_string(content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid t_bus") || err.contains("negative"));
    }
}
