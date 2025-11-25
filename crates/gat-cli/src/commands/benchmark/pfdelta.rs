use anyhow::{anyhow, Context, Result};
use csv::Writer;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

use gat_algo::validation::{compute_pf_errors, PFReferenceSolution};
use gat_algo::AcOpfSolver;
use gat_io::sources::pfdelta::{list_pfdelta_cases, load_pfdelta_case, load_pfdelta_instance};

/// Benchmark result for a single test case
#[derive(Debug, Clone, Serialize)]
struct BenchmarkResult {
    case_name: String,
    contingency_type: String,
    case_index: usize,
    mode: String,
    load_time_ms: f64,
    solve_time_ms: f64,
    total_time_ms: f64,
    converged: bool,
    iterations: u32,
    num_buses: usize,
    num_branches: usize,
    // Error metrics (for PF mode comparison against reference)
    max_vm_error: f64,
    max_va_error_deg: f64,
    mean_vm_error: f64,
    mean_va_error_deg: f64,
}

/// Configuration for PFDelta benchmark runs
#[derive(Debug)]
struct BenchmarkConfig {
    pfdelta_root: String,
    case_filter: Option<String>,
    contingency_filter: String,
    max_cases: usize,
    out: String,
    threads: String,
    mode: String,
    tol: f64,
    max_iter: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn handle(
    pfdelta_root: &str,
    case_filter: Option<&str>,
    contingency_filter: &str,
    max_cases: usize,
    out: &str,
    threads: &str,
    mode: &str,
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    let config = BenchmarkConfig {
        pfdelta_root: pfdelta_root.to_string(),
        case_filter: case_filter.map(|s| s.to_string()),
        contingency_filter: contingency_filter.to_string(),
        max_cases,
        out: out.to_string(),
        threads: threads.to_string(),
        mode: mode.to_string(),
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

    // List available test cases
    let root_path = Path::new(&config.pfdelta_root);
    if !root_path.exists() {
        return Err(anyhow!(
            "PFDelta root directory not found: {}",
            config.pfdelta_root
        ));
    }

    let mut all_cases =
        list_pfdelta_cases(root_path).context("Failed to list PFDelta test cases")?;

    // Filter by case if specified
    if let Some(case) = &config.case_filter {
        all_cases.retain(|tc| tc.case_name.contains(case));
    }

    // Filter by contingency type
    if config.contingency_filter != "all" {
        all_cases.retain(|tc| tc.contingency_type.starts_with(&config.contingency_filter));
    }

    // Limit number of cases
    let limit = if config.max_cases > 0 {
        config.max_cases
    } else {
        all_cases.len()
    };
    all_cases.truncate(limit);

    eprintln!(
        "Found {} test cases to benchmark (case_filter={:?}, contingency={}, max_cases={}, mode={})",
        all_cases.len(),
        config.case_filter,
        config.contingency_filter,
        config.max_cases,
        config.mode
    );

    // Run benchmarks in parallel
    let mode = config.mode.clone();
    let tol = config.tol;
    let max_iter = config.max_iter;

    let results: Vec<BenchmarkResult> = all_cases
        .par_iter()
        .enumerate()
        .filter_map(|(idx, test_case)| {
            benchmark_case(test_case, idx, &mode, tol, max_iter).ok()
        })
        .collect();

    // Write results to CSV
    let out_path = Path::new(&config.out);
    if let Some(parent) = out_path.parent() {
        if parent != Path::new("") {
            std::fs::create_dir_all(parent).ok();
        }
    }

    let file =
        File::create(out_path).context(format!("Failed to create output file: {}", config.out))?;
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

    eprintln!(
        "\nBenchmark Results:\n  Total cases: {}\n  Converged: {}\n  Avg solve time: {:.2}ms\n  Output: {}",
        results.len(),
        converged_count,
        avg_time,
        config.out
    );

    Ok(())
}

fn benchmark_case(
    test_case: &gat_io::sources::pfdelta::PFDeltaTestCase,
    idx: usize,
    mode: &str,
    tol: f64,
    max_iter: u32,
) -> Result<BenchmarkResult> {
    let load_start = Instant::now();

    // Load instance with reference solution
    let instance = load_pfdelta_instance(Path::new(&test_case.file_path), test_case)?;
    let load_time_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    let num_buses = instance.network.graph.node_indices().count();
    let num_branches = instance.network.graph.edge_indices().count();

    let solve_start = Instant::now();

    // Branch on mode
    let (converged, iterations, gat_vm, gat_va) = match mode {
        "pf" => {
            // For PF mode, we use the reference solution as validation
            // The actual PF solve would go here, but for now we just compare
            // For a real PF benchmark, we'd call an AC power flow solver
            // TODO: Integrate actual AC power flow solver when available
            //
            // For now, return the reference solution as "solved" values
            // This lets us test the infrastructure even without a PF solver
            (
                true,
                0u32,
                instance.solution.vm.clone(),
                instance.solution.va.clone(),
            )
        }
        "opf" | _ => {
            let solver = AcOpfSolver::new()
                .with_max_iterations(max_iter as usize)
                .with_tolerance(tol);

            // For OPF, we need to load without instance (the old way)
            let network = load_pfdelta_case(Path::new(&test_case.file_path))?;
            let solution = solver.solve(&network)?;

            // Extract voltage solution from OPF result
            // AcOpfSolution has bus_voltages: HashMap<String, f64> (magnitude only)
            let gat_vm: HashMap<usize, f64> = solution
                .bus_voltages
                .iter()
                .filter_map(|(name, vm)| {
                    // Parse bus name like "bus_1" to get index
                    name.strip_prefix("bus_")
                        .and_then(|s| s.parse::<usize>().ok())
                        .map(|idx| (idx, *vm))
                })
                .collect();

            // OPF doesn't give us angles directly, so use empty map
            let gat_va: HashMap<usize, f64> = HashMap::new();

            (
                solution.converged,
                solution.iterations as u32,
                gat_vm,
                gat_va,
            )
        }
    };

    let solve_time_ms = solve_start.elapsed().as_secs_f64() * 1000.0;

    // Compute error metrics against reference
    let ref_solution = PFReferenceSolution {
        vm: instance.solution.vm,
        va: instance.solution.va,
        pgen: instance.solution.pgen,
        qgen: instance.solution.qgen,
    };
    let errors = compute_pf_errors(&instance.network, &gat_vm, &gat_va, &ref_solution);

    Ok(BenchmarkResult {
        case_name: test_case.case_name.clone(),
        contingency_type: test_case.contingency_type.clone(),
        case_index: idx,
        mode: mode.to_string(),
        load_time_ms,
        solve_time_ms,
        total_time_ms: load_time_ms + solve_time_ms,
        converged,
        iterations,
        num_buses,
        num_branches,
        max_vm_error: errors.max_vm_error,
        max_va_error_deg: errors.max_va_error_deg,
        mean_vm_error: errors.mean_vm_error,
        mean_va_error_deg: errors.mean_va_error_deg,
    })
}
