OPFData (a.k.a. GridOpt on HF) gives you, for each PGLib grid, **300k load-perturbation samples** and **300k topology-perturbation samples** (random line/trafo/gen removals), all solved with AC-OPF via PowerModels.jl + Ipopt. ([arXiv][1])

I’ll mirror the previous structure: goals → phases → concrete tasks with file paths / flags your agents can implement.

---

## 0. Targets / Success Criteria

For OPFData/GridOpt:

1. **Reproduce** their AC-OPF solutions on a subset of the dataset (e.g. 118, 300, 1354_pegase) for both:

   * **Load-only perturbations**, and
   * **Topology perturbations** (line/trafo/gen removal).

2. **Benchmark** GAT vs their baseline:

   * Per-instance solve time for GAT AC-OPF.
   * Aggregate statistics (median, tail, failures).

3. **Validate numerically**:

   * Cost gap vs their stored objective.
   * Constraint violation metrics (bus power balance, branch limits, gen limits, voltage limits).

4. **Tutorial & docs**:

   * A runnable example + doc page showing “AC-OPF under topology variation with OPFData”.

---

## 1. Dataset Plumbing (GridOpt / OPFData → local folder)

OPFData/GridOpt is shipped as **JSON files**, grouped by grid, with 300k load-only and 300k topology-perturbation samples per grid. ([arXiv][1])

### 1.1 Decide on canonical on-disk layout

Assume user downloads from HF or GCS into:

```text
data/opfdata/
  case118/
    load/
      shard_0001.jsonl
      shard_0002.jsonl
      ...
    topo/
      shard_0001.jsonl
      ...
  case300/
    load/
    topo/
  case1354_pegase/
    load/
    topo/
  ...
```

Each shard can contain many samples; exact naming can follow the HF dataset structure (e.g. `train_load.jsonl`, `train_topology.jsonl`), but at the GAT layer you just want:

* `grid_id` (e.g. `118_ieee` → `case118`),
* `variation_type` in `{load, topo}`,
* file path.

**Task A1:** Define and document this expected layout in a new `gat-io` source module `crates/gat-io/src/sources/opfdata.rs` (module-level doc string).

### 1.2 Optional “sample” dataset entry

HF readme notes 300k+300k per grid; full dataset is huge. ([Hugging Face][2])

Like with PFΔ, you probably want a **small sample** for CI/tutorials.

**Task A2 (optional but nice):**

* Add an entry to `crates/gat-cli/src/dataset.rs`:

```rust
PublicDataset {
    id: "opfdata-sample",
    description: "Small OPFData subset (e.g. 118_ieee load+topology perturbations).",
    url: "<your-own hosted zip of a few thousand JSON lines>",
    filename: "opfdata-sample.zip",
    license: "CC BY 4.0 / GridOpt license",
    tags: &["opf", "ac-opf", "topology", "benchmark"],
    extract: true,
}
```

* Have it unpack into `data/opfdata-sample/` with the layout from A1.

---

## 2. Implement `gat-io` support: OPFData → Network + Reference Solution

Goal: Given a single OPFData JSON record, produce:

* A **GAT `Network`** with the perturbed topology + loads, generator limits/costs etc.
* A **reference OPF solution** (generator dispatch, bus voltages, objective value, reserves etc).

From the HF readme + paper: each example is a solved AC-OPF instance derived from a PGLib network; load and topology are perturbed; solved with PowerModels.jl/Ipopt. ([arXiv][1])

### 2.1 OPFData metadata structs

**File:** `crates/gat-io/src/sources/opfdata.rs`

**Task B1:** Define:

```rust
pub enum OpfDataVariation {
    Load,
    Topology,
}

pub struct OpfDataSampleId {
    pub grid_id: String,            // e.g. "118_ieee"
    pub variation: OpfDataVariation,
    pub shard: String,              // filename or shard id
    pub index_in_shard: usize,      // line index
}

pub struct OpfDataSampleMeta {
    pub sample_id: OpfDataSampleId,
    pub num_buses: usize,
    pub num_branches: usize,
    pub num_gens: usize,
    pub objective: f64,             // reference optimal cost
}

pub struct OpfDataSolution {
    pub vm: HashMap<BusId, f64>,
    pub va: HashMap<BusId, f64>,
    pub pgen: HashMap<GenId, f64>,
    pub qgen: HashMap<GenId, f64>,
    // optionally line flows, duals, etc. if available
}

pub struct OpfDataInstance {
    pub meta: OpfDataSampleMeta,
    pub network: Network,
    pub solution: OpfDataSolution,
}
```

You can refine types once you peek at the actual JSON schema (fields for bus voltages, P/Q injections, etc.).

