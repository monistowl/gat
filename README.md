![image](./screenshot.png)

# GRID ANALYSIS TOOLKIT (GAT)

*A fast Rust-powered command-line toolkit for power-system modeling, flows, dispatch, and time-series analysis.*

If you’re comfortable running simple CLI commands and want to start doing *real* grid analysis — without needing a giant Python stack or a full simulation lab — **GAT gives you industrial-grade tools in a form you can actually tinker with.** Everything runs as standalone commands, and all the heavy lifting is Rust-fast.

---

## 🌟 What Makes GAT worth learning?

**For beginners:**

* You can start with *one command at a time*.
* Outputs are in Parquet/Arrow/CSV — easy to open in Python, R, DuckDB, Polars.
* Commands behave like Unix tools: pipeable, scriptable, reproducible.

**For advanced users (where you may grow into):**

* Full DC/AC power-flow solvers
* DC/AC optimal power-flow (OPF)
* N-1 contingency analysis
* Time-series resampling, joining, aggregation
* State estimation (WLS)
* **Distribution automation** (FLISR/VVO/outage coordination via ADMS)
* **DER analytics** (envelope aggregation, pricing-based scheduling via DERMS)
* **Distribution system modeling** (hosting-capacity analysis, AC optimal power flows)
* **Interactive terminal UI** (TUI) for workflows, datasets, pipelines, and batch jobs
* **Reliability metrics** (energy unserved, loss-of-load expectation, delivery capability)

**Why Rust?**
Because Rust gives you C-like execution speed without unsafe foot-guns. For grid models with thousands of buses/branches, that matters. Even on a laptop.

GAT scales with you:

* Two lines for a DC power flow.
* A thousand AC-OPF scenarios on 20 machines when you need throughput.
* All without Conda, Jupyter, or heavyweight clusters.

---

## 🖥️ Choose Your Interface

GAT works the way you do:

**Command Line** — For scripting, batch jobs, CI/CD pipelines, and reproducible workflows.
- All features available through `gat` CLI
- Outputs in Arrow/Parquet for downstream tools (Polars, DuckDB, Spark)
- See `docs/guide/overview.md` for command reference

**Terminal UI (TUI)** — For interactive exploration, workflow visualization, and real-time status.
- Dashboard with reliability metrics and workflow status
- Commands pane with snippet library and dry-run mode
- Datasets, Pipeline, and Operations panes for job tracking
- `cargo run -p gat-tui --release` to launch

**GUI Dashboard** — Coming in Horizon 7 (planned).

---

# 📦 Installation

### 1. Install Rust (required)

Go to https://rustup.rs. This installs `cargo` and the toolchain helpers used across the workspace.

### 2. Optional helpers

These tools make documentation changes and CLI workflows easier:

* `bd` — the beads issue tracker (run `bd ready` before you start work).
* `beads-mcp` — so MCP-compatible agents can inspect docs via `gat-mcp-docs`.
* `jq` — required by `scripts/package.sh`.

### 2.1 Shell completions

Generate shell completions once `gat` is installed:

```bash
gat completions bash | sudo tee /etc/bash_completion.d/gat > /dev/null
gat completions zsh --out ~/.local/share/zsh/site-functions/_gat
gat completions fish --out ~/.config/fish/completions/gat.fish
gat completions powershell --out ~/gat.ps1
```

Alternatively source them on the fly:

```bash
source <(gat completions bash)
```

### 3. Binary-first install (recommended)

The installer fetches the right tarball for your OS/arch and only compiles from source when no binary is available.

```bash
# Headless: CLI + core (defaults)
scripts/install.sh --variant headless

# Full: GUI + TUI + docs
scripts/install.sh --variant full
```

Environment knobs:

* `GAT_RELEASE_BASE` — override the release bucket (default: `https://releases.gat.dev/gat`).
* `GAT_VERSION` — pin a specific version; `latest` fetches `latest.txt` from the bucket.
* `GAT_PREFIX` — change the install location (defaults to `~/.local`).

If your platform is not covered by prebuilt binaries, the installer falls back to a cargo build with the appropriate feature set for the variant you requested.

