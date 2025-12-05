# gat-tui — Terminal Dashboard for GAT

Interactive terminal UI (TUI) for browsing datasets, executing commands, monitoring batch jobs, and visualizing grid analysis results without leaving the terminal.

Provides terminal-native equivalents of `gat-gui` features including power flow visualization, N-1 contingency screening, PTDF transfer analysis, and Y-bus matrix exploration—all accessible via keyboard navigation in any terminal emulator.

## Quick Start

```bash
cargo run -p gat-tui --release
```

Supports the latest CLI additions (pandapower import, dataset fetch/describe, runs replays).

## Architecture

**Seven-Pane Layout:**

1. **Dashboard** — System health, KPIs (deliverability score, LOLE, EUE), quick-action shortcuts
2. **Commands** — Snippet library with dry-run/full execution modes, execution history, output viewer
3. **Datasets** — Catalog browser, upload manager, scenario template browser
4. **Pipeline** — Workflow DAG visualization, transform step tracking
5. **Operations** — Batch job monitor, allocation results, job status polling
6. **Analytics** — Seven analysis tabs: Reliability, Deliverability, ELCC, Power Flow, N-1 Contingency, PTDF, Y-bus
7. **Settings** — Display, data, execution, and advanced preferences

## Features

### Dashboard Pane
- System health status (online/offline/warning)
- KPI cards: Deliverability Score, LOLE (hours/year), EUE (MWh/year)
- Quick-action toolbar: Reliability, Deliverability, ELCC, Power Flow buttons
- Recent runs list with timestamps and status

### Commands Pane
- Built-in command snippets (imports, datasets, analytics, batch) including pandapower auto-detect
- Custom command editor with syntax highlighting
- Dry-run vs. full execution modes (toggle with `d`)
- Execution history with status (✓ Success, ✗ Failed, ⟳ Running)
- Output viewer modal for detailed results

### Datasets Pane
- Three tabs: Catalog, Uploads, Scenarios
- Browse public datasets with tags and descriptions
- Upload progress tracking for new datasets
- Scenario template browser with validation status
- Metadata display (size, row count, last updated)

### Pipeline Pane
- DAG visualization of workflow steps
- Node types: Input, Transform, Analytics, Output
- Feature engineering and transformation tracking
- Click-through to view step details

### Operations Pane
- Batch job list with status indicators
- Job details: ID, type, progress, completion time
- Allocation results table (nodal revenue adequacy, cost recovery)
- Aggregate statistics: total contributions, average factors

### Analytics Pane

Seven analysis tabs provide comprehensive grid analytics in the terminal:

- **Reliability tab**: LOLE, EUE, thermal violations per scenario
- **Deliverability tab**: Delivery capability scores by zone
- **ELCC tab**: Effective load carrying capability distributions
- **Power Flow tab**: Congestion hotspots, line loading %, voltage violations
- **N-1 Contingency tab**: Single-branch outage screening with overload detection
- **PTDF tab**: Power Transfer Distribution Factor analysis for transfer sensitivity
- **Y-bus tab**: Admittance matrix explorer with multiple view modes

Contextual metrics display with status colors (Good/Warning/Critical).

#### N-1 Contingency Analysis

Systematic security screening for single-branch outages:

- Summary statistics: total contingencies, violations, failed solves
- Status badge: "SECURE" (green) or "N VIOLATIONS" (red)
- Sortable results table: outage branch, max loading %, violation count
- Per-contingency details: overloaded branches, loading percentages

#### PTDF Analysis (Power Transfer Distribution Factors)

Quantify how power injections redistribute across branches:

- **Bus selection**: Pick injection and withdrawal buses from dropdown
- **Transfer sensitivity**: PTDF factors for each branch (-1 to +1 range)
- **Flow change preview**: MW impact for a standard 100 MW transfer
- **Branch ranking**: Sorted by absolute PTDF factor for critical path identification

**Theory:** PTDF[ℓ, i→j] = PTDF[ℓ, i] - PTDF[ℓ, j] represents the fraction of a transfer from bus i to bus j that flows on branch ℓ.

#### Y-bus Matrix Explorer

Interactive admittance matrix visualization in the terminal:

- **Three view modes** (cycle with `v`):
  - **Heatmap**: ASCII grid with `░▒▓█` characters by magnitude
  - **List**: Table of (row, col, G, B, magnitude) entries
  - **Sparsity**: Pattern view with `·` for zero, `█` for non-zero
- Matrix dimensions and bus count header
- Cell selection shows complex value: Y[i,j] = G + jB

### Settings Pane
- **Display**: Theme, font size, color scheme
- **Data**: Cache TTL, max dataset size, memory limits
- **Execution**: Max parallel jobs, command timeout, retry policy
- **Advanced**: Debug logging, custom MCP server, CLI path configuration

## Navigation & Keyboard Shortcuts

**General:**
- `q` — Quit
- `Tab` / `Shift+Tab` — Cycle panes
- `↑` / `↓` `←` / `→` — Navigate within panes
- `Enter` — Select/Execute
- `Esc` — Cancel/Close modal

**Dashboard:**
- `r` — Run selected quick action
- `1-4` — Quick jump to action (Reliability, Deliverability, ELCC, PF)

**Commands:**
- `l` — Load selected snippet into editor
- `d` — Toggle dry-run/full mode
- `r` — Execute custom command
- `c` — Clear execution history
- `f` — Filter snippets

**Datasets:**
- `u` — Upload dataset
- `v` — Validate selected dataset
- `f` — Filter by name

**Operations:**
- `p` — Poll job status
- `s` — View allocation summary

