# GAT arXiv Draft 3: Full Experimental Validation Plan

> **For Claude:** Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Comprehensive experimental validation across three reference paper datasets, culminating in `gat-arxiv-preprint-draft3.tex` with publication-quality results.

**Constraints:**
- 456GB disk space available (can use up to 400GB)
- GPU available for potential acceleration
- Overnight runtime acceptable
- Fix validation issues that blocked 27/66 PGLib cases

---

## Executive Summary

| Dataset | Purpose | Current State | Target |
|---------|---------|---------------|--------|
| **PGLib-OPF** | AC-OPF validation | 39/66 cases (27 blocked by validation) | 66/66 cases |
| **PFDelta** | AC-PF accuracy | 11k samples, 0 error | 50k+ samples, timing comparison |
| **OPFData** | Topology perturbation | Not yet run | 10k+ samples per case |

---

## Phase 1: Fix Validation to Unlock All PGLib Cases

### Task 1.1: Update AC-OPF Validation for Phase-Shifters

**Files:**
- `crates/gat-algo/src/opf/ac_nlp/ybus.rs`
- `crates/gat-algo/src/opf/ac_nlp/problem.rs`

**Problem:** Y-bus construction may fail on negative R/X values even when `is_phase_shifter=true`.

**Fix:** Update Y-bus construction to handle negative impedance:
```rust
// In ybus.rs, when computing y_series = 1/(R + jX)
// For phase shifters, R or X can be negative - this is physically valid
let z = Complex64::new(branch.resistance, branch.reactance);
if z.norm() < 1e-12 {
    // Zero impedance (bus tie) - skip or use small value
    continue;
}
let y_series = z.inv();  // Works for negative values too
```

**Verification:**
```bash
cargo test -p gat-algo ybus
./target/release/gat-cli opf ac data/pglib-opf/pglib_opf_case300_ieee/case.m
```

### Task 1.2: Ensure MATPOWER Importer Sets Flags Correctly

**Files:**
- `crates/gat-io/src/importers/matpower.rs` (line ~166-198)

**Current Logic:**
```rust
let is_syncon = (gen.pmax <= 0.0 || gen.pg < 0.0) && gen.qmax > gen.qmin;
let is_phase_shifter = br.shift.abs() > 1e-6 || br.br_x < 0.0 || br.br_r < 0.0;
```

**Verification:** Run on a previously-failing case:
```bash
./target/release/gat-cli opf ac data/pglib-opf/pglib_opf_case89_pegase/case.m
```

### Task 1.3: Test All 66 PGLib Cases

**Command:**
```bash
./target/release/gat-cli benchmark pglib \
  --pglib-dir data/pglib-opf \
  --out results/pglib_all_66.csv \
  --threads auto \
  --max-iter 200 \
  --tol 1e-4
```

**Success Criteria:**
- 66 cases attempted (not 39)
- >90% convergence rate
- Objective values within 5% of baseline where comparable

---

## Phase 2: Full PGLib Benchmark with Baseline Comparison

### Task 2.1: Parse PGLib BASELINE.md for Reference Objectives

**File:** `data/pglib-opf/BASELINE.md`

**New Helper:** Create `examples/scripts/parse_baseline.py`:
```python
import re
import csv

def parse_baseline(baseline_md: str) -> dict:
    """Extract case_name -> objective from BASELINE.md"""
    pattern = r'\|\s*(pglib_opf_case\d+[a-z_]*)\s*\|\s*([0-9.e+-]+)\s*\|'
    results = {}
    with open(baseline_md) as f:
        for line in f:
            match = re.search(pattern, line)
            if match:
                results[match.group(1)] = float(match.group(2))
    return results
```

Output: `data/pglib-opf/baseline.csv` with columns `case_name,objective`

### Task 2.2: Run Full PGLib Benchmark

```bash
./target/release/gat-cli benchmark pglib \
  --pglib-dir data/pglib-opf \
  --baseline data/pglib-opf/baseline.csv \
  --out results/pglib_full_benchmark.csv \
  --threads auto \
  --max-iter 300 \
  --tol 1e-5
```

### Task 2.3: Generate Benchmark Analysis

**File:** `examples/scripts/analyze_pglib.py`

Outputs:
- Convergence summary by case size category
- Objective gap distribution (histogram)
- Solve time vs network size (scatter)
- Cases with >1% objective gap (table for investigation)

---

## Phase 3: PFDelta Power Flow Benchmark

### Task 3.1: Extract PFDelta Test Cases

**Data Location:** `data/pfdelta/case{30,57,118}/{n,n-1,n-2}/raw/`

**Structure:**
```
case118/
  n/raw/          # Base case samples
  n-1/raw/        # N-1 contingency samples
  n-2/raw/        # N-2 contingency samples
```

