# GUI dashboard

`gat gui run` is a placeholder that loads an Arrow grid, exercises the shared `gat-gui` helper, and prints status messages. The stub keeps parity with the planned `M7 GUI` milestone so the CLI surface mirrors future `egui`/`eframe` dashboards.

```bash
gat gui run test_data/matpower/ieee14.arrow -o out/gui-stub.txt
```

The helper writes a stub string ("stub visualization") to `-o/--out` (if given) and prints a summary. This command is a reference implementation to ensure the CLI, shared plotting primitives, and installer scripts stay aligned until a full GUI ships.