### 2.2 Shard listing + lazy loading

**Task B2:** Implement:

```rust
pub fn list_shards(root: &Path) -> Result<Vec<(String, OpfDataVariation, PathBuf)>> {
    // e.g. returns (grid_id, variation, path) for each JSONL shard
}
```

**Task B3:** Implement a loader that yields an iterator over samples:

```rust
pub fn iter_samples(
    shard_path: &Path,
    grid_id: &str,
    variation: OpfDataVariation,
) -> Result<impl Iterator<Item = Result<OpfDataInstance>>> {
    // read JSONL line by line, parse each into OpfDataInstance
}
```

JSONL is a good assumption; worst case, if it’s pure JSON arrays, you adapt. The paper says datasets are distributed as JSON in a cloud bucket and are “agnostic to ML framework.” ([arXiv][1])

### 2.3 JSON → `Network` mapping

Each OPFData example is derived from PGLib-OPF, with:

* Node/bus objects,
* Branches, transformers, shunts,
* Generators with cost coefficients and bounds,
* Loads and shunts,
* Topology perturbations: a line/trafo/gen removed (for `topology` set). ([Hugging Face][2])

**Task B4:** Implement:

```rust
fn build_network_from_opfdata(json: &serde_json::Value) -> Result<Network>;
```

Mapping:

* Buses → `Node::Bus(Bus { id, nominal_kv, type, ... })`
* Loads → `Node::Load` with active/reactive power.
* Gens → `Node::Gen` with `p_min/p_max`, `q_min/q_max`, cost curve coefficients (`c2, c1, c0`).
* Branches/transformers → `Edge::Branch(Branch { r, x, b, tap, shift, rate_a, status, ... })`.
* Shunts → `Node::Shunt` or bus-shunt attributes if GAT models them that way.

For topology-perturbed examples:

* Apply **line/trafo/gen removals** by:

  * either skipping those edges/gens when building the network if `status=0` or `removed=true`, or
  * adding them but marking `in_service=false`.

Use PGLib field semantics as a guide for mapping, since OPFData is derived directly from PGLib cases. ([GitHub][3])

### 2.4 Extract reference OPF solution

The JSON will include solved values:

* Generator outputs,
* Bus voltages,
* Branch flows,
* Objective value, etc. ([arXiv][4])

**Task B5:** Implement:

```rust
fn build_solution_from_opfdata(json: &serde_json::Value) -> Result<OpfDataSolution>;
```

Populate the `HashMap`s keyed by whatever `BusId` and `GenId` GAT uses (likely integer indices or canonical IDs matched from the JSON).

---

## 3. GAT CLI benchmark: `gat benchmark opfdata`

Mirror `benchmark pfdelta`, but for AC-OPF.

### 3.1 Extend CLI enum

**File:** `crates/gat-cli/src/cli.rs`

**Task C1:** Add:

```rust
#[derive(Subcommand)]
pub enum BenchmarkCommands {
    // existing entries...

    /// Benchmark AC-OPF on OPFData/GridOpt datasets
    Opfdata {
        /// Root of OPFData/GridOpt dataset
        #[arg(long, value_name = "PATH")]
        opfdata_root: PathBuf,

        /// Grid ID (e.g. 118_ieee, 1354_pegase)
        #[arg(long)]
        grid: String,

        /// Variation type: load or topo
        #[arg(long, value_enum)]
        variation: OpfDataVariationCli, // a small enum convertible into OpfDataVariation

        /// Maximum number of samples to evaluate
        #[arg(long, default_value = "10000")]
        max_samples: usize,

        /// Number of threads (default: num_cpus)
        #[arg(long, default_value = "auto")]
        threads: String,

        /// AC-OPF tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,

        /// Max iterations for the solver
        #[arg(long, default_value = "100")]
        max_iter: usize,

        /// Output CSV file
        #[arg(long)]
        out: PathBuf,
    },
}
```

Wire it in `main.rs` to call a new `commands::benchmark::opfdata::handle(...)`.

### 3.2 Implement `benchmark/opfdata.rs`

**File:** `crates/gat-cli/src/commands/benchmark/opfdata.rs`

**Task C2:** Implement core loop:

```rust
pub fn handle(args: OpfdataArgs) -> Result<()> {
    let shards = opfdata::list_shards(&args.opfdata_root)?;
    let filtered_shards = filter_by_grid_and_variation(shards, &args.grid, args.variation);

    let pool = thread_pool_from_args(&args.threads)?;
    let mut results = Vec::new();

    for shard in filtered_shards {
        for (i, instance_res) in opfdata::iter_samples(&shard.path, &args.grid, args.variation)?.enumerate() {
            if results.len() >= args.max_samples { break; }
            let instance = instance_res?;

            // dispatch to thread pool
            let res = pool.submit(move || benchmark_instance(instance, args.tol, args.max_iter));
            results.push(res);
        }
    }

    // join results, write CSV
}
```

