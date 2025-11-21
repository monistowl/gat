# CLI Architecture

`gat-cli`’s entry point is now intentionally small: `crates/gat-cli/src/main.rs` only parses the `cli::Commands` enum, configures `tracing`, and calls `run_and_log` with a handler for each subcommand. This keeps the `match` in `main.rs` short, makes every branch easy to audit, and keeps runtime telemetry uniformly funneled through `crates/gat-cli/src/commands/telemetry.rs`.

## `commands/` modules

All subcommands now live under `crates/gat-cli/src/commands/`. Each file or submodule focuses on a single domain:

* `commands/import.rs` and `commands/validate.rs` handle the data import/validation helpers and emit `record_run_timed` manifests.
* `commands/datasets/` contains `archives/` (RTS-GMLC, HIREN, Sup3rCC), `catalog/` (list/describe/fetch public datasets), and `formats/` (dsgrid, PRAS) so the dataset CLI is both modular and testable.
* `commands/runs/{list,describe,resume}.rs` each own the listing, manifest viewing, and resume flows that `gat runs` exposes.
* `commands/analytics/ptdf.rs` wraps the PTDF helper with solver setup, partition parsing, and telemetry so future analytics commands can follow the same pattern.
* `commands/version.rs` is a lightweight helper that reads `cargo metadata`, prints the canonical release version, optionally verifies a tag (`gat version sync --tag vX.Y.Z`), and writes a small manifest file so release automation can spot drift before packaging.
* Additional domain modules (dist/pf/opf/adms/derms/se/ts) remain in their own files but could be submodule split in future iterations.
* Helper modules such as `commands/completions.rs`, `commands/gui.rs`, `commands/viz.rs`, and `commands/tui.rs` keep UI/auxiliary logic outside of `main.rs` while still exposing a `handle` entry point.

## Telemetry manifest recording

`commands/telemetry.rs` bridges CLI commands to the manifest recorder in `gat_cli::manifest`. Each handler records the command name, CLI arguments, success/failure flag, and duration, which keeps automation dashboards in sync with the new modular layout.

## Shared release metadata

Scripts such as `scripts/install.sh` and `scripts/package.sh` now source `scripts/release-utils.sh` (which exposes `release_version`, `detect_os`, `detect_arch`, and `release_asset_base_name`). To avoid duplicating that logic in xtask, we added `scripts/platform-info.sh`, which prints the canonical os/arch/version/asset names, and the new `xtask release info` command merely runs that script so any automation (including release pipelines) can ask xtask for the same canonical metadata without re-encoding detection rules.

## Why this matters

This architecture makes it easier to reason about each command’s behavior, lowers the risk of merge conflicts in `main.rs`, and lets us add new analytics/dataset helpers without bloating the dispatcher. When you add a new CLI variant, add a new module under `commands/`, hook it up to telemetry, and call `run_and_log` from `main.rs` without touching the large match arms.
