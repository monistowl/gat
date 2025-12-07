+++
title = "Benchmarking"
description = "Benchmark GAT solver performance against public datasets"
weight = 45
+++

# Benchmarking GAT Against Public Datasets

GAT includes integrated benchmarking tools for systematically evaluating AC OPF solver performance against public power flow datasets. The primary benchmark suite uses the **PFDelta** project: 859,800 solved power flow instances across IEEE standard test cases with N/N-1/N-2 contingencies.

## PFDelta Integration (v0.3)

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

## PGLib-OPF Benchmarking

GAT also supports benchmarking against **PGLib-OPF**, the IEEE PES benchmark library for optimal power flow. Unlike PFDelta (power flow), PGLib provides pre-formulated OPF test cases in MATPOWER format.

### Dataset Overview

PGLib-OPF (https://github.com/power-grid-lib/pglib-opf) provides:
- **Standardized OPF test cases** from IEEE and industry
- **MATPOWER format** (.m files) directly importable
- **Published optimal values** for validation
- **Multiple difficulty levels**: from 5-bus to 10,000+ bus networks

### Basic PGLib Benchmark

```bash
gat benchmark pglib \
  --pglib-dir /path/to/pglib-opf \
  --out results_pglib.csv \
  --method socp \
  --max-cases 50
```

### OPF Method Selection

The `--method` option selects which OPF solver to benchmark:

| Method | Description | Default |
|--------|-------------|---------|
| `socp` | SOCP relaxation (convex, reliable) | ✅ Default |
| `ac` | Fast-decoupled linear approximation | |
| `dc` | DC optimal power flow (LP) | |
| `economic` | Economic dispatch (no network) | |

**Why SOCP is the default:** SOCP converges reliably on most test cases and provides a good balance of accuracy and speed. Full AC-NLP can be run separately using `gat opf ac-nlp` for individual cases where higher fidelity is needed.

### Full PGLib Example

```bash
# Benchmark all PGLib cases with SOCP
gat benchmark pglib \
  --pglib-dir ~/data/pglib-opf \
  --out pglib_socp_results.csv \
  --method socp \
  --threads auto \
  --tol 1e-6 \
  --max-iter 200

# Compare with DC-OPF
gat benchmark pglib \
  --pglib-dir ~/data/pglib-opf \
  --out pglib_dc_results.csv \
  --method dc

# Filter to specific cases
gat benchmark pglib \
  --pglib-dir ~/data/pglib-opf \
  --case-filter "case118" \
  --out pglib_case118.csv
```

### Comparing Against Baseline

If you have a CSV with published optimal values:

```bash
gat benchmark pglib \
  --pglib-dir ~/data/pglib-opf \
  --baseline published_optima.csv \
  --out comparison.csv
```

The output will include objective gap percentages against the baseline.

### CLI Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `--pglib-dir` | Path to PGLib-OPF directory | Required |
| `--out` | Output CSV path | Required |
| `--method` | OPF method (socp, ac, dc, economic) | `socp` |
| `--case-filter` | Filter cases by name pattern | All |
| `--max-cases` | Limit number of cases (0=all) | `0` |
| `--threads` | Parallel threads (auto=CPU count) | `auto` |
| `--tol` | Convergence tolerance | `1e-6` |
| `--max-iter` | Maximum solver iterations | `200` |
| `--baseline` | Optional baseline CSV for comparison | None |

## References

- **PFDelta Project**: https://github.com/MOSSLab-MIT/pfdelta
- **PFDelta Dataset**: https://huggingface.co/datasets/pfdelta/pfdelta
- **PFDelta Paper**: https://arxiv.org/abs/2510.22048 (MOSSLab preprint)
- **PGLib-OPF**: https://github.com/power-grid-lib/pglib-opf
- **Crate**: `crates/gat-io/src/sources/pfdelta.rs`
- **CLI**: `gat benchmark pfdelta --help`, `gat benchmark pglib --help`
- **Tests**: `crates/gat-cli/tests/benchmark_pfdelta.rs`

## DPLib: Distributed ADMM OPF Benchmarking

GAT implements the **ADMM (Alternating Direction Method of Multipliers)** algorithm for distributed optimal power flow, following the DPLib paper approach (arXiv:2506.20819). The `gat benchmark dplib` command compares distributed ADMM-OPF against centralized SOCP solutions.

### Algorithm Overview

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

### Basic DPLib Benchmark

Compare ADMM distributed OPF against centralized SOCP on PGLib cases:

```bash
gat benchmark dplib \
  --pglib-dir /path/to/pglib-opf \
  --out results_dplib.csv \
  --num-partitions 4 \
  --max-cases 50
```

### CLI Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `--pglib-dir` | Path to PGLib-OPF directory | Required |
| `--out` | Output CSV path | Required |
| `--case-filter` | Filter cases by name pattern | All |
| `--max-cases` | Limit number of cases (0=all) | `0` |
| `--threads` | Parallel threads (auto=CPU count) | `auto` |
| `--num-partitions` | Number of ADMM partitions (0=auto) | `0` |
| `--max-iter` | Maximum ADMM iterations | `100` |
| `--tol` | Primal/dual convergence tolerance | `1e-4` |
| `--rho` | Initial penalty parameter (ρ) | `1.0` |
| `--subproblem-method` | Local OPF method (dc, socp) | `dc` |

### Output Format

Results CSV includes columns for both centralized and distributed solves:

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

### Example Experiments

#### Partition Scaling Study

Test how ADMM performance scales with partition count:

```bash
for k in 2 4 8 16; do
  gat benchmark dplib \
    --pglib-dir ~/data/pglib-opf \
    --num-partitions $k \
    --case-filter "case300" \
    --out dplib_k${k}.csv
done
```

#### Subproblem Method Comparison

Compare DC vs SOCP for local subproblems:

```bash
# DC subproblems (fastest)
gat benchmark dplib \
  --pglib-dir ~/data/pglib-opf \
  --subproblem-method dc \
  --out dplib_dc.csv

# SOCP subproblems (more accurate)
gat benchmark dplib \
  --pglib-dir ~/data/pglib-opf \
  --subproblem-method socp \
  --out dplib_socp.csv
```

#### Penalty Parameter Tuning

Test different ρ values for convergence behavior:

```bash
for rho in 0.1 1.0 10.0 100.0; do
  gat benchmark dplib \
    --pglib-dir ~/data/pglib-opf \
    --rho $rho \
    --case-filter "case118" \
    --out dplib_rho${rho}.csv
done
```

### Performance Characteristics

Expected behavior on typical hardware:

| Network Size | Partitions | ADMM Time | Iterations | Speedup vs Centralized |
|-------------|-----------|-----------|------------|----------------------|
| 118-bus | 4 | ~50ms | 15-25 | 0.8-1.2x |
| 300-bus | 4 | ~100ms | 20-30 | 1.0-1.5x |
| 1354-bus | 8 | ~300ms | 25-40 | 1.5-2.5x |
| 2853-bus | 16 | ~500ms | 30-50 | 2.0-4.0x |

**Key observations:**
- ADMM overhead dominates on small networks (< 200 buses)
- Speedup becomes significant for large networks (> 1000 buses)
- More partitions = more parallelism but more consensus iterations
- DC subproblems are 5-10x faster than SOCP but may need more iterations

### Analysis with Python

```python
import pandas as pd
import matplotlib.pyplot as plt

# Load results
df = pd.read_csv('dplib_results.csv')

# Convergence rate
converged = df['admm_converged'].sum() / len(df)
print(f"ADMM convergence rate: {converged*100:.1f}%")

# Objective accuracy
df['gap_pct'] = df['objective_gap_rel'] * 100
print(f"Mean objective gap: {df['gap_pct'].mean():.3f}%")
print(f"Max objective gap: {df['gap_pct'].max():.3f}%")

# Speedup analysis
speedup_median = df['speedup_ratio'].median()
print(f"Median speedup: {speedup_median:.2f}x")

# Phase timing breakdown
total_admm = df['admm_time_ms'].sum()
x_pct = df['x_update_ms'].sum() / total_admm * 100
z_pct = df['z_update_ms'].sum() / total_admm * 100
dual_pct = df['dual_update_ms'].sum() / total_admm * 100
print(f"Phase breakdown: x={x_pct:.1f}%, z={z_pct:.1f}%, λ={dual_pct:.1f}%")
```

### Theoretical Background

ADMM solves the distributed OPF by forming an augmented Lagrangian:

```
L_ρ(x, z, λ) = Σ_k f_k(x_k) + λᵀ(x_k - z) + (ρ/2)||x_k - z||²
```

The penalty parameter ρ controls the trade-off:
- **Large ρ**: Faster consensus but suboptimal local solutions
- **Small ρ**: Better local solutions but slower consensus

Adaptive penalty scaling adjusts ρ based on residual ratios to balance convergence.

### References

- **DPLib Paper**: arXiv:2506.20819
- **Boyd et al.**: "Distributed Optimization and Statistical Learning via ADMM"
- **Crate**: `crates/gat-algo/src/opf/admm.rs`
- **Partitioning**: `crates/gat-algo/src/graph/partition.rs`
- **CLI**: `gat benchmark dplib --help`
