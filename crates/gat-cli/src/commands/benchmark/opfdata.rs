//! OPFData centralized AC-OPF benchmark command.
//!
//! Runs AC-OPF on OPFData test samples and compares against reference objectives.

use anyhow::{bail, Context, Result};
use csv::Writer;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use gat_algo::validation::ObjectiveGap;
use gat_algo::{OpfMethod, OpfSolver};
use gat_io::sources::opfdata::{list_sample_refs, load_opfdata_instance, OpfDataSampleRef};
use std::str::FromStr;

/// Benchmark result for a single OPFData sample
#[derive(Debug, Clone, Serialize)]
struct OpfDataBenchmarkResult {
    sample_id: String,
    file_name: String,
    load_time_ms: f64,
    solve_time_ms: f64,
    total_time_ms: f64,
    converged: bool,
    iterations: u32,
    num_buses: usize,
    num_branches: usize,
    num_gens: usize,
    objective_value: f64,
    // Baseline comparison
    baseline_objective: f64,
    objective_gap_abs: f64,
    objective_gap_rel: f64,
}

/// JSONL diagnostics entry for batch analysis
#[derive(Debug, Clone, Serialize)]
struct DiagnosticsEntry {
    case: String,
    status: String,
    warnings: Vec<String>,
    objective: f64,
    baseline_objective: f64,
    converged: bool,
}

/// Configuration for OPFData benchmark runs
#[derive(Debug)]
struct BenchmarkConfig {
    opfdata_dir: String,
    case_filter: Option<String>,
    max_cases: usize,
    out: String,
    threads: String,
    method: OpfMethod,
    tol: f64,
    max_iter: u32,
    diagnostics_log: Option<String>,
    strict: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn handle(
    opfdata_dir: &str,
    case_filter: Option<&str>,
    max_cases: usize,
    out: &str,
    threads: &str,
    method: &str,
    tol: f64,
    max_iter: u32,
    diagnostics_log: Option<&str>,
    strict: bool,
) -> Result<()> {
    // Parse OPF method
    let opf_method = OpfMethod::from_str(method)
        .map_err(|e| anyhow::anyhow!("Invalid OPF method '{}': {}", method, e))?;

    let config = BenchmarkConfig {
        opfdata_dir: opfdata_dir.to_string(),
        case_filter: case_filter.map(|s| s.to_string()),
        max_cases,
        out: out.to_string(),
        threads: threads.to_string(),
        method: opf_method,
        tol,
        max_iter,
        diagnostics_log: diagnostics_log.map(|s| s.to_string()),
        strict,
    };

    eprintln!("Using OPF method: {}", config.method);
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

    // Discover samples
    let opfdata_path = Path::new(&config.opfdata_dir);
    let mut sample_refs = list_sample_refs(opfdata_path)?;

    // Filter by case name if specified
    if let Some(filter) = &config.case_filter {
        sample_refs.retain(|r| {
            r.file_path
                .to_string_lossy()
                .to_lowercase()
                .contains(&filter.to_lowercase())
        });
    }

    // Limit number of samples
    let limit = if config.max_cases > 0 {
        config.max_cases
    } else {
        sample_refs.len()
    };
    sample_refs.truncate(limit);

    eprintln!(
        "Found {} OPFData samples to benchmark (filter={:?}, max_cases={})",
        sample_refs.len(),
        config.case_filter,
        config.max_cases
    );

    // Group samples by file for efficient loading (future optimization)
    let _samples_by_file: HashMap<String, Vec<&OpfDataSampleRef>> =
        sample_refs.iter().fold(HashMap::new(), |mut acc, r| {
            acc.entry(r.file_path.to_string_lossy().to_string())
                .or_default()
                .push(r);
            acc
        });

    // Run benchmarks in parallel
    let tol = config.tol;
    let max_iter = config.max_iter;
    let method = config.method;

    let outputs: Vec<BenchmarkSampleOutput> = sample_refs
        .par_iter()
        .filter_map(|sample_ref| {
            match benchmark_opfdata_sample(sample_ref, method, tol, max_iter) {
                Ok(output) => Some(output),
                Err(e) => {
                    eprintln!("Error benchmarking {}: {}", sample_ref.sample_id, e);
                    None
                }
            }
        })
        .collect();

    // Separate results and diagnostics
    let results: Vec<OpfDataBenchmarkResult> = outputs.iter().map(|o| o.result.clone()).collect();
    let diagnostics_entries: Vec<&DiagnosticsEntry> =
        outputs.iter().map(|o| &o.diagnostics_entry).collect();

    // Write JSONL diagnostics if path specified
    if let Some(diag_path) = &config.diagnostics_log {
        let diag_file = File::create(diag_path)
            .context(format!("Failed to create diagnostics log: {}", diag_path))?;
        let mut diag_writer = BufWriter::new(diag_file);
        for entry in &diagnostics_entries {
            let line =
                serde_json::to_string(entry).context("Failed to serialize diagnostics entry")?;
            writeln!(diag_writer, "{}", line).context("Failed to write diagnostics line")?;
        }
        diag_writer
            .flush()
            .context("Failed to flush diagnostics log")?;
        eprintln!("Diagnostics log written to: {}", diag_path);
    }

    // Strict mode: fail if any sample had warnings
    if config.strict {
        let samples_with_warnings: Vec<&DiagnosticsEntry> = diagnostics_entries
            .iter()
            .filter(|e| !e.warnings.is_empty())
            .copied()
            .collect();
        if !samples_with_warnings.is_empty() {
            let summary: Vec<String> = samples_with_warnings
                .iter()
                .map(|e| format!("{}: {}", e.case, e.warnings.join("; ")))
                .collect();
            bail!(
                "Strict mode: {} sample(s) had import warnings:\n  - {}",
                samples_with_warnings.len(),
                summary.join("\n  - ")
            );
        }
    }

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
        "\nOPFData Benchmark Results:\n  Total samples: {}\n  Converged: {}\n  Avg solve time: {:.2}ms\n  Avg obj gap: {:.4}%\n  Output: {}",
        results.len(),
        converged_count,
        avg_time,
        avg_gap * 100.0,
        config.out
    );