Use `scripts/check-install-fallback.sh` to exercise that path: the script points the installer at a non-existent release URL so the download always fails and the `Falling back to building from source ...` branch runs, producing headless binaries into a temporary prefix and proving the fallback path still logs/behaves as expected.

### 4. Build GAT from source (fallback)

Headless (no GUI/TUI) builds keep the dependency footprint small:

```bash
cargo build -p gat-cli --no-default-features
```

Enable the optional UI helpers when you need them:

```bash
cargo build -p gat-cli --features "viz gui tui"
```

GAT produces a `gat` binary under `target/debug/`.

For fast iteration that skips the heavier IO/Polars features, run:

```bash
cargo check -p gat-cli --no-default-features --features minimal-io
```
**Feature flags (optional):**

* Default builds use the lightweight Clarabel backend. Enable other `good_lp` solvers with:

  ```bash
  cargo build --package gat-cli --no-default-features --features "all-backends"
  ```

* To keep dependencies lean while still supporting Parquet/IPC I/O, use the minimal feature set:

  ```bash
  cargo build --package gat-cli --no-default-features --features "minimal"
  ```

### 5. Package artifacts locally

```bash
scripts/package.sh
```

This produces both variants under `dist/`:

* `gat-<version>-<os>-<arch>-headless.tar.gz` (CLI + core)
* `gat-<version>-<os>-<arch>-full.tar.gz` (CLI + GUI + TUI + docs)

---

# 🚀 Quick Demo: Your First GAT Workflow

## 1. DC Power Flow (fastest starter)

```bash
gat pf dc test_data/matpower/case9.arrow --out out/dc-flows.parquet
```

**What this does:**

* Loads MATPOWER case9 as Arrow.
* Solves the DC approximation.
* Writes a Parquet branch-flow summary.

---

## 2. DC Optimal Power Flow (dispatch with costs)

```bash
gat opf dc test_data/matpower/case9.arrow \
  --cost test_data/opf/costs.csv \
  --limits test_data/opf/limits.csv \
  --piecewise test_data/opf/piecewise.csv \
  --out out/dc-opf.parquet
```

**Inputs:**

* Cost per bus
* Dispatch limits
* Demand
* Optional piecewise curve segments

**Outputs:**

* Feasible dispatch
* Branch flows
* Violation flags

---

## 3. AC Optimal Power Flow (nonlinear)

```bash
gat opf ac test_data/matpower/case9.arrow \
  --out out/ac-opf.parquet \
  --tol 1e-6 --max-iter 20
```

AC OPF lets you graduate from the linear DC baseline to Newton–Raphson solves.

---

# 🧠 Beginner’s Section: Understanding GAT Concepts

### **Power Flow (PF)** — “What are the voltages and flows right now?”

* **DC PF:** fast, linear approximation
* **AC PF:** nonlinear, more accurate

### **Optimal Power Flow (OPF)** — “What’s the cheapest feasible dispatch?”

* Adds costs, limits, and optional branch constraints
* Produces an optimized operating point

### **N-1 Screening** — “What happens if one thing breaks?”

* Remove one branch at a time
* Re-solve DC flows
* Summarize and rank violations

### **State Estimation (SE)** — “Given measurements, what’s happening?”

Weighted least squares over branch flows & injections.

### **Time-Series Tools** — “Make telemetry usable.”

* Resample fixed-width windows
* Join multiple streams on timestamp
* Aggregate across sensors

All outputs follow consistent Arrow/Parquet schemas.

---

# 🛠 CLI Reference

```
gat <category> <subcommand> [options]
```

### **Data Import & Management**

```
gat import {psse,matpower,cim}    # Import grid models
gat dataset public {list,describe,fetch}  # Fetch public datasets
gat runs {list,describe,resume}   # Manage previous runs
```

### **Grid Analysis**

```
gat graph {stats,islands,export,visualize}  # Network topology
gat pf {dc,ac}                    # Power flows
gat opf {dc,ac}                   # Optimal dispatch
gat nminus1 {dc,ac}               # Contingency screening
gat se wls                         # State estimation
```

### **Time Series & Feature Engineering**

