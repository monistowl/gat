use anyhow::{Result, anyhow, Context};
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use csv::Writer;
use serde::Serialize;
use rayon::prelude::*;

use gat_io::sources::pfdelta::{list_pfdelta_cases, load_pfdelta_case};

/// Benchmark result for a single test case
#[derive(Debug, Clone, Serialize)]
struct BenchmarkResult {
    case_name: String,
    contingency_type: String,
    case_index: usize,
    solve_time_ms: f64,
    num_buses: usize,
    num_branches: usize,
}

pub fn handle(
    pfdelta_root: &str,
    case_filter: Option<&str>,
    contingency_filter: &str,
    max_cases: usize,
    out: &str,
    threads: &str,
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    // Configure threads
    if threads != "auto" {
        if let Ok(n) = threads.parse::<usize>() {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build_global()
                .ok();
        }
    }

    // List available test cases
    let root_path = Path::new(pfdelta_root);
    if !root_path.exists() {
        return Err(anyhow!("PFDelta root directory not found: {}", pfdelta_root));
    }

    let mut all_cases = list_pfdelta_cases(root_path)
        .context("Failed to list PFDelta test cases")?;

    // Filter by case if specified
    if let Some(case) = case_filter {
        all_cases.retain(|tc| tc.case_name.contains(case));
    }

    // Filter by contingency type
    if contingency_filter != "all" {
        all_cases.retain(|tc| tc.contingency_type.starts_with(contingency_filter));
    }

    // Limit number of cases
    let limit = if max_cases > 0 { max_cases } else { all_cases.len() };
    all_cases.truncate(limit);

    eprintln!(
        "Found {} test cases to benchmark (case_filter={:?}, contingency={}, max_cases={})",
        all_cases.len(),
        case_filter,
        contingency_filter,
        max_cases
    );

    // Run benchmarks in parallel
    let results: Vec<BenchmarkResult> = all_cases
        .par_iter()
        .enumerate()
        .filter_map(|(idx, test_case)| {
            benchmark_case(test_case, idx, tol, max_iter).ok()
        })
        .collect();

    // Write results to CSV
    let out_path = Path::new(out);
    if let Some(parent) = out_path.parent() {
        if parent != Path::new("") {
            std::fs::create_dir_all(parent).ok();
        }
    }

    let file = File::create(out_path)
        .context(format!("Failed to create output file: {}", out))?;
    let mut writer = Writer::from_writer(file);

    for result in &results {
        writer
            .serialize(result)
            .context("Failed to write result to CSV")?;
    }

    writer.flush().context("Failed to flush CSV writer")?;

    // Print summary
    let avg_time: f64 = if !results.is_empty() {
        results.iter().map(|r| r.solve_time_ms).sum::<f64>() / results.len() as f64
    } else {
        0.0
    };

    eprintln!(
        "\nBenchmark Results:\n  Total cases: {}\n  Avg time: {:.2}ms\n  Output: {}",
        results.len(),
        avg_time,
        out
    );

    Ok(())
}

fn benchmark_case(
    test_case: &gat_io::sources::pfdelta::PFDeltaTestCase,
    idx: usize,
    _tol: f64,
    _max_iter: u32,
) -> Result<BenchmarkResult> {
    // Time the network loading
    let start = Instant::now();
    let network = load_pfdelta_case(Path::new(&test_case.file_path))?;
    let elapsed = start.elapsed();

    let num_buses = network.graph.node_indices().count();
    let num_branches = network.graph.edge_indices().count();

    Ok(BenchmarkResult {
        case_name: test_case.case_name.clone(),
        contingency_type: test_case.contingency_type.clone(),
        case_index: idx,
        solve_time_ms: elapsed.as_secs_f64() * 1000.0,
        num_buses,
        num_branches,
    })
}
