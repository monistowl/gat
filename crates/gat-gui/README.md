# gat-gui — Interactive Grid Analysis Dashboard

**Status: Experimental (Active Development)**

A native desktop application for interactive power grid visualization and analysis, built with Tauri 2.0 + Svelte 5 + D3.js. Provides real-time power flow solving, contingency analysis, and batch execution with a force-directed graph visualization.

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

### PTDF Analysis (PtdfPanel)

Power Transfer Distribution Factors quantify how power injections redistribute across branches:

- **Interactive transfer selection** — Pick injection and withdrawal buses
- **Real-time PTDF computation** — Calculates sensitivity matrix on demand
- **Branch impact ranking** — Branches sorted by absolute PTDF factor
- **Flow change preview** — Shows MW impact for a standard 100 MW transfer
- **Integration with N-1** — PTDF underpins LODF-based contingency screening

**Theory:** PTDF[ℓ, i→j] = PTDF[ℓ, i] - PTDF[ℓ, j] represents the fraction of a transfer from bus i to bus j that flows on branch ℓ.

### Batch Job Execution (BatchJobPane)

Run parametric studies across multiple grid cases:

- **File pattern matching** — Select cases by wildcard pattern (e.g., `*.m`, `*.arrow`)
- **Analysis types** — DC/AC power flow, DC/AC OPF
- **Parallel execution** — Configure worker threads for throughput
- **Progress tracking** — Real-time status updates via polling
- **Results aggregation** — Success/failure summary with per-job details

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
│       ├── PtdfPanel.svelte     # PTDF transfer analysis
│       ├── ArchitectureDiagram  # System overview
│       ├── EducationDrawer      # Learning content
│       ├── ConfigPane           # Settings
│       ├── CommandBuilder       # CLI command builder
│       ├── BatchJobPane         # Batch execution UI
│       └── NotebookPane         # Research notebooks
├── src-tauri/              # Rust backend
│   └── src/
│       ├── lib.rs          # Tauri app setup
│       ├── commands.rs     # Tauri commands (IPC)
│       └── state.rs        # AppState for batch job tracking
└── static/                 # Static assets
```

### Tauri Commands (Backend → Frontend)

| Command | Description |
|---------|-------------|
| `list_cases` | List available test cases from pglib-opf |
| `load_case` | Load network from MATPOWER/Arrow file |
| `solve_power_flow` | Run AC Newton-Raphson solver |
| `solve_dc_power_flow` | Run DC linear approximation (B'θ = P) |
| `solve_dc_opf` | DC Optimal Power Flow with LMPs and congestion |
| `run_n1_contingency` | N-1 security screening with LODF |
| `get_ybus` | Get sparse admittance matrix |
| `compute_ptdf` | Compute PTDF factors for bus-to-bus transfer |
| `get_lodf_matrix` | Line Outage Distribution Factors for N-k analysis |
| `get_grid_summary` | Bus/branch counts, MW totals, voltage ranges, graph stats |
| `detect_islands` | Find disconnected network components |
| `get_thermal_analysis` | Pre-contingency thermal headroom analysis |
| `export_network_json` | Full network model export for external tools |
| `run_batch_job` | Start async batch job execution |
| `get_batch_status` | Poll batch job progress and results |
| `get_config` / `save_config` | Configuration management (~/.gat/config/gat.toml) |
| `get_config_path` | Get path to config file |
| `get_notebook_manifest` | List demo notebooks and quick actions |
| `read_notebook` / `init_notebook_workspace` | Notebook workspace support |

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

### PtdfPanel.svelte

Interactive PTDF transfer analysis:

- Two bus selectors (injection/withdrawal) populated from loaded network
- "Compute" button triggers `compute_ptdf` command
- Results table showing all branches with:
  - Branch name and terminal buses
  - PTDF factor (dimensionless, typically -1 to +1)
  - Flow change for 100 MW transfer
- Sorted by absolute impact for quick identification of critical branches

### State Management (state.rs)

Thread-safe application state for async operations:

```rust
pub struct AppState {
    pub batch_runs: Arc<Mutex<HashMap<String, BatchRun>>>,
}
```

- `BatchRun` tracks: run_id, status, completed/total counts, results, errors
- Enables polling-based progress updates for long-running batch jobs
- Managed by Tauri's state injection system

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
- CLI command builder is a stub (use terminal for complex commands)
- No undo/redo for layout changes
- Y-bus explorer doesn't scale well for very large matrices (>500 buses)

### Recently Implemented

- ✅ Batch job execution with async tracking and parallel workers
- ✅ PTDF analysis panel for transfer sensitivity studies
- ✅ LODF-based N-1 contingency screening with correct transfer formula
- ✅ DC Optimal Power Flow with LMPs and congestion detection
- ✅ Grid summary statistics (bus/branch counts, MW totals, graph density)
- ✅ Island detection for disconnected network analysis
- ✅ LODF matrix computation for N-k contingency analysis
- ✅ Pre-contingency thermal headroom analysis
- ✅ JSON export for external tools integration

Contributions and feedback welcome.

## Related Crates

| Crate | Description |
|-------|-------------|
| `gat-core` | Network graph model, linear system solvers, ID types |
| `gat-algo` | Power flow (AC/DC), OPF, contingency analysis (PTDF/LODF) |
| `gat-batch` | Parallel batch job runner with manifest support |
| `gat-io` | File I/O: MATPOWER, Arrow, PSS/E parsers |
| `gat-cli` | Command-line interface |
| `gat-tui` | Terminal UI (Ratatui-based) |