Each `sample_XXXXX.json` contains network + reference solution.

### Task 3.2: Run PFDelta Benchmark - Base Case (N)

```bash
./target/release/gat-cli benchmark pfdelta \
  --pfdelta-root data/pfdelta \
  --case case118 \
  --contingency n \
  --max-cases 20000 \
  --mode pf \
  --out results/pfdelta_case118_n.csv \
  --threads auto
```

### Task 3.3: Run PFDelta Benchmark - N-1 Contingencies

```bash
./target/release/gat-cli benchmark pfdelta \
  --pfdelta-root data/pfdelta \
  --case case118 \
  --contingency n-1 \
  --max-cases 20000 \
  --mode pf \
  --out results/pfdelta_case118_n1.csv \
  --threads auto
```

### Task 3.4: Run All PFDelta Cases

Loop over case30, case57, case118 and all contingency types:

```bash
for case in case30 case57 case118; do
  for cont in n n-1 n-2; do
    ./target/release/gat-cli benchmark pfdelta \
      --pfdelta-root data/pfdelta \
      --case $case \
      --contingency $cont \
      --max-cases 10000 \
      --mode pf \
      --out results/pfdelta_${case}_${cont}.csv \
      --threads auto
  done
done
```

**Target:** 50,000+ total samples

---

## Phase 4: OPFData AC-OPF with Topology Perturbations

### Task 4.1: Extract OPFData Archives

**Location:** `data/opfdata/pglib_opf_case118_ieee/`

```bash
cd data/opfdata/pglib_opf_case118_ieee
for f in group_*.tar.gz; do
  tar -xzf "$f"
done
```

### Task 4.2: Inspect OPFData JSON Format

```bash
head -1 data/opfdata/pglib_opf_case118_ieee/group_0/sample_0.json | python3 -m json.tool | head -50
```

Document the schema for the loader.

### Task 4.3: Implement OPFData Loader (if not complete)

**File:** `crates/gat-io/src/sources/opfdata.rs`

Key functions:
- `list_samples(root: &Path) -> Vec<PathBuf>`
- `load_opfdata_instance(path: &Path) -> Result<(Network, OpfDataSolution)>`
- `OpfDataSolution` struct with `vm`, `va`, `pgen`, `qgen`, `objective`

### Task 4.4: Run OPFData Benchmark

```bash
./target/release/gat-cli benchmark opfdata \
  --opfdata-dir data/opfdata \
  --case-filter case118 \
  --max-cases 10000 \
  --out results/opfdata_case118.csv \
  --threads auto \
  --tol 1e-4 \
  --max-iter 200
```

**Success Criteria:**
- >85% convergence rate
- Objective gap <5% vs reference
- Constraint violations <1e-3 p.u.

---

## Phase 5: Analysis and Visualization

### Task 5.1: Create Master Analysis Script

**File:** `examples/scripts/analyze_all_benchmarks.py`

```python
import polars as pl
import matplotlib.pyplot as plt

# Load all results
pglib = pl.read_csv("results/pglib_full_benchmark.csv")
pfdelta = pl.concat([
    pl.read_csv(f"results/pfdelta_{case}_{cont}.csv")
    for case in ["case30", "case57", "case118"]
    for cont in ["n", "n-1", "n-2"]
])
opfdata = pl.read_csv("results/opfdata_case118.csv")

# Generate tables for LaTeX
def latex_summary_table(df, name):
    ...

# Generate figures
def plot_scaling(df):
    ...
```

### Task 5.2: Generate LaTeX Tables

Output to `docs/papers/tables/`:
- `pglib_summary.tex` - Convergence by size category
- `pfdelta_accuracy.tex` - Voltage error statistics
- `opfdata_performance.tex` - AC-OPF under topology variation
- `timing_comparison.tex` - GAT vs reference solver timings

### Task 5.3: Generate Figures

Output to `docs/papers/figures/`:
- `scaling_plot.pdf` - Solve time vs network size
- `objective_gap_hist.pdf` - Distribution of objective gaps
- `convergence_rate.pdf` - Convergence rate by difficulty

---

## Phase 6: Write Draft 3

### Task 6.1: Create gat-arxiv-preprint-draft3.tex

**Base:** Copy from `docs/papers/gat-arxiv-preprint-draft2.tex`

**Key Updates:**

1. **Abstract:** Update with full results (66 PGLib, 50k+ PFDelta, 10k+ OPFData)

2. **Section 5 (Results):**
   - Update Table 2 (PFDelta) with full sample counts
   - Update Table 3 (PGLib) with all 66 cases
   - Add Table 4 (OPFData) for topology perturbation results
   - Add objective gap analysis (was "objective=0" in draft2)

