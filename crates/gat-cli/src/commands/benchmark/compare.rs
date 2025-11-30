//! Benchmark comparison utilities.
//!
//! Provides `gat benchmark compare` command for comparing two benchmark runs.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use super::summary::{load_benchmark_csv, BenchmarkRow};

/// Status change between two benchmark runs
#[derive(Debug, Clone, PartialEq)]
pub enum StatusChange {
    /// Case now passes (was failing)
    NowPasses,
    /// Case now fails (was passing)
    NowFails,
    /// No status change
    NoChange,
}

/// Comparison result for a single case
#[derive(Debug, Clone)]
pub struct CaseComparison {
    pub case_name: String,
    pub status_change: StatusChange,
    pub before_converged: bool,
    pub after_converged: bool,
    pub solve_time_delta_ms: Option<f64>,
    pub solve_time_delta_pct: Option<f64>,
    pub objective_delta: Option<f64>,
    pub objective_delta_pct: Option<f64>,
}

/// Aggregate comparison statistics
#[derive(Debug, Clone)]
pub struct ComparisonStats {
    pub before_pass_rate: f64,
    pub after_pass_rate: f64,
    pub cases_now_passing: Vec<String>,
    pub cases_now_failing: Vec<String>,
    pub avg_solve_time_delta_pct: Option<f64>,
}

/// Compare two benchmark runs
pub fn compare_benchmarks(
    before: &[BenchmarkRow],
    after: &[BenchmarkRow],
) -> (Vec<CaseComparison>, ComparisonStats) {
    let before_map: HashMap<_, _> = before.iter().map(|r| (r.case_name.clone(), r)).collect();
    let after_map: HashMap<_, _> = after.iter().map(|r| (r.case_name.clone(), r)).collect();

    let mut comparisons = Vec::new();
    let mut cases_now_passing = Vec::new();
    let mut cases_now_failing = Vec::new();
    let mut time_deltas = Vec::new();

    // Compare cases present in both
    for (name, after_row) in &after_map {
        if let Some(before_row) = before_map.get(name) {
            let status_change = match (before_row.converged, after_row.converged) {
                (false, true) => {
                    cases_now_passing.push(name.clone());
                    StatusChange::NowPasses
                }
                (true, false) => {
                    cases_now_failing.push(name.clone());
                    StatusChange::NowFails
                }
                _ => StatusChange::NoChange,
            };

            let (solve_time_delta_ms, solve_time_delta_pct) =
                if before_row.converged && after_row.converged {
                    let delta = after_row.solve_time_ms - before_row.solve_time_ms;
                    let pct = if before_row.solve_time_ms > 0.0 {
                        delta / before_row.solve_time_ms * 100.0
                    } else {
                        0.0
                    };
                    time_deltas.push(pct);
                    (Some(delta), Some(pct))
                } else {
                    (None, None)
                };

            let (objective_delta, objective_delta_pct) =
                if before_row.converged && after_row.converged {
                    let delta = after_row.objective_value - before_row.objective_value;
                    let pct = if before_row.objective_value.abs() > 1e-6 {
                        delta / before_row.objective_value * 100.0
                    } else {
                        0.0
                    };
                    (Some(delta), Some(pct))
                } else {
                    (None, None)
                };

            comparisons.push(CaseComparison {
                case_name: name.clone(),
                status_change,
                before_converged: before_row.converged,
                after_converged: after_row.converged,
                solve_time_delta_ms,
                solve_time_delta_pct,
                objective_delta,
                objective_delta_pct,
            });
        }
    }

    // Sort by case name
    comparisons.sort_by(|a, b| a.case_name.cmp(&b.case_name));
    cases_now_passing.sort();
    cases_now_failing.sort();

    // Compute aggregate stats
    let before_converged = before.iter().filter(|r| r.converged).count();
    let after_converged = after.iter().filter(|r| r.converged).count();
    let before_pass_rate = if !before.is_empty() {
        before_converged as f64 / before.len() as f64
    } else {
        0.0
    };
    let after_pass_rate = if !after.is_empty() {
        after_converged as f64 / after.len() as f64
    } else {
        0.0
    };

    let avg_solve_time_delta_pct = if !time_deltas.is_empty() {
        Some(time_deltas.iter().sum::<f64>() / time_deltas.len() as f64)
    } else {
        None
    };

    let stats = ComparisonStats {
        before_pass_rate,
        after_pass_rate,
        cases_now_passing,
        cases_now_failing,
        avg_solve_time_delta_pct,
    };

    (comparisons, stats)
}

