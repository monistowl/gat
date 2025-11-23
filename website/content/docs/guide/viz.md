+++
title = "Visualization CLI"
description = "Visualization commands and plotting"
weight = 180
+++

# Visualization CLI

`gat viz plot` is a placeholder that runs the `gat-viz` helper and prints a summary string. It exists so visualization primitives stay wired into the CLI for future exporters.

```bash
gat viz plot test_data/matpower/ieee14.arrow --output out/example.png
```

The current backend returns a static string (`gat_viz::visualize_data()`), but the command demonstrates how the visualization helpers can hook into the CLI and default logging. Use it as a pattern when adding real SVG/PNG/Parquet exporters later.
