+++
title = "State Estimation"
description = "Practical state estimation with WLS, bad data detection, and observability analysis"
weight = 13
+++

# State Estimation

State estimation determines the most likely operating state of a power network from noisy, redundant measurements. GAT provides weighted least squares (WLS) estimation with bad data detection and observability analysis.

<div class="grid-widget" data-network="ieee14" data-height="380" data-voltage="true" data-flow="true" data-legend="true" data-caption="Interactive: State estimation determines voltage magnitudes (V) and power flows (⚡) from noisy measurements."></div>

## Overview

| Command | Purpose |
|---------|---------|
| `gat se wls` | Run WLS state estimation |
| `gat se validate` | Check measurement observability |
| `gat se generate` | Generate synthetic measurements from power flow |

## Quick Start

### 1. Generate Synthetic Measurements

Create test measurements from a solved power flow:

```bash
# First run power flow to get "true" state
gat pf ac grid.arrow --out pf_results.parquet

# Generate measurements with noise
gat se generate \
  --grid grid.arrow \
  --pf-results pf_results.parquet \
  --noise-sigma 0.02 \
  --out measurements.csv
```

### 2. Run State Estimation

```bash
gat se wls grid.arrow \
  --measurements measurements.csv \
  --out se_results.parquet \
  --state-out estimated_state.parquet
```

### 3. Compare Estimated vs True State

```bash
# View estimation quality
gat inspect summary se_results.parquet

# Compare in Python
python3 << 'EOF'
import polars as pl

true_state = pl.read_parquet("pf_results.parquet")
estimated = pl.read_parquet("estimated_state.parquet")

# Join and compute errors
comparison = true_state.join(estimated, on="bus_id")
print(f"Max voltage error: {comparison['vm_error'].max():.4f} pu")
print(f"Max angle error: {comparison['va_error'].max():.4f} deg")
EOF
```

## Measurement File Format

### Schema

The measurement CSV must contain these columns:

| Column | Type | Description |
|--------|------|-------------|
| `measurement_type` | string | `voltage`, `flow`, or `injection` |
| `bus_id` | int | Bus ID for voltage/injection (null for flows) |
| `branch_id` | int | Branch ID for flow measurements (null for others) |
| `value` | float | Measured value (MW, MVAr, or pu voltage) |
| `weight` | float | Measurement weight (typically 1/σ²) |
| `label` | string | Optional identifier for reporting |

### Example Measurements File

```csv
measurement_type,bus_id,branch_id,value,weight,label
voltage,1,,1.02,10000,V1
voltage,2,,0.98,10000,V2
injection,1,,45.2,2500,P1
injection,2,,-23.1,2500,P2
flow,,1-2,15.3,2500,P12
flow,,2-3,8.7,2500,P23
```

### Measurement Types

**Voltage Magnitude (`voltage`)**
- Measures |V| at a bus in per-unit
- High accuracy (σ ≈ 0.004-0.01 pu)
- Weight typically 10,000 - 60,000

**Power Injection (`injection`)**
- Net P or Q at a bus in MW/MVAr
- Moderate accuracy (σ ≈ 0.01-0.02 pu)
- Weight typically 2,500 - 10,000

**Branch Flow (`flow`)**
- P or Q flow on a branch in MW/MVAr
- Moderate accuracy (σ ≈ 0.01-0.02 pu)
- Branch ID format: `from-to` (e.g., `1-2`)

## WLS State Estimation

### Basic Usage

```bash
gat se wls grid.arrow \
  --measurements measurements.csv \
  --out residuals.parquet
```

### Full Options

```bash
gat se wls grid.arrow \
  --measurements measurements.csv \
  --out residuals.parquet \
  --state-out estimated_state.parquet \
  --max-iterations 20 \
  --tolerance 1e-6 \
  --format json
```

**Options:**
- `--measurements` — Input measurement file (CSV or Parquet)
- `--out` — Output residuals file
- `--state-out` — Optional: estimated state (bus voltages/angles)
- `--max-iterations` — Maximum Gauss-Newton iterations (default: 20)
- `--tolerance` — Convergence tolerance (default: 1e-6)
- `--format` — Output format: `parquet` (default) or `json`

### Output: Residuals File

| Column | Description |
|--------|-------------|
| `measurement_id` | Original measurement label |
| `value` | Original measured value |
| `estimate` | Estimated value from state |
| `residual` | value - estimate |
| `normalized_residual` | residual / σ |
| `weight` | Measurement weight |

### Output: State File

| Column | Description |
|--------|-------------|
| `bus_id` | Bus identifier |
| `vm` | Estimated voltage magnitude (pu) |
| `va` | Estimated voltage angle (rad) |

### Interpreting Results

```bash
# Check convergence and fit quality
gat se wls grid.arrow --measurements m.csv --out r.parquet --format json | jq

# Output example:
# {
#   "converged": true,
#   "iterations": 4,
#   "objective_j": 23.45,
#   "degrees_of_freedom": 25,
#   "chi_squared_threshold": 37.65,
#   "bad_data_detected": false
# }
```