    Ok(())
}

/// Result of benchmarking a single sample, including diagnostics for JSONL output
struct BenchmarkSampleOutput {
    result: OpfDataBenchmarkResult,
    diagnostics_entry: DiagnosticsEntry,
}

fn benchmark_opfdata_sample(
    sample_ref: &OpfDataSampleRef,
    method: OpfMethod,
    tol: f64,
    max_iter: u32,
) -> Result<BenchmarkSampleOutput> {
    let load_start = Instant::now();
    let (instance, diagnostics) =
        load_opfdata_instance(&sample_ref.file_path, &sample_ref.sample_id)?;
    let load_time_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    // Collect warnings for diagnostics log
    let warnings: Vec<String> = diagnostics
        .issues
        .iter()
        .map(|i| i.message.clone())
        .collect();

    // Log any import warnings to stderr (silent defaults that were applied)
    if diagnostics.warning_count() > 0 {
        eprintln!(
            "  [WARN] {}: {} import warnings",
            sample_ref.sample_id,
            diagnostics.warning_count()
        );
        for issue in &diagnostics.issues {
            eprintln!("    - {}", issue.message);
        }
    }

    // Count network elements
    let mut num_buses = 0;
    let mut num_gens = 0;
    for node in instance.network.graph.node_weights() {
        match node {
            gat_core::Node::Bus(_) => num_buses += 1,
            gat_core::Node::Gen(_) => num_gens += 1,
            gat_core::Node::Load(_) => {}
            gat_core::Node::Shunt(_) => {}
        }
    }
    let num_branches = instance.network.graph.edge_count();

    // Solve OPF using selected method
    let solver = OpfSolver::new()
        .with_method(method)
        .with_max_iterations(max_iter as usize)
        .with_tolerance(tol);

    let solve_start = Instant::now();
    let solution = solver
        .solve(&instance.network)
        .map_err(|e| anyhow::anyhow!("OPF ({}) failed: {}", method, e))?;
    let solve_time_ms = solve_start.elapsed().as_secs_f64() * 1000.0;

    // Reference objective from the dataset
    let baseline_objective = instance.solution.objective;

    let gap = if baseline_objective > 0.0 {
        ObjectiveGap::new(solution.objective_value, baseline_objective)
    } else {
        ObjectiveGap::default()
    };

    let file_name = sample_ref
        .file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let result = OpfDataBenchmarkResult {
        sample_id: sample_ref.sample_id.clone(),
        file_name,
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
    };

    let status = if !solution.converged {
        "failed"
    } else if !warnings.is_empty() {
        "warnings"
    } else {
        "ok"
    };

    let diagnostics_entry = DiagnosticsEntry {
        case: sample_ref.sample_id.clone(),
        status: status.to_string(),
        warnings,
        objective: solution.objective_value,
        baseline_objective,
        converged: solution.converged,
    };

    Ok(BenchmarkSampleOutput {
        result,
        diagnostics_entry,
    })
}
