Here’s a full implementation roadmap for **`gat analytics ds`** as a Deliverability Score engine, wired into the existing GAT crates and CLI.

I’ll assume we’re doing a **practical v0** that:

* Uses **DC** approximations (PTDF/flows).
* Works **per bus** (RA resource = capacity at a bus, using `limits.csv`).
* Uses one or more **“stress” flow snapshots** (from DC PF/OPF), optionally with `scenario_id` / `time` columns.
* Produces **per-bus DS metrics** in Parquet, ready for pipelines.

---

## 0. Semantics for DS v0

For this roadmap, define **Deliverability Score (DS)** roughly as:

> For each bus (i), and for each stress case (scenario/time), how much additional injection (\Delta P_i) (up to its nameplate (P_{i,\max})) can we push from bus (i) to the rest of the system before we hit **branch limits**, under DC assumptions? DS is the **fraction of nameplate** that is deliverable on average (or on some quantile) across stress cases.

Formally (v0, DC, single sink):

* Input:

  * Network (N) with branches and reactances.
  * Branch limits (F_\ell) from `branch_limits.csv`.
  * Bus-level capacity (P_{i,\max}) from `limits.csv`.
  * For each stress case (k):

    * Branch flows (f_{\ell,k}) from DC PF or DC-OPF solutions.
* For a candidate bus (i) and stress case (k):

  * Compute PTDF vector for **injection at bus (i)** and **withdrawal at a chosen sink bus** (e.g. reference/slack or a “system demand hub”):
    [
    \Delta f_{\ell,k} = \mathrm{PTDF}_{\ell,i} \cdot \Delta P_i
    ]
  * Branch constraint:
    [
    |f_{\ell,k} + \mathrm{PTDF}*{\ell,i} \cdot \Delta P_i| \le F*\ell
    ]
  * Solve for the max feasible (\Delta P_i) across branches:
    [
    \Delta P_{i,k}^{\max} = \min_{\ell} \Delta P_{\ell,k}^{\max}
    ]
    (closed-form per branch; we’ll derive below in implementation).
  * Define **scenario-level DS**:
    [
    \mathrm{DS}*{i,k} = \min\left(1,; \frac{\Delta P*{i,k}^{\max}}{P_{i,\max}}\right)
    ]
* Aggregate across stress cases (weighting by optional scenario weights):

  * E.g. simple mean:
    [
    \mathrm{DS}*i = \sum_k w_k \mathrm{DS}*{i,k}
    ]

We’ll design the analytics engine to compute both:

* **Per-scenario DS** (for debugging / research).
* **Aggregated DS** per bus.

---

## 1. Where code lives

To minimize sprawl and match current patterns:

* **Keep implementation in `gat-algo`** (as another analytics function alongside `ptdf_analysis`), and
* Expose it via **`gat-cli` under `AnalyticsCommands::Ds`**.

You can factor out into a dedicated `analytics/ds.rs` module *inside* `gat-algo` if `power_flow.rs` is getting too big; but I’ll describe it as a new module for clarity.

### 1.1 New module in `gat-algo`

Add:

* `crates/gat-algo/src/analytics_ds.rs` – DS engine.
* Re-export in `lib.rs`.

`crates/gat-algo/src/lib.rs`:

```rust
pub mod io;
pub mod power_flow;
pub mod test_utils;
pub mod analytics_ds;

pub use io::*;
pub use power_flow::*;
pub use analytics_ds::*;
```

---

## 2. Inputs & outputs for DS

### 2.1 Inputs (to the library API)

We’ll make a function like:

```rust
pub fn deliverability_scores_dc(
    network: &Network,
    limits_csv: &str,
    branch_limits_csv: &str,
    flows_parquet: &Path,
    output_file: &Path,
    partitions: &[String],
    sink_bus: usize,
    agg_mode: DsAggregationMode,
) -> Result<()>
```

Where:

* `network`: base `Network` (from Arrow).
* `limits_csv`: same CSV used by DC OPF:

  * Schema: `bus_id, pmin, pmax, demand`.
