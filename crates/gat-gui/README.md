# gat-gui — GUI Launcher for GAT

`gat-gui` is a small CLI that launches GUI experiences packaged with GAT. Today it wraps the notebook; future builds will add the dashboard.

## Usage

List available apps:
```bash
gat-gui list
```

Launch the notebook (Twinsong-inspired, tailored for GAT):
```bash
gat-gui notebook --workspace ./gat-notebook --port 8787 --open-browser
```

Write a launch summary to a file:
```bash
gat-gui notebook --workspace ./scratch --port 8787 --output launch.txt
```

## Options
- `--workspace PATH` (default `./gat-notebook`): directory to bootstrap.
- `--port PORT` (default `8787`): port to bind.
- `--open-browser`: request opening a browser after launch.
- `--output FILE`: write a human-readable launch summary to FILE.
- `--json`: emit a JSON summary to stdout (for scripts).

## Roadmap
- Add `dashboard` subcommand once the web UI is ready.
- Share CLI execution layer with `gat-tui` to avoid duplication.
