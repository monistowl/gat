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

**Why Rust?**
Because Rust gives you C-like execution speed without unsafe foot-guns. For grid models with thousands of buses/branches, that matters. Even on a laptop.

---

# 📦 Installation

### 1. Install Rust (required)

Go to https://rustup.rs. This installs `cargo` and the toolchain helpers used across the workspace.

### 2. Optional helpers

These tools make documentation changes and CLI workflows easier:

* `bd` — the beads issue tracker (run `bd ready` before you start work).
* `beads-mcp` — so MCP-compatible agents can inspect docs via `gat-mcp-docs`.
* `jq` — required by `scripts/package.sh`.

### 3. Build GAT

```bash
cargo build --package gat-cli
```

GAT produces a `gat` binary under `target/debug/`.

### 4. Package and install

```bash
scripts/package.sh
scripts/install.sh
```

The scripts create release tarballs (`dist/gat-<version>-<os>-<arch>.tar.gz`) and install `gat-cli`/`gat-gui` into `~/.local/bin` by default.

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

# 🛠 CLI Reference (Simplified View)

```
gat <category> <subcommand> [options]
```

### **Importers**

```
gat import psse
gat import matpower
gat import cim
```

### **Graph tools**

```
gat graph stats
gat graph islands
gat graph export
```

### **Power Flow & OPF**

```
gat pf dc
gat pf ac
gat opf dc
gat opf ac
```

### **Time Series**

```
gat ts resample
gat ts join
gat ts agg
```

### **Contingency & SE**

```
gat nminus1 dc
gat se wls
gat gui run
```

Use `gat --help` and `gat <command> --help` for detailed flags and device-specific options.

---

# 📤 Outputs & Formats

All major commands emit **Parquet** because it is fast, columnar, and compatible with Polars, DuckDB, Pandas, Spark, R, etc.

Every run also emits `run.json` with the full argument list so you can reproduce runs with:

```bash
gat runs resume run.json --execute
```

This makes CI, batch jobs, and fan-out pipelines reproducible.

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

# 🗂 Documentation & Workflows

All curated docs now live under `docs/guide/` and the generated assets live under `docs/cli`, `docs/schemas`, `docs/arrow`, and `site/book/`. Key references:

* `docs/guide/doc-workflow.md` lays out the `bd` issue workflow plus the `cargo xtask doc all` steps that keep helpful docs in sync.
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

# 🧩 Final Notes

GAT scales with you:

* Two lines for a DC power flow.
* A thousand AC-OPF scenarios on 20 machines when you need throughput.
* All without Conda, Jupyter, or heavyweight clusters.

Future demos/examples in the backlog:

1. A beginner tutorial (“Your first week with GAT”).
2. A sample notebook using DuckDB + Polars to explore outputs.
3. A cluster-fanout cheat sheet for students and undergrads.

## Terminal dashboard

`gat-tui` is a Ratatui-based visualizer (see [awesome-ratatui](https://github.com/ratatui/awesome-ratatui) for inspiration) that lives in `crates/gat-tui`. It keeps workflows, statuses, logs, and layout previews in one terminal screen so newcomers can picture the pipeline before opening a browser or GUI. Run it with `cargo run -p gat-tui --release`.

The UI pulls its demo metrics from `out/demos/cournot/cournot_results.csv` (run `test_data/demos/storage_cournot.sh` to refresh) and renders:

* A workflow table plus log ticks describing each stage.
* Gauges/summary pulled from the shared `DemoStats` model (avg price, EENS, storage profits, consumer surplus).
* A force-directed layout preview powered by `gat graph visualize`/`fdg-sim` so the terminal view and the CLI `graph visualize` formatter use the same coordinates.
* A demo chart and workflow graph providing quick snapshots without leaving the terminal.

Controls: `↑`/`↓` to change the highlighted workflow, `l` adds a log entry, and `q` quits.