* `branch_limits_csv`: same CSV used by OPF (if you use it):

  * Schema: `branch_id, flow_limit`.
* `flows_parquet`: DC PF/OPF branch flows, with columns:

  * `branch_id`, `from_bus`, `to_bus`, `flow_mw`,
  * optionally `scenario_id`, `time`, etc.
* `output_file`: base Parquet path (we’ll partition by `bus_id`, `scenario_id`, etc.).
* `partitions`: partition column names (e.g. `["bus_id", "scenario_id"]`).
* `sink_bus`: bus ID used as sink in PTDF (or 0 / slack bus).
* `agg_mode`: how to aggregate DS across stress cases.

### 2.2 Outputs

We’ll write a **Parquet table** with:

Mandatory columns:

* `bus_id` (usize / i64)
* `ds_raw` (per-scenario DS; if aggregated, we can still store aggregated result)
* `pmax_mw`
* `scenario_id` (if present in flows)
* `time` (if present)

Optional aggregated view:

* If `agg_mode` is enabled, add:

  * `ds_mean`
  * `ds_p05`, `ds_p50`, `ds_p95` (optional)

To keep things simple, I’d do:

* **One stage**: `analytics-ds` (like `analytics-ptdf`), with:

  * If scenario/time columns exist:

    * output includes both per-case DS and aggregated DS per bus.
  * If not, just one row per bus.

We’ll route this through `persist_dataframe` with a new `OutputStage` variant.

---

## 3. Implementing DS in `gat-algo`

### 3.1 Extend `OutputStage` in `io.rs`

Add new variant:

```rust
pub enum OutputStage {
    PfDc,
    PfAc,
    OpfDc,
    OpfAc,
    Nminus1Dc,
    SeWls,
    AnalyticsPtdf,
    AnalyticsDs,           // NEW
}

impl OutputStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputStage::PfDc => "pf-dc",
            OutputStage::PfAc => "pf-ac",
            OutputStage::OpfDc => "opf-dc",
            OutputStage::OpfAc => "opf-ac",
            OutputStage::Nminus1Dc => "nminus1-dc",
            OutputStage::SeWls => "se-wls",
            OutputStage::AnalyticsPtdf => "analytics-ptdf",
            OutputStage::AnalyticsDs => "analytics-ds",    // NEW
        }
    }
}
```

We’ll use `"analytics-ds"` as subdirectory name.

### 3.2 DS aggregation mode enum

In `analytics_ds.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DsAggregationMode {
    /// Only per-case DS (scenario/time) is written
    PerCaseOnly,
    /// Also aggregate across all cases for each bus
    Mean,
    /// Later: Quantiles, etc.
}
```

We can make this configurable from CLI via strings.

### 3.3 Helper: load limits & branch limits

We’ll reuse patterns from `power_flow.rs`, but re-export them or replicate in `analytics_ds.rs`.

Option 1: **Expose helpers**:

* In `power_flow.rs`, make `load_limits` and `load_branch_limits` `pub(crate)` in a new `limits.rs` module and import there.
* For roadmap simplicity, assume we move them to `crates/gat-algo/src/limits.rs`.

Example minimal helper in `analytics_ds.rs` (if you just copy):

```rust
#[derive(Deserialize)]
struct LimitRecord {
    bus_id: usize,
    pmin: f64,
    pmax: f64,
    demand: f64,
}

#[derive(Deserialize)]
struct BranchLimitRecord {
    branch_id: i64,
    flow_limit: f64,
}

fn load_limits(path: &str) -> Result<Vec<LimitRecord>> { /* clone of power_flow.rs code */ }
fn load_branch_limits(path: &str) -> Result<HashMap<i64, f64>> { /* same as power_flow.rs */ }
```

But better long-term is to share the implementation; you can refactor once DS is working.

### 3.4 Helper: load flows Parquet

Use Polars to load the flows table:

