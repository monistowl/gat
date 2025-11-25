Below is a concrete, agent-parsable plan to get a **PF + contingencies** PFΔ demo working end-to-end, using what’s already in `gat-main` (notably `gat-io::sources::pfdelta` and `gat-cli benchmark pfdelta`).

I’ll structure it as: goals → phases → specific tasks with file paths + example commands.

---

## 0. Targets / Success Criteria

For PFΔ (PFDelta) ([GitHub][1]):

1. **Reproduce** their traditional PF solves on a subset of the dataset (e.g. IEEE-118, N-1) using GAT’s AC power flow.
2. **Benchmark**: generate a CSV with per-case timing and convergence info; compute speedup vs their baseline.
3. **Validate numerically**: per-case error metrics between PFΔ’s stored solution and GAT’s PF (voltage magnitude/angle, branch flows).
4. **Tutorial**: a docs page + example scripts that a new user can follow.

---

## 1. Dataset plumbing (PFΔ → local folder)

### 1.1 Decide on a default layout

Pick a canonical local layout, e.g.:

```text
data/pfdelta/
  ieee14/
    N/
      case_000001.json
      ...
    N-1/
    N-2/
  ieee118/
    N/
    N-1/
    N-2/
  goc500/
  goc2000/
```

PFΔ’s HuggingFace repo already has a structured tree by case + contingency + shard. ([GitHub][1])

**Task A1:** Document the expected on-disk layout in `gat-io/src/sources/pfdelta.rs` module docs.

### 1.2 Simple fetch helper (optional but nice)

You already have dataset helpers (`crates/gat-cli/src/dataset.rs` and `DatasetCommands`). Add a “pointer” for PFΔ:

* **File:** `crates/gat-cli/src/dataset.rs`
* **Task A2:** Add a `PublicDataset` entry for PFΔ *or* a “pfdelta-sample” zip you host somewhere small. Something like:

```rust
PublicDataset {
    id: "pfdelta-sample",
    description: "Small PFΔ subset (e.g. IEEE-118, N and N-1).",
    url: "<your hosted zip or a specific HuggingFace file>",
    filename: "pfdelta-sample.zip",
    license: "CC-BY 4.0",
    tags: &["pf", "contingency", "benchmark"],
    extract: true,
},
```

* **Task A3:** Optionally add a convenience wrapper `import_pfdelta_sample(out: &Path)` that untars/unzips into `data/pfdelta-sample`.

For the **full** dataset, I’d just expect the user to download from HuggingFace or `git lfs` and point `--pfdelta-root` at it; don’t try to automate that in GAT (it’s huge).

---

## 2. Finish / polish the Rust PFΔ loader

There’s already `gat-main/crates/gat-io/src/sources/pfdelta.rs`. The goal:

* **Input:** PFΔ JSON file.
* **Output 1:** `PFDeltaTestCase` metadata struct (case name, contingency type, index, path).
* **Output 2:** `Network` + the **reference PF solution** extracted from the JSON.

### 2.1 `PFDeltaTestCase` + listing

**Task B1:** Define and fully implement:

```rust
pub struct PFDeltaTestCase {
    pub case_name: String,          // e.g. "ieee118"
    pub contingency_type: String,   // "N", "N-1", "N-2"
    pub index: usize,               // case index within that bucket
    pub path: PathBuf,              // JSON file
}

pub fn list_pfdelta_cases(root: &Path) -> Result<Vec<PFDeltaTestCase>> {
    // Walk root, discover cases / contingency types / files.
}
```

* Scan subdirectories for patterns like `*/N/*.json`, `*/N-1/*.json` etc.
* Populate `case_name` from directory name (e.g. `ieee118`), `contingency_type` from `N`, `N-1`, `N-2`, and `index` from file naming or extracted JSON fields.

### 2.2 JSON → `Network` conversion

You already started `convert_pfdelta_to_network(data: &Value) -> Result<Network>`.

**Task B2:** Finish mapping all PFΔ fields to GAT’s `Network`:

* Buses: `bus` object → `Node::Bus(Bus { id, name, voltage_kv })`.
* Lines/transformers: `branch`/`line` sections → `Edge::Branch(Branch { r, x, b, rate_a, tap, shift, status })`.
* Generators: `gen` → `Node::Gen(Gen { p_min, p_max, q_min, q_max, cost? })`.
* Loads: `load` → `Node::Load(Load { p, q })`.

Use PFΔ JSON schema from their repo for field names (`vn`, `r`, `x`, `b`, `rateA`, etc.). ([GitHub][1])

### 2.3 Extract PF reference solution

PFΔ stores solved PF values (bus voltages, gen dispatch, etc.) along with the instance. ([arXiv][2])

**Task B3:** Define something like:

```rust
pub struct PFDeltaSolution {
    pub vm: HashMap<BusId, f64>,
    pub va: HashMap<BusId, f64>,
    pub pgen: HashMap<GenId, f64>,
    pub qgen: HashMap<GenId, f64>,
    // plus maybe branch flows if available
}

pub struct PFDeltaInstance {
    pub test_case: PFDeltaTestCase,
    pub network: Network,
    pub solution: PFDeltaSolution,
}
```

