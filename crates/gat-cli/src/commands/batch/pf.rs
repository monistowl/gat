use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{anyhow, Result};
use gat_batch::{jobs_from_artifacts, run_batch, BatchRunnerConfig, BatchSummary, TaskKind};
use gat_cli::cli::BatchCommands;
use gat_core::solver::SolverKind;
use gat_scenarios::manifest::load_manifest;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};

/// Format duration in human-readable form
fn format_duration(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{:.1}ms", ms)
    } else if ms < 60_000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else {
        let mins = (ms / 60_000.0).floor();
        let secs = (ms % 60_000.0) / 1000.0;
        format!("{:.0}m {:.1}s", mins, secs)
    }
}

/// Print rich batch summary with statistics
fn print_batch_summary(summary: &BatchSummary, task_name: &str) {
    let stats = &summary.stats;

    println!();
    println!("╭─────────────────────────────────────────────────────────╮");
    println!("│  Batch {} Summary                                      │", task_name);
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  Jobs: {:>6} total  │  Success: {:>6}  │  Failed: {:>4} │",
             summary.jobs.len(), summary.success, summary.failure);
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  Timing Statistics                                      │");
    println!("│    Total:   {:>12}                                │", format_duration(stats.total_time_ms));
    println!("│    Mean:    {:>12}    Median: {:>12}       │",
             format_duration(stats.mean_time_ms), format_duration(stats.median_time_ms));
    println!("│    Min:     {:>12}    Max:    {:>12}       │",
             format_duration(stats.min_time_ms), format_duration(stats.max_time_ms));
    println!("│    P95:     {:>12}                                │", format_duration(stats.p95_time_ms));

    // Show AC solver stats if available
    if let (Some(avg_iter), Some(conv_rate)) = (stats.avg_iterations, stats.convergence_rate) {
        println!("├─────────────────────────────────────────────────────────┤");
        println!("│  Solver Statistics                                      │");
        println!("│    Avg Iterations: {:>6.1}    Convergence: {:>6.1}%      │",
                 avg_iter, conv_rate * 100.0);
    }

    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  Manifest: {}  │", summary.manifest_path.display());
    println!("╰─────────────────────────────────────────────────────────╯");

    // Show failed jobs if any
    if summary.failure > 0 {
        println!();
        println!("Failed jobs:");
        for job in summary.jobs.iter().filter(|j| j.status == "error") {
            println!("  ✗ {} - {}", job.job_id, job.error.as_deref().unwrap_or("unknown error"));
        }
    }
}

pub fn handle(command: &BatchCommands) -> Result<()> {
    let BatchCommands::Pf {
        manifest,
        out,
        solver,
        mode,
        threads,
        tol,
        max_iter,
        max_jobs,
        out_partitions,
    } = command
    else {
        unreachable!();
    };
    configure_threads(threads);
    let solver_kind = solver.parse::<SolverKind>()?;
    let artifacts = load_manifest(Path::new(manifest))?;
    if artifacts.is_empty() {
        return Err(anyhow!(
            "scenario manifest '{}' contains no artifacts",
            manifest
        ));
    }
    let task = match mode.as_str() {
        "dc" => TaskKind::PfDc,
        "ac" => TaskKind::PfAc,
        other => return Err(anyhow!("unknown pf mode '{}'; use 'dc' or 'ac'", other)),
    };
    let jobs = jobs_from_artifacts(&artifacts, task);
    if jobs.is_empty() {
        return Err(anyhow!("no jobs could be built from {}", manifest));
    }
    let partitions = parse_partitions(out_partitions.as_ref());
    let config = BatchRunnerConfig {
        jobs,
        output_root: PathBuf::from(out),
        task,
        solver: solver_kind.clone(),
        lp_solver: None,
        partitions,
        tol: *tol,
        max_iter: *max_iter,
        cost: None,
        limits: None,
        branch_limits: None,
        piecewise: None,
        threads: *max_jobs,
    };
    let start = Instant::now();
    let mut summary = None;
    let res = (|| -> Result<()> {
        let batch_summary = run_batch(&config)?;
        let task_name = if *mode == "dc" { "PF-DC" } else { "PF-AC" };
        print_batch_summary(&batch_summary, task_name);
        summary = Some(batch_summary);
        Ok(())
    })();
    let mut params = vec![
        ("manifest".to_string(), manifest.to_string()),
        ("solver".to_string(), solver_kind.as_str().to_string()),
        ("mode".to_string(), mode.clone()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
        ("threads".to_string(), threads.to_string()),
        ("max_jobs".to_string(), max_jobs.to_string()),
    ];
    if let Some(summary) = summary.as_ref() {
        params.push(("num_jobs".to_string(), summary.jobs.len().to_string()));
        params.push(("success".to_string(), summary.success.to_string()));
        params.push(("failure".to_string(), summary.failure.to_string()));
        params.push((
            "manifest_path".to_string(),
            summary.manifest_path.display().to_string(),
        ));
    }
    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "batch pf", &param_refs, start, &res);
    res
}
