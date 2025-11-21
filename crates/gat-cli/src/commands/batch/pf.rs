use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{anyhow, Result};
use gat_batch::{jobs_from_artifacts, run_batch, BatchRunnerConfig, TaskKind};
use gat_cli::cli::BatchCommands;
use gat_core::solver::SolverKind;
use gat_scenarios::manifest::load_manifest;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};

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
        println!(
            "batch pf {} -> {}/{} ok/fail",
            batch_summary.jobs.len(),
            batch_summary.success,
            batch_summary.failure
        );
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
