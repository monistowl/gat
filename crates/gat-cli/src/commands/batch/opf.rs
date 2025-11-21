use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};
use anyhow::{anyhow, Result};
use gat_batch::{jobs_from_artifacts, run_batch, BatchRunnerConfig, TaskKind};
use gat_cli::cli::BatchCommands;
use gat_core::solver::SolverKind;
use gat_scenarios::manifest::load_manifest;

pub fn handle(command: &BatchCommands) -> Result<()> {
    let BatchCommands::Opf {
        manifest,
        out,
        solver,
        lp_solver,
        mode,
        threads,
        tol,
        max_iter,
        cost,
        limits,
        branch_limits,
        piecewise,
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
        "dc" => TaskKind::OpfDc,
        "ac" => TaskKind::OpfAc,
        other => return Err(anyhow!("unknown opf mode '{}'; use 'dc' or 'ac'", other)),
    };
    let jobs = jobs_from_artifacts(&artifacts, task);
    if jobs.is_empty() {
        return Err(anyhow!("no jobs could be built from {}", manifest));
    }
    let partitions = parse_partitions(out_partitions.as_ref());
    let mut config = BatchRunnerConfig {
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
        branch_limits: branch_limits.clone(),
        piecewise: piecewise.clone(),
        threads: *max_jobs,
    };
    if matches!(task, TaskKind::OpfDc) {
        if cost.is_empty() || limits.is_empty() {
            return Err(anyhow!("cost and limits files are required for DC OPF"));
        }
        config.cost = Some(cost.clone());
        config.limits = Some(limits.clone());
        config.lp_solver = Some(lp_solver.parse()?);
    }
    let start = Instant::now();
    let mut summary = None;
    let res = (|| -> Result<()> {
        let batch_summary = run_batch(&config)?;
        println!(
            "batch opf {} -> {}/{} ok/fail",
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
        ("lp_solver".to_string(), lp_solver.clone()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
        ("threads".to_string(), threads.to_string()),
        ("max_jobs".to_string(), max_jobs.to_string()),
    ];
    if matches!(task, TaskKind::OpfDc) {
        params.push(("cost".to_string(), cost.clone()));
        params.push(("limits".to_string(), limits.clone()));
        if let Some(br) = branch_limits.as_deref() {
            params.push(("branch_limits".to_string(), br.to_string()));
        }
    }
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
    record_run_timed(out, "batch opf", &param_refs, start, &res);
    res
}
