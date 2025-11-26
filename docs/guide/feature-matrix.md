# gat-cli Feature Matrix

The `.github/workflows/cli-feature-matrix.yml` CI job runs on every push or pull request to `main` and optionally via `workflow_dispatch`. It exercises `gat-cli` under multiple feature sets to ensure all solver combinations and UI integrations stay healthy.

## CI Configuration

- **Where it runs:** `ubuntu-latest` (the job installs `coinor-libcbc-dev` to make `coinor-libcbc` available to the `all-backends` feature set). The release workflow runs a similar solver discovery script on macOS so both Linux and macOS installers have parity in tooling.
- **What it runs:** `cargo test -p gat-cli --locked --no-default-features --features "<set>" -- --nocapture`.
- **Feature sets:**
  * `minimal` (solver-clarabel + lean I/O).
  * `minimal,full-io` (adds the full I/O stack so more data sources/types are exercised).
  * `minimal,full-io,viz` (also enables `gat-viz` for visualization helpers).
  * `all-backends` (switches to `all-backends`; runs the full solver pool, including `solver-coin_cbc` and `solver-highs`).

Running this matrix catches regressions where a feature flag environment might compile but not run under different solver stacks. Trigger it manually from the Actions tab with `workflow_dispatch`, e.g. when you need to rerun the entire matrix after a local fix before merging.

## v0.3.4 Feature Summary

| Feature | Status | Notes |
|---------|--------|-------|
| DC Power Flow | ✅ Stable | Linear B'θ=P with partitioned Parquet output |
| AC Power Flow | ✅ Stable | Newton-Raphson with Q-limit enforcement |
| DC OPF | ✅ Stable | LP with piecewise cost curves |
| SOCP OPF | ✅ Stable | Convex relaxation for fast solves |
| **Full AC-OPF** | ✅ New | Penalty-based L-BFGS (95.6% PGLib convergence) |
| N-1 Contingency | ✅ Stable | DC/AC screening |
| N-2 Contingency | ✅ Stable | 100% convergence validated |
| **PGLib Benchmark** | ✅ New | 68 MATPOWER cases, baseline comparison |
| PFDelta Benchmark | ✅ Stable | 859,800 contingency instances |
| Reliability (LOLE/EUE) | ✅ Stable | Monte Carlo simulation |
| Multi-Area CANOS | ✅ Stable | Zone-to-zone LOLE, corridor utilization |
| ADMS (FLISR/VVO) | ✅ Stable | Distribution automation |
| DERMS | ✅ Stable | DER aggregation and scheduling |
| TUI Dashboard | ✅ Stable | 7-pane interactive terminal UI |

## Solver Backends

| Backend | Status | Use Case |
|---------|--------|----------|
| Clarabel | ✅ Default | QP/SOCP (default, no external deps) |
| Coin CBC | ✅ Optional | MIP/LP (requires coinor-libcbc) |
| HiGHS | ✅ Optional | LP/MIP (high performance) |
| argmin L-BFGS | ✅ New | Full AC-OPF nonlinear optimization |
