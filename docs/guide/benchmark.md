# Benchmarking GAT Against Public Datasets

GAT includes integrated benchmarking tools for systematically evaluating solver performance against public power flow datasets. Four benchmark suites are supported:

1. **PGLib-OPF** (recommended) — 68 MATPOWER cases from industry-standard IEEE/PEGASE/GOC networks
2. **PFDelta** — 859,800 solved power flow instances with N/N-1/N-2 contingencies
3. **OPFData** — GNN-format JSON for machine learning benchmarks
4. **DSS²** — State estimation accuracy benchmark on CIGRE MV distribution network

## PGLib-OPF Integration (v0.5.6)

### Dataset Overview

PGLib-OPF (https://github.com/power-grid-lib/pglib-opf) is the industry-standard benchmark suite:
- **68 test cases** from 14 to 19,402 buses
- **Multiple network types**: IEEE, PEGASE, GOC, RTE, NESTA, epigrids
- **Baseline solutions** for objective comparison
- **MATPOWER format** (.m files)

### Basic Usage

```bash
# Clone PGLib-OPF
git clone --depth 1 https://github.com/power-grid-lib/pglib-opf.git

# Organize files into directories (GAT expects case directories)
cd pglib-opf
for f in pglib_opf_*.m; do
  dir="${f%.m}"
  mkdir -p "$dir"
  mv "$f" "$dir/case.m"
done

# Run benchmark
gat benchmark pglib \
  --pglib-dir /path/to/pglib-opf \
  --baseline /path/to/pglib-opf/baseline.csv \
  --out results.csv
```

### Options

- `--pglib-dir`: Directory containing PGLib MATPOWER case directories
- `--baseline`: Optional CSV with reference objective values
- `--case-filter`: Filter cases by name (e.g., "case14", "case118")
- `--max-cases`: Limit number of cases (0 = all)
- `--out`: Output CSV path
- `--threads`: Parallel threads (auto = CPU count)
- `--tol`: Convergence tolerance (default 1e-6)
- `--max-iter`: Maximum iterations (default 20)

### v0.5.6 Benchmark Results

Running against all 68 PGLib cases with the **IPOPT backend**:

| Metric | Value |
|--------|-------|
| Cases tested | 68 |
| Cases converged | **68 (100%)** |
| Cases with <0.01% gap | **68 (100%)** |
| Median objective gap | **<0.01%** |
| Best case | 0.00% (exact match on IEEE 14, 118, and others) |

**Validated Reference Values:**
- IEEE 14-bus: $2,178.08/hr (ref: $2,178.10) — **Gap: -0.00%**
- IEEE 118-bus: $97,213.61/hr (ref: $97,214.00) — **Gap: -0.00%**
- All 68 cases match PGLib reference objectives within solver tolerance

**Key improvements in v0.5.6:**
- Analytical Jacobian and Hessian computation for IPOPT
- Proper handling of synchronous condensers and negative Pg generators
- Bus shunt support (fixed capacitors/reactors) in Y-bus construction
- Constraint scaling / row equilibration for improved DC-OPF LP conditioning
- Zero-reactance epsilon handling (1e-6) for bus tie transformers
- Unit-aware newtype wrappers (Megawatts, Kilovolts, PerUnit) for compile-time unit safety

**Note:** The L-BFGS penalty method backend achieves ~2.9% median gap and is useful when IPOPT is unavailable.

---

## PFDelta Integration (v0.5.6)

### Dataset Overview

PFDelta (https://github.com/MOSSLab-MIT/pfdelta) provides:
- **859,800 test cases** across 6 standard IEEE networks
- **6 network sizes**: IEEE 14-bus, 30-bus, 57-bus, 118-bus, GOC 500-bus, GOC 2000-bus
- **3 contingency types**: N (no contingencies), N-1 (single line), N-2 (double line)
- **Pre-solved optimal solutions** with ground truth data
- **Two subdirectories per contingency**: `raw/` (feasible) and `nose/` (near-infeasible/boundary cases)
- **JSON format** with bus/gen/load/branch data in per-unit representation

### Directory Structure

```
pfdelta/
├── case14/
│   ├── n/
│   │   ├── raw/
│   │   │   ├── pfdelta_0.json
│   │   │   ├── pfdelta_1.json
│   │   │   └── ...
│   │   └── nose/
│   │       └── ...
│   ├── n-1/
│   │   ├── raw/
│   │   └── nose/
│   └── n-2/
│       ├── raw/
│       └── nose/
├── case30/
├── case57/
├── case118/
├── case500/
└── case2000/
```

## Usage Examples

### Basic Benchmark Run

Benchmark AC OPF solver on a subset of test cases:

```bash
gat benchmark pfdelta \
  --pfdelta-root /path/to/pfdelta \
  --case 57 \
  --contingency n \
  --max-cases 100 \
  --out results_case57_n.csv \
  --threads 8
```

**Parameters:**
- `--pfdelta-root`: Path to PFDelta root directory
- `--case`: Filter by network size (14, 30, 57, 118, 500, 2000) or omit for all
- `--contingency`: Type of contingencies (`n`, `n-1`, `n-2`, or `all`)
- `--max-cases`: Limit number of test cases (0 = all)
- `--out`: Output CSV file path
- `--threads`: Parallel solver threads (auto = CPU count)
- `--tol`: Convergence tolerance (default 1e-6)
- `--max-iter`: Maximum iterations (default 20)

### Output Format

Results written to CSV with columns:
- `case_name`: Network identifier (case14, case30, etc.)
- `contingency_type`: Contingency class (n, n-1, n-2, etc.)
- `case_index`: Index in result set
- `solve_time_ms`: AC OPF solve time (milliseconds)
- `converged`: Boolean (true/false)
- `iterations`: Number of Newton-Raphson iterations
- `max_voltage_error`: Largest voltage residual (p.u.)
- `max_power_error`: Largest power balance error (p.u.)
- `num_buses`: Network size
- `num_branches`: Transmission lines

### Full Test Suite (All Cases)

Benchmark all 859,800 instances across all network sizes and contingencies:

```bash
# On a 16-core system, takes ~8-10 hours
gat benchmark pfdelta \
  --pfdelta-root /path/to/pfdelta \
  --contingency all \
  --out results_full_suite.csv \
  --threads auto
```

For faster preview, sample:

```bash
# Test 1000 random cases (representative sample)
gat benchmark pfdelta \
  --pfdelta-root /path/to/pfdelta \
  --max-cases 1000 \
  --out results_sample.csv
```

### Contingency-Type Comparison

Compare solver behavior across different contingency levels:

```bash
# N (no contingencies)
gat benchmark pfdelta \
  --pfdelta-root /path/to/pfdelta \
  --contingency n \
  --max-cases 1000 \
  --out results_n.csv

# N-1 (single line outages)
gat benchmark pfdelta \
  --pfdelta-root /path/to/pfdelta \
  --contingency n-1 \
  --max-cases 1000 \
  --out results_n1.csv

# N-2 (double line outages)
gat benchmark pfdelta \
  --pfdelta-root /path/to/pfdelta \
  --contingency n-2 \
  --max-cases 1000 \
  --out results_n2.csv
```

### Network-Size Scaling

Test how solver scales with network complexity:

```bash
for case in 14 30 57 118 500 2000; do
  gat benchmark pfdelta \
    --pfdelta-root /path/to/pfdelta \
    --case $case \
    --max-cases 100 \
    --out results_case${case}.csv
done
```

Expected behavior:
- **14-bus**: ~0.5-1ms per case
- **30-bus**: ~1-2ms per case
- **57-bus**: ~2-4ms per case
- **118-bus**: ~5-15ms per case
- **500-bus**: ~50-200ms per case (dense interconnection)
- **2000-bus**: ~200-500ms per case (sparse geographic distribution)

## Analysis and Visualization

### Post-Processing Results

With Python/Pandas:

```python
import pandas as pd
import numpy as np

# Load results
df = pd.read_csv('results_case57_n.csv')

# Convergence rate
converged = df['converged'].sum() / len(df)
print(f"Convergence rate: {converged*100:.1f}%")

# Performance statistics
print(f"Mean solve time: {df['solve_time_ms'].mean():.2f}ms")
print(f"Median solve time: {df['solve_time_ms'].median():.2f}ms")
print(f"95th percentile: {df['solve_time_ms'].quantile(0.95):.2f}ms")
print(f"Max solve time: {df['solve_time_ms'].max():.2f}ms")

# Error statistics
print(f"Mean voltage error: {df['max_voltage_error'].mean():.2e} p.u.")
print(f"Mean power error: {df['max_power_error'].mean():.2e} p.u.")

# Failed cases
failed = df[~df['converged']]
print(f"Failed cases: {len(failed)}")
for idx, row in failed.iterrows():
    print(f"  {row['case_name']} #{row['case_index']}")
```

### Comparison Against Ground Truth

PFDelta includes pre-solved optimal solutions. Post-processing can validate:

```python
# If ground truth is available in JSON files:
import json

for idx, row in df.iterrows():
    case_file = f"{pfdelta_root}/{row['case_name']}/n/raw/pfdelta_{row['case_index']}.json"
    with open(case_file) as f:
        ground_truth = json.load(f)

    # Compare optimal objective value, bus voltages, branch flows, etc.
    # This requires parsing the JSON and extracting solution data
```

## Performance Expectations

On modern hardware (e.g., 16-core Ryzen 5950X):

| Case Size | Typical Time | Throughput | Notes |
|-----------|-------------|-----------|-------|
| IEEE 14   | 0.8ms       | 1250 cases/sec | Trivial |
| IEEE 30   | 1.5ms       | 667 cases/sec | Quick |
| IEEE 57   | 3.0ms       | 333 cases/sec | Fast |
| IEEE 118  | 10ms        | 100 cases/sec | Standard |
| GOC 500   | 100ms       | 10 cases/sec | Large |
| GOC 2000  | 300ms       | 3.3 cases/sec | Very large |

**Full suite estimate:** 859,800 cases at weighted average ~50ms/case = ~12 hours (16 cores).

## Test Suite

The benchmark implementation includes integration tests:

```bash
cargo test -p gat-cli --test benchmark_pfdelta -- --nocapture
```

Tests verify:
- CLI command parsing
- Result CSV schema validation
- Parallel execution correctness
- Error handling for missing files

## Known Limitations

1. **No Convergence Guarantee**: Near-infeasible cases (in `nose/` directories) may not converge. This is expected; GAT reports these as failed.

2. **Per-Unit Normalization**: PFDelta uses different base MVA for different cases. Ensure proper scaling during network conversion.

3. **Ground Truth Comparison**: PFDelta solutions are provided as reference; comparing against them requires additional parsing logic not included in the loader.

4. **Memory Scaling**: Very large cases (2000-bus) with 500 Monte Carlo scenarios require ~1GB per case. Use `--max-cases` to limit.

## Future Enhancements

Planned for subsequent releases:

- [ ] Direct JSON solution comparison (vs. ground truth)
- [ ] Reliability metrics per test case (LOLE/EUE impact)
- [ ] Visualization dashboard (Parquet → interactive web UI)
- [ ] Integration with dsgrid/RTS-GMLC time-series data
- [ ] Distributed benchmark runner (fan-out across compute cluster)

## References

- **PFDelta Project**: https://github.com/MOSSLab-MIT/pfdelta
- **PFDelta Dataset**: https://huggingface.co/datasets/pfdelta/pfdelta
- **Paper**: https://arxiv.org/abs/2510.22048 (MOSSLab preprint)
- **Crate**: `crates/gat-io/src/sources/pfdelta.rs`
- **CLI**: `gat benchmark pfdelta --help`
- **Tests**: `crates/gat-cli/tests/benchmark_pfdelta.rs`

---

## DSS² State Estimation Benchmark (v0.5.6)

### Overview

The DSS² benchmark evaluates Weighted Least Squares (WLS) state estimation accuracy and performance using the CIGRE Medium-Voltage 14-bus test network. This benchmark reproduces the WLS baseline from the DSS² paper ("Deep Statistical Solver for Distribution System State Estimation", arXiv:2301.01835).

### Test Network: CIGRE MV 14-Bus

The benchmark uses a programmatically-constructed CIGRE MV distribution network:

```
Topology (radial feeder):
  0 (slack) -- 1 -- 2 -- 3 -- 4 -- 5 -- 6 -- 7
                    |         |
                    8 -- 9    10 -- 11 -- 12 -- 13
```

**Network characteristics:**
- **14 buses** (bus 0 = slack/HV-MV substation)
- **13 branches** (overhead lines and cables)
- **~28 MW total load** (typical residential/commercial)
- **20 kV base voltage**, 100 MVA base

### Basic Usage

```bash
# Run 100 Monte Carlo trials with 2% measurement noise
gat benchmark dss2 \
  --out results/dss2.csv \
  --trials 100 \
  --noise-std 0.02 \
  --seed 42
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--out` | (required) | Output CSV path for per-trial results |
| `--trials` | 100 | Number of Monte Carlo trials |
| `--noise-std` | 0.02 | Measurement noise σ (fraction of true value) |
| `--load-scale` | 1.0 | Load scaling factor (1.0 = nominal) |
| `--num-flow` | 10 | Number of branch flow measurements |
| `--num-injection` | 5 | Number of bus injection measurements |
| `--seed` | (random) | Random seed for reproducibility |
| `--threads` | auto | Parallel threads (auto = CPU count) |

### Benchmark Workflow

Each trial performs:

1. **Build CIGRE MV network** — Construct 14-bus radial distribution network
2. **Run DC power flow** — Compute true bus angles (θ) from B'θ = P
3. **Generate measurements** — Create synthetic flow/injection measurements with Gaussian noise
4. **Run WLS state estimation** — Solve normal equations (HᵗWH)x = HᵀWz
5. **Compute error metrics** — Compare estimated vs. true angles

### Output Files

**Per-trial CSV** (`--out results.csv`):

| Column | Description |
|--------|-------------|
| `trial` | Trial index (0-based) |
| `seed` | Random seed for this trial |
| `num_buses` | Network size (14) |
| `num_branches` | Branch count (13) |
| `num_measurements` | Total measurements used |
| `noise_std` | Noise level (σ) |
| `load_scale` | Load scaling factor |
| `pf_time_ms` | Power flow solve time |
| `meas_gen_time_ms` | Measurement generation time |
| `se_time_ms` | State estimation solve time |
| `total_time_ms` | Total trial time |
| `mae_deg` | Mean absolute error (degrees) |
| `rmse_deg` | Root mean square error (degrees) |
| `max_error_deg` | Maximum error (degrees) |
| `converged` | WLS convergence status |

**Summary JSON** (`results.summary.json`):

```json
{
  "total_trials": 100,
  "converged_trials": 100,
  "convergence_rate": 1.0,
  "mean_mae_deg": 0.081,
  "std_mae_deg": 0.011,
  "mean_rmse_deg": 0.093,
  "mean_max_error_deg": 0.15,
  "median_se_time_ms": 6.2,
  "mean_se_time_ms": 6.1,
  "p95_se_time_ms": 7.3
}
```

### Expected Results

With default settings (2% noise, 15 measurements):

| Metric | Typical Value | DSS² Paper Reference |
|--------|--------------|---------------------|
| Convergence Rate | 100% | 100% |
| MAE | 0.08° ± 0.01° | < 0.5° |
| RMSE | 0.09° | — |
| Median SE Time | 6 ms | — |

**GAT exceeds the DSS² paper's WLS baseline accuracy** (MAE < 0.5° threshold) by approximately 6x.

### Noise Sensitivity Analysis

```bash
# Compare different noise levels
for noise in 0.01 0.02 0.05 0.10; do
  gat benchmark dss2 \
    --out results/dss2_noise_${noise}.csv \
    --noise-std $noise \
    --trials 50 \
    --seed 42
done
```

Expected scaling: MAE increases approximately linearly with noise σ.

### Measurement Redundancy Study

```bash
# Test with varying measurement counts
for flow in 5 10 13; do
  for inj in 3 5 10; do
    gat benchmark dss2 \
      --out results/dss2_f${flow}_i${inj}.csv \
      --num-flow $flow \
      --num-injection $inj \
      --trials 50
  done
done
```

Higher redundancy (more measurements) typically improves accuracy but increases solve time.

### Post-Processing with Python

```python
import pandas as pd
import json

# Load trial results
df = pd.read_csv('results/dss2.csv')

# Summary statistics
print(f"Convergence: {df['converged'].mean()*100:.1f}%")
print(f"MAE: {df['mae_deg'].mean():.4f}° ± {df['mae_deg'].std():.4f}°")
print(f"RMSE: {df['rmse_deg'].mean():.4f}°")
print(f"Median SE time: {df['se_time_ms'].median():.2f} ms")

# Load summary
with open('results/dss2.summary.json') as f:
    summary = json.load(f)
print(f"P95 SE time: {summary['p95_se_time_ms']:.2f} ms")

# Plot error distribution
import matplotlib.pyplot as plt
df['mae_deg'].hist(bins=20)
plt.xlabel('MAE (degrees)')
plt.ylabel('Frequency')
plt.title('DSS² WLS State Estimation Error Distribution')
plt.savefig('dss2_error_hist.png')
```

### Implementation Details

**Source files:**
- `crates/gat-io/src/sources/cigre.rs` — CIGRE MV network builder + measurement generator
- `crates/gat-cli/src/commands/benchmark/dss2.rs` — Benchmark command implementation
- `crates/gat-algo/src/power_flow.rs` — WLS state estimation solver

**Key algorithms:**
- DC power flow: B'θ = P matrix solve
- Branch flow calculation: P_ij = (θ_i - θ_j) / x_ij
- WLS state estimation: Normal equations (HᵗWH)θ = HᵀWz
- Measurement noise: Box-Muller transform for Gaussian samples

### References

- **DSS² Paper**: https://arxiv.org/abs/2301.01835
- **CIGRE Task Force C6.04.02**: "Benchmark Systems for Network Integration of Renewable and Distributed Energy Resources" (2014)
- **State Estimation**: See `docs/guide/se.md` for WLS mathematical background
- **CLI**: `gat benchmark dss2 --help`
- **Tests**: `crates/gat-cli/src/commands/benchmark/dss2.rs` (unit tests)

---

## DPLib: Distributed ADMM OPF Benchmark (v0.5.6)

### Overview

GAT implements the **ADMM (Alternating Direction Method of Multipliers)** algorithm for distributed optimal power flow, following the DPLib paper approach (arXiv:2506.20819). The `gat benchmark dplib` command compares distributed ADMM-OPF against centralized SOCP solutions.

### Algorithm

The network is partitioned into regions, each solving a local OPF subproblem. Boundary buses are shared via consensus constraints:

```
min  Σ_k f_k(x_k)
s.t. A_k x_k = b_k           (local constraints)
     x_k|_boundary = z       (consensus constraints)
```

ADMM iterates three phases:
1. **x-update**: Each partition solves local OPF with augmented Lagrangian
2. **z-update**: Average boundary variables across partitions
3. **λ-update**: Update dual variables (Lagrange multipliers)

Convergence is achieved when primal residual (consensus violation) and dual residual (consensus change rate) are below tolerance.

### Basic Usage

```bash
gat benchmark dplib \
  --pglib-dir /path/to/pglib-opf \
  --out results_dplib.csv \
  --num-partitions 4 \
  --max-cases 50
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--pglib-dir` | (required) | Path to PGLib-OPF directory |
| `--out` | (required) | Output CSV path |
| `--case-filter` | (all) | Filter cases by name pattern |
| `--max-cases` | 0 | Limit number of cases (0=all) |
| `--threads` | auto | Parallel threads (auto=CPU count) |
| `--num-partitions` | 0 | Number of ADMM partitions (0=auto) |
| `--max-iter` | 100 | Maximum ADMM iterations |
| `--tol` | 1e-4 | Primal/dual convergence tolerance |
| `--rho` | 1.0 | Initial penalty parameter (ρ) |
| `--subproblem-method` | dc | Local OPF method (dc, socp) |

### Output Columns

| Column | Description |
|--------|-------------|
| `case_name` | Network identifier |
| `num_buses` | Network size |
| `num_partitions` | Partitions used |
| `num_tie_lines` | Branches crossing partition boundaries |
| `centralized_time_ms` | Centralized SOCP solve time |
| `centralized_objective` | Centralized optimal cost |
| `admm_time_ms` | Distributed ADMM solve time |
| `admm_objective` | ADMM optimal cost |
| `admm_iterations` | ADMM iterations to convergence |
| `primal_residual` | Final consensus violation |
| `dual_residual` | Final dual residual |
| `objective_gap_rel` | Relative gap: (ADMM - centralized) / centralized |
| `speedup_ratio` | Centralized time / ADMM time |
| `x_update_ms` | Time in x-update phase |
| `z_update_ms` | Time in z-update phase |
| `dual_update_ms` | Time in λ-update phase |

### Experiments

**Partition scaling:**
```bash
for k in 2 4 8 16; do
  gat benchmark dplib \
    --pglib-dir ~/data/pglib-opf \
    --num-partitions $k \
    --case-filter "case300" \
    --out dplib_k${k}.csv
done
```

**Subproblem method comparison:**
```bash
# DC subproblems (faster)
gat benchmark dplib --subproblem-method dc --out dplib_dc.csv

# SOCP subproblems (more accurate)
gat benchmark dplib --subproblem-method socp --out dplib_socp.csv
```

**Penalty parameter tuning:**
```bash
for rho in 0.1 1.0 10.0 100.0; do
  gat benchmark dplib --rho $rho --out dplib_rho${rho}.csv
done
```

### Expected Results

| Network Size | Partitions | ADMM Time | Iterations | Speedup |
|-------------|-----------|-----------|------------|---------|
| 118-bus | 4 | ~50ms | 15-25 | 0.8-1.2x |
| 300-bus | 4 | ~100ms | 20-30 | 1.0-1.5x |
| 1354-bus | 8 | ~300ms | 25-40 | 1.5-2.5x |
| 2853-bus | 16 | ~500ms | 30-50 | 2.0-4.0x |

Key observations:
- ADMM overhead dominates on small networks (< 200 buses)
- Speedup becomes significant for large networks (> 1000 buses)
- DC subproblems are 5-10x faster than SOCP

### References

- **DPLib Paper**: arXiv:2506.20819
- **Boyd et al.**: "Distributed Optimization and Statistical Learning via ADMM"
- **ADMM Solver**: `crates/gat-algo/src/opf/admm.rs`
- **Graph Partitioning**: `crates/gat-algo/src/graph/partition.rs`
- **CLI**: `gat benchmark dplib --help`