/// Format comparison as a pretty table
pub fn format_comparison_table(
    comparisons: &[CaseComparison],
    stats: &ComparisonStats,
    before_name: &str,
    after_name: &str,
) -> String {
    let mut output = String::new();

    output.push_str(
        "╭─────────────────────────────────────────────────────────────────────────────╮\n",
    );
    output.push_str(&format!(
        "│ Benchmark Comparison: {} → {}",
        truncate(before_name, 25),
        truncate(after_name, 25)
    ));
    output.push_str(&" ".repeat(77 - 26 - before_name.len().min(25) - after_name.len().min(25)));
    output.push_str("│\n");
    output.push_str(
        "├─────────────────────────────────────────────────────────────────────────────┤\n",
    );

    // Pass rate change
    output.push_str(&format!(
        "│ Pass Rate: {:.1}% → {:.1}% ({:+.1}%)                                              │\n",
        stats.before_pass_rate * 100.0,
        stats.after_pass_rate * 100.0,
        (stats.after_pass_rate - stats.before_pass_rate) * 100.0
    ));

    // Status changes
    if !stats.cases_now_passing.is_empty() {
        output.push_str(
            "│ Status Changes:                                                             │\n",
        );
        for case in &stats.cases_now_passing {
            output.push_str(&format!(
                "│   ✗→✓  {} (now converges!)                         │\n",
                truncate(case, 40)
            ));
        }
    }
    if !stats.cases_now_failing.is_empty() {
        if stats.cases_now_passing.is_empty() {
            output.push_str(
                "│ Status Changes:                                                             │\n",
            );
        }
        for case in &stats.cases_now_failing {
            output.push_str(&format!(
                "│   ✓→✗  {} (regression!)                            │\n",
                truncate(case, 40)
            ));
        }
    }

    // Performance summary
    if let Some(avg_delta) = stats.avg_solve_time_delta_pct {
        let direction = if avg_delta < 0.0 { "faster" } else { "slower" };
        output.push_str(&format!(
            "│ Avg Performance: {:+.1}% ({})                                              │\n",
            avg_delta, direction
        ));
    }

    output.push_str(
        "╰─────────────────────────────────────────────────────────────────────────────╯\n",
    );
    output
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}…", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}

/// Handle the `gat benchmark compare` command
pub fn handle(before_csv: &str, after_csv: &str) -> Result<()> {
    let before = load_benchmark_csv(Path::new(before_csv))?;
    let after = load_benchmark_csv(Path::new(after_csv))?;

    let (comparisons, stats) = compare_benchmarks(&before, &after);

    // Extract filenames for display
    let before_name = Path::new(before_csv)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| before_csv.to_string());
    let after_name = Path::new(after_csv)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| after_csv.to_string());

    let table = format_comparison_table(&comparisons, &stats, &before_name, &after_name);
    println!("{}", table);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(name: &str, converged: bool, solve_time: f64, objective: f64) -> BenchmarkRow {
        BenchmarkRow {
            case_name: name.to_string(),
            load_time_ms: 1.0,
            solve_time_ms: solve_time,
            total_time_ms: solve_time + 1.0,
            converged,
            iterations: 10,
            num_buses: 14,
            num_branches: 20,
            num_gens: 5,
            objective_value: objective,
            baseline_objective: objective,
            objective_gap_abs: 0.0,
            objective_gap_rel: 0.0,
            max_vm_violation_pu: 0.0,
            max_gen_p_violation_mw: 0.0,
            max_branch_flow_violation_mva: 0.0,
        }
    }

    #[test]
    fn test_compare_detects_new_pass() {
        let before = vec![
            make_row("case14", false, 0.0, 0.0), // was failing
            make_row("case30", true, 100.0, 1000.0),
        ];
        let after = vec![
            make_row("case14", true, 50.0, 500.0), // now passes
            make_row("case30", true, 90.0, 1000.0),
        ];

        let (comparisons, stats) = compare_benchmarks(&before, &after);

        assert_eq!(stats.cases_now_passing, vec!["case14"]);
        assert!(stats.cases_now_failing.is_empty());

        let case14 = comparisons
            .iter()
            .find(|c| c.case_name == "case14")
            .unwrap();
        assert_eq!(case14.status_change, StatusChange::NowPasses);
    }

    #[test]
    fn test_compare_detects_regression() {
        let before = vec![
            make_row("case14", true, 50.0, 500.0), // was passing
        ];
        let after = vec![
            make_row("case14", false, 0.0, 0.0), // now fails
        ];

        let (_, stats) = compare_benchmarks(&before, &after);

        assert!(stats.cases_now_passing.is_empty());
        assert_eq!(stats.cases_now_failing, vec!["case14"]);
    }

    #[test]
    fn test_compare_computes_time_delta() {
        let before = vec![make_row("case14", true, 100.0, 1000.0)];
        let after = vec![make_row("case14", true, 80.0, 1000.0)]; // 20% faster

        let (comparisons, stats) = compare_benchmarks(&before, &after);

        let case14 = comparisons
            .iter()
            .find(|c| c.case_name == "case14")
            .unwrap();
        assert!((case14.solve_time_delta_ms.unwrap() - (-20.0)).abs() < 0.1);
        assert!((case14.solve_time_delta_pct.unwrap() - (-20.0)).abs() < 0.1);

        assert!((stats.avg_solve_time_delta_pct.unwrap() - (-20.0)).abs() < 0.1);
    }

    #[test]
    fn test_compare_pass_rate() {
        let before = vec![
            make_row("case14", true, 50.0, 500.0),
            make_row("case30", false, 0.0, 0.0),
        ];
        let after = vec![
            make_row("case14", true, 50.0, 500.0),
            make_row("case30", true, 100.0, 1000.0),
        ];

        let (_, stats) = compare_benchmarks(&before, &after);

        assert!((stats.before_pass_rate - 0.5).abs() < 0.001);
        assert!((stats.after_pass_rate - 1.0).abs() < 0.001);
    }
}
