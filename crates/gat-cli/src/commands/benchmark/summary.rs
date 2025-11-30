//! Benchmark summary and comparison utilities.
//!
//! Provides `gat benchmark summary` and `gat benchmark compare` commands for
//! analyzing benchmark results.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// A single benchmark result row from CSV
#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkRow {
    pub case_name: String,
    pub load_time_ms: f64,
    pub solve_time_ms: f64,
    pub total_time_ms: f64,
    pub converged: bool,
    pub iterations: u32,
    pub num_buses: usize,
    pub num_branches: usize,
    pub num_gens: usize,
    pub objective_value: f64,
    pub baseline_objective: f64,
    pub objective_gap_abs: f64,
    pub objective_gap_rel: f64,
    pub max_vm_violation_pu: f64,
    pub max_gen_p_violation_mw: f64,
    pub max_branch_flow_violation_mva: f64,
}

/// Aggregated statistics for a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkStats {
    pub total_cases: usize,
    pub converged_cases: usize,
    pub pass_rate: f64,
    pub avg_solve_time_ms: f64,
    pub max_solve_time_ms: f64,
    pub slowest_case: String,
    pub avg_objective_gap_rel: f64,
    pub max_objective_gap_rel: f64,
    pub worst_gap_case: String,
}

/// Load benchmark results from a CSV file
pub fn load_benchmark_csv(path: &Path) -> Result<Vec<BenchmarkRow>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open benchmark CSV: {}", path.display()))?;
    let mut reader = csv::Reader::from_reader(file);
    let mut rows = Vec::new();
    for result in reader.deserialize() {
        let row: BenchmarkRow = result.context("Failed to parse benchmark row")?;
        rows.push(row);
    }
    Ok(rows)
}

/// Compute aggregate statistics from benchmark results
pub fn compute_stats(rows: &[BenchmarkRow]) -> BenchmarkStats {
    let total_cases = rows.len();
    let converged_cases = rows.iter().filter(|r| r.converged).count();
    let pass_rate = if total_cases > 0 {
        converged_cases as f64 / total_cases as f64
    } else {
        0.0
    };

    // Compute timing stats (only for converged cases)
    let converged_rows: Vec<_> = rows.iter().filter(|r| r.converged).collect();
    let (avg_solve_time_ms, max_solve_time_ms, slowest_case) = if !converged_rows.is_empty() {
        let sum: f64 = converged_rows.iter().map(|r| r.solve_time_ms).sum();
        let avg = sum / converged_rows.len() as f64;
        let (max, slowest) = converged_rows
            .iter()
            .map(|r| (r.solve_time_ms, r.case_name.clone()))
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap_or((0.0, String::new()));
        (avg, max, slowest)
    } else {
        (0.0, 0.0, String::new())
    };

    // Compute gap stats (only for converged cases with baseline)
    let with_baseline: Vec<_> = converged_rows
        .iter()
        .filter(|r| r.baseline_objective > 0.0)
        .collect();
    let (avg_objective_gap_rel, max_objective_gap_rel, worst_gap_case) = if !with_baseline.is_empty()
    {
        let sum: f64 = with_baseline.iter().map(|r| r.objective_gap_rel).sum();
        let avg = sum / with_baseline.len() as f64;
        let (max, worst) = with_baseline
            .iter()
            .map(|r| (r.objective_gap_rel, r.case_name.clone()))
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap_or((0.0, String::new()));
        (avg, max, worst)
    } else {
        (0.0, 0.0, String::new())
    };

    BenchmarkStats {
        total_cases,
        converged_cases,
        pass_rate,
        avg_solve_time_ms,
        max_solve_time_ms,
        slowest_case,
        avg_objective_gap_rel,
        max_objective_gap_rel,
        worst_gap_case,
    }
}

/// Format benchmark results as a pretty table
pub fn format_summary_table(rows: &[BenchmarkRow], stats: &BenchmarkStats) -> String {
    let mut output = String::new();

    // Header
    output.push_str("╭──────────────────────────────────────────────────────────────────────────────╮\n");
    output.push_str("│ Benchmark Summary                                                            │\n");
    output.push_str("├──────────────────────────────────────────────────────────────────────────────┤\n");
    output.push_str(&format!(
        "│ Pass Rate: {}/{} ({:.1}%)                                                      │\n",
        stats.converged_cases,
        stats.total_cases,
        stats.pass_rate * 100.0
    ));
    output.push_str(&format!(
        "│ Avg Solve Time: {:.1}ms  │  Max: {:.1}ms ({})                  │\n",
        stats.avg_solve_time_ms, stats.max_solve_time_ms, stats.slowest_case
    ));
    if stats.avg_objective_gap_rel > 0.0 {
        output.push_str(&format!(
            "│ Avg Gap to Baseline: {:.2}%  │  Max: {:.2}% ({})             │\n",
            stats.avg_objective_gap_rel * 100.0,
            stats.max_objective_gap_rel * 100.0,
            stats.worst_gap_case
        ));
    }
    output.push_str("├──────────────────────────────────────────────────────────────────────────────┤\n");

    // Table header
    output.push_str("│ Case                  │ Status │ Time(ms) │ Gap(%) │ Vm Viol │ Branch Viol  │\n");
    output.push_str("├───────────────────────┼────────┼──────────┼────────┼─────────┼──────────────┤\n");

    // Rows
    for row in rows {
        let status = if row.converged { "  ✓   " } else { "  ✗   " };
        let time_str = if row.converged {
            format!("{:8.1}", row.solve_time_ms)
        } else {
            " (fail) ".to_string()
        };
        let gap_str = if row.converged && row.baseline_objective > 0.0 {
            format!("{:6.2}%", row.objective_gap_rel * 100.0)
        } else {
            "   --  ".to_string()
        };
        let vm_str = if row.converged {
            format!("{:7.3}", row.max_vm_violation_pu)
        } else {
            "   --  ".to_string()
        };
        let branch_str = if row.converged {
            format!("{:12.3}", row.max_branch_flow_violation_mva)
        } else {
            "     --     ".to_string()
        };

        // Truncate case name to 21 chars
        let case_display = if row.case_name.len() > 21 {
            format!("{}…", &row.case_name[..20])
        } else {
            format!("{:<21}", row.case_name)
        };

        output.push_str(&format!(
            "│ {} │{}│{}│{}│{}│{}│\n",
            case_display, status, time_str, gap_str, vm_str, branch_str
        ));
    }

    output.push_str("╰──────────────────────────────────────────────────────────────────────────────╯\n");
    output
}

