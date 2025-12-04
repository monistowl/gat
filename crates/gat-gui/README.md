# gat-gui — Interactive Grid Analysis Dashboard

**Status: Experimental Work in Progress**

A native desktop application for interactive power grid visualization and analysis, built with Tauri 2.0 + Svelte 5 + D3.js.

## Features

### Grid Visualization (GridView)

- **Three layout modes:**
  - **Force-directed** — Physics simulation for automatic node placement
  - **Schematic** — Engineering-style grid layout with voltage tiers
  - **Geographic** — Manual positioning with persistent coordinates

- **Visual encodings:**
  - Node color = voltage magnitude (green=1.0 pu, yellow=0.95, red<0.9)
  - Node size = power (MW load or generation)
  - Branch color = loading percentage (green<50%, yellow 50-80%, red>100%)
  - Generator markers (blue triangle)
  - Animated particles show power flow direction

- **Interaction:**
  - Drag nodes to reposition
  - Hover for detailed bus tooltips (voltage, angle, load, generation)
  - Zoom/pan with mouse or controls
  - Click to select buses

### Power Flow Analysis

- **DC Power Flow** — Fast linearized approximation (B'θ = P)
- **AC Power Flow** — Full Newton-Raphson with Q-limit enforcement
- Real-time solution status in HUD
- Branch flow and loading calculation

### N-1 Contingency Analysis

- Systematic single-branch outage screening
- DC power flow for each contingency
- Overload detection (>100% loading)
- Island detection (singular matrix)
- Results panel with:
  - Summary statistics
  - Worst contingency highlight
  - Sortable violation list
  - SECURE/VIOLATIONS status badge

### Y-bus Explorer (YbusExplorer)

- Interactive admittance matrix heatmap
- Sparsity pattern visualization
- Click cells to view complex values (G + jB)
- Color scale for magnitude
- Bus selection syncs with GridView

### Additional Features

- **Case browser** — Load PGLib-OPF test cases from sidebar
- **Hero cases** — One-click load + auto-solve (14, 118, 9241 bus)
- **Theme support** — Light/dark/system modes
- **Architecture diagram** — Visual system overview
- **Education drawer** — Context-sensitive learning content

## Getting Started

### Prerequisites

- Node.js 18+ and pnpm
- Rust toolchain (for Tauri backend)
- System dependencies for Tauri (see [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites))

### Development

```bash
cd crates/gat-gui

# Install dependencies
pnpm install

# Run in development mode (hot reload)
pnpm tauri dev
```

### Production Build

```bash
pnpm tauri build
```

Outputs to `src-tauri/target/release/`.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Space` | Solve AC power flow |
| `Shift+Space` | Solve DC power flow |
| `F` | Fit view to network |
| `1` | Force-directed layout |
| `2` | Schematic layout |
| `3` | Geographic layout |
| `+` / `=` | Zoom in |
| `-` / `_` | Zoom out |
| `0` | Reset zoom |

## Architecture

```
gat-gui/
├── src/                    # Svelte frontend
│   ├── routes/
│   │   └── +page.svelte    # Main application shell
│   └── lib/
│       ├── GridView.svelte      # D3 network visualization
│       ├── YbusExplorer.svelte  # Admittance matrix viewer
│       ├── ArchitectureDiagram  # System overview
│       ├── EducationDrawer      # Learning content
│       ├── ConfigPane           # Settings
│       ├── CommandBuilder       # CLI command builder
│       ├── BatchJobPane         # Batch execution
│       └── NotebookPane         # Research notebooks
├── src-tauri/              # Rust backend
│   └── src/
│       ├── lib.rs          # Tauri app setup
│       └── commands.rs     # Tauri commands (IPC)
└── static/                 # Static assets
```

### Tauri Commands (Backend → Frontend)

| Command | Description |
|---------|-------------|
| `list_cases` | List available test cases |
| `load_case` | Load network from file path |
| `solve_power_flow` | Run AC Newton-Raphson |
| `solve_dc_power_flow` | Run DC linear approximation |
| `run_n1_contingency` | N-1 security screening |
| `get_ybus` | Get admittance matrix |
| `get_config` / `save_config` | Configuration management |
| `get_notebook_manifest` | List demo notebooks |
| `read_notebook` / `init_notebook_workspace` | Notebook support |

### Data Flow

1. User selects case from sidebar
2. Frontend calls `load_case` via Tauri IPC
3. Backend parses MATPOWER file, returns `NetworkJson`
4. D3.js renders network graph
5. User clicks "DC" or "AC" solve button
6. Backend runs solver, returns updated voltages/flows
7. Frontend updates visualization colors

## Component Details

### GridView.svelte

The main visualization component using D3.js force simulation:

- `SimNode` — D3 node with bus data, generator flag, positions
- `SimLink` — D3 link with branch data
- Scales: `voltageColor` (node fill), `loadingColor` (branch stroke)
- HUD overlays: layout controls, zoom controls, legend, network stats, solve buttons

### YbusExplorer.svelte

Matrix heatmap with D3 scales:

- Fetches Y-bus via `get_ybus` command
- Renders sparse matrix as colored cells
- Hover shows G+jB values
- Click navigates to bus in GridView

### N-1 Panel

Slide-out results panel triggered by N-1 analysis:

- Summary grid: total contingencies, violations, islands, time
- Worst contingency card with overloaded branches
- Sortable list (by severity or branch ID)
- Status badge: "SECURE" (green) or "N VIOLATIONS" (red)

## Technology Stack

- **Tauri 2.0** — Rust backend with webview frontend
- **Svelte 5** — Reactive UI with runes (`$state`, `$derived`, `$effect`)
- **SvelteKit** — Routing and build tooling
- **D3.js v7** — Force simulation, scales, zoom behavior
- **TypeScript** — Type safety for frontend
- **Vite** — Fast dev server with HMR

## Experimental Status

This GUI is under active development. Known limitations:

- Large networks (>1000 buses) may have performance issues in force layout
- Geographic positions are not persisted across sessions
- Some features are stubs (batch jobs, CLI builder)
- No undo/redo for layout changes
- Y-bus explorer doesn't scale well for very large matrices

Contributions and feedback welcome.

## Related

- `gat-core` — Core solver library (power flow, contingency)
- `gat-tui` — Terminal UI alternative
- `gat-cli` — Command-line interface