### 3.3 Benchmarking a single instance

You likely already have an `AcOpfSolver` in `gat-algo` (or similar).

**Task C3:** Implement:

```rust
#[derive(Debug, Clone, Serialize)]
struct OpfDataBenchmarkResult {
    grid_id: String,
    variation: String,        // "load" / "topo"
    shard: String,
    index_in_shard: usize,

    load_time_ms: f64,
    solve_time_ms: f64,
    total_time_ms: f64,

    converged: bool,
    iterations: u32,

    num_buses: usize,
    num_branches: usize,
    num_gens: usize,

    objective_ref: f64,
    objective_gat: f64,
    objective_gap_abs: f64,
    objective_gap_rel: f64,

    max_p_balance_violation: f64,
    max_q_balance_violation: f64,
    max_branch_flow_viol: f64,
    max_gen_p_viol: f64,
    max_vm_viol: f64,
}
```

And:

```rust
fn benchmark_instance(
    instance: OpfDataInstance,
    tol: f64,
    max_iter: usize,
) -> Result<OpfDataBenchmarkResult> {
    let OpfDataInstance { meta, network, solution: ref_sol } = instance;

    let t0 = Instant::now();
    let net = network; // maybe clone or into_owned
    let t1 = Instant::now();

    // run GAT AC-OPF
    let (opf_sol, stats) = ac_opf_solve(&net, tol, max_iter)?;
    let t2 = Instant::now();

    // extract objective_gat from opf_sol
    // compute constraint violations vs net, opf_sol
    // compute objective gaps vs meta.objective
    // compute P/Q balance, branch flows, gen limits, voltage limits violations

    Ok(OpfDataBenchmarkResult {
        grid_id: meta.sample_id.grid_id,
        variation: format!("{:?}", meta.sample_id.variation),
        shard: meta.sample_id.shard,
        index_in_shard: meta.sample_id.index_in_shard,
        load_time_ms: (t1 - t0).as_secs_f64() * 1000.0,
        solve_time_ms: (t2 - t1).as_secs_f64() * 1000.0,
        total_time_ms: (t2 - t0).as_secs_f64() * 1000.0,
        converged: stats.converged,
        iterations: stats.iterations as u32,
        num_buses: meta.num_buses,
        num_branches: meta.num_branches,
        num_gens: meta.num_gens,
        objective_ref: meta.objective,
        objective_gat,
        objective_gap_abs,
        objective_gap_rel,
        max_p_balance_violation,
        max_q_balance_violation,
        max_branch_flow_viol,
        max_gen_p_viol,
        max_vm_viol,
    })
}
```

You can borrow violation computation from anywhere in GAT that already checks KKT/feasibility, or implement a small helper in `gat-algo`.

---

## 4. Link to `gat batch opf` & scenarios (optional but powerful)

In addition to `benchmark opfdata`, you probably want a “plain” workflow:

* **Convert OPFData samples into a GAT scenario manifest**, and
* Run `gat batch opf ac` over them.

### 4.1 Scenario manifest generator

**File:** `examples/opfdata-benchmark/opfdata_to_manifest.rs` (or a small CLI binary in `crates/gat-tools`)

**Task D1:** Tool that:

1. Reads a subset of OPFData samples (e.g., first N of `grid=118_ieee`, `variation=topo`).
2. For each sample, writes a **scenario entry** in a JSON manifest compatible with `gat batch opf ac`:

```jsonc
{
  "base_grid": "grids/pglib_case118.arrow",
  "scenarios": [
    {
      "id": "opfdata_118_topo_000001",
      "perturbations": {
        "loads": [ ... ],
        "branches": [ ... ],
        "gens": [ ... ]
      },
      "target_objective": 12345.67
    },
    ...
  ]
}
```

3. Saves as `manifests/opfdata_118_topo_manifest.json`.

### 4.2 Run batch AC-OPF

Assuming you already have `gat batch opf ac`:

**Task D2:** Example script:

```bash
gat batch opf ac \
  --manifest manifests/opfdata_118_topo_manifest.json \
  --out runs/opfdata_118_topo_batch \
  --max-jobs 8
```

And a small analysis script that compares `target_objective` vs `gat_objective`, plus timings.

This is a nice complement to `benchmark opfdata` and shows “plain” GAT usage.

---

## 5. Tests & CI

### 5.1 Tiny OPFData fixture