3. **Section 6 (Discussion):**
   - Remove "OPF cost minimization" from limitations (now implemented)
   - Update "Synchronous condenser" and "Phase-shifting transformer" to show support
   - Add performance comparison discussion

4. **Section 7 (Conclusion):**
   - Update summary statistics
   - Revise future work based on what's now complete

5. **Appendix:**
   - Full PGLib results table (66 cases)
   - Add OPFData results table

### Task 6.2: Compile and Review

```bash
cd docs/papers
pdflatex gat-arxiv-preprint-draft3.tex
bibtex gat-arxiv-preprint-draft3
pdflatex gat-arxiv-preprint-draft3.tex
pdflatex gat-arxiv-preprint-draft3.tex
```

---

## Phase 7: Overnight Batch Run Script

### Task 7.1: Create Master Run Script

**File:** `examples/scripts/run_all_experiments.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

RESULTS_DIR="${RESULTS_DIR:-results}"
DATA_DIR="${DATA_DIR:-data}"
THREADS="${THREADS:-auto}"

mkdir -p "$RESULTS_DIR"

echo "=== Starting GAT Benchmark Suite ==="
echo "Results: $RESULTS_DIR"
echo "Data: $DATA_DIR"
echo "Started: $(date)"

# Phase 1: PGLib
echo ">>> PGLib Benchmark..."
./target/release/gat-cli benchmark pglib \
  --pglib-dir "$DATA_DIR/pglib-opf" \
  --out "$RESULTS_DIR/pglib.csv" \
  --threads "$THREADS" \
  --max-iter 300 \
  --tol 1e-5

# Phase 2: PFDelta
echo ">>> PFDelta Benchmark..."
for case in case30 case57 case118; do
  for cont in n n-1 n-2; do
    echo "  $case / $cont"
    ./target/release/gat-cli benchmark pfdelta \
      --pfdelta-root "$DATA_DIR/pfdelta" \
      --case "$case" \
      --contingency "$cont" \
      --max-cases 15000 \
      --mode pf \
      --out "$RESULTS_DIR/pfdelta_${case}_${cont}.csv" \
      --threads "$THREADS" || true
  done
done

# Phase 3: OPFData
echo ">>> OPFData Benchmark..."
./target/release/gat-cli benchmark opfdata \
  --opfdata-dir "$DATA_DIR/opfdata" \
  --max-cases 10000 \
  --out "$RESULTS_DIR/opfdata.csv" \
  --threads "$THREADS" \
  --tol 1e-4 \
  --max-iter 200 || true

echo "=== Benchmark Suite Complete ==="
echo "Finished: $(date)"
```

### Task 7.2: Run and Monitor

```bash
chmod +x examples/scripts/run_all_experiments.sh
nohup ./examples/scripts/run_all_experiments.sh > benchmark_log.txt 2>&1 &
tail -f benchmark_log.txt
```

---

## Data Requirements Summary

| Dataset | Size | Samples | Notes |
|---------|------|---------|-------|
| PGLib-OPF | ~10 MB | 66 cases | Already cloned |
| PFDelta case30 | ~450 MB | ~19k samples | Downloaded |
| PFDelta case57 | ~1.5 GB | ~19k samples | Downloaded |
| PFDelta case118 | ~3.5 GB | ~19k samples | Downloaded |
| OPFData case118 | ~20 GB (compressed) | 600k samples | Needs extraction |

**Total Estimated:** ~30 GB extracted

---

## Success Criteria for Draft 3

1. **PGLib:** 66/66 cases attempted, >60 converged, objective gaps documented
2. **PFDelta:** 50,000+ samples with <1e-6 voltage error
3. **OPFData:** 10,000+ samples with convergence + objective gap analysis
4. **Timing:** Sub-millisecond for small cases, <100ms for large
5. **Paper:** Complete draft with all tables and figures

---

## Appendix: Troubleshooting

### If PGLib cases still fail after validation fix:

Check specific case:
```bash
./target/release/gat-cli opf ac data/pglib-opf/pglib_opf_case89_pegase/case.m --verbose 2>&1 | head -50
```

### If OPFData JSON parsing fails:

Debug with:
```bash
python3 -c "
import json
with open('data/opfdata/pglib_opf_case118_ieee/group_0/sample_0.json') as f:
    d = json.load(f)
    print(d.keys())
    print('buses:', len(d.get('bus', [])))
    print('branches:', len(d.get('branch', [])))
"
```

### If overnight run times out:

Reduce max_cases:
```bash
--max-cases 5000  # Instead of 15000
```

Or run phases sequentially with checkpointing.
