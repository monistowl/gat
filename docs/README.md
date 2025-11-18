# GAT Auto-Docs

The `docs/` tree combines auto-generated content (`docs/cli`, `docs/schemas`, `docs/arrow`, `site/book`) with hand-authored guides under `docs/guide`.

## Helpful commands

- `cargo xtask doc cli`: regenerate the Markdown CLI reference and `gat.1` man page.
- `cargo xtask doc schemas`: emit JSON schemas (manifests + Arrow outputs) into `docs/schemas/`.
- `cargo xtask doc site`: rebuild the minimal `site/book/` bundle that references the generated Markdown.
- `cargo xtask doc all`: run every doc target and refresh the guide overlays that `gat-mcp-docs` publishes.

Use `gat-mcp-docs --docs docs --addr 127.0.0.1:4321` to preview the generated tree and expose it as MCP resources for agents.

## Guide highlights

- `docs/guide/overview.md`: explains how to keep the CLI references and man pages in sync via `cargo xtask doc all`.
- `docs/guide/doc-workflow.md`: outlines the beads (`bd`) issue workflow plus the doc-regeneration steps you should follow.
- `docs/guide/datasets.md`, `docs/guide/opf.md`, `docs/guide/se.md`, `docs/guide/ts.md`, `docs/guide/gui.md`, `docs/guide/viz.md`, and `docs/guide/packaging.md` document the curated workflows you can script from the CLI.
- `docs/guide/demos/README.md` shows how to add training-focused demos (the first one lives in `test_data/demos/reliability_pricing.py`).
- `docs/guide/scaling.md`: describes the multi-horizon scaling roadmap along with concrete code/CLI targets.
- `docs/ROADMAP.md`: the canonical plan for the workspace with phases, milestones, and deliverables.

Running `cargo xtask doc all` keeps `docs/index.md`, `docs/README.md`, and the guide content aligned so the MCP server has a single authoritative view.
