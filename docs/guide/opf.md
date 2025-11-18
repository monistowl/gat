# Optimal Power Flow guide

This guide covers the current DC and AC OPF commands exposed by the `gat` CLI. Both use the DC-style network model (lossless, small angles) and write Parquet outputs that align with the schemas under `docs/schemas/`.

## DC OPF (`gat opf dc`)

The DC OPF command solves a linear dispatch using generator costs, limits, and optional branch constraints.

**Command shape**
```
gat opf dc grid.arrow \
  --cost test_data/opf/costs.csv \
  --limits test_data/opf/limits.csv \
  --out results/dc-opf.parquet \
  [--branch-limits test_data/opf/branch_limits.csv] \
  [--piecewise test_data/opf/piecewise.csv] \
  [--lp-solver clarabel|coin_cbc|highs] \
  [--solver gauss|faer] \
  [--threads auto|<N>] \
  [--out-partitions run_id,date]
```

**Inputs**
- `--cost` (required): CSV with `bus_id,marginal_cost`. Missing entries default to `1.0`.
- `--limits` (required): CSV with `bus_id,pmin,pmax,demand`. Dispatch variables are clamped to `[pmin,pmax]` and must meet total demand.
- `--branch-limits` (optional): CSV with `branch_id,flow_limit` to enforce per-branch flow caps.
- `--piecewise` (optional): CSV with `bus_id,start,end,slope`. Segments must cover `[pmin,pmax]` without gaps; the solver charges `slope * segment_volume` per piece instead of `marginal_cost`.
- `--lp-solver` (optional): LP backend exposed through `good_lp` (`clarabel`, `coin_cbc`, or `highs`).
- `--solver` (optional): numerical backend for the DC linear solves (`gauss` or `faer`).
- `--threads` (optional): `auto` or an explicit thread count for parallel sections.
- `--out-partitions` (optional): comma-separated Parquet partitions (e.g., `run_id,date/contingency`).

**Output**
- `--out`: Parquet branch-flow table (`branch_id`, `from_bus`, `to_bus`, `flow_mw`) with a summary printed to stdout. Partitioning respects `--out-partitions`.

## AC OPF (`gat opf ac`)

The AC OPF command currently runs the same DC-style susceptance solve used by DC PF/OPF but iterates Newton updates for angles. It is useful as a bridge toward a fuller AC formulation.

**Command shape**
```
gat opf ac grid.arrow \
  --out results/ac-opf.parquet \
  [--tol 1e-6] \
  [--max-iter 20] \
  [--solver gauss|faer] \
  [--threads auto|<N>] \
  [--out-partitions run_id,date]
```

**Options**
- `--tol`: Convergence tolerance on the maximum power mismatch (default `1e-6`).
- `--max-iter`: Iteration cap before the solver aborts.
- `--solver`: Linear solver backend for each Newton step (`gauss` or `faer`).
- `--threads` / `--out-partitions`: same semantics as the DC command.

**Output**
- `--out`: Parquet branch-flow table derived from the converged angles (same schema as DC OPF). The CLI prints iteration count and mismatch statistics.

**TODO:** Add a worked nonlinear AC example once a full AC model replaces the current DC-style approximation.

## Fixtures (`test_data/opf`)

Use the bundled CSVs under `test_data/opf` to exercise both commands:

- `costs.csv`: sample marginal costs for a two-bus system.
- `limits.csv`: `pmin`, `pmax`, and `demand` entries solvable by the DC model.
- `branch_limits.csv`: modest limit for branch `0` to demonstrate constraint handling.
- `piecewise.csv`: contiguous segments for buses 0 and 1 covering each `[pmin,pmax]` range.

Combine these with `test_data/nminus1/contingencies.csv` to validate branch-limit enforcement during `gat nminus1 dc` runs.