**Key metrics:**
- **objective_j**: Sum of weighted squared residuals (should be < chi_squared_threshold)
- **degrees_of_freedom**: m - n (measurements minus state variables)
- **chi_squared_threshold**: At 99% confidence level

## Bad Data Detection

GAT automatically performs chi-squared and largest normalized residual (LNR) tests.

### Automatic Detection

```bash
gat se wls grid.arrow \
  --measurements measurements.csv \
  --detect-bad-data \
  --chi-sq-confidence 0.99 \
  --lnr-threshold 3.0 \
  --out results.parquet
```

### Analyzing Bad Data

```bash
# Find measurements with large normalized residuals
gat se wls grid.arrow --measurements m.csv --out r.parquet

# Filter bad measurements
python3 << 'EOF'
import polars as pl

residuals = pl.read_parquet("r.parquet")
bad = residuals.filter(pl.col("normalized_residual").abs() > 3.0)
print(f"Suspicious measurements:\n{bad}")
EOF
```

### Iterative Bad Data Removal

```bash
# Remove bad data and re-estimate
gat se wls grid.arrow \
  --measurements measurements.csv \
  --remove-bad-data \
  --max-removals 3 \
  --out cleaned_results.parquet \
  --removed-out bad_measurements.csv
```

**Options:**
- `--remove-bad-data` — Enable iterative removal
- `--max-removals` — Maximum measurements to remove (default: 5)
- `--removed-out` — File listing removed measurements

## Observability Analysis

Check if the measurement set can determine all state variables:

```bash
gat se validate grid.arrow --measurements measurements.csv
```

**Output:**
```
Observability Analysis
──────────────────────
Total buses:         14
State variables:     27 (2N-1)
Measurements:        45
Redundancy ratio:    1.67

Status: OBSERVABLE

Measurement coverage:
  Voltage:    10 buses (71%)
  Injection:  12 buses (86%)
  Flow:       23 branches (100%)
```

### Handling Unobservable Systems

If the system is unobservable:

```bash
# Identify unobservable buses
gat se validate grid.arrow --measurements measurements.csv --verbose

# Output shows:
# WARNING: System is UNOBSERVABLE
# Observable islands: 2
# Island 1: buses [1, 2, 3, 4, 5]
# Island 2: buses [6, 7, 8, 9]
# Unobservable buses: [10, 11, 12, 13, 14]
#
# Suggested pseudo-measurements:
#   - Add injection at bus 10
#   - Add flow on branch 9-10
```

### Adding Pseudo-Measurements

For unobservable regions, add pseudo-measurements with large variance:

```csv
measurement_type,bus_id,branch_id,value,weight,label
injection,10,,0.0,100,pseudo_P10
injection,11,,0.0,100,pseudo_P11
```

Low weight (100 vs 10000) ensures pseudo-measurements don't override real data.

## DC State Estimation

For faster estimation using DC power flow model (angles only, flat voltage assumed):

```bash
gat se wls grid.arrow \
  --measurements measurements.csv \
  --dc \
  --out dc_results.parquet
```

**DC mode:**
- Only uses real power measurements (P flows and injections)
- Estimates voltage angles only (magnitudes assumed 1.0 pu)
- Much faster for large systems
- Good approximation for transmission networks

## Workflows

### Real-Time State Estimation Pipeline

```bash
#!/bin/bash
# se_pipeline.sh - Production SE workflow

GRID="network.arrow"
MEASUREMENTS="scada_$(date +%Y%m%d_%H%M).csv"
RESULTS_DIR="./se_results"

mkdir -p "$RESULTS_DIR"

# 1. Validate measurements
echo "Checking observability..."
gat se validate "$GRID" --measurements "$MEASUREMENTS" || {
    echo "ERROR: Unobservable system"
    exit 1
}

# 2. Run state estimation with bad data detection
echo "Running state estimation..."
gat se wls "$GRID" \
    --measurements "$MEASUREMENTS" \
    --detect-bad-data \
    --remove-bad-data \
    --max-removals 5 \
    --out "$RESULTS_DIR/residuals.parquet" \
    --state-out "$RESULTS_DIR/state.parquet" \
    --removed-out "$RESULTS_DIR/bad_data.csv" \
    --format json > "$RESULTS_DIR/summary.json"

# 3. Check results
if jq -e '.converged' "$RESULTS_DIR/summary.json" > /dev/null; then
    echo "SE converged successfully"
    jq '.iterations, .objective_j' "$RESULTS_DIR/summary.json"
else
    echo "WARNING: SE did not converge"
fi

# 4. Alert on bad data
if [ -s "$RESULTS_DIR/bad_data.csv" ]; then
    echo "WARNING: Bad measurements detected:"
    cat "$RESULTS_DIR/bad_data.csv"
fi
```

### Comparing SE Methods

