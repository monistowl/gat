# Power flow commands

`gat pf` runs the DC or AC power-flow solvers built on `gat-core` graphs plus the shared solver registry (`gat-core::solver::SolverKind`). The CLI exposes the same options for both solvers so you can switch backends, control threading, and stage the output for downstream tools.

## `gat pf dc <grid.arrow> --out flows.parquet`

Performs the classical DC approximation (`B' θ = P`) with configurable solver/backends:

* `--solver gauss|faer`: selects the linear solver registered in `gat-core::solver`.
* `--threads auto|N`: hints for Rayon’s thread pool.
* `--out-partitions col1,col2`: writes partitioned Parquet under `pf-dc/` while copying a canonical file to `flows.parquet`.

The command prints branch counts, min/max flow, and the `pf-dc/.run.json` manifest for `gat runs resume`.

## `gat pf ac <grid.arrow> --out flows.parquet`

Runs the AC Newton–Raphson driver with tolerance/iteration controls:

* `--tol 1e-6`: exit when max mismatch drops below this tolerance.
* `--max-iter 20`: stop after this many Newton steps even if not converged.
* `--solver` and `--threads` behave as in the DC command.
* The output lives under `pf-ac/` plus the canonical `flows.parquet`.

Both commands share `history/milestone3-plan.md`, which lists the planned validators, regression fixtures, and docs updates that make Milestone 3 trackable.