```
gat ts {resample,join,agg}        # Time-series tools
gat featurize {gnn,kpi}           # Generate features
```

### **Scenarios & Batch Execution**

```
gat scenarios {validate,materialize,expand}  # Define what-if cases
gat batch {pf,opf}                # Parallel job execution
```

### **Distribution Systems (ADMS/DERMS/DIST)**

```
gat dist {pf,opf,hosting}         # Distribution modeling
gat adms {flisr,vvo,outage}       # Distribution automation
gat derms {aggregate,schedule,stress}  # DER analytics
gat alloc {rents,kpi}             # Allocation metrics
```

### **Analytics & Insights**

```
gat analytics {ptdf,reliability,elcc,ds,deliverability}  # Grid metrics
```

### **Interfaces**

```
gat tui                           # Interactive terminal dashboard
gat gui run                       # Web dashboard (stub)
gat viz [options]                 # Visualization helpers
gat completions {bash,zsh,fish,powershell}  # Shell completion
```

Use `gat --help` and `gat <command> --help` for detailed flags and examples.

---

# ### **Scenario definitions & materialization**

Use `gat scenarios` to validate, expand, and materialize what-if cases before batch execution:

```bash
gat scenarios validate --spec examples/scenarios/rts_nminus1.yaml
gat scenarios materialize \
  --spec examples/scenarios/rts_nminus1.yaml \
  --grid-file test_data/matpower/ieee14.arrow \
  --out-dir runs/scenarios/rts_nminus1
```

Each run writes per-scenario `grid.arrow` files under `runs/scenarios/rts_nminus1/<scenario_id>/grid.arrow`, plus a `scenario_manifest.json` that catalogs `scenario_id`, `time_slices`, scaling factors, and tags so downstream `gat batch`/`gat analytics` commands know which artifacts to consume. Manifest metadata mirrors canonical contingency analysis conventions (`N-1` reliability per IEEE 1551/TPWRS DOI:10.1109/TPWRS.2007.901018) and is recorded in `run.json` for reproducibility.

Use `gat runs list` to inspect the most recent scenario materialization and then feed `runs/scenarios/rts_nminus1/scenario_manifest.json` into your batch/analytics pipelines.

---

### **Batch runs (`gat batch`)**

With the scenario manifest ready, `gat batch` fans out the desired solver (DC/AC PF or OPF) and writes one job per scenario plus a `batch_manifest.json` summary. The layout and counts follow IEEE-standard `N-1` enumeration practices (doi:10.1109/TPWRS.2007.899019).

```bash
gat batch pf \
  --mode dc \
  --manifest runs/scenarios/rts_nminus1/scenario_manifest.json \
  --out runs/batch/rts_pf \
  --threads auto \
  --max-jobs 4

gat batch opf \
  --mode dc \
  --manifest runs/scenarios/rts_nminus1/scenario_manifest.json \
  --out runs/batch/rts_opf \
  --cost examples/opf/costs.csv \
  --limits examples/opf/limits.csv \
  --branch-limits examples/opf/branch_limits.csv \
  --lp-solver clarabel
```

Each run emits `runs/batch/<name>/<job-id>/result.parquet` plus a `runs/batch/<name>/batch_manifest.json` that records job-level status, errors, and output paths; downstream tools can read this manifest instead of wrestling with raw directories.

---

# 🔬 Specialized domain workflows

Need more than the core CLI? These specialized crates back the higher-level workflows that handle reliability, DER, and distribution planning:

- **`gat-adms`** — FLISR/VVO/outage helpers for automatic distribution management. See [the crate README](crates/gat-adms/README.md) and `docs/guide/adms.md` for solver setups, reliability table expectations, and how the CLI wraps these routines.
- **`gat-derms`** — DER envelope aggregation, pricing-based scheduling, and stress-test runners. The crate README at `crates/gat-derms/README.md` plus `docs/guide/derms.md` explain how to source assets/prices and ingest the results.
- **`gat-dist`** — MATPOWER import, AC flows, OPF, and hosting-capacity sweeps for distribution cases; see `crates/gat-dist/README.md` and the PF/OPF guides in `docs/guide/pf.md`, `docs/guide/opf.md`, and `docs/guide/scaling.md` for detail.
- **`gat-schemas`** — Schema helpers placing Arrow/Parquet expectations; read `crates/gat-schemas/README.md` plus the generated schema artifacts under `docs/schemas/`.

