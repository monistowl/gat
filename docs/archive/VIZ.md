# Visualization CLI

`gat viz` currently exposes a placeholder `plot` command that loads an Arrow grid, runs the shared `gat-viz` helper, and prints a summary message (plus optional output notice). It is designed to be extended with real plotting/graph exporting later.

## Command
```
gat viz plot test_data/matpower/ieee14.arrow --output out/example.png
```

Because the backend currently returns a static string (`gat_viz::visualize_data()`), this command primarily demonstrates how visualization helpers will eventually integrate with the CLI and default logging. Use it as a template when you add actual exporters (SVG/PNG/Parquet) in future iterations.
