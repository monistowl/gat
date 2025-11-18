# Milestone 2 Implementation Plan

## Summary
Milestone 2 is all about exposing the topology/graph data that `gat` has already built during ingestion. The CLI should make it easy to inspect stats, find islands, and export the topology in Graphviz (DOT) or other human-readable formats, while the libraries underneath provide reusable helpers for future tooling.

## Deliverables
1. `graph_stats(network)` returns node/edge counts, degree distribution summaries, island counts, and density.
2. `find_islands(network)` enumerates connected components plus optional mapping of nodes to islands.
3. CLI commands:
   * `gat graph stats <grid.arrow>` prints the stats report.
   * `gat graph islands <grid.arrow> [--emit]` lists island counts and, when `--emit`, prints node→island assignments.
   * `gat graph export <grid.arrow> --format graphviz --out topo.dot` writes DOT (extendable to other formats).
4. Tests and docs: coverage for the new helpers plus updated doc sections (`docs/guide/graph.*`, roadmap mention) describing the CLI and expected outputs.

## Steps
1. **Graph utilities refinement**
   * Ensure `crates/gat-core/src/graph_utils.rs` exports the data needed by stats/islands (already contains `graph_stats`/`find_islands`/`export_graph`). Verify invariants (empty graphs, disconnected nodes) via unit tests.
   * Add helper for mapping `NodeIndex` → label when emitting Graphviz (reuse `node_label`).

2. **CLI subcommands**
   * Update `crates/gat-cli/src/cli.rs` to ensure `gat graph` group clearly exposes `stats`, `islands`, `export` (already present but confirm descriptions). Document default output formats (`graphviz/dot`).
   * In `gat-cli/src/main.rs`, hook each subcommand to `gat_core::graph_utils::*` functions, format output text (per current `GraphCommands::` logic), and add error handling/`tracing::info` lines for each path.
   * For `export`, allow specifying format values and output file; re-use `graph_utils::export_graph`'s format names.

3. **Testing & regression**
   * Add integration tests (or expand existing ones) in `gat-cli/tests` or `gat-core` to cover stats/islands/export outputs for small Arrow fixtures (use `test_data/matpower/`, `test_data/psse/sample.raw`).
   * Ensure CLI commands can read existing Arrow fixtures (maybe use `gat-io` helper to produce sample Arrow data for tests). Use `assert_cmd` to check the right messages/exports.

4. **Documentation updates**
   * Extend `docs/guide/graph` or create README sections describing how to use the new CLI commands, expected outputs, and tips for interpreting `gat graph stats`.
   * Update `docs/ROADMAP.md` to mention Milestone 2 deliverables are now live/completed (move into “Recent deliverables” once ready). Add CLI command references near the milestone section.

5. **Follow-ups**
   * Consider adding `gat graph export --format dot|graphviz` to accept custom attribute templates in future, or hooking in `gat viz`/`gat tui` for direct visualization.

## Risks & Notes
- Graph exports rely on node labels to be unique; if parsing imports produce duplicates, consider adding sanitized names or numbering.
- Large graphs may make DOT exports unwieldy; the CLI should document this and optionally allow filtering by notebook/CSV later.