**Task E1:** Create a tiny fixture (just a handful of samples) committed under:

```text
test_data/opfdata/case118/load/small.jsonl
test_data/opfdata/case118/topo/small.jsonl
```

Each JSON record should be a *real* OPFData sample (or synthetic equivalent) with just enough info to build a small `Network` and solution.

### 5.2 Unit tests for loader

**File:** `crates/gat-io/tests/opfdata_tests.rs`

**Task E2:** Tests:

* `list_shards` finds `load` and `topo` shards.
* `iter_samples` yields correct `OpfDataInstance`s:

  * `num_buses`, `num_branches`, `num_gens` match expectations.
  * A couple of bus voltage / generator values match the JSON.

### 5.3 Integration test for CLI

**File:** `crates/gat-cli/tests/opfdata_benchmark.rs`

**Task E3:** Use `assert_cmd`:

* Set `OPFDATA_ROOT` to a temp dir with `test_data/opfdata/...`.
* Run:

```rust
Command::cargo_bin("gat")?
    .args([
        "benchmark", "opfdata",
        "--opfdata-root", root,
        "--grid", "case118",
        "--variation", "topo",
        "--max-samples", "5",
        "--tol", "1e-6",
        "--max-iter", "50",
        "--threads", "1",
        "--out", out_csv,
    ])
    .assert()
    .success();
```

* Parse CSV:

  * Check there are 5 rows.
  * Validate `objective_gap_rel` is below some small threshold (e.g. `< 1e-4`) for the fixture.
  * Ensure `converged` is true and violation metrics are small.

---

## 6. Analysis + Tutorial

### 6.1 Analysis script

**File:** `examples/opfdata-benchmark/analyze_opfdata_benchmark.py`

**Task F1:** Script that:

* Reads one or more `*_gat.csv` files.

* Outputs:

  * Histogram / stats of `objective_gap_rel`.
  * Distribution of `max_branch_flow_viol`, etc.
  * Solve time vs grid size; separate curves for `variation=load` vs `variation=topo`.

* Optionally: overlay their **baseline timings** if you extract them from the original AC-OPF runs (using their JAX/Julia tooling) – but even without that, you get a nice “GAT AC-OPF under topology variation” story.

### 6.2 Docs page

**File:** `docs/src/tutorials/opfdata_acopf_topology.md`

**Task F2:** Narrative structure:

1. **What is OPFData/GridOpt & why topology perturbations matter.** ([arXiv][1])
2. **Download dataset**: link to HF and explain folder layout.
3. **Run benchmark**:

   * `gat benchmark opfdata --grid 118_ieee --variation topo ...`.
4. **Inspect results**:

   * Show CSV columns, Polars snippet to compute average objective gap and runtime.
5. **Topology variation angle**:

   * Group by `variation` and show that GAT handles “load only” and “topology removal” with similar reliability.
6. **Next steps**:

   * How to use the same dataset with `gat featurize gnn` and ML models (OPFDataset / SafePowerGraph / CANOS). ([PyTorch Geometric][5])

### 6.3 README link

**Task F3:** Add a short “AC-OPF Benchmarks” section to the main `README.md`:

* Mention OPFData/GridOpt, link to the tutorial.
* Include a copy-paste command for running a small benchmark on 118_ieee topology perturbations.

---

## 7. How this serves your A/B/C goals for this paper

* **A. Reproduce their results:**

  * You directly reuse their networks and perturbations.
  * You compare the GAT AC-OPF objective to their stored optimal cost and track feasibility violations.

* **B. Benchmark GAT speed:**

  * `benchmark opfdata` yields per-sample solve times; you can compare vs their reported runtimes or JAX/Julia baselines.

* **C. Turn into a tutorial:**

  * Small fixture + docs page + example scripts give new users a concrete “AC-OPF under topology variation” playground, backed by a widely cited dataset.

[1]: https://arxiv.org/html/2406.07234v1?utm_source=chatgpt.com "OPFData: Large-scale datasets for AC optimal power flow ..."
[2]: https://huggingface.co/datasets/AI4Climate/OPFData/blob/main/README "README · AI4Climate/OPFData at main"
[3]: https://github.com/power-grid-lib/pglib-opf?utm_source=chatgpt.com "power-grid-lib/pglib-opf: Benchmarks for the Optimal ..."
[4]: https://arxiv.org/pdf/2406.07234?utm_source=chatgpt.com "OPFData: Large-scale datasets for AC optimal power flow ..."
[5]: https://pytorch-geometric.readthedocs.io/en/2.6.0/_modules/torch_geometric/datasets/opf.html?utm_source=chatgpt.com "torch_geometric.datasets.opf - PyTorch Geometric"
