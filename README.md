![image](./screenshot.png)

# GRID ANALYSIS TOOLKIT (GAT)

*A fast Rust-powered command-line toolkit for power-system modeling, flows, dispatch, and time-series analysis.*

If you're comfortable running simple CLI commands and want to start doing *real* grid analysis — without needing a giant Python stack or a full simulation lab — **GAT gives you industrial-grade tools in a form you can actually tinker with.** Everything runs as standalone commands, and all the heavy lifting is Rust-fast.

## Table of Contents

- [Why GAT?](#why-gat)
- [Interfaces](#interfaces)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [CLI Reference](#cli-reference)
- [Common Workflows](#common-workflows)
- [Documentation](#documentation)
- [Architecture & Crates](#architecture--crates)

---

## Why GAT?

### For Beginners

* Start with *one command at a time*
* Outputs are Parquet/Arrow/CSV — easy to open in Python, R, DuckDB, Polars
* Commands behave like Unix tools: pipeable, scriptable, reproducible

### For Advanced Users

* Full DC/AC power-flow solvers
* DC/AC optimal power-flow (OPF) with costs and constraints
* N-1 contingency analysis and screening
* Time-series resampling, joining, aggregation
* State estimation (weighted least squares)
* **Distribution automation** (FLISR/VVO/outage coordination via ADMS)
* **DER analytics** (envelope aggregation, pricing-based scheduling via DERMS)
* **Distribution system modeling** (hosting-capacity analysis, AC OPF)
* **Interactive terminal UI** (TUI) for workflows, datasets, pipelines, and batch jobs
* **Reliability metrics** (LOLE, EUE, deliverability scores)

### Why Rust?

Rust gives you C-like execution speed without unsafe foot-guns. For grid models with thousands of buses/branches, that matters. Even on a laptop.

GAT scales with you:

* Two lines for a DC power flow
* A thousand AC-OPF scenarios on 20 machines when you need throughput
* All without Conda, Jupyter, or heavyweight clusters

---

## Interfaces

GAT works the way you do. Pick your interface:

### Command Line Interface (CLI)

For scripting, batch jobs, CI/CD pipelines, and reproducible workflows.

- All features available through the `gat` CLI
- Outputs in Arrow/Parquet for downstream tools (Polars, DuckDB, Spark)
- See `docs/guide/overview.md` for command reference

```bash
gat pf dc grid.arrow --out flows.parquet
gat opf dc grid.arrow --cost costs.csv --limits limits.csv --out dispatch.parquet
gat batch pf --manifest scenario_manifest.json --out batch_results
```

### Terminal UI (TUI)

For interactive exploration, workflow visualization, and real-time status monitoring.

The TUI is a 7-pane interactive dashboard built with Ratatui:

1. **Dashboard** — System health, KPIs (Deliverability Score, LOLE, EUE), quick-action toolbar
2. **Commands** — 19+ built-in command snippets, dry-run/execute modes, execution history, output viewer
3. **Datasets** — Catalog browser, upload manager, scenario template browser with validation
4. **Pipeline** — Workflow DAG visualization, transform step tracking, node details
5. **Operations** — Batch job monitor, allocation results, job status polling
6. **Analytics** — Multi-tab results: Reliability, Deliverability Score, ELCC, Power Flow with context metrics
7. **Settings** — Display, data, execution, and advanced preferences

Launch it with:

```bash
cargo run -p gat-tui --release
```

Navigate with arrow keys, Tab to switch panes, Enter to select, Esc to close modals, `q` to quit. See `crates/gat-tui/README.md` for full keyboard shortcuts and feature details.

### GUI Dashboard

Coming in Horizon 7 (planned).

---

## Installation

### 1. Install Rust (Required)

Go to https://rustup.rs. This installs `cargo` and the toolchain helpers used across the workspace.

### 2. Optional Helpers

These tools make documentation changes and CLI workflows easier:

* `bd` — the beads issue tracker (run `bd ready` before you start work)
* `beads-mcp` — so MCP-compatible agents can inspect docs via `gat-mcp-docs`
* `jq` — required by `scripts/package.sh`

### 3. Shell Completions (After Installation)

Generate shell completions once `gat` is installed:

```bash
gat completions bash | sudo tee /etc/bash_completion.d/gat > /dev/null
gat completions zsh --out ~/.local/share/zsh/site-functions/_gat
gat completions fish --out ~/.config/fish/completions/gat.fish
gat completions powershell --out ~/gat.ps1
```

Or source them on the fly:

```bash
source <(gat completions bash)
```

### 4. Binary-First Install (Recommended)

The installer fetches the right tarball for your OS/arch and only compiles from source when no binary is available.

```bash
# Headless: CLI + core (smallest footprint)
scripts/install.sh --variant headless

# Full: CLI + TUI + core + analysis tools
scripts/install.sh --variant full
```

Environment variables:

* `GAT_RELEASE_BASE` — override the release bucket (default: `https://releases.gat.dev/gat`)
* `GAT_VERSION` — pin a specific version; `latest` fetches `latest.txt` from the bucket
* `GAT_PREFIX` — change the install location (defaults to `~/.local`)

If your platform is not covered by prebuilt binaries, the installer falls back to a cargo build with the appropriate feature set for the variant you requested.

### 5. Build from Source (Fallback)

Headless (no TUI) builds keep the dependency footprint small:

```bash
cargo build -p gat-cli --no-default-features --features minimal-io
```

Enable optional UI and analysis tools:

```bash
cargo build -p gat-cli --features "viz"
cargo build -p gat-cli --all-features
```

GAT produces a `gat` binary under `target/debug/` or `target/release/`.

#### Feature Flags

* Default builds use the lightweight Clarabel backend. Enable other `good_lp` solvers:

  ```bash
  cargo build -p gat-cli --no-default-features --features "all-backends"
  ```

* To keep dependencies lean while supporting Parquet/IPC I/O:

  ```bash
  cargo build -p gat-cli --no-default-features --features "minimal-io"
  ```

### 6. Package Artifacts Locally

```bash
scripts/package.sh
```

This produces both variants under `dist/`:

* `gat-<version>-<os>-<arch>-headless.tar.gz` (CLI + core)
* `gat-<version>-<os>-<arch>-full.tar.gz` (CLI + TUI + docs)

---

## Quick Start

### 1. DC Power Flow (Fastest Starter)

```bash
gat pf dc test_data/matpower/case9.arrow --out out/dc-flows.parquet
```

**What this does:**

* Loads MATPOWER case9 as Arrow
* Solves the DC approximation (linear, very fast)
* Writes a Parquet branch-flow summary

### 2. DC Optimal Power Flow (Dispatch with Costs)

```bash
gat opf dc test_data/matpower/case9.arrow \
  --cost test_data/opf/costs.csv \
  --limits test_data/opf/limits.csv \
  --piecewise test_data/opf/piecewise.csv \
  --out out/dc-opf.parquet
```

**Inputs:**

* Cost per bus/generator
* Dispatch limits
* Demand
* Optional piecewise cost curve segments

**Outputs:**

* Feasible dispatch
* Branch flows
* Violation flags

### 3. AC Optimal Power Flow (Nonlinear)

```bash
gat opf ac test_data/matpower/case9.arrow \
  --out out/ac-opf.parquet \
  --tol 1e-6 --max-iter 20
```

Graduate from the linear DC baseline to Newton–Raphson solves.

### 4. Interactive Exploration with TUI

```bash
cargo run -p gat-tui --release
```

Browse datasets, check pipeline status, run commands with dry-run mode, view reliability metrics.

---

## CLI Reference

```
gat <category> <subcommand> [options]
```

### Data Import & Management

```
gat import {psse,matpower,cim}    # Import grid models
gat dataset public {list,describe,fetch}  # Fetch public datasets
gat runs {list,describe,resume}   # Manage previous runs
```

### Grid Analysis

```
gat graph {stats,islands,export,visualize}  # Network topology
gat pf {dc,ac}                    # Power flows
gat opf {dc,ac}                   # Optimal dispatch
gat nminus1 {dc,ac}               # Contingency screening
gat se wls                         # State estimation
```

### Time Series & Feature Engineering

```
gat ts {resample,join,agg}        # Time-series tools
gat featurize {gnn,kpi}           # Generate features
```

### Scenarios & Batch Execution

```
gat scenarios {validate,materialize,expand}  # Define what-if cases
gat batch {pf,opf}                # Parallel job execution
```

### Distribution Systems (ADMS/DERMS/DIST)

```
gat dist {pf,opf,hosting}         # Distribution modeling
gat adms {flisr,vvo,outage}       # Distribution automation
gat derms {aggregate,schedule,stress}  # DER analytics
gat alloc {rents,kpi}             # Allocation metrics
```

### Analytics & Insights

```
gat analytics {ptdf,reliability,elcc,ds,deliverability}  # Grid metrics
```

### Interfaces

```
gat tui                           # Interactive terminal dashboard
gat gui run                       # Web dashboard (stub)
gat viz [options]                 # Visualization helpers
gat completions {bash,zsh,fish,powershell}  # Shell completion
```

Use `gat --help` and `gat <command> --help` for detailed flags and examples.

---

## Common Workflows

### Import a Grid and Run Power Flow

```bash
gat import matpower case9.raw --out grid.arrow
gat pf dc grid.arrow --out flows.parquet
```

### Explore Interactively with TUI

```bash
cargo run -p gat-tui --release
# Then: Browse datasets, check pipeline, view reliability metrics
```

### Run N-1 Contingency Analysis at Scale

```bash
# 1. Define scenarios
gat scenarios validate --spec rts_nminus1.yaml

# 2. Materialize into executable form
gat scenarios materialize \
  --spec rts_nminus1.yaml \
  --grid-file grid.arrow \
  --out-dir runs/scenarios

# 3. Execute as batch
gat batch opf \
  --manifest runs/scenarios/rts_nminus1/scenario_manifest.json \
  --out runs/batch/rts_opf \
  --max-jobs 4

# 4. Inspect results
gat runs describe $(gat runs list --root runs --format json | jq -r '.[0].id')
```

### Analyze DER Hosting Capacity

```bash
gat dist hosting --grid grid.arrow --der-file ders.csv --out hosting_curves.parquet
```

### Extract Reliability Metrics

```bash
gat analytics reliability --grid grid.arrow --outages contingencies.yaml --out results.parquet
```

### Reproduce a Previous Run

All runs emit `run.json` with full argument list:

```bash
gat runs resume run.json --execute
```

Use `gat runs list --root <dir>` to inspect saved manifests and `gat runs describe <run_id> --root <dir> --format json` for metadata before resuming.

---

## Concepts (For Beginners)

### Power Flow (PF) — "What are the voltages and flows right now?"

* **DC PF:** fast, linear approximation
* **AC PF:** nonlinear, more accurate

### Optimal Power Flow (OPF) — "What's the cheapest feasible dispatch?"

* Adds costs, limits, and optional branch constraints
* Produces an optimized operating point

### N-1 Screening — "What happens if one thing breaks?"

* Remove one branch at a time
* Re-solve DC flows
* Summarize and rank violations

### State Estimation (SE) — "Given measurements, what's happening?"

Weighted least squares over branch flows & injections.

### Time-Series Tools — "Make telemetry usable"

* Resample fixed-width windows
* Join multiple streams on timestamp
* Aggregate across sensors

All outputs follow consistent Arrow/Parquet schemas.

---

## Outputs & Formats

All major commands emit **Parquet** because it is fast, columnar, and compatible with Polars, DuckDB, Pandas, Spark, R, etc.

Every run also emits `run.json` with the full argument list so you can reproduce runs with:

```bash
gat runs resume run.json --execute
```

This makes CI, batch jobs, and fan-out pipelines reproducible.

---

## Performance & Scalability

* Rust delivers fast execution in a single binary — no Conda or Python stack
* Each CLI command stands alone so you can fan them out across multiple machines
* Slice work embarrassingly parallel:

```bash
parallel gat pf dc grid.arrow --out out/flows_{}.parquet ::: {1..500}
```

If you know `xargs -P` or GNU `parallel`, you already know the essence of the workflow.

---

## Test Fixtures (Great for Learning)

* `test_data/matpower/` — MATPOWER cases
* `test_data/opf/` — cost curves, limits, branch limits
* `test_data/nminus1/` — contingency definitions
* `test_data/se/` — measurement CSVs
* `test_data/ts/` — telemetry examples

Modify these freely while experimenting.

---

## Public Datasets

Use `gat dataset public list` to preview curated datasets, optionally filtering with `--tag` or `--query`:

```bash
gat dataset public list --tag "ieee"
gat dataset public describe <id>
gat dataset public fetch <id>
```

Downloaded datasets default to `~/.cache/gat/datasets` (or `data/public` if unavailable). Override with:

* `--out <path>` — specify staging location
* `GAT_PUBLIC_DATASET_DIR` — set environment variable
* `--force` — refresh a cached copy
* `--extract` — unpack a ZIP file

Available datasets include:
- `opsd-time-series-2020` — Open Power System Data time series (CC-BY-SA 4.0)
- `airtravel` — lightweight US air travel CSV for time-series examples

---

## Documentation

### Getting Started

- `docs/guide/overview.md` — CLI architecture and command organization
- `docs/guide/pf.md` — Power flow (DC/AC) examples and troubleshooting
- `docs/guide/opf.md` — Optimal power flow with costs, limits, and solver selection

### Advanced Domains

- `docs/guide/adms.md` — Distribution automation (FLISR, VVO, outage coordination)
- `docs/guide/derms.md` — DER management (envelope aggregation, pricing, stress testing)
- `docs/guide/dist.md` — Distribution system analysis (AC flows, hosting capacity)

### Common Tasks

- `docs/guide/ts.md` — Time-series operations (resample, join, aggregate)
- `docs/guide/se.md` — State estimation (weighted least squares)
- `docs/guide/graph.md` — Network topology tools (stats, islands, visualization)
- `docs/guide/datasets.md` — Public dataset fetching and caching
- `docs/guide/gat-tui.md` — Terminal UI architecture and pane navigation

### Infrastructure & Workflows

- `docs/guide/cli-architecture.md` — Dispatcher, command modules, telemetry
- `docs/guide/feature-matrix.md` — CI/CD matrix testing with solver combinations
- `docs/guide/mcp-onboarding.md` — MCP server setup for agent integration
- `docs/guide/packaging.md` — Binary distribution and installation
- `docs/guide/scaling.md` — Multi-horizon scaling roadmap and performance tuning

### Auto-Generated Documentation

- `docs/cli/gat.md` — Full CLI command reference
- `docs/schemas/` — JSON schema for manifests and outputs

### Regenerate Documentation

After documentation changes, run:

```bash
cargo xtask doc all
```

This regenerates:

* CLI Markdown (`docs/cli/gat.md`)
* `gat.1` man page (`docs/man/gat.1`)
* JSON schemas (`docs/schemas/`)
* A minimal book site (`site/book/`)

Expose the tree to agents:

```bash
gat-mcp-docs --docs docs --addr 127.0.0.1:4321
```

---

## Architecture & Crates

### Core Crates

- **`gat-core`** — Grid types, DC/AC solvers, contingency analysis, state estimation
- **`gat-io`** — Data formats (Arrow, Parquet, CSV), schema definitions, I/O utilities
- **`gat-cli`** — Command-line interface, command modules, dispatcher
- **`gat-tui`** — Terminal UI (Ratatui-based), 7-pane dashboard

### Domain-Specific Crates

- **`gat-adms`** — FLISR/VVO/outage helpers for automatic distribution management
- **`gat-derms`** — DER envelope aggregation, pricing-based scheduling, stress-test runners
- **`gat-dist`** — MATPOWER import, AC flows, OPF, and hosting-capacity sweeps
- **`gat-algo`** — Advanced algorithms and solver backends (LP/QP abstraction)

### Support Crates

- **`gat-batch`** — Parallel job orchestration for batch solves
- **`gat-scenarios`** — Scenario definition, materialization, and manifest generation
- **`gat-schemas`** — Schema helpers for Arrow/Parquet consistency
- **`gat-ts`** — Time-series resampling, joining, aggregation
- **`gat-viz`** — Visualization and graph layout tools

For details on any crate, see its `README.md` in `crates/<crate>/`.

---

## Contributing

For local development:

1. Read `RELEASE_PROCESS.md` for our branch strategy (experimental → staging → main)
2. Check `AGENTS.md` for agent integration and MCP setup
3. Run tests with `cargo test -p gat-tui` (536+ tests currently)
4. Use `bd` to track issues: `bd ready` before starting, `bd close` when done

---

## License

See `LICENSE` in the repository root.
