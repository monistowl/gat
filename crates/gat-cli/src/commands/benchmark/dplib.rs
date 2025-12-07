//! DPLib Distributed OPF Benchmark
//!
//! Benchmarks ADMM-based distributed optimal power flow against centralized solutions
//! using PGLib test cases. This reproduces experiments for distributed/decomposition
//! approaches to large-scale OPF.
//!
//! The benchmark:
//! 1. Loads PGLib MATPOWER cases
//! 2. Partitions the network into regions using spectral clustering
//! 3. Solves OPF using ADMM with configurable consensus parameters
//! 4. Compares objective value and timing against centralized SOCP
//!
//! Key metrics:
//! - ADMM iterations to convergence
//! - Final primal/dual residuals
//! - Per-partition solve times
//! - Speedup ratio vs centralized solver
//! - Objective gap vs centralized solution

use anyhow::{anyhow, Context, Result};
use csv::Writer;
use rayon::prelude::*;
use serde::Serialize;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

use gat_algo::graph::{partition_network, PartitionStrategy};
use gat_algo::opf::admm::PartitionStrategyConfig;
use gat_algo::opf::{AdmmConfig, AdmmOpfSolver, OpfMethod, OpfSolver};
use gat_io::importers::load_matpower_network;

/// Result for a single DPLib benchmark case
#[derive(Debug, Clone, Serialize)]
pub struct DplibBenchmarkResult {
    /// Test case name
    pub case_name: String,
    /// Number of buses in the network
    pub num_buses: usize,
    /// Number of branches
    pub num_branches: usize,
    /// Number of generators
    pub num_gens: usize,
    /// Number of partitions used
    pub num_partitions: usize,
    /// Number of tie-lines between partitions
    pub num_tie_lines: usize,

    // === Centralized Reference ===
    /// Centralized SOCP solve time (ms)
    pub centralized_time_ms: f64,
    /// Centralized SOCP objective value ($/hr)
    pub centralized_objective: f64,
    /// Whether centralized solve converged
    pub centralized_converged: bool,

    // === ADMM Distributed ===
    /// ADMM total solve time (ms)
    pub admm_time_ms: f64,
    /// ADMM objective value ($/hr)
    pub admm_objective: f64,
    /// Whether ADMM converged
    pub admm_converged: bool,
    /// Number of ADMM iterations
    pub admm_iterations: usize,
    /// Final primal residual
    pub primal_residual: f64,
    /// Final dual residual
    pub dual_residual: f64,

    // === Phase Timing (ADMM) ===
    /// Time spent in x-update phase (ms)
    pub x_update_ms: f64,
    /// Time spent in z-update phase (ms)
    pub z_update_ms: f64,
    /// Time spent in dual variable update phase (ms)
    pub dual_update_ms: f64,

    // === Comparison Metrics ===
    /// Objective gap: (admm - centralized) / centralized
    pub objective_gap_rel: f64,
    /// Speedup ratio: centralized_time / admm_time
    pub speedup_ratio: f64,
}

/// Configuration for DPLib benchmark runs
#[derive(Debug, Clone)]
pub struct DplibConfig {
    /// Directory containing PGLib MATPOWER files
    pub pglib_dir: String,
    /// Filter cases by name pattern
    pub case_filter: Option<String>,
    /// Maximum number of cases to run
    pub max_cases: usize,
    /// Output CSV path
    pub out: String,
    /// Number of threads
    pub threads: String,
    /// Number of partitions (0 = auto based on network size)
    pub num_partitions: usize,
    /// Maximum ADMM iterations
    pub max_iter: usize,
    /// ADMM convergence tolerance
    pub tol: f64,
    /// Initial penalty parameter (ρ)
    pub rho: f64,
    /// Subproblem OPF method (dc, socp)
    pub subproblem_method: OpfMethod,
}

#[allow(clippy::too_many_arguments)]
pub fn handle(
    pglib_dir: &str,
    case_filter: Option<&str>,
    max_cases: usize,
    out: &str,
    threads: &str,
    num_partitions: usize,
    max_iter: usize,
    tol: f64,
    rho: f64,
    subproblem_method: &str,
) -> Result<()> {
    // Parse subproblem method
    let method: OpfMethod = subproblem_method
        .parse()
        .map_err(|e| anyhow!("Invalid subproblem method: {}", e))?;

    let config = DplibConfig {
        pglib_dir: pglib_dir.to_string(),
        case_filter: case_filter.map(|s| s.to_string()),
        max_cases,
        out: out.to_string(),
        threads: threads.to_string(),
        num_partitions,
        max_iter,
        tol,
        rho,
        subproblem_method: method,
    };

    eprintln!("DPLib Distributed OPF Benchmark");
    eprintln!("================================");
    eprintln!("PGLib directory: {}", config.pglib_dir);
    eprintln!("Partitions: {}", if num_partitions == 0 { "auto".to_string() } else { num_partitions.to_string() });
    eprintln!("Max ADMM iterations: {}", config.max_iter);
    eprintln!("Tolerance: {:.2e}", config.tol);
    eprintln!("Initial ρ: {:.1}", config.rho);
    eprintln!("Subproblem method: {}", config.subproblem_method);
    eprintln!();

    run_benchmark(&config)
}

