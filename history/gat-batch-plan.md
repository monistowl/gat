Here’s a full roadmap for **`gat batch`**, assuming the `gat-scenarios` crate from the previous plan exists (or will exist in roughly that shape).

I’ll keep the same style: library crate + CLI, phases, and bead-sized tasks for agents.

---

## 0. What `gat batch` is supposed to do

**Goal (v0):**

> Given a set of scenario-specific grid snapshots (from `gat scenarios materialize`) and PF/OPF options, **run DC/AC PF/OPF for all scenarios** and write out **partitioned Parquet** with `scenario_id` (and optionally other keys) baked into the partitions.

**Goal (v1):**

> Extend the above to **scenario × time** grids, using `gat-ts` and/or extended scenario artifacts, so you can do CANOS-style fanouts over time-slices as well.

So think of `gat batch` as:

* a library crate that knows how to:

  * enumerate jobs,
  * call `gat-algo` PF/OPF routines many times,
  * coordinate output layout, and
* CLI sugar under `gat batch pf` / `gat batch opf`.

---

## 1. High-level semantics

### 1.1 Input assumptions

For **v0**:

* You already ran:

  ```bash
  gat scenarios materialize \
    --spec scenarios.yaml \
    --grid-file grid.arrow \
    --out-dir out/scenarios
  ```

* This created:

  * `out/scenarios/scenario_manifest.json` (or `.parquet`) listing `ScenarioArtifact`s, and
  * One Arrow grid per scenario, e.g.:

    ```
    out/scenarios/<scenario_id>/grid.arrow
    ```

`gat batch` will:

* Read **one manifest**.
* For each `scenario_id`, locate its `grid.arrow`.
* Run PF or OPF once per scenario (v0).
* Emit Parquet files with `scenario_id` in partitions.

For **v1**, add time dimension:

* Either:

  * each `ScenarioArtifact` has per-time-slice grid paths, or
  * you have a separate “time-expanded” manifest (e.g., `scenario_id`, `time`, `grid_file`).

---

## 2. Create `gat-batch` crate (library)

### 2.1 Crate skeleton

New directory:

* `crates/gat-batch/`

  * `Cargo.toml`
  * `src/lib.rs`
  * `src/spec.rs` (if you define a batch spec)
  * `src/job.rs`
  * `src/runner.rs`
  * `src/io.rs` (for manifests / Parquet metadata)

**Cargo.toml** (rough):

```toml
[package]
name = "gat-batch"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
rayon = "1"
gat-core = { path = "../gat-core" }
gat-algo = { path = "../gat-algo" }
gat-scenarios = { path = "../gat-scenarios" }
gat-io = { path = "../gat-io" }
polars = { version = "...", features = ["parquet", "lazy"] } # match workspace
tracing = "0.1"
```

(You can also reuse workspace-level polars/tracing versions to avoid duplication.)

### 2.2 Job abstraction (`job.rs`)

Define the basic units of work the runner operates on:

```rust
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum TaskKind {
    PfDc,
    PfAc,
    OpfDc,
    OpfAc,
    // Future: nminus1, sensitivity, etc.
}

#[derive(Debug, Clone)]
pub struct BatchJob {
    pub job_id: String,        // unique, e.g. "pf_dc:scenario=foo"
    pub scenario_id: String,
    pub time: Option<DateTime<Utc>>,
    pub grid_file: PathBuf,
    pub task_kind: TaskKind,

    /// Optional weight or probability from ScenarioArtifact
    pub weight: Option<f64>,

    /// Free-form labels, e.g. tags from scenario, partitions, etc.
    pub labels: Vec<String>,
}
```

Add some helpers to derive `job_id` (e.g. `format!("{:?}:{}", task_kind, scenario_id)` plus time).

### 2.3 Batch configuration (`spec.rs`)

You probably want a minimal **batch spec** that tells `gat batch`:

