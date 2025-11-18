# State Estimation (WLS) — archived

> **Note:** The up-to-date state-estimation guide lives at `docs/guide/state_estimation.md`. Use this file only for historical reference.

Run the `gat se wls` command to perform a weighted least-squares estimation of bus angles given DC-style measurements (branch flows or bus injections).

## Command
```
gat se wls grid.arrow \
  --measurements test_data/se/measurements.csv \
  --out results/se-measurements.parquet \
  [--state-out results/se-states.parquet]
```

## Measurement CSV schema

| Column | Description |
| --- | --- |
| `measurement_type` | `flow` or `injection` |
| `branch_id` | Branch ID for `flow` measurements (leave empty for injections) |
| `bus_id` | Bus ID for `injection` measurements (leave empty for flows) |
| `value` | Measured MW value |
| `weight` | Positive weight (typically 1/variance) |
| `label` | Optional friendly name for reporting |

Flow measurements are modeled as `(θ_i - θ_j) / x_ij`, while injection measurements use the row of the susceptance (B′) matrix at the target bus. The solver fixes the smallest bus ID as the slack angle (0) and solves the reduced normal equations `(HᵗWH)x = HᵗW(z - o)` directly via a simple elimination routine.

## Outputs

- Measurement residuals (Parquet): columns include `value`, `estimate`, `residual`, `normalized_residual`, and `weight`. This file helps you inspect bad data or poorly instrumented branches.
- Optional state Parquet: `bus_id` → estimated `angle_rad` via `--state-out`.
- CLI summary prints the solved degrees of freedom and chi-squared cost.

Use `test_data/se/measurements.csv` as a minimal fixture for regression or quick experimentation.
