---

## 0. Targets / Success Criteria

Use **PGLib-OPF** as a standard centralized benchmark suite:

1. **Coverage**

   * Run GAT’s AC-OPF across a curated subset of PGLib cases spanning:

     * *small* (3, 5, 14, 30, 57, 118),
     * *medium* (300, 1354_pegase, 2000_goc, 2383wp_k),
     * *large* (4661_sdet, 6495_rte, 9241_pegase, 6515_rte, etc.).([GitHub][1])

2. **Quality**

   * Compare GAT’s objective value and feasibility against:

     * PGLib baseline solutions (BASELINE.md), and/or
     * Some external solver (Matpower, PowerModels, OPOMO) run offline.([GitHub][1])

3. **Performance**

   * Produce per-case statistics:

     * Solve time, iterations, convergence flags.
     * Size metrics (#buses, #branches, #gens).
   * Optional: match OPOMO-style performance profiles.([GitHub][2])

4. **User-Facing**

   * A single `gat benchmark pglib` command.
   * A docs page/README section and a tiny fixture for CI.

---

## 1. Dataset plumbing: PGLib → local folder

PGLib provides all cases as MATPOWER `.m` files plus a `BASELINE.md` summarizing reference AC-OPF results.([GitHub][1])

### 1.1 Expected layout

Assume the user checks out PGLib somewhere and points GAT at it:

```text
data/pglib-opf/
  pglib_opf_case3_lmbd.m
  pglib_opf_case5_pjm.m
  pglib_opf_case14_ieee.m
  ...
  BASELINE.md
```

**Task A1** (docs, not code):
Document in the CLI help and tutorial that `--pglib-root` should point at such a directory.

### 1.2 Optional: small curated subset in repo

For CI / examples you don’t want the full repo, so:

**Task A2:** Under `test_data/datasets/pglib/`, stage a tiny subset of MATPOWER files:

```text
test_data/datasets/pglib/
  pglib_opf_case5_pjm.m
  pglib_opf_case14_ieee.m
  pglib_opf_case30_ieee.m
  baseline.csv         # your own small baseline csv (see below)
```

This can be a manual copy from PGLib v23.07 (or whatever version you standardize on).([GitHub][1])

### 1.3 Optional: dataset helper

**File:** `crates/gat-cli/src/dataset.rs`

**Task A3:** Add a helper similar to RTS-GMLC:

```rust
pub fn fetch_pglib_subset(out: &Path) -> Result<()> {
    let src_dir = repo_data_dir().join("pglib");
    if !src_dir.exists() {
        return Err(anyhow!("PGLib subset not staged in repo"));
    }
    fs::create_dir_all(out)?;
    for entry in fs::read_dir(src_dir)? {
        let ent = entry?;
        let src = ent.path();
        let dst = out.join(
            src.file_name()
                .ok_or_else(|| anyhow!("bad pglib filename"))?,
        );
        fs::copy(&src, &dst)
            .with_context(|| format!("copying {} to {}", src.display(), dst.display()))?;
    }
    println!("PGLib subset ready at {}", out.display());
    Ok(())
}
```

This gives you a one-liner in docs: `gat dataset fetch-pglib-subset`.

---

## 2. Network import: MATPOWER → `Network`

GAT already has a MATPOWER importer (`crates/gat-io/src/importers/matpower.rs`) and uses it elsewhere; you don’t need new IO formats.

**Task B1:** Confirm the importer successfully parses a PGLib case (e.g. `pglib_opf_case14_ieee.m`) into `Network` with consistent bus/gen/branch counts. If needed, extend MATPOWER parsing to handle any PGLib quirks (extra fields, cost curves, etc.).([GitHub][1])

If any PGLib fields are not yet mapped (e.g., shunt data, tighter angle diff limits), add them to `Network` and the importer so GAT sees the “official” formulation PGLib defines.([arXiv][3])

---

## 3. Benchmark CLI: `gat benchmark pglib`

Parallel the existing PFΔ benchmark structure.

### 3.1 CLI enum extension

**File:** `crates/gat-cli/src/cli.rs`

You currently have `BenchmarkCommands::Pfdelta { ... }`. Add:

```rust
#[derive(Subcommand, Debug)]
pub enum BenchmarkCommands {
    /// Run PFDelta AC OPF benchmark suite
    Pfdelta { /* existing fields */ },

    /// Run centralized AC-OPF benchmarks on PGLib-OPF cases
    Pglib {
        /// Root directory containing PGLib-OPF .m files
        #[arg(long, value_hint = ValueHint::DirPath)]
        pglib_root: String,

        /// Case filter: substring or exact case name (e.g. "case118", "pglib_opf_case118_ieee")
        #[arg(long)]
        case: Option<String>,

        /// Case size filter: small, medium, large, or all
        #[arg(long, default_value = "all")]
        size: String,

        /// Maximum number of cases to run (0 = all)
        #[arg(long, default_value_t = 0)]
        max_cases: usize,

        /// Optional baseline CSV (objectives/feasibility from another solver)
        #[arg(long, value_hint = ValueHint::FilePath)]
        baseline: Option<String>,

        /// Output CSV for benchmark results
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,

        /// Number of parallel solver threads (auto = CPU count)
        #[arg(long, default_value = "auto")]
        threads: String,

        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,

        /// Maximum AC solver iterations
        #[arg(long, default_value_t = 100)]
        max_iter: u32,
    },
}
```

### 3.2 Benchmark dispatcher

**File:** `crates/gat-cli/src/commands/benchmark/mod.rs`

**Task C2:** Extend `handle`:

```rust
pub mod pfdelta;
pub mod pglib;

pub fn handle(command: &BenchmarkCommands) -> Result<()> {
    match command {
        BenchmarkCommands::Pfdelta { /* ... */ } => { /* existing */ }

        BenchmarkCommands::Pglib {
            pglib_root,
            case,
            size,
            max_cases,
            baseline,
            out,
            threads,
            tol,
            max_iter,
        } => pglib::handle(
            pglib_root,
            case.as_deref(),
            size,
            *max_cases,
            baseline.as_deref(),
            out,
            threads,
            *tol,
            *max_iter,
        ),
    }
}
```

### 3.3 Implement `commands/benchmark/pglib.rs`

**File:** `crates/gat-cli/src/commands/benchmark/pglib.rs` (new)

**Task C3:** Define the benchmark result record:

```rust
use anyhow::{anyhow, Context, Result};
use csv::Writer;
use rayon::prelude::*;
use serde::Serialize;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Instant;

use gat_algo::AcOpfSolver;
use gat_io::importers::matpower::load_matpower_network;

#[derive(Debug, Clone, Serialize)]
struct PglibBenchmarkResult {
    case_name: String,
    variant: String,                 // base/api/sad if you choose to encode it
    size_class: String,              // small/medium/large
    load_time_ms: f64,
    solve_time_ms: f64,
    total_time_ms: f64,
    converged: bool,
    iterations: u32,
    num_buses: usize,
    num_branches: usize,
    num_gens: usize,
    objective_gat: f64,
    objective_baseline: Option<f64>,
    objective_gap_abs: Option<f64>,
    objective_gap_rel: Option<f64>,
    max_p_balance_violation: f64,
    max_q_balance_violation: f64,
    max_branch_flow_viol: f64,
    max_gen_p_viol: f64,
    max_vm_viol: f64,
}
```

**Task C4:** Implement core `handle`:

1. **Discover cases** in `pglib_root`:

   * Walk directory, collect `pglib_opf_case*.m`.
   * Infer size class (e.g., small < 200 buses; 200–2000 medium; >2000 large) using a quick pass that counts buses or via a precomputed map.

2. Apply `case` substring filter and `size` filter; enforce `max_cases`.

3. **Configure rayon pool** from `threads` (copy the pattern from `pfdelta.rs`).

4. For each selected case, run `benchmark_case(...)` in parallel, collect `PglibBenchmarkResult` rows, and write to CSV `out`.

Skeleton:

```rust
pub fn handle(
    pglib_root: &str,
    case_filter: Option<&str>,
    size_filter: &str,
    max_cases: usize,
    baseline_csv: Option<&str>,
    out: &str,
    threads: &str,
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    // set up thread pool as in pfdelta
    // discover cases
    // load baseline map if provided
    // par_iter over cases -> benchmark_case
    // write CSV
}
```

---

## 4. Baseline data & feasibility checks

### 4.1 Optional baseline CSV ingest

You can either:

* Parse PGLib’s BASELINE.md (slightly annoying to scrape), or
* Expect the user to provide a **simple CSV** with `case_name, objective` (e.g., exported from PowerModels/MATPOWER/OPOMO).([GitHub][1])

**Task D1:** Implement a helper:

```rust
fn load_baseline_objectives(path: &Path) -> Result<HashMap<String, f64>> { /* ... */ }
```

* Simple CSV: `case_name,objective` (no header assumptions beyond that).

Within `benchmark_case`, look up `baseline_map.get(&case_name)` and fill the objective gap fields.

### 4.2 AC-OPF solve + metrics

**Task D2:** Implement:

```rust
fn benchmark_case(
    case_path: &Path,
    size_class: &str,
    baseline_obj: Option<f64>,
    tol: f64,
    max_iter: u32,
) -> Result<PglibBenchmarkResult> {
    let case_name = case_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("invalid case filename"))?
        .to_string();

    let t0 = Instant::now();
    let network = load_matpower_network(case_path)
        .with_context(|| format!("loading {}", case_path.display()))?;
    let t1 = Instant::now();

    let num_buses = network.buses().len();
    let num_branches = network.branches().len();
    let num_gens = network.generators().len();

    let mut solver = AcOpfSolver::new(tol, max_iter, /* maybe more config */);
    let (opf_solution, stats) = solver
        .solve(&network)
        .with_context(|| format!("solving {}", case_name))?;
    let t2 = Instant::now();

    let objective_gat = opf_solution.objective_value();

    // Feasibility metrics – helper in gat_algo, or implement here:
    let (max_p_viol, max_q_viol, max_line_viol, max_gen_p_viol, max_vm_viol) =
        compute_violations(&network, &opf_solution);

    let (objective_gap_abs, objective_gap_rel) = if let Some(obj_ref) = baseline_obj {
        let abs_gap = (objective_gat - obj_ref).abs();
        let rel_gap = if obj_ref.abs() > 1e-6 {
            abs_gap / obj_ref.abs()
        } else {
            0.0
        };
        (Some(abs_gap), Some(rel_gap))
    } else {
        (None, None)
    };

    Ok(PglibBenchmarkResult {
        case_name,
        variant: "base".to_string(), // optional: detect api/sad from folder
        size_class: size_class.to_string(),
        load_time_ms: (t1 - t0).as_secs_f64() * 1000.0,
        solve_time_ms: (t2 - t1).as_secs_f64() * 1000.0,
        total_time_ms: (t2 - t0).as_secs_f64() * 1000.0,
        converged: stats.converged,
        iterations: stats.iterations as u32,
        num_buses,
        num_branches,
        num_gens,
        objective_gat,
        objective_baseline: baseline_obj,
        objective_gap_abs,
        objective_gap_rel,
        max_p_balance_violation: max_p_viol,
        max_q_balance_violation: max_q_viol,
        max_branch_flow_viol: max_line_viol,
        max_gen_p_viol,
        max_vm_viol,
    })
}
```

**Task D3:** Either:

* Reuse an existing feasibility routine from `gat-algo` (if there’s something like `check_acopf_feasibility`), or
* Implement `compute_violations` there (simple loop computing max constraint residuals).

---

## 5. Tests & CI

### 5.1 Tiny PGLib fixture

Already done in **A2**. For CI, use only 3–4 very small cases.

### 5.2 Unit test: MATPOWER import of PGLib

**File:** `crates/gat-io/src/importers/tests.rs` or new `tests/pglib_import.rs`.

**Task E1:** Add a test that:

* Loads `test_data/datasets/pglib/pglib_opf_case14_ieee.m`.
* Asserts bus/branch/gen counts match comments in the case file (e.g., 14 buses, 20 branches, etc.).([GitHub][1])

### 5.3 CLI integration test

**File:** `crates/gat-cli/tests/benchmark_pglib.rs` (new)

**Task E2:** Use `assert_cmd` to run:

```rust
Command::cargo_bin("gat")?
    .args([
        "benchmark", "pglib",
        "--pglib-root", pglib_root,
        "--size", "small",
        "--max-cases", "2",
        "--out", out_csv.to_str().unwrap(),
        "--threads", "1",
        "--tol", "1e-6",
        "--max-iter", "50",
    ])
    .assert()
    .success();
```

Then:

* Parse `out_csv` with `csv` crate.
* Assert:

  * 2 rows.
  * `converged == true` for both.
  * Feasibility metrics under some threshold.
  * If you provide tiny baseline CSV, check `objective_gap_rel < 1e-4` or similar.

---

## 6. Example scripts & docs

### 6.1 Example shell script

**File:** `examples/pglib-benchmark/run_pglib_small.sh`

**Task F1:** Minimal script:

```bash
#!/usr/bin/env bash
set -euo pipefail

PGLIB_ROOT="${PGLIB_ROOT:-data/pglib-opf}"
OUT_DIR="${OUT_DIR:-results/pglib}"

mkdir -p "$OUT_DIR"

gat benchmark pglib \
  --pglib-root "$PGLIB_ROOT" \
  --size small \
  --max-cases 20 \
  --threads auto \
  --tol 1e-6 \
  --max-iter 100 \
  --out "$OUT_DIR/pglib_small_gat.csv"
```

Optional: second script for “large” cases.

### 6.2 Analysis script

**File:** `examples/pglib-benchmark/analyze_pglib_benchmark.py`

**Task F2:** Python script (Polars/Pandas) that:

* Reads `pglib_small_gat.csv` (and optionally another solver’s CSV).
* Computes:

  * Histogram of `objective_gap_rel`.
  * Mean/median solve time vs `num_buses` (scatter plot).
  * Count of convergence failures.

If you also export **baseline solver** CSV from OPOMO / PowerModels / MATPOWER, you can join on `case_name` and draw performance profiles like in OPOMO’s README.([GitHub][2])

### 6.3 Tutorial doc

**File:** `docs/src/tutorials/pglib_acopf_benchmarks.md`

**Task F3:** Outline:

1. **What is PGLib-OPF?** (link to GitHub + task force report).([GitHub][1])
2. **Downloading the cases**:

   * `git clone https://github.com/power-grid-lib/pglib-opf` or stage subset.
3. **Running GAT benchmarks**:

   * Show `gat benchmark pglib --pglib-root ...`.
4. **Understanding outputs**:

   * Explain CSV fields (objective, violations, iterations).
5. **Comparing against other solvers**:

   * How to feed a baseline CSV and interpret objective gaps.
6. **Scaling up**:

   * Example: run on all `size=large` cases and inspect runtime vs system size.

### 6.4 README link

**Task F4:** Add a short “Centralized AC-OPF Benchmarks” subsection in `README.md` that:

* Mentions GAT supports running AC-OPF on PGLib cases and exporting results.
* Links to the new tutorial.
* Includes a one-liner command.

---

## 7. How this fits your validation story

* **A. Reproduce results / validate**

  * PGLib is the de facto standard AC-OPF benchmark suite; you use the exact same MATPOWER case files and (optionally) baseline objectives.

* **B. Benchmark speed**

  * `gat benchmark pglib` gives per-case runtime and iterations over a standard set comparable to OPOMO, PowerModels, etc. ([GitHub][2])

* **C. Tutorial / onboarding**

  * New users can grab a couple of PGLib cases, run a single GAT command, and instantly see how the solver behaves on “real” benchmarks—then pivot from here into the more involved PFΔ / OPFData demos.


[1]: https://github.com/power-grid-lib/pglib-opf?utm_source=chatgpt.com "power-grid-lib/pglib-opf: Benchmarks for the Optimal ..."
[2]: https://github.com/hhijazi/OPOMO?utm_source=chatgpt.com "hhijazi/OPOMO: Fast and reliable solver for the Optimal ..."
[3]: https://arxiv.org/abs/1908.02788?utm_source=chatgpt.com "The Power Grid Library for Benchmarking AC Optimal Power Flow Algorithms"