```rust
use polars::prelude::*;

fn load_flows(path: &Path) -> Result<DataFrame> {
    let lf = LazyFrame::scan_parquet(path.to_str().unwrap(), Default::default())
        .context("opening flows parquet for DS")?;
    let df = lf.collect().context("collecting flows dataframe")?;
    // Expect at least these columns: branch_id, flow_mw
    if !df.get_column_names().iter().any(|c| c == "branch_id") {
        return Err(anyhow!("flows parquet is missing 'branch_id' column"));
    }
    if !df.get_column_names().iter().any(|c| c == "flow_mw") {
        return Err(anyhow!("flows parquet is missing 'flow_mw' column"));
    }
    Ok(df)
}
```

We’ll use any additional columns (`scenario_id`, `time`, etc.) if they exist.

### 3.5 Helper: PTDF row for bus i → sink

We want a **reusable internal function** that returns PTDF coefficients as a `Vec<f64>` aligned with branch ordering.

Reuse existing machinery used by `ptdf_analysis`:

* `build_bus_susceptance` and `compute_dc_angles`.
* `branch_flow_dataframe_with_angles`.

Simpler:

1. Build injections HashMap `{source_bus: +1.0, sink_bus: -1.0}`.
2. Use `compute_dc_angles`.
3. Use `branch_flow_dataframe_with_angles`.
4. Extract `flow_mw` vector; since we used injection of 1 MW, these are PTDF values directly.

In `analytics_ds.rs`:

```rust
use crate::power_flow::{compute_dc_angles, branch_flow_dataframe_with_angles}; 
// may need to mark those as pub(crate) in power_flow.rs

fn ptdf_row_for_bus(
    network: &Network,
    solver: &dyn SolverBackend,
    source_bus: usize,
    sink_bus: usize,
) -> Result<(Vec<i64>, Vec<f64>)> {
    let mut injections = HashMap::new();
    injections.insert(source_bus, 1.0f64);
    injections.insert(sink_bus, -1.0f64);

    let angles = compute_dc_angles(network, &injections, None, solver)?;
    let (df, _, _) = branch_flow_dataframe_with_angles(network, &angles, None)?;

    // Extract branch_ids and PTDF values (flow per 1 MW)
    let branch_ids: Vec<i64> = df
        .column("branch_id")?
        .i64()?
        .into_iter()
        .flatten()
        .collect();
    let ptdf_vals: Vec<f64> = df
        .column("flow_mw")?
        .f64()?
        .into_iter()
        .flatten()
        .collect();

    Ok((branch_ids, ptdf_vals))
}
```

> **Note:** `compute_dc_angles` and `branch_flow_dataframe_with_angles` are currently private; they’ll need to be `pub(crate)` or moved into a shared internal module so `analytics_ds.rs` can import them.

### 3.6 Core DS computation per bus × case

Given:

* vector of `branch_id`s,
* vector of PTDF values `ptdf_l`,
* for each stress case k:

  * branch flows `f_{l,k}`,
  * branch limits `F_l`.

We want:

[
\max \Delta P_i \quad \text{s.t.} \quad |f_{\ell,k} + \mathrm{PTDF}*{\ell,i} \Delta P_i| \le F*\ell
]

For each branch:

* Let `p = ptdf_l`.
* Let `f = flow_lk`.
* We want ( |f + p \Delta P| \le F).

This yields bounds:

* If `p > 0`:

  * ( f + p \Delta P \le F \Rightarrow \Delta P \le (F - f)/p )
  * ( f + p \Delta P \ge -F \Rightarrow \Delta P \ge (-F - f)/p )
* If `p < 0`, inequalities flip.

We only care about **positive injection** (\Delta P ≥ 0). So:

* compute upper bound `ub` per branch:

  * if `p > 0`: `ub = (F - f) / p`
  * if `p < 0`: `ub = (F + f) / -p` (from `-F ≤ f + pΔP`).
* ignore branches where `p == 0` (they don’t constrain DS).
* ignore any `ub <= 0` (no positive capacity on that branch in this case).

Scenario-level capacity limit:

```rust
let delta_p_max = ubs.into_iter().filter(|v| *v > 0.0).fold(f64::INFINITY, f64::min);
if !delta_p_max.is_finite() { delta_p_max = 0.0; }
let ds_case = (delta_p_max / pmax).min(1.0).max(0.0);
```