and implement:

```rust
pub fn load_pfdelta_case(path: &Path) -> Result<PFDeltaInstance>;
```

Pull voltages/flows from the right JSON fields (e.g. `solution.bus.Vm`, `solution.bus.Va`, etc.).

### 2.4 Handle contingencies

Each PFΔ instance already includes a particular contingency (N / N-1 / N-2) encoded by line/gen statuses. ([arXiv][2])

**Task B4:** Ensure `convert_pfdelta_to_network` respects:

* `status: 0` for outaged branches/generators.
* Any topology changes (e.g., removed lines) by skipping them or marking out of service.

No need to call GAT’s `nminus1` here; the scenario is baked into the JSON.

---

## 3. Extend the `gat benchmark pfdelta` CLI

You already have:

* `crates/gat-cli/src/commands/benchmark/mod.rs`
* `crates/gat-cli/src/commands/benchmark/pfdelta.rs`
* `enum BenchmarkCommands::Pfdelta { ... }` in `crates/gat-cli/src/cli.rs`

Right now, `pfdelta.rs` is wired around `AcOpfSolver` and just measures timings.

### 3.1 Add a mode flag: PF vs OPF

**Task C1:** Modify `BenchmarkCommands::Pfdelta` in `cli.rs`:

```rust
    /// Benchmark PFΔ dataset
    Pfdelta {
        // existing args...
        /// Solve mode: pf or opf
        #[arg(long, default_value = "pf")]
        mode: String,
        // existing tol, max_iter ...
    },
```

Update `benchmark::mod.rs` to pass `mode` through to `pfdelta::handle`.

### 3.2 Implement PF mode using GAT’s PF solver

In `crates/gat-cli/src/commands/benchmark/pfdelta.rs`:

**Task C2:** Change `benchmark_case` to branch on `mode`:

* If `mode == "pf"`:

  * Call GAT’s AC PF (look at `commands/pf.rs` for the right API; it uses `gat_algo::power_flow`).
  * Use the `tol`/`max_iter` parameters.
* If `mode == "opf"` (future / optional):

  * Keep the existing `AcOpfSolver` path.

Rough sketch (agents will fill in actual calls):

```rust
let network = instance.network;
let t1 = Instant::now();

let solution = match mode {
    "pf" => power_flow::ac_solve(&network, tol, max_iter)?,
    "opf" => {
        let mut solver = AcOpfSolver::new(tol, max_iter, ...);
        solver.solve(&network)?
    }
    _ => return Err(anyhow!("Unknown mode {}", mode)),
};

let solve_time_ms = t1.elapsed().as_secs_f64() * 1000.0;
```

### 3.3 Add error metrics vs PFΔ solution

Currently `BenchmarkResult` only has timing + size info. Expand it:

**Task C3:** Update `BenchmarkResult` in `pfdelta.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
struct BenchmarkResult {
    case_name: String,
    contingency_type: String,
    case_index: usize,
    load_time_ms: f64,
    solve_time_ms: f64,
    total_time_ms: f64,
    converged: bool,
    iterations: u32,
    num_buses: usize,
    num_branches: usize,
    max_vm_error: f64,
    max_va_error_deg: f64,
    max_branch_p_error: f64,
}
```

And compute these in `benchmark_case`:

**Task C4:**

* Map GAT’s solution bus voltages to `BusId` → `(Vm, Va)`.

* Compare with `PFDeltaSolution` from `load_pfdelta_case`:

  * `max_vm_error = max_b |Vm_gat(b) - Vm_ref(b)|`
  * `max_va_error_deg = max_b |Va_gat(b) - Va_ref(b)|` (convert to degrees if needed)

* If branch flows are in PFΔ JSON, compute `max_branch_p_error` similarly.

This is what makes the demo **validation-grade**, not just benchmarking.

---

## 4. “User-level” scripts & workflow

Create a small example package in the repo, e.g. `examples/pfdelta-benchmark/`.

### 4.1 Runner script for GAT

**File:** `examples/pfdelta-benchmark/run_gat_pfdelta.sh`

**Task D1:** Script that:

1. Assumes `pfdelta_root=data/pfdelta` (or takes it as an env var).
2. Runs a representative subset, e.g.:

```bash
#!/usr/bin/env bash
set -euo pipefail

PFDELTA_ROOT="${PFDELTA_ROOT:-data/pfdelta}"
OUT_DIR="${OUT_DIR:-results/pfdelta}"

mkdir -p "$OUT_DIR"

gat benchmark pfdelta \
  --pfdelta-root "$PFDELTA_ROOT" \
  --case ieee118 \
  --contingency N-1 \
  --max-cases 10000 \
  --mode pf \
  --tol 1e-6 \
  --max-iter 20 \
  --threads auto \
  --out "$OUT_DIR/pfdelta_ieee118_N-1_gat.csv"
```

### 4.2 Baseline runner (PFΔ’s traditional solver)

