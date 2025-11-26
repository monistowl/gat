# GAT Documentation

The `docs/` tree combines auto-generated content with hand-authored guides.

## Contents

- **CLI reference** – `docs/cli/gat.md` (with `docs/man/gat.1` for Unix installs)
- **Schemas** – `docs/schemas/manifest.schema.json`, `docs/schemas/flows.schema.json`
- **Arrow layouts** – `docs/arrow/README.md` plus exported `schema.json` files
- **Historical plans** – `docs/archive/` (previously under `docs/plans/`)
- **Guides** – `docs/guide/` (see highlights below)

## Helpful commands

- `cargo xtask doc cli`: regenerate the Markdown CLI reference and `gat.1` man page.
- `cargo xtask doc schemas`: emit JSON schemas (manifests + Arrow outputs) into `docs/schemas/`.
- `cargo xtask doc site`: rebuild the minimal `site/book/` bundle that references the generated Markdown.
- `cargo xtask doc all`: run every doc target and refresh the guide overlays that `gat-mcp-docs` publishes.

Or just run `scripts/mcp-onboard.sh` to regenerate every doc target and start `gat-mcp-docs --docs docs --addr 127.0.0.1:4321` in one shot.

Use `gat-mcp-docs --docs docs --addr 127.0.0.1:4321` to preview the generated tree and expose it as MCP resources for agents.

## Guide highlights

- `docs/guide/overview.md`: explains how to keep the CLI references and man pages in sync via `cargo xtask doc all`.
- `docs/guide/doc-workflow.md`: outlines the beads (`bd`) issue workflow plus the doc-regeneration steps you should follow.
- `docs/guide/datasets.md`, `docs/guide/opf.md`, `docs/guide/se.md`, `docs/guide/ts.md`, `docs/guide/gui.md`, `docs/guide/viz.md`, and `docs/guide/packaging.md` document the curated workflows you can script from the CLI.
- `docs/guide/demos/README.md` shows how to add training-focused demos (the first one lives in `test_data/demos/reliability_pricing.py`).
- `docs/guide/gat-tui.md` introduces the Ratatui-powered `gat-tui` monitor for workflow visibility.
- `docs/guide/scaling.md`: describes the multi-horizon scaling roadmap along with concrete code/CLI targets.
- `docs/guide/arrow_schema.md`: provides a table-by-table tour of the folder-based Arrow dataset, highlighting how each column supports Newton–Raphson and DC power flow solvers.
- `docs/guide/convert.md`: documents the new `gat convert format` command that uses Arrow as the intermediary before emitting MATPOWER/PSS/E/CIM/PandaPower exports.
- `docs/guide/pandapower_schema.md`: maps PandaPower’s `net.bus`, `net.gen`, `net.line`, etc., into the shared Arrow schema so people can follow the conversion.
- `docs/ROADMAP.md`: the canonical plan for the workspace with phases, milestones, and deliverables.

Running `cargo xtask doc all` keeps this README and the guide content aligned so the MCP server has a single authoritative view.
