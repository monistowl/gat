use anyhow::{anyhow, Context, Result};
use csv::Writer;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

use gat_algo::validation::ObjectiveGap;
use gat_algo::AcOpfSolver;
use gat_io::importers::load_matpower_network;

use super::baseline::{load_baseline_objectives, normalize_case_name};

/// Benchmark result for a single PGLib case
#[derive(Debug, Clone, Serialize)]
struct PglibBenchmarkResult {
    case_name: String,
    load_time_ms: f64,
    solve_time_ms: f64,
    total_time_ms: f64,
    converged: bool,
    iterations: u32,
    num_buses: usize,
    num_branches: usize,
    num_gens: usize,
    objective_value: f64,
    // Baseline comparison (if available)
    baseline_objective: f64,
    objective_gap_abs: f64,
    objective_gap_rel: f64,
}

/// Configuration for PGLib benchmark runs
#[derive(Debug)]
struct BenchmarkConfig {
    pglib_dir: String,
    baseline: Option<String>,
    case_filter: Option<String>,
    max_cases: usize,
    out: String,
    threads: String,
    tol: f64,
    max_iter: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn handle(
    pglib_dir: &str,
    baseline: Option<&str>,
    case_filter: Option<&str>,
    max_cases: usize,
    out: &str,
    threads: &str,
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    let config = BenchmarkConfig {
        pglib_dir: pglib_dir.to_string(),
        baseline: baseline.map(|s| s.to_string()),
        case_filter: case_filter.map(|s| s.to_string()),
        max_cases,
        out: out.to_string(),
        threads: threads.to_string(),
        tol,
        max_iter,
    };

    run_benchmark(&config)
}

fn run_benchmark(config: &BenchmarkConfig) -> Result<()> {
    // Configure threads
    if config.threads != "auto" {
        if let Ok(n) = config.threads.parse::<usize>() {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build_global()
                .ok();
        }
    }

    // Discover MATPOWER files
    let pglib_path = Path::new(&config.pglib_dir);
    if !pglib_path.exists() {
        return Err(anyhow!(
            "PGLib directory not found: {}",
            config.pglib_dir
        ));
    }

    let mut matpower_files = discover_matpower_files(pglib_path)?;

    // Filter by case name if specified
    if let Some(filter) = &config.case_filter {
        matpower_files.retain(|(name, _)| name.contains(filter));
    }

    // Limit number of cases
    let limit = if config.max_cases > 0 {
        config.max_cases
    } else {
        matpower_files.len()
    };
    matpower_files.truncate(limit);

    eprintln!(
        "Found {} MATPOWER cases to benchmark (filter={:?}, max_cases={})",
        matpower_files.len(),
        config.case_filter,
        config.max_cases
    );

    // Load baseline if provided
    let baseline_map: HashMap<String, f64> = if let Some(baseline_path) = &config.baseline {
        load_baseline_objectives(Path::new(baseline_path))?
    } else {
        HashMap::new()
    };

    // Run benchmarks in parallel
    let tol = config.tol;
    let max_iter = config.max_iter;

    let results: Vec<PglibBenchmarkResult> = matpower_files
        .par_iter()
        .filter_map(|(case_name, path)| {
            match benchmark_pglib_case(case_name, path, &baseline_map, tol, max_iter) {
                Ok(result) => Some(result),
                Err(e) => {
                    eprintln!("Error benchmarking {}: {}", case_name, e);
                    None
                }
            }
        })
        .collect();

    // Write results to CSV
    let out_path = Path::new(&config.out);
    if let Some(parent) = out_path.parent() {
        if parent != Path::new("") {
            std::fs::create_dir_all(parent).ok();
        }
    }

    let file = File::create(out_path)
        .context(format!("Failed to create output file: {}", config.out))?;
    let mut writer = Writer::from_writer(file);

    for result in &results {
        writer
            .serialize(result)
            .context("Failed to write result to CSV")?;
    }

    writer.flush().context("Failed to flush CSV writer")?;

    // Print summary
    let converged_count = results.iter().filter(|r| r.converged).count();
    let avg_time: f64 = if !results.is_empty() {
        results.iter().map(|r| r.solve_time_ms).sum::<f64>() / results.len() as f64
    } else {
        0.0
    };

    let avg_gap: f64 = if !results.is_empty() {
        results
            .iter()
            .filter(|r| r.baseline_objective > 0.0)
            .map(|r| r.objective_gap_rel)
            .sum::<f64>()
            / results
                .iter()
                .filter(|r| r.baseline_objective > 0.0)
                .count()
                .max(1) as f64
    } else {
        0.0
    };

    eprintln!(
        "\nBenchmark Results:\n  Total cases: {}\n  Converged: {}\n  Avg solve time: {:.2}ms\n  Avg obj gap: {:.4}%\n  Output: {}",
        results.len(),
        converged_count,
        avg_time,
        avg_gap * 100.0,
        config.out
    );

    Ok(())
}

/// Discover MATPOWER case directories in a PGLib directory
///
/// Looks for directories that contain .m files (the format expected by caseformat)
fn discover_matpower_files(dir: &Path) -> Result<Vec<(String, std::path::PathBuf)>> {
    let mut cases = Vec::new();

    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("reading PGLib directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        // Look for directories that contain .m files
        if path.is_dir() {
            // Check if directory contains any .m files
            let has_m_files = std::fs::read_dir(&path)
                .ok()
                .map(|entries| {
                    entries.filter_map(|e| e.ok()).any(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "m")
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);

            if has_m_files {
                let case_name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                cases.push((case_name, path));
            }
        }
    }

    // Sort by name for deterministic order
    cases.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(cases)
}

fn benchmark_pglib_case(
    case_name: &str,
    path: &Path,
    baseline_map: &HashMap<String, f64>,
    tol: f64,
    max_iter: u32,
) -> Result<PglibBenchmarkResult> {
    let load_start = Instant::now();
    let network = load_matpower_network(path)?;
    let load_time_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    // Count network elements
    let mut num_buses = 0;
    let mut num_gens = 0;
    for node in network.graph.node_weights() {
        match node {
            gat_core::Node::Bus(_) => num_buses += 1,
            gat_core::Node::Gen(_) => num_gens += 1,
            gat_core::Node::Load(_) => {}
        }
    }
    let num_branches = network.graph.edge_count();

    // Solve
    let solver = AcOpfSolver::new()
        .with_max_iterations(max_iter as usize)
        .with_tolerance(tol);

    let solve_start = Instant::now();
    let solution = solver.solve(&network)?;
    let solve_time_ms = solve_start.elapsed().as_secs_f64() * 1000.0;

    // Look up baseline
    let normalized_name = normalize_case_name(case_name);
    let baseline_objective = baseline_map
        .get(&normalized_name)
        .or_else(|| baseline_map.get(case_name))
        .copied()
        .unwrap_or(0.0);

    let gap = if baseline_objective > 0.0 {
        ObjectiveGap::new(solution.objective_value, baseline_objective)
    } else {
        ObjectiveGap::default()
    };

    Ok(PglibBenchmarkResult {
        case_name: case_name.to_string(),
        load_time_ms,
        solve_time_ms,
        total_time_ms: load_time_ms + solve_time_ms,
        converged: solution.converged,
        iterations: solution.iterations as u32,
        num_buses,
        num_branches,
        num_gens,
        objective_value: solution.objective_value,
        baseline_objective,
        objective_gap_abs: gap.gap_abs,
        objective_gap_rel: gap.gap_rel,
    })
}