PFΔ’s repo includes code for running traditional PF solvers on their dataset. ([GitHub][1])

**Task D2:** Add `examples/pfdelta-benchmark/run_pfdelta_baseline.sh` that:

* Clones or expects a checkout of `github.com/MOSSLab-MIT/pfdelta`.
* Calls their evaluation script for the same subset (same `case`, `contingency`, `max_cases`).
* Produces e.g. `results/pfdelta/pfdelta_ieee118_N-1_baseline.csv` with per-case timing and (if supported) error stats.

Even if PFΔ doesn’t ship per-case timings out of the box, you can lightly patch their script in this example directory (documented, not vendored).

### 4.3 Analysis script for speedup & validation

**File:** `examples/pfdelta-benchmark/analyze_pfdelta_benchmark.py`

**Task D3:** Python script (Polars or Pandas) that:

* Reads both CSVs:

  * `*_gat.csv` (from `gat benchmark pfdelta`)
  * `*_baseline.csv` (traditional solver)

* Checks:

  * Distribution of `max_vm_error`, `max_va_error_deg` (should be tiny).
  * Fraction of cases where `converged == true`.
  * GAT median / mean `solve_time_ms` vs baseline.

* Prints a nice summary:

  * Average speedup.
  * How many cases GAT fails vs baseline and vice versa.

This script can double as the backbone for your docs tutorial.

---

## 5. Test coverage / CI

### 5.1 Small PFΔ fixture

**Task E1:** Add a very small PFΔ sample (e.g. 3 cases across N, N-1, N-2 for IEEE-14) to:

```text
test_data/pfdelta/ieee14/N/...
test_data/pfdelta/ieee14/N-1/...
test_data/pfdelta/ieee14/N-2/...
```

(You could generate a tiny synthetic subset using PFΔ’s generator and commit it, if licensing and size permit.)

### 5.2 Unit tests in `gat-io`

**File:** `crates/gat-io/tests/pfdelta_tests.rs`

**Task E2:** Tests for:

* `list_pfdelta_cases` finds the expected 3 cases.
* `load_pfdelta_case` returns a `Network` with the expected numbers of buses/branches/gens/loads.
* Voltage/reference mapping is consistent (e.g. confirm one known bus’s Vm/Va matches the JSON exactly).

### 5.3 CLI integration test

**File:** `crates/gat-cli/tests/pfdelta_benchmark.rs`

Using `assert_cmd`:

**Task E3:**

* Set up a temp dir, copy `test_data/pfdelta` into it.
* Run:

```rust
Command::cargo_bin("gat")?
    .args([
        "benchmark", "pfdelta",
        "--pfdelta-root", tmp_root,
        "--case", "ieee14",
        "--contingency", "N-1",
        "--max-cases", "3",
        "--mode", "pf",
        "--tol", "1e-6",
        "--max-iter", "20",
        "--threads", "1",
        "--out", out_csv,
    ])
    .assert()
    .success();
```

* Parse `out_csv` and assert:

  * `converged == true` for 3 rows.
  * `max_vm_error` and `max_va_error_deg` below some threshold.

---

## 6. Docs / tutorial wiring

### 6.1 New tutorial page

**File:** `docs/src/tutorials/pfdelta.md` (or similar, depending on your doc engine)

**Task F1:** Write a narrative tutorial roughly:

1. **What is PFΔ and why do we care?** (short recap with link). ([arXiv][3])
2. **Download the dataset** (either full from HuggingFace or `pfdelta-sample` via `gat dataset`).
3. **Run the benchmark**: show the `gat benchmark pfdelta` command.
4. **Inspect results**: small Polars/DuckDB snippet reading the CSV and plotting solve time vs contingency type.
5. **Validation**: show how `max_vm_error` looks, maybe a distribution plot.
6. **Optional:** mention how the same dataset can be used with `gat featurize gnn` to generate ML-ready features.

### 6.2 Link from main README

**Task F2:** Add a “Benchmarks” or “Tutorials” section in `README.md` that links to the PFΔ tutorial and shows a minimal invocation snippet.

---

## 7. How this hits your A/B/C goals

* **A. Reproduce results / validate:**
  B3 + C3/C4 + E2/E3 give you per-case numerical error vs PFΔ’s reference solution and tests.

* **B. Benchmark speed:**
  Benchmark CSV from `gat benchmark pfdelta` + baseline CSV + D3 analysis yields per-case and aggregate speedup numbers.

* **C. Tutorial example:**
  F1/F2 + D1–D3 turn it into a polished, copy-pasteable example for new users.

---

[1]: https://github.com/MOSSLab-MIT/pfdelta?utm_source=chatgpt.com "MOSSLab-MIT/pfdelta"
[2]: https://arxiv.org/html/2510.22048v1?utm_source=chatgpt.com "PFΔ: A Benchmark Dataset for Power Flow under Load, ..."
[3]: https://arxiv.org/abs/2510.22048?utm_source=chatgpt.com "PF$Δ$: A Benchmark Dataset for Power Flow under Load, Generation, and Topology Variations"
