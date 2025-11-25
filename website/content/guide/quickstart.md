+++
title = "Quickstart"
description = "Get started with GAT in 5 minutes"
weight = 2

[extra]
next_steps = [
  { title = "Power Flow Analysis", description = "Learn about DC and AC power flow analysis", link = "/guide/pf/" },
  { title = "Command Builder", description = "Visually build commands without memorizing syntax", link = "/tools/" },
  { title = "Explore Examples", description = "See real-world examples and use cases", link = "https://github.com/monistowl/gat/tree/main/examples" }
]
+++

# Quickstart: Your First Power Flow Analysis

This guide will get you from zero to running power flow analysis in 5 minutes. No prerequisites required.

## 1. Installation (1 minute)

Install GAT using the modular installer:

```bash
curl -fsSL https://github.com/monistowl/gat/releases/download/v0.3.1/install-modular.sh | bash
```

The installer will:
- Download the latest GAT binary for your OS (Linux or macOS)
- Extract it to `~/.gat/bin/`
- Create a config directory at `~/.gat/config/`

Add GAT to your PATH:

```bash
export PATH="$HOME/.gat/bin:$PATH"
```

Or make it permanent by adding the line above to your `~/.bashrc` or `~/.zshrc`.

**Verify installation:**

```bash
gat --version
```

You should see: `gat-cli 0.3.1`