### CLI feature matrix workflow

Every push or PR to `main` triggers `.github/workflows/cli-feature-matrix.yml`, which runs `cargo test -p gat-cli --locked --no-default-features` under four feature combinations: `minimal`, `minimal+full-io`, `minimal+full-io+viz`, and `all-backends`. Each job installs `coinor-libcbc-dev` (via `apt` on Ubuntu) before the tests run, matching the solver coverage documented in `gat`’s LP-solver flags, and you can re-run the matrix manually via `workflow_dispatch` from the Actions tab when you need to validate a new feature combo.

For more detail about the matrix strategy and solver dependencies, see `docs/guide/feature-matrix.md`.

---

# 📤 Outputs & Formats

All major commands emit **Parquet** because it is fast, columnar, and compatible with Polars, DuckDB, Pandas, Spark, R, etc.

Every run also emits `run.json` with the full argument list so you can reproduce runs with:

```bash
gat runs resume run.json --execute
```

This makes CI, batch jobs, and fan-out pipelines reproducible.

Use `gat runs list --root <dir>` to inspect all saved manifests and `gat runs describe <run_id> --root <dir> --format json` when you need the metadata before resuming.

---

# 🏎 Why Rust & Cluster Fan-Outs?

* Rust delivers fast execution in a single binary — no Conda or Python stack.
* Each CLI command stands alone so you can fan them out across multiple machines (different AC-PF cases, OPF scenarios, thousands of N-1 contingencies, telemetry streams).
* Instead of running one monolith, slice work embarrassingly parallel:

```bash
parallel gat pf dc grid.arrow --out out/flows_{}.parquet ::: {1..500}
```

If you know `xargs -P` or GNU `parallel`, you already know the essence of the workflow.

---

# 📚 Test Fixtures (Great for Learning)

* `test_data/matpower/` — MATPOWER cases
* `test_data/opf/` — cost curves, limits, branch limits
* `test_data/nminus1/` — contingency definitions
* `test_data/se/` — measurement CSVs
* `test_data/ts/` — telemetry examples

Modify these freely while experimenting.

---

# 📥 Public Dataset Fetching

Use `gat dataset public list` to preview the curated datasets, optionally filtering with `--tag` or `--query`, and `gat dataset public describe <id>` to inspect the source, license, and tags. Run `gat dataset public fetch <id>` to download the file — it defaults to `~/.cache/gat/datasets` (falling back to `data/public` if the cache directory is unavailable) but you can override the staging location with `--out` or set `GAT_PUBLIC_DATASET_DIR` in your environment.

Available datasets (network connectivity permitting):

- `opsd-time-series-2020` — an October 6, 2020 snapshot of the Open Power System Data 60-minute single-index time series (CC-BY-SA 4.0).
- `airtravel` — a compact US air travel passenger CSV that introduces a lightweight time-series size in the CLI.

Add `--force` to refresh a cached copy and `--extract` to unpack a ZIP file if a future entry ships compressed. The CLI prints the staged path so you can reuse it in downstream commands such as `gat ts join` or `gat pf dc`.

---

# 🗂 Documentation & Workflows

All curated docs now live under `docs/guide/` and the generated assets live under `docs/cli`, `docs/schemas`, `docs/arrow`, and `site/book/`. Key references:

