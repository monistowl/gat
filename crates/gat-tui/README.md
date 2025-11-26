# gat-tui — Terminal Dashboard for GAT

Interactive terminal UI (TUI) for browsing datasets, executing commands, monitoring batch jobs, and visualizing grid analysis results without leaving the terminal.

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
6. **Analytics** — Multi-tab results: Reliability, Deliverability Score, ELCC, Power Flow
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
- **Reliability tab**: LOLE, EUE, thermal violations per scenario
- **Deliverability tab**: Delivery capability scores by zone
- **ELCC tab**: Effective load carrying capability distributions
- **Power Flow tab**: Congestion hotspots, line loading %, voltage violations
- Contextual metrics display with status colors (Good/Warning/Critical)

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
- `Tab` — Switch analytics tab
- `f` — Cycle between scenarios (when viewing results)

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

- **gat-cli** — Backend command execution
- **gat-core** — Grid types and solvers
- **gat-io** — Data formats and schemas
- **ratatui** — TUI framework

## See Also

- [GAT Main README](../../README.md)
- [AGENTS.md](../../AGENTS.md) for agent integration
- [RELEASE_PROCESS.md](../../RELEASE_PROCESS.md) for development workflow
