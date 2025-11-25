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

## References

- **PFDelta Project**: https://github.com/MOSSLab-MIT/pfdelta
- **PFDelta Dataset**: https://huggingface.co/datasets/pfdelta/pfdelta
- **Paper**: https://arxiv.org/abs/2510.22048 (MOSSLab preprint)
- **Crate**: `crates/gat-io/src/sources/pfdelta.rs`
- **CLI**: `gat benchmark pfdelta --help`
- **Tests**: `crates/gat-cli/tests/benchmark_pfdelta.rs`
