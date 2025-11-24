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
gat-tui
```

Or, if running from source during development:

```bash
cargo run -p gat-tui --release
```

Navigate with arrow keys, Tab to switch panes, Enter to select, Esc to close modals, `q` to quit. See `crates/gat-tui/README.md` for full keyboard shortcuts and feature details.

### GUI Dashboard

Coming in Horizon 7 (planned).

---

## Installation

### Quick Install (Recommended)

The modular installer lets you choose components on the fly and installs to `~/.gat` with no dependency on Rust:

```bash
curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh | bash
```

Then add to your PATH:

```bash
export PATH="$HOME/.gat/bin:$PATH"
```

#### Component Selection

By default, only the CLI is installed. Choose additional components:

```bash
# CLI + TUI (interactive dashboard)
GAT_COMPONENTS=cli,tui bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh)

# CLI + TUI + GUI dashboard (future)
GAT_COMPONENTS=cli,tui,gui bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh)

# Everything (CLI + TUI + GUI + solvers)
GAT_COMPONENTS=cli,tui,gui,solvers bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh)
```

Or from the downloaded script:

```bash
bash scripts/install-modular.sh --components cli,tui
bash scripts/install-modular.sh --prefix /opt/gat --components cli,tui,solvers
```

#### Installation Directory Structure

Everything installs under `~/.gat/`:

```
~/.gat/
├── bin/           # Executables (gat, gat-tui, gat-gui, gat-cli)
├── config/        # Configuration (gat.toml, tui.toml, gui.toml)
├── lib/solvers/   # Solver binaries and data
└── cache/         # Dataset cache, run history
```

### Alternative: Bundle Variants (Full Tarball)

If you prefer bundled releases with docs, download and unpack a variant:

```bash
# Full variant (CLI + TUI + all features)
curl -fsSL https://github.com/monistowl/gat/releases/download/v0.3.1/gat-0.3.1-linux-x86_64-full.tar.gz | tar xz
cd gat-0.3.1-linux-x86_64-full
./install.sh

# Headless variant (CLI only, minimal footprint)
curl -fsSL https://github.com/monistowl/gat/releases/download/v0.3.1/gat-0.3.1-linux-x86_64-headless.tar.gz | tar xz
cd gat-0.3.1-linux-x86_64-headless
./install.sh --variant headless
```

### Build from Source (Fallback)

If no binary is available for your platform, both installers fall back to a source build. This requires Rust:

Go to https://rustup.rs to install the Rust toolchain.

Then:

```bash
# Full variant (default): CLI + TUI + all features
cargo build -p gat-cli --release --all-features

# Headless (CLI only, minimal dependencies)
cargo build -p gat-cli --release --no-default-features --features minimal-io

