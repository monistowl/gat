+++
title = "Feature Matrix"
description = "gat-cli Feature Matrix"
weight = 33
+++

# gat-cli Feature Matrix

The `.github/workflows/cli-feature-matrix.yml` CI job runs on every push or pull request to `main` and optionally via `workflow_dispatch`. It exercises `gat-cli` under multiple feature sets to ensure all solver combinations and UI integrations stay healthy.

- **Where it runs:** `ubuntu-latest` (the job installs `coinor-libcbc-dev` to make `coinor-libcbc` available to the `all-backends` feature set). The release workflow runs a similar solver discovery script on macOS so both Linux and macOS installers have parity in tooling.
- **What it runs:** `cargo test -p gat-cli --locked --no-default-features --features "<set>" -- --nocapture`.
- **Feature sets:**
  * `minimal` (solver-clarabel + lean I/O).
  * `minimal,full-io` (adds the full I/O stack so more data sources/types are exercised).
  * `minimal,full-io,viz` (also enables `gat-viz` for visualization helpers).
  * `all-backends` (switches to `all-backends`; runs the full solver pool, including `solver-coin_cbc` and `solver-highs`).

Running this matrix catches regressions where a feature flag environment might compile but not run under different solver stacks. Trigger it manually from the Actions tab with `workflow_dispatch`, e.g. when you need to rerun the entire matrix after a local fix before merging.
