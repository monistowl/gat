# GRID ANALYSIS TOOLKIT (GAT)

*A fast Rust-powered command-line toolkit for power-system modeling, flows, dispatch, and time-series analysis.*

If you’re comfortable running simple CLI commands and want to start doing *real* grid analysis — without needing a giant Python stack or a full simulation lab — **GAT gives you industrial-grade tools in a form you can actually tinker with.**
Everything runs as standalone commands, and all the heavy lifting is Rust-fast.

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
Because Rust gives you C-like execution speed without unsafe foot-guns.
For grid models with thousands of buses/branches, that matters.
Even on a laptop.

---

# 📦 Installation

### 1. Install Rust (required)

Go to [https://rustup.rs](https://rustup.rs).
This gives you the `cargo` build system.

### 2. Optional tools

These help with documentation and agent workflows:

* `bd` — lightweight issue-tracking tool
* `beads-mcp` — integrates MCP agents with the repo
* `jq` — required for packaging scripts

### 3. Build GAT

```bash
cargo build --package gat-cli
```

This produces a `gat` binary under `target/debug/`.

### 4. Package and Install

```bash
scripts/package.sh
scripts/install.sh
```

This installs `gat-cli` and `gat-gui` into `~/.local/bin` by default.

---

# 🚀 Quick Demo: Your First GAT Workflow

These are real commands from the toolkit — great for learning.

## 1. DC Power Flow (fastest starter)

```bash
gat pf dc test_data/matpower/case9.arrow --out out/dc-flows.parquet
```

**What this does:**

* Loads a grid (MATPOWER case9 converted into Arrow)
* Solves the DC approximation
* Writes a Parquet file with MW flows on each branch

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

* A cost function per bus
* Min/max generation limits
* Demand
* Optional piecewise cost curves

**Outputs:**

* Feasible dispatch
* Branch flows
* Flags for violations

---

## 3. AC Optimal Power Flow (nonlinear)

```bash
gat opf ac test_data/matpower/case9.arrow \
  --out out/ac-opf.parquet \
  --tol 1e-6 --max-iter 20
```

If you’ve only worked with DC flows before, this is a great next step.

---

# 🧠 Beginner’s Section: Understanding GAT Concepts

### **Power Flow (PF) — “What are the voltages and flows right now?”**

* **DC PF**: linear, fast, good rough approximation
* **AC PF**: nonlinear, more accurate

### **Optimal Power Flow (OPF) — “What’s the cheapest feasible dispatch?”**

* Takes cost curves
* Respects line & generator limits
* Produces an optimal operating point

### **N-1 Screening — “What happens if one thing breaks?”**

* Remove each branch one at a time
* Re-solve DC flows
* Summarize violations
* Rank which outages are worst

### **State Estimation (SE) — “Given measurements, what’s actually happening?”**

Weighted least squares solution.

### **Time-Series Tools — “Make telemetry usable.”**

* Resample ("align everything into, say, 5-second bins")
* Join ("merge streams on timestamp")
* Aggregate ("sum/avg per sensor")

All with consistent Arrow/Parquet outputs.

---

# 🛠 CLI Reference (Simplified View)

You will mostly call the top-level `gat` command:

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

### **Power Flow**

```
gat pf dc
gat pf ac
```

### **Optimal Power Flow**

```
gat opf dc
gat opf ac
```

### **Time Series**

```
gat ts resample
gat ts join
gat ts agg
```

### **Contingency Analysis**

```
gat nminus1 dc
```

### **State Estimation**

```
gat se wls
```

### **GUI stub**

```
gat gui run
```

Use `gat --help` and `gat <command> --help` for the authoritative flags.

---

# 📤 Outputs & Formats (Beginner-Friendly Notes)

All major commands output **Parquet** because:

* it’s fast
* it’s columnar
* it works with Polars, Pandas, R, DuckDB, Spark, etc.
* you won’t outgrow it

Every run also creates a **run.json**, which stores all arguments so you can reproduce the run consistently:

```bash
gat runs resume run.json --execute
```

This is hugely useful when you later build larger pipelines, batch jobs, or cluster fans-out.

---

# 🏎 Why Rust & Ad-Hoc Cluster Fanouts?

A quick conceptual pitch:

* Rust gives you **fast execution in a single binary** with no Python environment headaches.
* GAT commands run independently, so **you can fan them out across many cheap machines**:

  * different AC-PF cases
  * multiple OPF scenarios
  * 1000 N-1 contingencies
  * long time-series streams
* Instead of running one giant slow model, you can do **embarrassingly parallel slicing**, e.g.:

  ```bash
  parallel gat pf dc grid.arrow --out out/flows_{}.parquet ::: {1..500}
  ```

This is one of the easiest ways to get high-throughput compute **without having to build a big cluster framework**.

If you’ve ever used `xargs -P` or GNU `parallel`, you already know enough.

---

# 📚 Test Fixtures (Great for Learning)

The workspace includes:

* `test_data/matpower/` — small MATPOWER cases
* `test_data/opf/` — cost curves, limits, branch limits
* `test_data/nminus1/` — contingency sets
* `test_data/se/` — state-estimation measurements
* `test_data/ts/` — telemetry examples

You can freely modify these files while experimenting.

---

# 📝 Auto-Documentation System

Run:

```bash
cargo xtask doc all
```

This generates:

* CLI reference (`docs/cli/gat.md`)
* Man page (`docs/man/gat.1`)
* JSON schemas (`docs/schemas/…`)
* A minimal browsable site (`site/book/`)

The docs server:

```bash
gat-mcp-docs --docs docs --addr 127.0.0.1:4321
```

Useful for MCP agents and for browsing functionality.

---

# 🗺 Roadmap (High-Level)

* More advanced DC/AC contingency screening
* Broader SE functionality
* Better GUI dashboards
* More dataset importers (PSSE/CIM variants)
* Improved packaging & distribution

See `ROADMAP.md` for the authoritative project plan.

---

# 🧩 Final Notes

GAT scales with you:

* If you just want to run a DC power flow: **2 lines.**
* If you want to run a thousand AC-OPF scenarios on 20 machines: **you can do that too.**
* All without needing Conda, Jupyter, or clusters.
