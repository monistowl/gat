# State estimation guide

The `gat se wls` subcommand performs weighted least squares (WLS) estimation of bus voltage angles using DC-style measurements. It solves the reduced normal equations directly and reports both residuals and (optionally) estimated states.

## Command
```
gat se wls grid.arrow \
  --measurements test_data/se/measurements.csv \
  --out results/se-measurements.parquet \
  [--state-out results/se-states.parquet] \
  [--solver gauss|faer] \
  [--threads auto|<N>] \
  [--out-partitions run_id,date]
```

## Measurement CSV schema

| Column | Description |
| --- | --- |
| `measurement_type` | `flow` or `injection` |
| `branch_id` | Branch ID for `flow` measurements (leave empty for injections) |
| `bus_id` | Bus ID for `injection` measurements (leave empty for flows) |
| `value` | Measured MW value |
| `weight` | Positive weight (typically 1/variance); defaults to `1.0` when omitted |
| `label` | Optional friendly name for reporting |

Flow measurements use `(θ_i - θ_j) / x_ij`, while injection measurements use the susceptance (B′) row for the target bus. The solver fixes the smallest bus ID as the slack angle (0 rad), builds the reduced normal equations, and solves them with the requested backend.

## Outputs

- Measurement residuals (Parquet): includes `value`, `estimate`, `residual`, `normalized_residual`, and `weight` for each input row.
- Optional state Parquet (`--state-out`): `bus_id` → estimated `angle_rad`.
- CLI summary: degrees of freedom, chi-squared cost, and whether the solve converged.

Use `test_data/se/measurements.csv` as a starting fixture for experiments or regression tests.

**TODO:** Add a residual plot example and chi-squared threshold guidance for common measurement mixes.
