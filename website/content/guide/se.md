+++
title = "State Estimation"
description = "State Estimation (WLS)"
weight = 13
+++

# State Estimation (WLS)

`gat se wls` runs a weighted least squares estimator over DC-style measurements.

```bash
gat se wls grid.arrow \
  --measurements test_data/se/measurements.csv \
  --out results/se-measurements.parquet \
  [--state-out results/se-states.parquet]
```

## Measurement schema

| Column | Description |
| --- | --- |
| `measurement_type` | `flow` or `injection` |
| `branch_id` | Branch ID for flow measurements (empty for injections) |
| `bus_id` | Bus ID for injection measurements (empty for flows) |
| `value` | Measured MW value |
| `weight` | Positive weight (typically 1/variance) |
| `label` | Optional name for reporting |

Flows model `(θ_i - θ_j) / x_ij` while injections use the row of the susceptance matrix at the target bus. The solver pins the smallest bus ID to angle 0 and solves the reduced normal equations `(HᵗWH)x = HᵗW(z - o)` with a sparse elimination routine.

## Outputs

* Measurement residuals (Parquet) with columns `value`, `estimate`, `residual`, `normalized_residual`, and `weight` for bad-data analysis.
* Optional state file (`--state-out`) mapping `bus_id` → estimated `angle_rad`.
* CLI summary prints the solved degrees of freedom and chi-squared cost.

Use `test_data/se/measurements.csv` as a regression fixture or quick experiment.