/// Handle the `gat benchmark summary` command
pub fn handle(csv_path: &str) -> Result<()> {
    let path = Path::new(csv_path);
    let rows = load_benchmark_csv(path)?;
    let stats = compute_stats(&rows);
    let table = format_summary_table(&rows, &stats);
    println!("{}", table);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CSV: &str = r#"case_name,load_time_ms,solve_time_ms,total_time_ms,converged,iterations,num_buses,num_branches,num_gens,objective_value,baseline_objective,objective_gap_abs,objective_gap_rel,max_vm_violation_pu,max_gen_p_violation_mw,max_branch_flow_violation_mva
pglib_opf_case14_ieee,0.42,67.0,67.42,true,0,14,20,5,2169.56,2178.1,8.54,0.0039,0.04,0.0,0.0
pglib_opf_case24_ieee_rts,0.77,130.69,131.46,true,0,24,38,33,63706.05,63352.0,354.05,0.0056,0.038,0.0,0.0
pglib_opf_case30_ieee,0.48,86.5,86.98,true,0,30,41,6,8169.04,8208.5,39.46,0.0048,0.04,0.0,0.0
pglib_opf_case118_ieee,1.2,500.0,501.2,false,200,118,186,54,0.0,0.0,0.0,0.0,0.0,0.0,0.0"#;

    fn parse_test_csv() -> Vec<BenchmarkRow> {
        let mut reader = csv::Reader::from_reader(TEST_CSV.as_bytes());
        reader.deserialize().map(|r| r.unwrap()).collect()
    }

    #[test]
    fn test_parse_benchmark_csv() {
        let rows = parse_test_csv();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].case_name, "pglib_opf_case14_ieee");
        assert!(rows[0].converged);
        assert!(!rows[3].converged);
    }

    #[test]
    fn test_compute_stats_pass_rate() {
        let rows = parse_test_csv();
        let stats = compute_stats(&rows);

        // 3 of 4 cases converged
        assert_eq!(stats.total_cases, 4);
        assert_eq!(stats.converged_cases, 3);
        assert!((stats.pass_rate - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_compute_stats_timing() {
        let rows = parse_test_csv();
        let stats = compute_stats(&rows);

        // Avg of converged: (67 + 130.69 + 86.5) / 3 = 94.73
        assert!((stats.avg_solve_time_ms - 94.73).abs() < 0.1);
        // Max is case24 at 130.69
        assert!((stats.max_solve_time_ms - 130.69).abs() < 0.1);
        assert_eq!(stats.slowest_case, "pglib_opf_case24_ieee_rts");
    }

    #[test]
    fn test_compute_stats_gap() {
        let rows = parse_test_csv();
        let stats = compute_stats(&rows);

        // All converged cases have baseline, avg gap: (0.0039 + 0.0056 + 0.0048) / 3 = 0.00477
        assert!((stats.avg_objective_gap_rel - 0.00477).abs() < 0.0001);
        // Max gap is case24 at 0.0056
        assert!((stats.max_objective_gap_rel - 0.0056).abs() < 0.0001);
        assert_eq!(stats.worst_gap_case, "pglib_opf_case24_ieee_rts");
    }

    #[test]
    fn test_format_table_contains_key_elements() {
        let rows = parse_test_csv();
        let stats = compute_stats(&rows);
        let table = format_summary_table(&rows, &stats);

        // Check header elements
        assert!(table.contains("Benchmark Summary"));
        assert!(table.contains("Pass Rate: 3/4"));
        assert!(table.contains("75.0%"));

        // Check case rows are present (using partial match since names may be truncated)
        assert!(table.contains("case14"));
        assert!(table.contains("case118"));

        // Check status symbols
        assert!(table.contains("✓")); // converged
        assert!(table.contains("✗")); // failed
    }
}