### 3.7 From flows DataFrame to DS table

Algorithm:

1. **Load inputs**:

   * `limits = load_limits(limits_csv)` → map `bus_id -> pmax`.
   * `branch_limits = load_branch_limits(branch_limits_csv)` → map `branch_id -> F`.
   * `flows_df = load_flows(flows_parquet)`.

2. **Identify grouping keys**:

   * Check if `flows_df` has `scenario_id`, `time` columns.
   * If yes, define `case_key = (scenario_id, time)`; if not, single case.

   In Polars:

   ```rust
   let group_cols: Vec<&str> = vec!["scenario_id", "time"]
       .into_iter()
       .filter(|c| flows_df.get_column_names().iter().any(|name| name == *c))
       .collect();
   ```

3. **Re-shape flows by case**:

   * Group by `group_cols`.
   * For each group, we get a DataFrame with rows `(branch_id, flow_mw, ...)`.

4. **Per bus**:

   * For each bus in `limits` (we’ll treat each as RA resource):

     * Compute `ptdf_row` once (`branch_ids + ptdf_vals`).
     * Build aligned arrays with branch limits and flows for each case (we need a mapping from `branch_id` to index in PTDF vector).

5. **Compute DS per case**:

   * For each case:

     * For each branch in PTDF row:

       * Look up branch limit `F` and case’s flow `f`.
       * Compute `ub` as above.
     * `delta_p_max_case` = min positive `ub`.
     * `ds_case = min(1, delta_p_max_case / pmax)`.

6. **Aggregate over cases** (if `agg_mode` != PerCaseOnly):

   * E.g. mean DS per bus:

     * `ds_mean = Σ_k w_k ds_case / Σ_k w_k` (if we add weights later; for v0 w_k=1).
   * Optionally compute quantiles using Polars’ `quantile`.

7. **Build output DataFrame**:

   * For per-case DS:

     Columns:

     * `bus_id`
     * *`scenario_id` / `time` if present*
     * `ds_case`
     * `pmax_mw`
     * maybe `sink_bus` (for documentation)

   * For aggregated DS (if requested), we can either:

     * Append per-bus aggregate rows with `scenario_id = "__ALL__"` or a separate stage, or
     * Add columns `ds_mean`, `ds_p05`, etc. to the per-bus rows and drop per-case entries.

   For v0, keep it simple:

   * Always output **per-case rows**.
   * If `agg_mode != PerCaseOnly`, add columns `ds_mean` etc., repeating the same aggregate for each case of a bus. Downstream tools can `distinct` if needed.

8. **Write with `persist_dataframe`**:

   * Wrap in a `Result<()>` and call:

     ```rust
     persist_dataframe(
         &mut df_out,
         output_file,
         partitions,
         OutputStage::AnalyticsDs.as_str(),
     )?;
     ```

   * Partition columns likely include `bus_id` and `scenario_id`/`time` as requested via CLI.

### 3.8 Library API function skeleton

In `analytics_ds.rs`:

```rust
use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use gat_core::{Network, solver::SolverBackend};
use polars::prelude::*;

use crate::io::{persist_dataframe, OutputStage};

pub fn deliverability_scores_dc(
    network: &Network,
    solver: &dyn SolverBackend,
    limits_csv: &str,
    branch_limits_csv: &str,
    flows_parquet: &Path,
    output_file: &Path,
    partitions: &[String],
    sink_bus: usize,
    agg_mode: DsAggregationMode,
) -> Result<()> {
    // 1) load limits, branch_limits, flows
    // 2) find group_cols in flows_df
    // 3) for each bus in limits:
    //      - compute PTDF row
    //      - for each case, compute ds_case
    //      - aggregate if requested
    // 4) assemble df_out
    // 5) persist_dataframe(df_out, output_file, partitions, "analytics-ds")
    // 6) println! summary
    todo!()
}
```

You’ll also need to acquire a `SolverBackend` instance from CLI (`SolverKind::parse(...).build_solver()`), analogous to how `analytics ptdf` does it.

---

## 4. CLI integration: `gat analytics ds`

### 4.1 Extend `AnalyticsCommands` in `cli.rs`

