# DERMS workflows (`gat derms`)

The `gat derms` namespace layers DER portfolio analytics on top of the distribution tables. Early commands validate assets, generate flexibility envelopes, produce simple schedules, and execute randomized stress tests.

## Schema
- `der_assets.parquet`: `asset_id`, `bus_id`, `phase`, `asset_type`, `p_min`, `p_max`, `q_min`, `q_max`, `ramp_up`, `ramp_down`, `energy_cap`, `soc_min`, `soc_max`, `efficiency`, `owner_id`, `agg_id`, `priority`, `cost_curve_id`.

## Commands
- `gat derms validate-assets der_assets.parquet`
  - Ensures all required columns are present.
- `gat derms envelope --assets der_assets.parquet --out der_envelopes.parquet [--group-by agg_id]`
  - Builds rectangular P–Q envelopes per group and writes vertex tables.
- `gat derms schedule --assets der_assets.parquet --out der_schedule.parquet [--summary-out der_schedule_summary.parquet --horizon 24 --timestep-mins 60]`
  - Emits a simple multi-period dispatch table (P/Q/SOC) with optional aggregation.
- `gat derms stress-test --assets der_assets.parquet --out derms_stress.parquet [--runs 20]`
  - Generates randomized curtailment/violation metrics for quick scenario sweeps.

## Notes
- Grouping defaults to `agg_id` but can target `bus_id` or any other asset column.
- Schedule outputs include a `timestep_minutes` column to simplify downstream joins with time-series data.
