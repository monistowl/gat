+++
title = "GUI Dashboard"
description = "Native desktop application for interactive grid analysis"
weight = 170
+++

# GUI Dashboard

**Status: Experimental (Active Development)**

A native desktop application for interactive power grid visualization and analysis, built with Tauri 2.0 + Svelte 5 + D3.js.

## Features

| Feature | Description |
|---------|-------------|
| **GridView** | Force-directed network visualization with D3.js |
| **Power Flow** | DC and AC Newton-Raphson solving |
| **DC-OPF** | DC Optimal Power Flow with LMPs and congestion |
| **N-1 Contingency** | Single-branch outage screening with LODF |
| **PTDF Analysis** | Transfer sensitivity factors for bus pairs |
| **LODF Matrix** | Line Outage Distribution Factors for N-k analysis |
| **Grid Summary** | Bus/branch counts, MW totals, voltage ranges |
| **Island Detection** | Find disconnected network components |
| **Thermal Analysis** | Pre-contingency thermal headroom |
| **Y-bus Explorer** | Interactive admittance matrix heatmap |
| **Batch Jobs** | Parallel execution with progress tracking |
| **JSON Export** | Full network model export for external tools |

## Running the GUI

```bash
cd crates/gat-gui
pnpm install
pnpm tauri dev
```

## Architecture

```
gat-gui/
├── src/                    # Svelte frontend
│   ├── routes/+page.svelte # Main application shell
│   └── lib/
│       ├── GridView.svelte      # D3 network visualization
│       ├── YbusExplorer.svelte  # Admittance matrix viewer
│       ├── PtdfPanel.svelte     # PTDF transfer analysis
│       └── BatchJobPane.svelte  # Batch execution UI
├── src-tauri/              # Rust backend
│   └── src/
│       ├── commands.rs     # Tauri commands (IPC)
│       └── state.rs        # AppState for batch tracking
└── static/                 # Static assets
```

## Tauri Commands

| Command | Description |
|---------|-------------|
| `load_case` | Load network from MATPOWER/Arrow |
| `solve_power_flow` | Run AC Newton-Raphson |
| `solve_dc_power_flow` | Run DC linear approximation |
| `solve_dc_opf` | DC Optimal Power Flow with LMPs |
| `run_n1_contingency` | N-1 security screening |
| `get_ybus` | Get sparse admittance matrix |
| `compute_ptdf` | PTDF for bus-to-bus transfer |
| `get_lodf_matrix` | Line Outage Distribution Factors |
| `get_grid_summary` | Network statistics and counts |
| `detect_islands` | Find disconnected components |
| `get_thermal_analysis` | Pre-contingency thermal headroom |
| `export_network_json` | Full network export to JSON |
| `run_batch_job` | Start async batch execution |

## Technology Stack

- **Tauri 2.0** — Rust backend with webview frontend
- **Svelte 5** — Reactive UI with runes
- **D3.js v7** — Force simulation, scales, zoom
- **TypeScript** — Type safety for frontend

See [`crates/gat-gui/README.md`](https://github.com/monistowl/gat/blob/main/crates/gat-gui/README.md) for full documentation.