**Analytics:**
- `Tab` / `Shift+Tab` — Switch analytics tab (forward/backward)
- `↑` / `↓` — Navigate result rows
- `f` — Cycle between scenarios (when viewing results)
- `c` — Run N-1 contingency analysis (on Contingency tab)
- `p` — Compute PTDF (on PTDF tab, after selecting buses)
- `y` — Load Y-bus matrix (on Y-bus tab)
- `v` — Cycle Y-bus view mode (Heatmap → List → Sparsity)
- `i` — Select injection bus (PTDF tab)
- `w` — Select withdrawal bus (PTDF tab)

## State Management

Uses **Ratatui** with event-driven architecture:
- **Pane States**: Each pane has a `*PaneState` struct tracking UI & data
- **Service Layer**: `TuiServiceLayer` bridges panes to real gat-cli execution
- **Async Events**: Background command execution with retry/backoff via event dispatcher
- **Caching**: 5-10 minute TTL on metrics to reduce command calls

## Real Backend Integration

All panes execute real `gat` CLI commands via `TuiServiceLayer`:

```rust
// Example: Dashboard refreshing KPIs
let metrics = service.get_dashboard_kpis(dataset_id, grid_id).await?;

// Example: Running analytics from Commands pane
let result = service.execute_custom_command(cmd_text, dry_run).await?;

// Example: Batch job execution from Operations pane
let job = service.execute_batch_power_flow(manifest, max_jobs).await?;
```

### GridService for Analysis Operations

The `GridService` manages loaded power system networks and provides analysis methods:

```rust
use gat_tui::services::GridService;

let service = GridService::new();

// Load a grid from Arrow file
let grid_id = service.load_grid_from_arrow("case14.arrow")?;

// Get Y-bus admittance matrix
let (n_bus, entries) = service.get_ybus(&grid_id)?;

// Compute PTDF for a transfer (bus 1 → bus 5)
let ptdf_results = service.compute_ptdf(&grid_id, 1, 5)?;

// Run N-1 contingency screening
let contingencies = service.run_n1_contingency(&grid_id)?;

// Get bus list for UI dropdowns
let buses = service.get_buses(&grid_id)?;
```

**Features:**
- Thread-safe network caching with `Arc<RwLock<HashMap>>`
- Support for Arrow and Matpower file formats via `gat-io`
- Y-bus extraction from branch impedance parameters
- PTDF sensitivity calculation for transfer analysis
- N-1 contingency screening with loading estimation

## Building & Features

**Standard build:**
```bash
cargo build -p gat-tui --release
```

**With all features enabled:**
```bash
cargo build -p gat-tui --all-features
```

**Run in debug mode:**
```bash
cargo run -p gat-tui
```

**Run with logging:**
```bash
RUST_LOG=gat_tui=debug cargo run -p gat-tui --release
```

## Configuration

Settings persist in `~/.config/gat-tui/config.toml`:

```toml
[display]
theme = "dark"
font_size = 12
color_scheme = "monokai"

[execution]
max_parallel_jobs = 4
command_timeout_secs = 30
retry_max_attempts = 3

[data]
cache_ttl_secs = 300
max_dataset_size_mb = 5000
```

## Testing

```bash
# Unit tests
cargo test -p gat-tui --lib

# Integration tests (covers pane workflows)
cargo test -p gat-tui --lib panes::integration_tests

# Full test suite
cargo test -p gat-tui
```

**Current Coverage (approx):**
- Core pane/unit tests plus integration coverage for multi-pane flows
- Run `cargo test -p gat-tui` for the exact count in your checkout

## Architecture Files

See `docs/guide/gat-tui.md` for:
- Detailed pane architecture and state management
- Event dispatcher and async integration
- Query builder adapters (CLI execution layer)
- Real command execution examples

## Related Crates

| Crate | Description |
|-------|-------------|
| `gat-cli` | Backend command execution |
| `gat-core` | Network graph model, Bus/Branch/Gen types, linear system solvers |
| `gat-algo` | Power flow (AC/DC), OPF, contingency analysis (PTDF/LODF) |
| `gat-io` | File I/O: Arrow, MATPOWER, PSS/E parsers |
| `gat-batch` | Parallel batch job runner with manifest support |
| `gat-gui` | Desktop GUI (Tauri + Svelte) with similar features |
| `ratatui` | TUI framework (Rust terminal rendering) |

## Feature Comparison: TUI vs GUI

| Feature | gat-tui (Terminal) | gat-gui (Desktop) |
|---------|-------------------|-------------------|
| **Power Flow** | Table + status colors | D3.js force graph |
| **N-1 Contingency** | Summary + sortable table | Panel with violation cards |
| **PTDF Analysis** | Bus selection + results table | Interactive dropdowns + table |
| **Y-bus Matrix** | ASCII heatmap/list/sparsity | D3.js color heatmap |
| **Batch Jobs** | Progress polling + status | Real-time progress bar |
| **Network View** | Tabular bus/branch lists | Force-directed graph |
| **Theme Support** | Terminal colors | Light/dark/system |
| **Keyboard Nav** | Full keyboard control | Mouse + keyboard |

**When to use TUI:**
- SSH sessions and remote servers
- Low-bandwidth connections
- Integration with tmux/screen workflows
- Preference for keyboard-driven interfaces

**When to use GUI:**
- Interactive network visualization
- Drag-and-drop node positioning
- Large matrix heatmaps (>100 buses)
- Mouse-driven exploration

## See Also

- [GAT Main README](../../README.md)
- [AGENTS.md](../../AGENTS.md) for agent integration
- [RELEASE_PROCESS.md](../../RELEASE_PROCESS.md) for development workflow