```bash
# Compare DC vs AC state estimation
gat se wls grid.arrow --measurements m.csv --dc --out dc_se.parquet
gat se wls grid.arrow --measurements m.csv --out ac_se.parquet

# Analyze differences
python3 << 'EOF'
import polars as pl

dc = pl.read_parquet("dc_se.parquet")
ac = pl.read_parquet("ac_se.parquet")

print("DC SE residual stats:")
print(dc["residual"].describe())

print("\nAC SE residual stats:")
print(ac["residual"].describe())
EOF
```

### Measurement Quality Analysis

```bash
# Generate report on measurement quality
gat se wls grid.arrow --measurements m.csv --out r.parquet

python3 << 'EOF'
import polars as pl

df = pl.read_parquet("r.parquet")

# Group by measurement type
quality = df.group_by("measurement_type").agg([
    pl.count().alias("count"),
    pl.col("normalized_residual").abs().mean().alias("mean_abs_residual"),
    pl.col("normalized_residual").abs().max().alias("max_abs_residual"),
    (pl.col("normalized_residual").abs() > 3.0).sum().alias("bad_count")
])

print("Measurement Quality by Type:")
print(quality)

# Identify worst measurements
worst = df.sort("normalized_residual", descending=True).head(5)
print("\nWorst measurements:")
print(worst.select(["measurement_id", "value", "estimate", "normalized_residual"]))
EOF
```

## Troubleshooting

### SE Does Not Converge

**Symptoms:** Max iterations reached, large objective function

**Solutions:**
1. Check measurement redundancy (should be > 1.5)
   ```bash
   gat se validate grid.arrow --measurements m.csv
   ```
2. Look for gross measurement errors
   ```bash
   gat se wls grid.arrow --measurements m.csv --out r.parquet
   # Check for |normalized_residual| > 10
   ```
3. Verify network topology matches measurements
4. Try DC estimation first (more robust)

### Large Objective Function Value

**Symptoms:** J >> chi-squared threshold, but converged

**Interpretation:** Bad data present

**Solutions:**
1. Enable bad data removal
   ```bash
   gat se wls --remove-bad-data --max-removals 5 ...
   ```
2. Check for systematic errors (calibration issues)
3. Review measurement weights (may be too tight)

### Unobservable System

**Symptoms:** `gat se validate` reports unobservable

**Solutions:**
1. Add measurements at critical buses
2. Add pseudo-measurements with low weight
3. Check for isolated islands (topology errors)
4. Verify branch status assumptions

### Memory Issues on Large Systems

For systems > 10,000 buses:

```bash
# Use iterative solver
gat se wls grid.arrow \
  --measurements m.csv \
  --solver iterative \
  --out results.parquet
```

## Integration Examples

### Python Integration

```python
import subprocess
import json
import polars as pl

def run_state_estimation(grid_path, measurements_path):
    """Run GAT state estimation and return results."""
    result = subprocess.run([
        "gat", "se", "wls", grid_path,
        "--measurements", measurements_path,
        "--out", "/tmp/se_results.parquet",
        "--state-out", "/tmp/se_state.parquet",
        "--format", "json"
    ], capture_output=True, text=True)

    summary = json.loads(result.stdout)
    residuals = pl.read_parquet("/tmp/se_results.parquet")
    state = pl.read_parquet("/tmp/se_state.parquet")

    return {
        "summary": summary,
        "residuals": residuals,
        "state": state
    }

# Usage
results = run_state_estimation("grid.arrow", "measurements.csv")
if results["summary"]["converged"]:
    print(f"Converged in {results['summary']['iterations']} iterations")
    print(f"Estimated state:\n{results['state']}")
```

### Rust Integration

```rust
use gat_algo::state_estimation::{StateEstimator, Measurement, SeConfig};

// Load network and measurements
let network = gat_io::load_network("grid.arrow")?;
let measurements = load_measurements("measurements.csv")?;

// Configure estimator
let config = SeConfig::default()
    .with_max_iterations(20)
    .with_tolerance(1e-6)
    .with_bad_data_detection(true);

// Run estimation
let estimator = StateEstimator::new(config);
let result = estimator.estimate(&network, &measurements)?;

println!("Converged: {}", result.converged);
println!("Iterations: {}", result.iterations);
println!("Objective J: {:.2}", result.objective);

// Access estimated state
for bus_state in &result.state {
    println!("Bus {}: V={:.4} pu, θ={:.2}°",
        bus_state.bus_id,
        bus_state.vm,
        bus_state.va.to_degrees()
    );
}
```

## Related Commands

- [Power Flow](@/guide/pf.md) — Generate "true" state for testing
- [Network Inspection](@/guide/inspect.md) — Validate network data
- [Time Series](@/guide/ts.md) — Process measurement time series

## Theory Reference

For mathematical foundations (WLS formulation, Jacobian structure, chi-squared tests):

- [State Estimation Theory](@/reference/state-estimation.md) — Detailed mathematics
- [Power Flow Theory](@/reference/power-flow.md) — Related equations
