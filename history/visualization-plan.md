# Grid visualization plan

## Objective
Provide a shared layout backend that uses `petgraph` + `fdg-sim` to compute node positions and surface that data through `gat-cli` (new `gat graph visualize` command) and `gat-tui` (Canvas preview), so both tools can show the same force-directed grid view.

## Steps
1. **Layout library (`gat-viz`)**
   - Add `fdg-sim` + `petgraph` dependencies.
   - Implement `layout_network(network, iterations)` that builds a `ForceGraph` from `gat-core::Network`, runs the simulation, and returns serializable nodes/edges.
2. **CLI integration**
   - Extend `GraphCommands` with `Visualize` (iteration count/out path options).
   - In `gat-cli`, compute layout via `gat-viz`, serialize to JSON or print to stdout, and optionally write to a file.
3. **TUI visualization**
   - Introduce `LayoutPreview` that wraps `gat-viz` output for a sample network and keeps edges/nodes coordinates.
   - Add a `Canvas` panel in the TUI that draws the layout using `Line`/`Points`, reusing the new preview structure.
4. **Documentation/testing**
   - Add docs for the new CLI command and mention the TUI preview in the overview.
   - Add CLI tests exercising `gat graph visualize`, verifying output file/JSON.
5. **Tracking**
   - Create a `bd` issue referencing this plan so progress is visible.