* `docs/guide/doc-workflow.md` lays out the `bd` issue workflow plus the `cargo xtask doc all` steps that keep helpful docs in sync.
* `docs/guide/mcp-onboarding.md` explains how to run `scripts/mcp-onboard.sh`, read `docs/mcp/manifest.json`, and then execute the curated commands (via `scripts/mcp-manifest-run.sh commands`) or dataset fetchers that agents need for automated workflows.
* `docs/guide/adms.md`, `docs/guide/derms.md`, and `docs/guide/pf.md` describe the new ADMS/DERMS/distribution workflows (FLISR/VVO, DER scheduling, PTDF/power-flow recipes) that the workspace now covers via `gat-adms`, `gat-derms`, and `gat-dist`.
* `docs/guide/datasets.md`, `docs/guide/opf.md`, `docs/guide/se.md`, `docs/guide/ts.md`, `docs/guide/gui.md`, `docs/guide/viz.md`, `docs/guide/packaging.md`, and `docs/guide/scaling.md` capture curated workflows and scaling guidance.
* `docs/README.md` explains the auto-doc targets and how `gat-mcp-docs --docs docs --addr 127.0.0.1:4321` exposes the tree for agents.
* `docs/ROADMAP.md` is the canonical plan for the workspace.

After documentation changes, run `cargo xtask doc all` (and optionally `cargo xtask doc site`) so the MCP server and `site/book/` stay up to date.

---

# 📝 Auto-Documentation System

```bash
cargo xtask doc all
```

This regenerates:

* CLI Markdown (`docs/cli/gat.md`)
* `gat.1` man page (`docs/man/gat.1`)
* JSON schemas (`docs/schemas/`)
* A minimal book site (`site/book/`)

Reload `gat-mcp-docs --docs docs --addr 127.0.0.1:4321` to preview the results with MCP tooling.

---

# 🗺 Roadmap (High-Level)

* More advanced DC/AC contingency screening
* Broader SE functionality
* Better GUI dashboards
* More dataset importers (PSSE/CIM variants)
* Improved packaging & distribution

See `docs/ROADMAP.md` for the authoritative project plan with milestones, phases, and acceptance criteria.

---

# 🎛️ Terminal dashboard

`gat-tui` is a Ratatui-based visualizer (see [awesome-ratatui](https://github.com/ratatui/awesome-ratatui) for inspiration) that lives in `crates/gat-tui`. It keeps workflows, statuses, logs, and layout previews in one terminal screen so newcomers can picture the pipeline before opening a browser or GUI. Run it with `cargo run -p gat-tui --release`.

The UI pulls its demo metrics from `out/demos/cournot/cournot_results.csv` (run `test_data/demos/storage_cournot.sh` to refresh) and renders:

* A workflow table backed by `DemoStats` for aggregate metrics (average price, EENS, storage profits, consumer surplus) plus log ticks describing each stage.
* A force-directed layout preview powered by `gat graph visualize`/`fdg-sim` so the terminal view and CLI layout tool share the same coordinates.
* A chart and workflow graph for quick snapshots without leaving the terminal.
* A control panel (poll frequency, solver selection, verbosity, command preview) paired with an in-line command editor (press `c` and edit `cargo run -p gat-cli -- --help`, Ctrl+S to save) plus presets that brand configurations as “Baseline”, “Cournot Demo”, or “Dispatch Check”.
* A config preview (auto reloads `~/.config/gat-tui/config.toml` with `L`) and a live-run status block that shows what the next `cargo run` command would do.
* A built-in file explorer (`e` toggles it, Enter loads the highlighted `.toml`, Esc cancels) so you can browse configs without leaving the TUI.
* A dataset browser: `j`/`k` scroll the catalog entries, `F` fetches the highlighted dataset into `data/public`, and the description panel summarizes tags and licenses.
* A PTDF analytics pane that keeps track of the configured grid/source/sink/transfer and launches `gat analytics ptdf <grid>` runs via `t` (adjust the source with `</>`, the sink with `()` and the transfer with `+/-`).

New controls: `↑`/`↓` to change the highlighted workflow, `l` for a manual log entry, `[`/`]` to tweak the polling rate, `s` to rotate solvers, `v` to toggle verbosity, `p`/`P` to cycle presets, `c` to edit the live command (Ctrl+S saves, Esc cancels), `r` to kick off the configured `gat-cli` command, `L` to reload the config file, `h` to open the help overlay, and `q` to quit.

You can bootstrap the configuration with `gat-cli tui config --out ~/.config/gat-tui/config.toml`, which writes a template containing the current default command, solver, and poll period. Once `gat-tui` runs, it watches the config file, so you can tweak the poll interval or command text without recompiling.
