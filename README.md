# GRID ANALYSIS TOOLKIT (GAT)

GAT is a CLI-first toolkit for modeling, analyzing, and solving power-system problems across graph topology, power flow, and optimal dispatch. The workspace mirrors the canonical layout of `gat-core`, `gat-io`, `gat-algo`, `gat-ts`, `gat-viz`, and `gat-cli` so shared logic can feed both command-line interactions and downstream GUIs while keeping data formats (Arrow/Parquet/CSV) consistent.

## Quick start

1. **Install prerequisites**
   - [Rust toolchain](https://rustup.rs) (stable).
   - Optional: `bd` for issue tracking and `beads-mcp` if you want MCP integration.

2. **Build the CLI**
   ```bash
   cargo build --package gat-cli
   ```

3. **Run a sample workflow**
   ```bash
   gat pf dc test_data/matpower/case9.arrow --out out/dc-flows.parquet
   gat opf dc test_data/matpower/case9.arrow \
     --cost test_data/opf/costs.csv \
     --limits test_data/opf/limits.csv \
     --piecewise test_data/opf/piecewise.csv \
     --out out/dc-opf.parquet
   gat opf ac test_data/matpower/case9.arrow \
     --out out/ac-opf.parquet \
     --tol 1e-6 --max-iter 20
   ```
   These commands demonstrate powering flows, enforcing generator limits, and exporting branch summaries in Parquet format.

4. **Inspect results**
   `gat pf dc` and `gat opf` commands emit branch flow tables (`branch_id`, `from_bus`, `to_bus`, `flow_mw`) which you can open with `polars`, `duckdb`, or any Parquet consumer.

## User manual

### CLI surface

`gat` exposes nested commands:

- `gat import {psse|matpower|cim}` — ingest RAW/MATPOWER/CIM into the internal Arrow network format.
- `gat validate dataset --spec spec.json` — ensure Arrow datasets follow expected schema.
- `gat graph {stats|islands|export}` — describe connectivity and export graph representations.
- `gat pf {dc|ac}` — run DC or AC power flow on stored networks.
- `gat nminus1 dc grid.arrow --contingencies test_data/nminus1/contingencies.csv --out results/nminus1.parquet` — run contingency screening and detect branch violations.
- `gat opf {dc|ac}` — solve optimal power flow variants.
- `gat ts {resample|join}` — resample telemetry feeds (time buckets) or align multiple series.
- `gat viz plot` — stub visualization helper using `gat-viz`.
- `gat viz` (future) — work with telemetry and plotting primitives.

Inspect `gat --help` and `gat <command> --help` for full flags.

### DC power flow (`gat pf dc`)

Runs a linear DC power flow with default injections (two bus injections if the network has ≥2 buses).
Outputs branch summary in Parquet via `--out`.

Command:
```
gat pf dc grid.arrow --out flows.parquet
```

### AC power flow (`gat pf ac`)

Newton–Raphson solver over the internal admittance matrices. Specify tolerance or iteration limit for convergence.
```
gat pf ac grid.arrow --tol 1e-8 --max-iter 20 --out flows.parquet
```

### DC optimal power flow (`gat opf dc`)

Inputs:
- `--cost BUS_ID,MARGINAL_COST` CSV (required).
- `--limits BUS_ID,PMIN,PMAX,DEMAND` CSV (required) describing dispatch bounds and the demanded injection.
- Optional `--branch-limits BRANCH_ID,FLOW_LIMIT` and `--piecewise BUS_ID,START,END,SLOPE`. Piecewise segments must cover `[pmin,pmax]` per bus, no gaps or overlaps.

Example:
```
gat opf dc grid.arrow \
  --cost test_data/opf/costs.csv \
  --limits test_data/opf/limits.csv \
  --piecewise test_data/opf/piecewise.csv \
  --out out/dc-opf.parquet
```

The solver uses `good_lp` with the Clarabel backend and enforces branch limits, reporting violations if any cost solution exceeds them.

### AC optimal power flow (`gat opf ac`)

Newton–Raphson linearization on B′, solves reduced system assuming bus angles, and emits branch flows. Tolerances/defaults:

```
gat opf ac grid.arrow --out ac-opf.parquet --tol 1e-6 --max-iter 20
```

Outputs follow the same Parquet schema as DC flows but are computed with the converged AC angles.

### Time-series tools (`gat ts resample/join`)

The `gat ts resample` command buckets telemetry into fixed intervals and reports per-bucket statistics:

```
gat ts resample test_data/ts/telemetry.parquet \
  --rule 5s \
  --timestamp timestamp \
  --value value \
  --out out/telemetry.resampled.parquet
```

Use `gat ts join` to align multiple feeds on a shared timestamp column before analysis or visualization:

```
gat ts join test_data/ts/telemetry.parquet \
  test_data/ts/telemetry_extra.parquet \
  --on timestamp \
  --out out/telemetry.joint.parquet
```

Refer to `docs/TS.md` for additional usage notes on file formats and grouping options.

### N-1 DC screening (`gat nminus1 dc`)

Runs a contingency screen by temporarily removing each listed branch and recomputing the DC flow. The command summarizes how the remaining lines behave and flags any branch that exceeds optional flow limits.

```
gat nminus1 dc grid.arrow \
  --contingencies test_data/nminus1/contingencies.csv \
  --branch-limits test_data/opf/branch_limits.csv \
  --out results/nminus1.parquet
```

Each row in the output Parquet file corresponds to a branch outage and includes columns such as `max_abs_flow_mw`, `violated`, and `violation_branch_id` so you can rank contingencies by their worst violations.

### State estimation WLS (`gat se wls`)

Run the Weighted Least Squares estimator by providing measurement values (flows or injections) along with optional measurement weights. The solver treats the smallest bus ID as the slack angle (0) and estimates the remaining bus angles.

```
gat se wls grid.arrow \
  --measurements test_data/se/measurements.csv \
  --out results/se-measurements.parquet \
  --state-out results/se-states.parquet
```

Measurement CSV format:

| column | description |
| --- | --- |
| `measurement_type` | `flow` or `injection` |
| `branch_id` | required for `flow`; the `branch` ID to compare |
| `bus_id` | required for `injection`; the bus ID for the power injection |
| `value` | measured value (MW) |
| `weight` | positive weight (1/variance) |
| `label` | optional descriptor for reporting |

The command writes a Parquet table of residuals (`value`, `estimate`, `residual`, `normalized_residual`, `weight`) and can emit the solved angles via `--state-out`. Use `test_data/se/measurements.csv` for a minimal fixture.

### Fixtures and regression data

`test_data/opf/` includes:

- `costs.csv`, `limits.csv` for dispatch modeling.
- `branch_limits.csv` demonstrating flow constraints.
- `piecewise.csv` covering `[pmin,pmax]` for multi-segment cost.
- `nminus1/contingencies.csv` describing branch outages (`branch_id,label`) used by `gat nminus1 dc`.
- `se/measurements.csv` with `measurement_type,branch_id,bus_id,value,weight,label` for the WLS estimator.
- `ts/telemetry.parquet` and `ts/telemetry_extra.parquet` for resample/join examples.

Use them to seed CLI runs or unit tests.

## Development hints

- Run `cargo test -p gat-algo` and `cargo test -p gat-cli` after changes.
- Follow `bd` instructions in `AGENTS.md` for tracking work (`bd create`, `bd ready`, etc.).
- Keep planning docs in `history/` if you need to record design decisions.

## Further work

Future milestones include DC/AC contingency screening, state estimation (WLS), time-series tools, visualization/export, and packaging scripts (`scripts/deploy_staging.sh`). Refer to `ROADMAP.md` for the overall plan and acceptance criteria.

See `docs/VIZ.md` for the current visualization stub and how it ties into `gat-viz`.
