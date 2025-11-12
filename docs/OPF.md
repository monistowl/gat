# OPF CLI Guide

This document describes how to run the DC and AC optimal power flow workflows that are available through the `gat` CLI and what inputs/outputs they expect.

## DC OPF (`gat opf dc`)

The DC OPF command solves a linear optimization problem with generator costs, dispatch limits, demand, and optional branch constraints.

**Command shape**
```
gat opf dc grid.arrow \
  --cost test_data/opf/costs.csv \
  --limits test_data/opf/limits.csv \
  --out results/dc-opf.parquet \
  [--branch-limits test_data/opf/branch_limits.csv] \
  [--piecewise test_data/opf/piecewise.csv]
```

**Inputs**
- `--cost` (required): CSV with `bus_id,marginal_cost`. Specifies the linear cost for each generator bus. Missing entries default to `1.0`.
- `--limits` (required): CSV with `bus_id,pmin,pmax,demand`. Defines the per-bus dispatch range and local demand that the solver must cover. The sum of dispatches equals the sum of the `demand` column.
- `--branch-limits` (optional): CSV with `branch_id,flow_limit`. If provided, DC OPF will refuse solutions that exceed any listed limit and emit a readable violation message.
- `--piecewise` (optional): CSV with `bus_id,start,end,slope`. Each bus can list contiguous segments that cover `[pmin,pmax]`. The solver creates incremental variables for each segment and enforces `segment.end >= next.start` with no gaps (up to numerical tolerance). When piecewise data exists for a bus, the solver ignores the `marginal_cost` entry for that bus and instead charges `slope * segment_volume` for each piece.

**Output**
- `--out`: Parquet file with a single table of branch flows (`branch_id`, `from_bus`, `to_bus`, `flow_mw`). The command prints a summary with the flow range and branch count.

## AC OPF (`gat opf ac`)

The AC OPF command uses a Newton–Raphson iteration to solve the linearized AC power flow, producing voltage angles and branch flows that respect the DC dispatch (the current implementation uses the default PTDF-style injections currently in the network data).

**Command shape**
```
gat opf ac grid.arrow \
  --out results/ac-opf.parquet \
  [--tol 1e-6] \
  [--max-iter 20]
```

**Inputs & options**
- `--out`: Parquet output containing the branch-flow summary (same schema as the DC command).
- `--tol`: Convergence tolerance for the maximum real-power mismatch (default `1e-6`).
- `--max-iter`: Maximum Newton iterations before the solver aborts with an error (default `20`).

The command logs the iteration count and final mismatch, and it uses the same branch summary as the DC solver, but the flows are computed with the converged AC angles.

## Fixtures (`test_data/opf`)

The repository ships a small `test_data/opf` directory with example CSVs:

- `costs.csv`: sample marginal costs for bus 0/1.
- `limits.csv`: `pmin`, `pmax`, and `demand` entries that the DC solver can solve.
- `branch_limits.csv`: lowish limit for branch `0` to demonstrate constraint enforcement.
- `piecewise.csv`: contiguous segments for bus 0 (0–3, 3–5) and bus 1 (0–4, 4–6) that cover each bus’s `[pmin,pmax]` range.

Use these fixtures in regression tests or as a starting point when authoring your own CSV inputs. The CLI commands above reference the same files so you can reproduce the tests locally.

For the state-estimation workflow, see `docs/SE.md` and `test_data/se/measurements.csv` for the measurement schema and CLI example.

## Contingency fixtures (`test_data/nminus1`)

- `contingencies.csv`: sample branch outages for `gat nminus1 dc` (`branch_id,label`). Combine with `test_data/opf/branch_limits.csv` to reproduce the N-1 scenario used in the regression test suite.