Add a new subcommand:

```rust
#[derive(Subcommand, Debug)]
pub enum AnalyticsCommands {
    /// PTDF sensitivity for a source→sink transfer
    Ptdf {
        // existing fields...
    },

    /// Deliverability Score (DS) under DC approximation
    Ds {
        /// Path to the grid data file (Arrow format)
        grid_file: String,

        /// Limits CSV used for DC-OPF (bus_id, pmin, pmax, demand)
        #[arg(long)]
        limits: String,

        /// Branch limits CSV (branch_id, flow_limit)
        #[arg(long)]
        branch_limits: String,

        /// Parquet file with branch flows (DC PF/OPF)
        #[arg(long)]
        flows: String,

        /// Output file path for DS table (Parquet)
        #[arg(short, long, default_value = "ds.parquet")]
        out: String,

        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,

        /// Threading hint (`auto` or integer)
        #[arg(long, default_value = "auto")]
        threads: String,

        /// Solver to use for PTDF computation (gauss, faer, etc.)
        #[arg(long, default_value = "gauss")]
        solver: String,

        /// Sink bus for PTDF (defaults to bus 0 or 1 depending on naming)
        #[arg(long, default_value = "0")]
        sink_bus: usize,

        /// Aggregation mode: "per_case" or "mean"
        #[arg(long, default_value = "mean")]
        agg: String,
    },
}
```

### 4.2 Wire CLI to commands module

In `crates/gat-cli/src/commands/analytics/mod.rs`, extend `handle`:

```rust
pub fn handle(command: &AnalyticsCommands) -> Result<()> {
    match command {
        AnalyticsCommands::Ptdf { .. } => ptdf::handle(command),
        AnalyticsCommands::Ds { .. } => ds::handle(command),  // NEW
    }
}
```

Create file: `crates/gat-cli/src/commands/analytics/ds.rs`.

### 4.3 Implement `commands::analytics::ds::handle`

Following the pattern of `ptdf.rs`:

```rust
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::AnalyticsCommands;
use gat_core::solver::SolverKind;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};

pub fn handle(command: &AnalyticsCommands) -> Result<()> {
    let AnalyticsCommands::Ds {
        grid_file,
        limits,
        branch_limits,
        flows,
        out,
        out_partitions,
        threads,
        solver,
        sink_bus,
        agg,
    } = command else {
        unreachable!()
    };

    let start = Instant::now();

    configure_threads(threads);

    let solver_kind = SolverKind::from_str(solver)
        .map_err(|_| anyhow::anyhow!("unknown solver '{}'", solver))?;
    let solver_impl = solver_kind.build_solver();

    let partitions = parse_partitions(out_partitions.as_ref());
    let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();

    let agg_mode = match agg.as_str() {
        "per_case" => DsAggregationMode::PerCaseOnly,
        "mean" => DsAggregationMode::Mean,
        other => anyhow::bail!("unknown aggregation mode '{}'", other),
    };

    let res = (|| -> Result<()> {
        let network = gat_io::importers::load_grid_from_arrow(grid_file.as_str())?;
        gat_algo::deliverability_scores_dc(
            &network,
            solver_impl.as_ref(),
            limits.as_str(),
            branch_limits.as_str(),
            Path::new(flows),
            Path::new(out),
            &partitions,
            *sink_bus,
            agg_mode,
        )
    })();

    record_run_timed(
        out,
        "analytics ds",
        &[
            ("grid_file", grid_file),
            ("limits", limits),
            ("branch_limits", branch_limits),
            ("flows", flows),
            ("solver", solver_kind.as_str()),
            ("sink_bus", &sink_bus.to_string()),
            ("agg", agg.as_str()),
            ("out_partitions", partition_spec.as_str()),
        ],
        start,
        &res,
    );
    res
}
```

---

## 5. Tests & fixtures

### 5.1 Unit tests in `gat-algo::analytics_ds`