fn run_benchmark(config: &DplibConfig) -> Result<()> {
    // Configure thread pool
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
        return Err(anyhow!("PGLib directory not found: {}", config.pglib_dir));
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
        "Found {} MATPOWER cases to benchmark",
        matpower_files.len()
    );

    // Run benchmarks
    let results: Vec<DplibBenchmarkResult> = matpower_files
        .par_iter()
        .filter_map(|(case_name, path)| {
            match benchmark_dplib_case(case_name, path, config) {
                Ok(result) => Some(result),
                Err(e) => {
                    eprintln!("Error benchmarking {}: {}", case_name, e);
                    None
                }
            }
        })
        .collect();

    // Write results to CSV
    write_results(&config.out, &results)?;

    // Print summary
    print_summary(&results);

    Ok(())
}

fn discover_matpower_files(dir: &Path) -> Result<Vec<(String, std::path::PathBuf)>> {
    let mut cases = Vec::new();

    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("reading PGLib directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let has_m_files = std::fs::read_dir(&path)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .any(|e| e.path().extension().map(|ext| ext == "m").unwrap_or(false))
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

fn benchmark_dplib_case(
    case_name: &str,
    path: &Path,
    config: &DplibConfig,
) -> Result<DplibBenchmarkResult> {
    // Load network
    let network = load_matpower_network(path)?;

    // Count network elements
    let mut num_buses = 0;
    let mut num_gens = 0;
    for node in network.graph.node_weights() {
        match node {
            gat_core::Node::Bus(_) => num_buses += 1,
            gat_core::Node::Gen(_) => num_gens += 1,
            _ => {}
        }
    }
    let num_branches = network.graph.edge_count();

    // Determine number of partitions (auto = sqrt(buses) clamped to [2, 16])
    let num_partitions = if config.num_partitions == 0 {
        let auto = (num_buses as f64).sqrt().round() as usize;
        auto.clamp(2, 16)
    } else {
        config.num_partitions
    };

    // Skip cases too small to partition meaningfully
    if num_buses < num_partitions * 3 {
        return Err(anyhow!(
            "Network too small ({} buses) for {} partitions",
            num_buses,
            num_partitions
        ));
    }

    // === Run centralized SOCP first ===
    let centralized_start = Instant::now();
    let centralized_solver = OpfSolver::new()
        .with_method(OpfMethod::SocpRelaxation)
        .with_max_iterations(200)
        .with_tolerance(1e-6);

    let centralized_result = centralized_solver.solve(&network);
    let centralized_time_ms = centralized_start.elapsed().as_secs_f64() * 1000.0;

    let (centralized_converged, centralized_objective) = match &centralized_result {
        Ok(sol) => (sol.converged, sol.objective_value),
        Err(_) => (false, 0.0),
    };

    // === Partition the network ===
    let partitions = partition_network(
        &network,
        PartitionStrategy::Spectral { num_partitions },
    )?;

    let num_tie_lines: usize = partitions.iter().map(|p| p.tie_lines.len()).sum::<usize>() / 2;

    // === Run ADMM distributed OPF ===
    let admm_config = AdmmConfig {
        penalty: config.rho,
        primal_tol: config.tol,
        dual_tol: config.tol,
        max_iter: config.max_iter,
        inner_method: config.subproblem_method,
        num_partitions,
        partition_strategy: PartitionStrategyConfig::Spectral,
        adaptive_penalty: true,
        penalty_scale: 2.0,
        max_penalty: 1e6,
        min_penalty: 1e-6,
        verbose: false,
        use_gpu: false, // GPU acceleration not used in benchmarks
    };

    let admm_solver = AdmmOpfSolver::new(admm_config);

    let admm_start = Instant::now();
    let admm_result = admm_solver.solve(&network);
    let admm_time_ms = admm_start.elapsed().as_secs_f64() * 1000.0;

    let (admm_converged, admm_objective, admm_iterations, primal_residual, dual_residual, phase_times) =
        match &admm_result {
            Ok(sol) => (
                sol.converged,
                sol.objective,
                sol.iterations,
                sol.primal_residual,
                sol.dual_residual,
                sol.phase_times_ms.clone(),
            ),
            Err(_) => (false, 0.0, 0, f64::INFINITY, f64::INFINITY, Default::default()),
        };

    // Compute comparison metrics
    let objective_gap_rel = if centralized_objective > 0.0 {
        (admm_objective - centralized_objective) / centralized_objective
    } else {
        0.0
    };

    let speedup_ratio = if admm_time_ms > 0.0 {
        centralized_time_ms / admm_time_ms
    } else {
        0.0
    };

    Ok(DplibBenchmarkResult {
        case_name: case_name.to_string(),
        num_buses,
        num_branches,
        num_gens,
        num_partitions,
        num_tie_lines,
        centralized_time_ms,
        centralized_objective,
        centralized_converged,
        admm_time_ms,
        admm_objective,
        admm_converged,
        admm_iterations,
        primal_residual,
        dual_residual,
        x_update_ms: phase_times.x_update_ms as f64,
        z_update_ms: phase_times.z_update_ms as f64,
        dual_update_ms: phase_times.dual_update_ms as f64,
        objective_gap_rel,
        speedup_ratio,
    })
}

fn write_results(out: &str, results: &[DplibBenchmarkResult]) -> Result<()> {
    let out_path = Path::new(out);

    if let Some(parent) = out_path.parent() {
        if parent != Path::new("") {
            std::fs::create_dir_all(parent).ok();
        }
    }

    let file = File::create(out_path)
        .context(format!("Failed to create output file: {}", out))?;
    let mut writer = Writer::from_writer(file);

    for result in results {
        writer.serialize(result)?;
    }
    writer.flush()?;

    eprintln!("Results written to: {}", out);
    Ok(())
}

fn print_summary(results: &[DplibBenchmarkResult]) {
    if results.is_empty() {
        eprintln!("\nNo results to summarize.");
        return;
    }

    let total = results.len();
    let admm_converged = results.iter().filter(|r| r.admm_converged).count();
    let centralized_converged = results.iter().filter(|r| r.centralized_converged).count();

    // Compute averages for converged cases
    let converged_results: Vec<_> = results
        .iter()
        .filter(|r| r.admm_converged && r.centralized_converged)
        .collect();

    if converged_results.is_empty() {
        eprintln!("\n=== DPLib Benchmark Summary ===");
        eprintln!("Total cases: {}", total);
        eprintln!("ADMM converged: {}/{}", admm_converged, total);
        eprintln!("Centralized converged: {}/{}", centralized_converged, total);
        eprintln!("No cases with both methods converged for comparison.");
        return;
    }

    let avg_admm_time: f64 = converged_results.iter().map(|r| r.admm_time_ms).sum::<f64>()
        / converged_results.len() as f64;
    let avg_centralized_time: f64 = converged_results
        .iter()
        .map(|r| r.centralized_time_ms)
        .sum::<f64>()
        / converged_results.len() as f64;
    let avg_iterations: f64 = converged_results
        .iter()
        .map(|r| r.admm_iterations as f64)
        .sum::<f64>()
        / converged_results.len() as f64;
    let avg_gap: f64 = converged_results
        .iter()
        .map(|r| r.objective_gap_rel.abs())
        .sum::<f64>()
        / converged_results.len() as f64;
    let avg_speedup: f64 = converged_results.iter().map(|r| r.speedup_ratio).sum::<f64>()
        / converged_results.len() as f64;

    eprintln!();
    eprintln!("=== DPLib Benchmark Summary ===");
    eprintln!();
    eprintln!("Convergence:");
    eprintln!("  Total cases: {}", total);
    eprintln!("  ADMM converged: {}/{} ({:.1}%)",
        admm_converged, total, 100.0 * admm_converged as f64 / total as f64);
    eprintln!("  Centralized converged: {}/{} ({:.1}%)",
        centralized_converged, total, 100.0 * centralized_converged as f64 / total as f64);
    eprintln!();
    eprintln!("Performance (converged cases):");
    eprintln!("  Avg ADMM time: {:.2} ms", avg_admm_time);
    eprintln!("  Avg centralized time: {:.2} ms", avg_centralized_time);
    eprintln!("  Avg ADMM iterations: {:.1}", avg_iterations);
    eprintln!("  Avg speedup ratio: {:.2}x", avg_speedup);
    eprintln!();
    eprintln!("Accuracy:");
    eprintln!("  Avg |objective gap|: {:.4}%", avg_gap * 100.0);
    eprintln!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dplib_config_default() {
        let config = DplibConfig {
            pglib_dir: "data/pglib-opf".to_string(),
            case_filter: None,
            max_cases: 0,
            out: "results.csv".to_string(),
            threads: "auto".to_string(),
            num_partitions: 4,
            max_iter: 100,
            tol: 1e-4,
            rho: 1.0,
            subproblem_method: OpfMethod::DcOpf,
        };

        assert_eq!(config.num_partitions, 4);
        assert_eq!(config.max_iter, 100);
    }
}
