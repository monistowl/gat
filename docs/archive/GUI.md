# GUI dashboard

`gat gui run` is a placeholder that currently loads an Arrow grid, ensures the shared `gat-gui` helper can produce a summary string (and optionally write a stub artifact), and prints status messages. It exists so the CLI surface mirrors the roadmap (`M7 GUI`), and it will be replaced with a real `egui`/`eframe` dashboard in future releases.

Command:
```
gat gui run test_data/matpower/ieee14.arrow --output out/gui-stub.txt
```

The helper writes `"stub visualization"` to the provided file path (if any) and returns a short summary so you can benchmark the glue between CLI and GUI crates before implementing the full UI.
