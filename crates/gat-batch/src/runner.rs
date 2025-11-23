use crate::job::{BatchJob, BatchJobRecord, TaskKind};
use crate::manifest::{write_batch_manifest, BatchManifest};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use gat_algo::power_flow;
use gat_algo::LpSolverKind;
use gat_core::solver::SolverKind;
use gat_io::importers;
use num_cpus;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::convert::TryInto;
use std::fs;
use std::path::PathBuf;

/// Runner settings that keep the batch aligned with the classic B′θ = P DC flow fan-out (doi:10.1109/TPWRS.2007.899019).
pub struct BatchRunnerConfig {
    pub jobs: Vec<BatchJob>,
    pub output_root: PathBuf,
    pub task: TaskKind,
    pub solver: SolverKind,
    pub lp_solver: Option<LpSolverKind>,
    pub partitions: Vec<String>,
    pub tol: f64,
    pub max_iter: usize,
    pub cost: Option<String>,
    pub limits: Option<String>,
    pub branch_limits: Option<String>,
    pub piecewise: Option<String>,
    pub threads: usize,
}

/// Summary returned after the run so clients can log success/failure counts and manifest location.
pub struct BatchSummary {
    pub success: usize,
    pub failure: usize,
    pub manifest_path: PathBuf,
    pub jobs: Vec<BatchJobRecord>,
}

pub fn run_batch(config: &BatchRunnerConfig) -> Result<BatchSummary> {
    // Create output directory structure
    fs::create_dir_all(&config.output_root).with_context(|| {
        format!(
            "creating batch output root '{}'",
            config.output_root.display()
        )
    })?;

    // Configure thread pool: auto-detect CPU count if threads=0, otherwise use specified count
    let thread_count = if config.threads == 0 {
        num_cpus::get()
    } else {
        config.threads
    };
    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .context("building Rayon thread pool for batch runs")?;

    // Execute all jobs in parallel using Rayon's parallel iterator
    // Each job runs PF/OPF on a scenario-specific grid snapshot
    let job_records: Vec<BatchJobRecord> = pool.install(|| {
        config
            .jobs
            .par_iter()
            .map(|job| run_job(job, config))
            .collect()
    });

    // Count successes and failures for summary
    let success = job_records
        .iter()
        .filter(|record| record.status == "ok")
        .count();
    let failure = job_records.len() - success;

    // Write batch manifest JSON for downstream tools (analytics, reporting)
    let manifest = BatchManifest {
        created_at: Utc::now(),
        task: config.task.as_str().to_string(),
        num_jobs: job_records.len(),
        success,
        failure,
        jobs: job_records.clone(),
    };
    let manifest_path = config.output_root.join("batch_manifest.json");
    write_batch_manifest(&manifest_path, &manifest)?;
    Ok(BatchSummary {
        success,
        failure,
        manifest_path,
        jobs: job_records,
    })
}

/// Execute a single batch job: load scenario grid and run PF/OPF.
///
/// **Algorithm:**
/// 1. Load scenario-specific grid from Arrow file.
/// 2. Build solver backend (DC: linear solver, AC: iterative solver).
/// 3. Dispatch to appropriate PF/OPF routine based on task type.
/// 4. Write results to Parquet with optional partitioning.
///
/// **Returns:** BatchJobRecord with status ("ok" or "error") and output path.
fn run_job(job: &BatchJob, config: &BatchRunnerConfig) -> BatchJobRecord {
    let output_file = config.output_root.join(&job.job_id).join("result.parquet");

    // Closure that performs the actual computation
    let runner = || -> Result<()> {
        let grid_path = job.grid_file.to_str().ok_or_else(|| {
            anyhow!(
                "grid path '{}' is not valid unicode",
                job.grid_file.display()
            )
        })?;

        // Load scenario-specific grid snapshot (from gat scenarios materialize)
        let network = importers::load_grid_from_arrow(grid_path)?;
        let solver_impl = config.solver.build_solver();

        // Dispatch to appropriate solver routine based on task type
        match config.task {
            TaskKind::PfDc => power_flow::dc_power_flow(
                &network,
                solver_impl.as_ref(),
                &output_file,
                &config.partitions,
            ),
            TaskKind::PfAc => {
                let max_iter = config
                    .max_iter
                    .try_into()
                    .map_err(|_| anyhow!("max_iter {} exceeds u32 range", config.max_iter))?;
                power_flow::ac_power_flow(
                    &network,
                    solver_impl.as_ref(),
                    config.tol,
                    max_iter,
                    &output_file,
                    &config.partitions,
                )
            }
            TaskKind::OpfDc => {
                let cost = config
                    .cost
                    .as_deref()
                    .ok_or_else(|| anyhow!("cost file is required for DC OPF"))?;
                let limits = config
                    .limits
                    .as_deref()
                    .ok_or_else(|| anyhow!("limits file is required for DC OPF"))?;
                let lp_solver = config
                    .lp_solver
                    .as_ref()
                    .ok_or_else(|| anyhow!("LP solver is required for DC OPF"))?;
                power_flow::dc_optimal_power_flow(
                    &network,
                    solver_impl.as_ref(),
                    cost,
                    limits,
                    &output_file,
                    &config.partitions,
                    config.branch_limits.as_deref(),
                    config.piecewise.as_deref(),
                    lp_solver,
                )
            }
            TaskKind::OpfAc => {
                let max_iter = config
                    .max_iter
                    .try_into()
                    .map_err(|_| anyhow!("max_iter {} exceeds u32 range", config.max_iter))?;
                power_flow::ac_optimal_power_flow(
                    &network,
                    solver_impl.as_ref(),
                    config.tol,
                    max_iter,
                    &output_file,
                    &config.partitions,
                )
            }
        }
    };
    let status = runner();
    let (status_label, error) = match status {
        Ok(_) => ("ok".to_string(), None),
        Err(err) => {
            eprintln!("batch job {} failed: {err}", job.job_id);
            ("error".to_string(), Some(err.to_string()))
        }
    };
    BatchJobRecord {
        job_id: job.job_id.clone(),
        scenario_id: job.scenario_id.clone(),
        time: job.time.map(|t| t.to_rfc3339()),
        status: status_label,
        error,
        output: output_file.display().to_string(),
    }
}