* Small synthetic network (reuse `test_utils::build_simple_network()` or similar).
* Create dummy `limits.csv` with 2–3 buses, `pmax`.
* Create dummy `branch_limits.csv` with tight limits on one branch.
* Create tiny flows DataFrame (no Parquet needed; you can create `DataFrame` in memory and call an internal `compute_ds_for_bus_and_cases` function).
* Assertions:

  * DS is 1.0 when flows are zero and limits big.
  * DS is < 1.0 when flows near limits.
  * DS is 0.0 when flows saturate at limit and PTDF in wrong direction.

### 5.2 Integration tests via CLI

In `crates/gat-cli/tests/analytics_ds.rs`:

* Use a tiny MATPOWER test case (`test_data/matpower/case9` if available).

* Run DC-OPF with branch limits:

  1. Build `limits.csv` & `branch_limits.csv`.

  2. Run:

     ```bash
     gat opf dc --grid-file case9.arrow --limits limits.csv \
         --costs costs.csv --branch-limits branch_limits.csv \
         --out opf.parquet
     ```

  3. Then run:

     ```bash
     gat analytics ds \
         --grid-file case9.arrow \
         --limits limits.csv \
         --branch-limits branch_limits.csv \
         --flows opf.parquet \
         --out ds.parquet
     ```

* Assertions:

  * `ds.parquet` exists.
  * Load it with Polars and check that each `bus_id` appears at least once, DS values (\in [0,1]), and DS is lower for buses behind tightly constrained branches.

---

## 6. Docs & examples

### 6.1 README section

Add under “Grid analytics helpers”:

````markdown
### Deliverability Score (DS) analytics

`gat analytics ds` computes a DC-approximate **Deliverability Score (DS)** for
each bus, measuring how much additional injection (relative to nameplate)
can be delivered under branch limits across one or more stress cases.

Example:

```bash
# 1. Solve DC-OPF for a set of stress cases (e.g. peak hours)
gat opf dc \
  --grid-file data/case9.arrow \
  --limits examples/case9_limits.csv \
  --costs examples/case9_costs.csv \
  --branch-limits examples/case9_branch_limits.csv \
  --out runs/case9_opf.parquet

# 2. Compute DS for each bus using those flows
gat analytics ds \
  --grid-file data/case9.arrow \
  --limits examples/case9_limits.csv \
  --branch-limits examples/case9_branch_limits.csv \
  --flows runs/case9_opf.parquet \
  --out runs/case9_ds.parquet
````

The output is a Parquet table with per-bus DS, which can be fed into
capacity-accreditation or RA analysis (e.g. DS × ELCC).

```

### 6.2 `docs/` / `gat-mcp-docs`

- Add a short doc describing:
  - DS definition used,
  - Expected input files,
  - Relationship to `limits.csv` and `branch_limits.csv`,
  - Example of using DS in an RA-like calculation.

`gat-mcp-docs` will pick up the new `AnalyticsCommands::Ds` automatically via clap introspection, so you just need to regenerate docs after adding the command.

---

## 7. Bead-sized tasks for agents

1. **Expose DC helpers in `power_flow.rs`**:
   - Make `compute_dc_angles` and `branch_flow_dataframe_with_angles` `pub(crate)` and move into a small internal module if needed.

2. **Add `OutputStage::AnalyticsDs`** and tests for `staged_output_path` + `persist_dataframe` with new stage.

3. **Implement `analytics_ds.rs` skeleton and `DsAggregationMode`.**

4. **Implement `deliverability_scores_dc` core logic**:
   - Load limits, branch limits, flows.
   - Group flows by scenario/time if columns exist.
   - For each bus:
     - Compute PTDF row.
     - Compute `ds_case` per case.
   - Aggregate if requested.
   - Build output DataFrame and call `persist_dataframe`.

5. **Wire up `AnalyticsCommands::Ds` in `cli.rs` and `commands::analytics::ds`.**

6. **Add unit tests in `gat-algo` for DS edge cases.**

7. **Add CLI integration test for `gat analytics ds` on a tiny case.**

8. **Update README/docs with DS section and notes.**

---

If you want, next we can design a small **test fixture** (toy grid + limits + flows) with exact expected DS values, so you can give agents a fully specified “compute and assert DS = X” target.
```

