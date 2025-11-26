+++
title = "Overview"
description = "High-level introduction to GAT architecture and documentation"
weight = 1
+++

# GAT Documentation Guide

Auto-generated docs live under `docs/cli`, `docs/schemas`, and `docs/arrow`. The `xtask` targets keep the Markdown, manifest schema, and CLI output definitions versioned with the code:

- CLI reference (Markdown + man page): `cargo xtask doc cli`.
- Schema exports: `cargo xtask doc schemas`.
- Minimal site bundle: `cargo xtask doc site`.

Run `cargo xtask doc all` after making CLI, manifest, or schema changes to refresh everything. The MCP docs server (`gat-mcp-docs`) reads this layout and exposes it via HTTP/Model Context Protocol resources. See `docs/guide/doc-workflow.md` for the full beads (`bd`) issue workflow that keeps documentation updates, auto-doc regen, and issue tracking synchronized.

Need a schema reference? The Arrow schema guide (`/guide/arrow-schema/`) walks through each table and why it matters for Newton–Raphson / DC power flow solvers. Need to move between formats? `gat convert format` is the new unified converter that writes Arrow as an intermediate before emitting MATPOWER/PSS/E/CIM/PandaPower outputs; see `/guide/convert/` for examples.

For a high-level explanation of the CLI architecture, including the dispatcher in `crates/gat-cli/src/main.rs` and the modular `commands/` handlers (dataset archives/catalog/formats, runs list/describe/resume, analytics helpers, and GUI/Viz/TUI helpers), see `docs/guide/cli-architecture.md`.

## Graph overview

Topology commands (`gat graph stats`, `gat graph islands`, `gat graph export`) are described in `docs/guide/graph.md`. They share the same `Network` helpers that power the CLI and can write DOT exports for downstream visualization.

## Power flow overview

`gat pf {dc,ac}` is documented in `docs/guide/pf.md`; the CLI supports solver selection, threading hints, tolerances, and Parquet output that lives under `pf-dc/` or `pf-ac/` for manifest-driven automation.

## Outputs and partitions

Every heavy command writes into a stage-named directory (for example `pf-dc`, `opf-dc`, `nminus1-dc`, or `se-wls`) so dashboards and artifact stores can tell where work was produced. Use `--out-partitions <comma-separated-columns>` to split the Parquet output inside that stage directory by column values (e.g., `--out-partitions run_id,date/contingency` writes `stage/run_id=.../date=.../part-0000.parquet`). The stage-aware helper also respects the `run.json`/manifest layout so `gat runs resume` and downstream tools can follow the same tree.

### Install fallback

The `scripts/install.sh` helper prefers downloading a platform-specific tarball but falls back to building from source + copying the artifacts into your prefix when no binary is available. The fallback path runs the same `cargo build` invocation as the manual instructions and logs `Falling back to building from source...` before installing. You can verify that path with `scripts/check-install-fallback.sh`, which forces a download failure and ensures the source build branch is exercised and logged.
