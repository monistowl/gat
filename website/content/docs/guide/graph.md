+++
title = "Graph Analysis"
description = "Graph commands"
weight = 15
+++

# Graph commands

`gat graph` exposes topology inspection helpers built on `gat-core` graph utilities.

## `gat graph stats <grid.arrow>`
Prints a concise summary of the topology:
- total nodes/edges
- number of connected components (islands)
- degree min/avg/max plus density

```
gat graph stats test_data/matpower/case9.arrow
```

This command is useful for a quick sanity check after every import to make sure the network contains the expected topology.

## `gat graph islands <grid.arrow> [--emit]`
Lists each connected component and, if you pass `--emit`, prints the node → island assignment table with node labels. Example:

```
gat graph islands out/demos/cournot/cournot_grid.arrow --emit
```

The emitted assignment includes the node index, label, and island ID, making it easy to find the buses/loads that are isolated or part of small subgraphs.

## `gat graph export <grid.arrow> --format graphviz [--out topo.dot]`
Converts the graph into Graphviz (DOT) or other supported formats. When you pass `--out`, the CLI writes the DOT file to disk and prints the filename; otherwise, it prints the DOT text to stdout.

```
gat graph export grid.arrow --format graphviz --out topology.dot
```

## `gat graph visualize <grid.arrow> [--iterations 150] [--out layout.json]`

Runs a force-directed layout (via `fdg-sim`) and emits JSON with node positions/edges. Omitting `--out` prints the JSON to stdout; otherwise it writes to the provided file path. The layout JSON is shared with `gat-tui` to render the same preview.

```
gat graph visualize test_data/matpower/case9.arrow --iterations 120 --out layout.json
```
