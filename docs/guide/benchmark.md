# Benchmarking GAT Against Public Datasets

GAT includes integrated benchmarking tools for systematically evaluating AC-OPF solver performance against public power flow datasets. Three benchmark suites are supported:

1. **PGLib-OPF** (recommended) — 68 MATPOWER cases from industry-standard IEEE/PEGASE/GOC networks
2. **PFDelta** — 859,800 solved power flow instances with N/N-1/N-2 contingencies
3. **OPFData** — GNN-format JSON for machine learning benchmarks

## PGLib-OPF Integration (v0.5.5)

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

### v0.5.5 Benchmark Results

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

**Key improvements in v0.5.5:**
- Analytical Jacobian and Hessian computation for IPOPT
- Proper handling of synchronous condensers and negative Pg generators
- Bus shunt support (fixed capacitors/reactors) in Y-bus construction
- Constraint scaling / row equilibration for improved DC-OPF LP conditioning
- Zero-reactance epsilon handling (1e-6) for bus tie transformers
- Unit-aware newtype wrappers (Megawatts, Kilovolts, PerUnit) for compile-time unit safety

**Note:** The L-BFGS penalty method backend achieves ~2.9% median gap and is useful when IPOPT is unavailable.

---

## PFDelta Integration (v0.5.5)

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
