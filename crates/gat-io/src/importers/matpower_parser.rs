//! MATPOWER .m file parser
//!
//! Parses MATPOWER case files in .m (MATLAB) format.
//! Supports the standard MATPOWER case format with mpc.bus, mpc.gen, mpc.branch, etc.

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;

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

/// Parse MATPOWER content from a string
pub fn parse_matpower_string(content: &str) -> Result<MatpowerCase> {
    let mut case = MatpowerCase::default();

    // Extract version
    if let Some(version) = extract_string_value(content, "mpc.version") {
        case.version = version;
    }

    // Extract baseMVA
    if let Some(base_mva) = extract_scalar_value(content, "mpc.baseMVA") {
        case.base_mva = base_mva;
    } else {
        case.base_mva = 100.0; // Default
    }

    // Parse matrices
    case.bus = parse_bus_matrix(content)?;
    case.gen = parse_gen_matrix(content)?;
    case.branch = parse_branch_matrix(content)?;
    case.gencost = parse_gencost_matrix(content).unwrap_or_default();

    Ok(case)
}

fn extract_string_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) && line.contains('=') {
            let value_part = line.split('=').nth(1)?;
            let value = value_part
                .trim()
                .trim_matches(|c| c == '\'' || c == '"' || c == ';');
            return Some(value.to_string());
        }
    }
    None
}

fn extract_scalar_value(content: &str, key: &str) -> Option<f64> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) && line.contains('=') {
            let value_part = line.split('=').nth(1)?;
            let value_str = value_part.trim().trim_end_matches(';');
            return value_str.parse().ok();
        }
    }
    None
}

/// Extract matrix content between '[' and '];'
fn extract_matrix(content: &str, key: &str) -> Option<String> {
    let pattern = format!("{} = [", key);
    let start = content.find(&pattern)?;
    let matrix_start = content[start..].find('[')? + start + 1;
    let matrix_end = content[matrix_start..].find("];")? + matrix_start;
    Some(content[matrix_start..matrix_end].to_string())
}

/// Parse a row of numeric values from MATPOWER format
fn parse_row(line: &str) -> Vec<f64> {
    line.split(|c: char| c.is_whitespace() || c == ';' || c == '\t')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

fn parse_bus_matrix(content: &str) -> Result<Vec<MatpowerBus>> {
    let matrix =
        extract_matrix(content, "mpc.bus").ok_or_else(|| anyhow!("mpc.bus matrix not found"))?;

    let mut buses = Vec::new();
    for line in matrix.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('%') {
            continue;
        }
        let values = parse_row(line);
        if values.len() >= 13 {
            buses.push(MatpowerBus {
                bus_i: values[0] as usize,
                bus_type: values[1] as i32,
                pd: values[2],
                qd: values[3],
                gs: values[4],
                bs: values[5],
                area: values[6] as i32,
                vm: values[7],
                va: values[8],
                base_kv: values[9],
                zone: values[10] as i32,
                vmax: values[11],
                vmin: values[12],
            });
        }
    }
    Ok(buses)
}

fn parse_gen_matrix(content: &str) -> Result<Vec<MatpowerGen>> {
    let matrix = match extract_matrix(content, "mpc.gen") {
        Some(m) => m,
        None => return Ok(Vec::new()),
    };

    let mut gens = Vec::new();
    for line in matrix.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('%') {
            continue;
        }
        let values = parse_row(line);
        if values.len() >= 10 {
            gens.push(MatpowerGen {
                gen_bus: values[0] as usize,
                pg: values[1],
                qg: values[2],
                qmax: values[3],
                qmin: values[4],
                vg: values[5],
                mbase: values[6],
                gen_status: values[7] as i32,
                pmax: values[8],
                pmin: values[9],
            });
        }
    }
    Ok(gens)
}

fn parse_branch_matrix(content: &str) -> Result<Vec<MatpowerBranch>> {
    let matrix = match extract_matrix(content, "mpc.branch") {
        Some(m) => m,
        None => return Ok(Vec::new()),
    };

    let mut branches = Vec::new();
    for line in matrix.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('%') {
            continue;
        }
        let values = parse_row(line);
        if values.len() >= 13 {
            branches.push(MatpowerBranch {
                f_bus: values[0] as usize,
                t_bus: values[1] as usize,
                br_r: values[2],
                br_x: values[3],
                br_b: values[4],
                rate_a: values[5],
                rate_b: values[6],
                rate_c: values[7],
                tap: values[8],
                shift: values[9],
                br_status: values[10] as i32,
                angmin: values[11],
                angmax: values[12],
            });
        }
    }
    Ok(branches)
}

fn parse_gencost_matrix(content: &str) -> Result<Vec<MatpowerGenCost>> {
    let matrix = match extract_matrix(content, "mpc.gencost") {
        Some(m) => m,
        None => return Ok(Vec::new()),
    };

    let mut gencosts = Vec::new();
    for line in matrix.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('%') {
            continue;
        }
        let values = parse_row(line);
        if values.len() >= 4 {
            let model = values[0] as i32;
            let startup = values[1];
            let shutdown = values[2];
            let ncost = values[3] as i32;
            let cost: Vec<f64> = values[4..].to_vec();
            gencosts.push(MatpowerGenCost {
                model,
                startup,
                shutdown,
                ncost,
                cost,
            });
        }
    }
    Ok(gencosts)
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

        let case = parse_matpower_string(content).unwrap();

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
}
