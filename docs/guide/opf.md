# Optimal Power Flow (OPF)

This reference describes the DC and AC OPF commands, their inputs, and the fixtures available in `test_data/opf`.

## DC OPF (`gat opf dc`)

Solves a linear dispatch problem with generator costs, limits, and demand. Optional branch or piecewise constraints extend the base model.

```bash
gat opf dc grid.arrow \
  --cost test_data/opf/costs.csv \
  --limits test_data/opf/limits.csv \
  --out results/dc-opf.parquet \
  [--branch-limits test_data/opf/branch_limits.csv] \
  [--piecewise test_data/opf/piecewise.csv]
```

### Inputs

* `--cost` (required): CSV with `bus_id,marginal_cost`. Missing rows default to `1.0`.
* `--limits` (required): CSV with `bus_id,pmin,pmax,demand`. Defines dispatch bounds and local load so the solver can balance injections.
* `--branch-limits` (optional): CSV with `branch_id,flow_limit`. The command rejects solutions that violate any listed limit and prints the violations.
* `--piecewise` (optional): CSV with `bus_id,start,end,slope`. Buses list contiguous segments covering their `[pmin,pmax]` range. When piecewise data exists, the solver charges `slope × volume` for each segment instead of the `marginal_cost` value.

### Output

* `--out` writes a Parquet table with `branch_id`, `from_bus`, `to_bus`, and `flow_mw`. The CLI prints flow ranges and counts for verification.

## AC OPF (`gat opf ac`)

Runs a Newton–Raphson solve over the AC equations and emits branch flows similar to DC.

```bash
gat opf ac grid.arrow \
  --out results/ac-opf.parquet \
  [--tol 1e-6] \
  [--max-iter 20]
```

* `--out`: same branch-summary schema as DC OPF.
* `--tol`: convergence tolerance for the largest real-power mismatch (default `1e-6`).
* `--max-iter`: cap on Newton iterations (default `20`). The command logs iteration counts and final mismatch.

## Fixtures

`test_data/opf` provides reusable CSVs for local experiments:

* `costs.csv`: sample marginal costs for buses `0` and `1`.
* `limits.csv`: matching `pmin`, `pmax`, and `demand` entries the DC solver can satisfy.
* `branch_limits.csv`: a tight limit that demonstrates violation reporting.
* `piecewise.csv`: two-piece segments covering each bus’s `[pmin,pmax]` range (e.g., bus 0: 0–3 + 3–5).

Use these fixtures for regression tests or quick experiments; the CLI commands above reference the same paths so you can rerun them locally.

For the state-estimation workflow, see `docs/guide/se.md` and `test_data/se/measurements.csv`. For telemetry resampling and joining, consult `docs/guide/ts.md` and the files under `test_data/ts`. The `test_data/nminus1/contingencies.csv` file pairs with `test_data/opf/branch_limits.csv` to recreate the sample N-1 scenario from the regression suite.