# Analyst (CLI + visualization/analysis tools)
cargo build -p gat-cli --release --no-default-features --features "minimal-io,viz,all-backends"
```

The binary lands under `target/release/gat-cli`.

#### Feature Flags

* Default builds use the lightweight Clarabel backend. Enable other `good_lp` solvers:

  ```bash
  cargo build -p gat-cli --no-default-features --features "all-backends"
  ```

* To keep dependencies lean while supporting Parquet/IPC I/O:

  ```bash
  cargo build -p gat-cli --no-default-features --features "minimal-io"
  ```

### Shell Completions (After Installation)

Generate shell completions once `gat` is in your PATH:

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

### For Development

If you're contributing to GAT:

1. Install Rust: https://rustup.rs
2. Clone the repository and run `cargo build`
3. Optional helpers:
   * `bd` — the beads issue tracker (run `bd ready` before you start work)
   * `beads-mcp` — so MCP-compatible agents can inspect docs via `gat-mcp-docs`
   * `jq` — required by `scripts/package.sh`
4. See `RELEASE_PROCESS.md` for our branch strategy (experimental → staging → main)

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
gat-tui
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
gat-tui
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

## FAQ & Migration Guide

### Installation & Upgrades

**Q: I have v0.1. How do I upgrade to v0.3.1?**

A: The v0.3.1 release introduces a new modular installation system. Upgrade simply by re-running the installer:

```bash
curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh | bash
```

This installs to `~/.gat/bin/` by default (changed from `~/.local/bin/` in v0.1). Update your PATH:

```bash
export PATH="$HOME/.gat/bin:$PATH"
```

**Q: Can I keep both v0.1 and v0.3.1 installed?**

A: Yes. Use the `--prefix` flag to install v0.3.1 elsewhere:

```bash
bash scripts/install-modular.sh --prefix /opt/gat-0.3.1
```

Then choose which to use in your PATH by ordering the paths or using full paths.

**Q: What changed between v0.1 and v0.3.1?**

A: Major improvements include:

* **Modular installation** — Install only what you need (CLI, TUI, GUI, solvers)
* **Centralized config** — All config in `~/.gat/config/` instead of scattered locations
* **New TUI** — Interactive 7-pane dashboard for exploration and batch jobs
* **Distribution tools** — ADMS, DERMS, hosting-capacity analysis
* **Binary-first delivery** — Pre-built binaries for Linux/macOS x86_64 and ARM64
* **Improved CLI** — Better error messages, more commands, faster execution

See the release notes for the full changelog.

### Components & Features

**Q: Do I need the TUI?**

A: No. The CLI is fully featured and standalone. The TUI is optional and great for:
- Interactive data exploration
- Workflow visualization
- Running batch jobs with live status monitoring
- Checking reliability metrics on-the-fly

Install it with:

```bash
GAT_COMPONENTS=cli,tui bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh)
```

**Q: What are the solver components for?**

A: The `solvers` component includes additional solver backends (CBC, HiGHS) beyond the default Clarabel. Install with:

```bash
GAT_COMPONENTS=cli,solvers bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh)
```

Then use them with:

```bash
gat opf dc grid.arrow --solver cbc  # Use CBC instead of default Clarabel
```

**Q: What features are in the "headless" variant?**

A: `headless` includes the core CLI without TUI/GUI and minimal I/O dependencies. It's great for:
- Embedded systems or minimal containers
- Server-side batch jobs where no UI is needed
- Minimal binary size (~5 MB vs ~20 MB for full)

Install with:

```bash
bash scripts/install.sh --variant headless
```

### Configuration

**Q: Where does GAT store configuration?**

A: All config is in `~/.gat/config/`:

* `gat.toml` — Core CLI settings (data paths, solver preferences, logging)
* `tui.toml` — Terminal UI display and behavior
* `gui.toml` — GUI dashboard preferences (future)

Edit these files directly or use the TUI Settings pane.

**Q: How do I use a custom solver?**

A: Edit `~/.gat/config/gat.toml`:

```toml
[solver]
default_backend = "cbc"  # or "highs", "clarabel"
```

Or pass it per-command:

```bash
gat opf dc grid.arrow --solver highs
```

### Data & Output

**Q: Where does GAT store datasets and cache?**

A: Under `~/.gat/`:

* `lib/solvers/` — Solver binaries (read-only)
* `cache/` — Downloaded datasets and run history

Override with environment variables:

```bash
GAT_PREFIX=/data/gat  # Use different install location
GAT_CACHE_DIR=/var/cache/gat  # Custom cache
GAT_CONFIG_DIR=/etc/gat  # Custom config location
```

**Q: Can I pipe data between GAT commands?**

A: Yes. GAT outputs Arrow/Parquet, which is pipe-friendly:

```bash
gat pf dc grid.arrow | gat opf dc --in - --out dispatch.parquet
```

However, most workflows use intermediate files (faster, debuggable):

```bash
gat pf dc grid.arrow --out flows.parquet
gat opf dc grid.arrow --pf-file flows.parquet --out dispatch.parquet
```

### Troubleshooting

**Q: `gat` command not found after install?**

A: Add `~/.gat/bin/` to your PATH:

```bash
export PATH="$HOME/.gat/bin:$PATH"
```

Add this to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) to make it permanent.

**Q: How do I uninstall GAT?**

A: Simply remove the install directory:

```bash
rm -rf ~/.gat/
```

Or, if you installed elsewhere:

```bash
rm -rf /opt/gat/  # Or whatever prefix you used
```

**Q: I'm getting solver errors. How do I fix it?**

A: Install the solver binaries:

```bash
GAT_COMPONENTS=cli,solvers bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh)
```

Or build from source (slower but self-contained):

```bash
bash scripts/install.sh
```

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