* where the scenario manifest lives,
* what tasks to run,
* where outputs go,
* and some concurrency options.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSpec {
    pub version: u32,
    pub scenario_manifest: String, // path to ScenarioArtifact manifest (json or parquet)
    pub task: BatchTaskSpec,
    pub output_root: String,       // base output dir
    pub concurrency: BatchConcurrencySpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTaskSpec {
    pub kind: String,             // "pf_dc" | "pf_ac" | "opf_dc" | "opf_ac"
    pub partitions: Vec<String>,  // keys to partition by (e.g., ["scenario_id"])
    pub tol: Option<f64>,         // for AC pf/opf
    pub max_iter: Option<usize>,  // for iterative solvers
    pub solver: Option<String>,   // e.g. "default", "ipopt", "clarabel"
    pub lp_solver: Option<String> // for DC OPF
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConcurrencySpec {
    pub max_jobs_in_flight: Option<usize>,
    pub threads_per_job: Option<usize>,
}
```

Also provide helper:

```rust
pub fn load_batch_spec(path: &Path) -> Result<BatchSpec>;
```

Although, for v0, you can skip the spec file and drive everything from CLI flags; but a spec makes it easier to orchestrate from MCP/agents later.

### 2.4 Job creation from scenario manifests

Add a function to **turn scenario artifacts into jobs**:

```rust
use gat_scenarios::ScenarioArtifact;

pub fn jobs_from_scenario_artifacts(
    artifacts: &[ScenarioArtifact],
    task_kind: TaskKind,
) -> Vec<BatchJob> {
    let mut jobs = Vec::new();
    for art in artifacts {
        // v0: one job per scenario, ignore time.
        jobs.push(BatchJob {
            job_id: format!("{:?}:{}", task_kind, art.scenario_id),
            scenario_id: art.scenario_id.clone(),
            time: None, // v0
            grid_file: PathBuf::from(&art.grid_file),
            task_kind: task_kind.clone(),
            weight: Some(art.weight),
            labels: art.tags.clone(),
        });
    }
    jobs
}
```

For **v1**, extend `ScenarioArtifact` to include per-time-slice info, or create a separate mapping table and adjust this function to yield multiple jobs per scenario.

---

## 3. Batch runner (`runner.rs`)

### 3.1 Runner configuration

Define a struct that collects everything needed to actually run:

```rust
use chrono::{DateTime, Utc};

pub struct BatchRunnerConfig<'a> {
    pub jobs: Vec<BatchJob>,
    pub output_root: PathBuf,
    pub partitions: Vec<String>, // e.g. vec!["scenario_id".to_string()]
    pub tol: f64,
    pub max_iter: usize,
    pub solver_name: &'a str,
    pub lp_solver_name: &'a str,
    pub threads_per_job: usize,
    pub max_jobs_in_flight: usize,
}
```

### 3.2 Core runner function

Signature:

```rust
pub fn run_batch(config: &BatchRunnerConfig) -> Result<BatchSummary>;
```

Where `BatchSummary` is something like:

```rust
#[derive(Debug, Clone)]
pub struct BatchSummary {
    pub num_jobs: usize,
    pub num_success: usize,
    pub num_failed: usize,
    pub first_error: Option<String>,
}
```

### 3.3 Inside `run_batch`

Broad steps:

1. Configure global threads for solver (like `configure_threads` in CLI), or call that from CLI before invoking `run_batch`.
2. Decide concurrency pattern:

   * Easiest: **Rayon parallel iterator** with `par_iter()` and a limit on number of threads.
3. For each job:

   * Build a per-job output path:

     * `output_file = output_root.join(job.task_kind_dir()).join(format!("{}.parquet", stage_name))`
     * or better: `output_root/<task>/<scenario_id>...` and rely on `persist_dataframe` partitioning for `scenario_id`.
4. Call appropriate `gat-algo` function:

   * `dc_power_flow`, `ac_power_flow`, `dc_opf`, `ac_opf`.
5. Track results; accumulate counts.

Pseudo-code:

```rust
use rayon::prelude::*;
use gat_core::solver::SolverKind;
use gat_algo::power_flow;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn run_batch(config: &BatchRunnerConfig) -> Result<BatchSummary> {
    let num_success = AtomicUsize::new(0);
    let num_failed = AtomicUsize::new(0);
    let mut first_error: Option<String> = None;

    // configure rayon global thread pool if needed, or rely on defaults

    config.jobs.par_iter().for_each(|job| {
        let result = run_single_job(job, config);
        match result {
            Ok(()) => { num_success.fetch_add(1, Ordering::Relaxed); }
            Err(e) => {
                num_failed.fetch_add(1, Ordering::Relaxed);
                // capture first_error using a Mutex or atomic once; omitted in pseudo
            }
        }
    });

    Ok(BatchSummary {
        num_jobs: config.jobs.len(),
        num_success: num_success.load(Ordering::Relaxed),
        num_failed: num_failed.load(Ordering::Relaxed),
        first_error,
    })
}
```

Where `run_single_job` looks like:

```rust
fn run_single_job(job: &BatchJob, config: &BatchRunnerConfig) -> Result<()> {
    let solver_kind = config.solver_name.parse::<SolverKind>()?;
    let solver_impl = solver_kind.build_solver();
    let lp_solver_kind = LpSolverKind::from_str(config.lp_solver_name)?; // for OPF
    let lp_solver_impl = lp_solver_kind.build_solver();

    let network = importers::load_grid_from_arrow(job.grid_file.to_str().unwrap())?;

    let out_base = build_output_path(&config.output_root, job);
    // partitions: typically include "scenario_id"
    let partitions = &config.partitions;

    match job.task_kind {
        TaskKind::PfDc => {
            power_flow::dc_power_flow(&network, solver_impl.as_ref(), &out_base, partitions)
        }
        TaskKind::PfAc => {
            power_flow::ac_power_flow(
                &network,
                solver_impl.as_ref(),
                config.tol,
                config.max_iter,
                &out_base,
                partitions,
            )
        }
        TaskKind::OpfDc => {
            power_flow::dc_opf(
                &network,
                solver_impl.as_ref(),
                lp_solver_impl.as_ref(),
                &out_base,
                partitions,
            )
        }
        TaskKind::OpfAc => {
            power_flow::ac_opf(
                &network,
                solver_impl.as_ref(),
                lp_solver_impl.as_ref(),
                config.tol,
                config.max_iter,
                &out_base,
                partitions,
            )
        }
    }
}
```

**Note:** `build_output_path(...)` should follow existing conventions: probably a directory path, and `persist_dataframe` will add stage names (`pf-dc.parquet`, `opf-ac.parquet`) internally.

---

## 4. Output layout & partitioning (`io.rs`)

### 4.1 Partitioning conventions

Re-use `gat-algo::io::persist_dataframe` partition mechanism:

* `output_file` is a base path, say:

  ```
  out/batch/pf_dc.parquet
  ```

* `partitions` is a list of column names; for `gat batch` the crucial one is `scenario_id` (and later `time`).

* Internally, `persist_dataframe` will group by those columns and write:

  ```
  out/batch/pf_dc/scenario_id=<id>/part-*.parquet
  ```

So for `BatchRunnerConfig`:

* For v0, default `partitions = vec!["scenario_id".to_string()]`.
* For v1, you’ll add `"time"`.

### 4.2 Additional batch manifest

It’s handy to write a **batch manifest** summarizing all jobs:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct BatchJobRecord {
    pub job_id: String,
    pub scenario_id: String,
    pub time: Option<DateTime<Utc>>,
    pub task_kind: String,
    pub grid_file: String,
    pub status: String, // "ok" | "error"
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchManifest {
    pub created_at: DateTime<Utc>,
    pub task_kind: String,
    pub num_jobs: usize,
    pub jobs: Vec<BatchJobRecord>,
}
```

After running, write:

```rust
write_manifest_json(&manifest, output_root.join("batch_manifest.json"))?;
```

This gives `gat runs` or orchestrator a simple way to introspect what happened.

---

## 5. CLI integration: `gat batch`

### 5.1 Extend CLI enum (`crates/gat-cli/src/cli.rs`)

Add:

```rust
#[derive(Subcommand, Debug)]
pub enum BatchCommands {
    /// Run DC/AC power flow for many scenarios
    Pf {
        /// Task: "dc" or "ac"
        #[arg(long, default_value = "dc")]
        mode: String,

        /// Scenario manifest (JSON or Parquet)
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: String,

        /// Output root for batch results
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        out: String,

        /// Solver type, e.g. "default"
        #[arg(long, default_value = "default")]
        solver: String,

        /// Threads (global) for solver
        #[arg(long, default_value = "0")]
        threads: String,

        /// Partitions, comma-separated list of column names for Parquet partitioning
        #[arg(long)]
        out_partitions: Option<String>,

        /// Tolerance for AC PF
        #[arg(long, default_value = "1e-6")]
        tol: f64,

        /// Max iter for AC PF
        #[arg(long, default_value = "50")]
        max_iter: usize,

        /// Maximum jobs to run in parallel (0 => auto)
        #[arg(long, default_value = "0")]
        max_jobs: usize,
    },
    /// Run DC/AC optimal power flow for many scenarios
    Opf {
        /// Task: "dc" or "ac"
        #[arg(long, default_value = "dc")]
        mode: String,

        /// Scenario manifest
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: String,

        /// Output root for batch results
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        out: String,

        #[arg(long, default_value = "default")]
        solver: String,
        #[arg(long, default_value = "clarabel")]
        lp_solver: String,

        #[arg(long, default_value = "0")]
        threads: String,

        #[arg(long)]
        out_partitions: Option<String>,

        #[arg(long, default_value = "1e-6")]
        tol: f64,

        #[arg(long, default_value = "50")]
        max_iter: usize,

        #[arg(long, default_value = "0")]
        max_jobs: usize,
    },
}
```

And in `Commands`:

```rust
    Batch {
        #[command(subcommand)]
        command: BatchCommands,
    },
```

Then in `src/main.rs`, route:

```rust
use crate::commands::batch as command_batch;

// in match:
Some(Commands::Batch { command }) => {
    run_and_log("batch", || command_batch::handle(command))
}
```

### 5.2 Implement `commands/batch/mod.rs`

File: `crates/gat-cli/src/commands/batch/mod.rs`

```rust
use anyhow::Result;
use gat_cli::cli::BatchCommands;

pub mod pf;
pub mod opf;

pub fn handle(command: &BatchCommands) -> Result<()> {
    match command {
        BatchCommands::Pf { .. } => pf::handle(command),
        BatchCommands::Opf { .. } => opf::handle(command),
    }
}
```

Each submodule will:

* Parse CLI args.
* Load scenario manifest via `gat-scenarios`.
* Build `BatchRunnerConfig`.
* Call `gat_batch::run_batch`.
* Use `record_run_timed` for telemetry.

### 5.3 `pf.rs` handler (sketch)

```rust
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::BatchCommands;
use gat_scenarios::{load_scenario_manifest}; // you'd define this
use gat_batch::{jobs_from_scenario_artifacts, BatchRunnerConfig, TaskKind};
use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};

pub fn handle(command: &BatchCommands) -> Result<()> {
    let BatchCommands::Pf {
        mode,
        manifest,
        out,
        solver,
        threads,
        out_partitions,
        tol,
        max_iter,
        max_jobs,
    } = command else {
        unreachable!()
    };

    let start = Instant::now();

    let partitions = parse_partitions(out_partitions.as_ref());
    let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();

    configure_threads(threads);

    let task_kind = match mode.as_str() {
        "dc" => TaskKind::PfDc,
        "ac" => TaskKind::PfAc,
        other => anyhow::bail!("unknown pf mode '{}', expected 'dc' or 'ac'", other),
    };

    let artifacts = load_scenario_manifest(Path::new(manifest))?;
    let jobs = jobs_from_scenario_artifacts(&artifacts, task_kind.clone());

    let config = BatchRunnerConfig {
        jobs,
        output_root: Path::new(out).to_path_buf(),
        partitions,
        tol: *tol,
        max_iter: *max_iter,
        solver_name: solver,
        lp_solver_name: "clarabel", // not used for PF
        threads_per_job: threads.parse().unwrap_or(0),
        max_jobs_in_flight: *max_jobs,
    };

    let res = gat_batch::run_batch(&config);
    record_run_timed(
        out,
        &format!("batch pf {}", mode),
        &[
            ("manifest", manifest),
            ("solver", solver),
            ("threads", threads),
            ("tol", &tol.to_string()),
            ("max_iter", &max_iter.to_string()),
            ("out_partitions", &partition_spec),
            ("max_jobs", &max_jobs.to_string()),
        ],
        start,
        &res,
    );
    res.map(|_| ())
}
```

`opf.rs` is analogous but sets `TaskKind::OpfDc/Ac`, passes `lp_solver`, etc.

---

## 6. Telemetry & `gat runs` integration

You already have solid patterns via `record_run_timed` (used in `pf`, `opf`, `nminus1`, `ts`).

For batch:

* Treat each `gat batch` invocation as **one run** in the `runs/` manifest.
* Optionally include summary fields:

  * `num_jobs`, `num_success`, `num_failed`.

You can do that by:

* Letting `run_batch` return `BatchSummary`.
* Logging these into `record_run_timed` params (as strings).

Example:

```rust
let res = gat_batch::run_batch(&config);
let params = vec![
    ("manifest", manifest),
    ("num_jobs", &summary.num_jobs.to_string()),
    ("num_failed", &summary.num_failed.to_string()),
    // ...
];
record_run_timed(out, "batch pf dc", &params, start, &res);
```

(You can pack summary into `res` type, or compute it separately.)

---

## 7. Tests & fixtures

### 7.1 Unit tests in `gat-batch`

* **`jobs_from_scenario_artifacts`**

  * Build fake `ScenarioArtifact`s, ensure jobs created correctly.

* **`run_single_job` (using a tiny test grid)**

  * Use `test_data/matpower/case9.arrow` with a fake `ScenarioArtifact` pointing to it.
  * Ensure PF/OPF runs and writes output to a temp dir.
  * Confirm `scenario_id` partition exists in Parquet structure (e.g. directory names).

### 7.2 Integration tests in `gat-cli`

Under `crates/gat-cli/tests`:

1. **`batch_pf_dc.rs`**

   * Use minimal scenario manifest with one scenario pointing to `test_data/matpower/case9.arrow`.

   * Run:

     ```rust
     let mut cmd = assert_cmd::Command::cargo_bin("gat").unwrap();
     cmd.args([
         "batch", "pf",
         "--mode", "dc",
         "--manifest", manifest_path,
         "--out", out_dir,
     ]);
     cmd.assert().success();
     ```

   * Check `out_dir/batch-pf-dc` or whatever naming you choose has Parquet and partitions.

2. **`batch_opf_dc.rs`**

   * Similar for DC OPF (if case9 is OPF-capable).

### 7.3 Quick performance sanity

Not strictly a test, but add a dev-only bench or a `scripts/batch-smoke.sh` that:

* Runs `gat scenarios materialize` on a small spec.
* Runs `gat batch pf dc` using the manifest.
* Prints some summary stats.

---

## 8. Docs & examples

### 8.1 README additions

Under “Core CLI workflows” add a section:

````markdown
### Batch scenario runs (`gat batch`)

Once you’ve materialized scenarios with `gat scenarios`, you can run
power flow or OPF for all of them in one go:

```bash
# 1. Materialize scenario grids
gat scenarios materialize \
  --spec examples/scenarios/case9.yaml \
  --grid-file test_data/matpower/case9.arrow \
  --out-dir runs/scenarios/case9

# 2. Run DC power flow for all scenarios
gat batch pf \
  --mode dc \
  --manifest runs/scenarios/case9/scenario_manifest.json \
  --out runs/batch/case9_pf_dc
````

This writes Parquet output partitioned by `scenario_id`, ready for
downstream analysis (e.g. Python/Polars, or `gat analytics`).

```

You can also document the batch spec if you decide to support `--spec batch.yaml` as an alternative to CLI flags.

### 8.2 `docs/` entry for CANOS alignment

Short doc page:

- Explains that `gat batch` ≈ CANOS v0.
- Shows how to go from:
  - `grid.arrow` + `scenarios.yaml` → scenario grids,
  - scenario manifest → PF/OPF batch,
  - Parquet outputs → Power-GNN/KPI pipeline.

---

## 9. Bead-sized tasks for agents

1. **Create `gat-batch` crate with `BatchJob`, `TaskKind`, and `BatchRunnerConfig`.**
2. **Implement `jobs_from_scenario_artifacts` using `gat-scenarios::ScenarioArtifact`.**
3. **Implement `run_single_job` calling `gat-algo::power_flow` functions and writing Parquet.**
4. **Implement `run_batch` with Rayon parallelism and `BatchSummary`.**
5. **Add `BatchCommands` to `gat-cli` and wire `batch::pf` and `batch::opf` handlers.**
6. **Write unit tests for job generation and `run_single_job` with a small test grid.**
7. **Write CLI integration tests for `gat batch pf dc` and `gat batch opf dc`.**
8. **Update README/docs with examples and relationship to scenarios.**
9. **(v1) Extend job creation to support scenario × time by evolving the scenario manifest.**

---

If you’d like, next step could be to nail down **exact module/file skeletons** for `gat-batch` (i.e., stub Rust files with `todo!()` bodies) so you can hand that directly to agents.
```