> **Troubleshooting?** See [Installation Troubleshooting](/guide/install-verify/#troubleshooting) for common issues.

## 2. Understand the Basics (1 minute)

GAT analyzes power grids. Here are the key concepts:

### What is Power Flow Analysis?

Power flow analysis calculates how electricity flows through a grid given demand and generation.

- **DC Power Flow** — Fast approximation, linearized equations
- **AC Power Flow** — Accurate simulation, full nonlinear equations

### Input: Grid Data

You need a **grid file** describing:
- Buses (nodes) with demand and generation
- Lines (branches) connecting buses with flow limits
- Generators with costs and limits
- Transformer settings, reactive power constraints

**Supported formats:** MATPOWER (.m), Pandapower (.pkl), CSV

### Output: Results

GAT outputs analysis results in **Parquet format** — a columnar format that works with:
- Python (Polars, Pandas, PyArrow)
- DuckDB for SQL analysis
- Any modern data tool

## 3. Get Sample Data (1 minute)

GAT includes example datasets. Clone the repository to access them:

```bash
git clone https://github.com/monistowl/gat.git
cd gat/examples
ls -la *.m
```

You'll see MATPOWER case files like:
- `case9.m` — 9-bus test case
- `case30.m` — 30-bus test case
- `case118.m` — 118-bus test case

For this quickstart, we'll use `case9.m` (the smallest and fastest).

## 4. Run Your First Power Flow (1 minute)

### DC Power Flow (Fastest)

DC power flow solves in milliseconds using linearized equations:

```bash
gat pf dc examples/case9.m --out flows_dc.parquet
```

**What this does:**
- `gat pf dc` — Run DC power flow
- `examples/case9.m` — Load the 9-bus grid
- `--out flows_dc.parquet` — Save results to `flows_dc.parquet`

**Output:**
```
✓ Loaded case9.m (9 buses, 9 lines)
✓ DC Power Flow solved in 12.3ms
✓ Results saved to flows_dc.parquet
```

### AC Power Flow (More Accurate)

AC power flow solves the full nonlinear equations:

```bash
gat pf ac examples/case9.m --out flows_ac.parquet
```

This takes slightly longer (still under 100ms for small cases) but gives more accurate voltages and reactive power.

## 5. Examine Your Results (1 minute)

### View Results in Python

Use Polars to examine results (install with `pip install polars`):

```python
import polars as pl

# Read the parquet file
df = pl.read_parquet('flows_dc.parquet')

# Show basic info
print(df.head())
print(f"Shape: {df.shape}")

# Get bus voltages
print(df.select(['bus_id', 'voltage_mag', 'voltage_ang']))

# Get line flows
print(df.select(['from_bus', 'to_bus', 'power_flow']))
```

### View Results in DuckDB

Or use DuckDB for SQL analysis:

```bash
duckdb :memory: "SELECT * FROM read_parquet('flows_dc.parquet') LIMIT 5"
```

### Simple Text View

For a quick look without installing tools:

```bash
# Show file info
file flows_dc.parquet

# Show first few rows (requires parquet-tools)
parquet-tools show flows_dc.parquet
```

## 6. Next Steps

Now that you've run your first analysis, explore these topics:

### 📚 Learn More About Power Flow
- [Power Flow Guide](/guide/pf/) — Deep dive into DC vs AC power flow
- [Solver Selection](/guide/pf/#solver-selection) — When to use each solver

### 🎯 Try Other Analyses
- [Optimal Power Flow (OPF)](/guide/opf/) — Economic dispatch
- [N-1 Contingency Analysis](/guide/dist/#n-1-screening) — What happens if a line fails?
- [State Estimation](/guide/se/) — Infer grid state from measurements

### 💻 Build Automation Workflows
- [Command-Line Interface](/guide/overview/#cli-architecture) — Automate analysis pipelines
- [Time Series](/guide/ts/) — Run multi-period analysis
- [Manifests](/internals/cli-architecture/#manifest-driven-workflows) — Batch processing

### 📊 Visualize Results
- [TUI Dashboard](/internals/gat-tui/) — Interactive terminal dashboard
  ```bash
  gat-tui  # Explore results in a fancy dashboard
  ```

### 🤖 Integrate with Other Tools
- [MCP Server](/internals/mcp-onboarding/) — AI agent integration
- [Agent Integration](/guide/overview/#agent-integration) — Use GAT with Claude, ChatGPT, etc.

## Common Tasks

### Run Analysis on Your Own Grid

If you have a MATPOWER file:

```bash
gat pf dc your_grid.m --out results.parquet
```

Or from CSV:

```bash
gat pf dc your_grid.csv --out results.parquet
```

### Compare DC vs AC Results

```bash
# Run both analyses
gat pf dc your_grid.m --out dc.parquet
gat pf ac your_grid.m --out ac.parquet

# Compare in Python
import polars as pl
dc = pl.read_parquet('dc.parquet')
ac = pl.read_parquet('ac.parquet')

# Show voltage differences
print((ac.select('voltage_mag') - dc.select('voltage_mag')).abs().max())
```

### Speed Benchmarks

On a modern laptop, typical analysis times:

| Grid Size | DC Power Flow | AC Power Flow |
|-----------|---------------|---------------|
| 9 buses | ~10ms | ~50ms |
| 30 buses | ~15ms | ~80ms |
| 118 buses | ~30ms | ~150ms |
| 1000+ buses | ~100ms | ~500ms |

(Times vary by solver and hardware)

## Troubleshooting

### "gat: command not found"
You need to add GAT to your PATH. Run:
```bash
export PATH="$HOME/.gat/bin:$PATH"
```

### "case9.m: No such file or directory"
Clone the GAT repository to get example files:
```bash
git clone https://github.com/monistowl/gat.git
cd gat
```

### Power flow doesn't converge
- **AC power flow:** Try relaxing convergence tolerance with `--tolerance 1e-3`
- **DC power flow:** Should always converge (it's linear)
- Check your grid has a slack bus (usually bus 1)

### Results file not created
- Check write permissions in current directory
- Try using absolute path: `--out /tmp/results.parquet`

## What You Learned

✅ Installed GAT
✅ Ran DC and AC power flow
✅ Examined results in Parquet format
✅ Understood basic power systems concepts

You're ready to explore deeper! Pick a topic from [Next Steps](#6-next-steps) or check the full [Documentation](/docs/).

## Get Help

- **Questions?** [Start a discussion]({{ config.extra.repo_url }}/discussions)
- **Issues?** [Report a bug]({{ config.extra.repo_url }}/issues)
- **Want to contribute?** See [Contributing Guide](/contributing/)
