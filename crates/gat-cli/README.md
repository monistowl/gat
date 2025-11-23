# gat-cli — Command-Line Interface for Grid Analysis

The primary entry point for GAT workflows. `gat` provides fast, reproducible command-line tools for grid modeling, power-flow analysis, optimization, and reliability assessment.

## Quick Start

```bash
# Power flow analysis
gat pf dc test_data/matpower/case9.arrow --out flows.parquet

# Optimal dispatch
gat opf dc grid.arrow --cost costs.csv --limits limits.csv --out opf.parquet

# Scenario-based batch analysis
gat scenarios materialize --spec rts_nminus1.yaml --out runs/scenarios
gat batch pf --manifest runs/scenarios/scenario_manifest.json --out runs/batch
```

## Architecture

**Categories:**
- `import` — Load grid models (MATPOWER, PSS/E, CIM)
- `pf` — Power flow (DC/AC)
- `opf` — Optimal power flow (DC/AC)
- `nminus1` — N-1 contingency screening
- `se` — State estimation
- `graph` — Network topology tools
- `ts` — Time-series operations
- `scenarios` — What-if case definition
- `batch` — Parallel job execution
- `analytics` — Grid metrics (reliability, deliverability, ELCC)
- `dist` / `adms` / `derms` — Distribution domain workflows
- `tui` / `gui` — Interactive interfaces

**Key Design Principles:**
- **Unix-like**: Each command stands alone, pipes work, outputs are reproducible
- **Fast**: Rust-based, single binary, no Python/Conda stack
- **Formats**: All outputs in Arrow/Parquet for compatibility with Python/R/DuckDB/Spark
- **Reproducibility**: Every run saves `run.json` for easy resumption with `gat runs resume`

## Features

### Data Import & Management
```bash
gat import matpower --file case9.raw --out grid.arrow
gat dataset public list --tag ieee
gat dataset public fetch opsd-time-series-2020 --out data/
```

### Grid Analysis
```bash
gat graph stats grid.arrow                    # Network topology
gat pf dc grid.arrow --out flows.parquet      # DC power flow
gat opf ac grid.arrow --out dispatch.parquet  # AC optimal dispatch
gat nminus1 dc grid.arrow --out nminus1.parquet  # Contingency screening
```

### Scenarios & Batch Jobs
```bash
gat scenarios validate --spec scenarios.yaml
gat scenarios materialize --spec scenarios.yaml --grid-file grid.arrow --out-dir runs
gat batch pf --manifest manifest.json --max-jobs 4 --out runs/batch
gat batch opf --manifest manifest.json --solver clarabel --out runs/batch
```

### Reliability & Metrics
```bash
gat analytics reliability --grid grid.arrow --outages contingencies.yaml
gat analytics deliverability --grid grid.arrow --assets assets.csv
gat analytics elcc --grid grid.arrow --scenarios 1000
```

### Specialized Domains
```bash
gat dist pf --grid grid.arrow --demand demand.csv          # Distribution PF
gat derms aggregate --assets ders.csv --pricing prices.csv  # DER aggregation
gat adms flisr --grid grid.arrow --outages faults.csv       # Distribution automation
```

## Output Formats

All major commands emit **Parquet** (columnar, fast, widely supported):
- Compatible with: Polars, DuckDB, Pandas, PySpark, R, Julia
- Metadata in `run.json` for full reproducibility
- Use `gat runs list --root <dir>` to inspect saved runs
- Use `gat runs resume run.json --execute` to re-run

## Features & Building

### Build Variants

**Headless (minimal dependencies):**
```bash
cargo build -p gat-cli --no-default-features
```

**With UI support (TUI + visualization):**
```bash
cargo build -p gat-cli --features "tui viz"
```

**All solvers:**
```bash
cargo build -p gat-cli --features "all-backends"
```

### Solver Backends
- **Clarabel** (default, open-source, pure Rust)
- **HiGHS** (dual simplex, branch-and-cut, high performance)
- **CBC** (COIN-OR, robust, mature)
- **IPOPT** (interior-point, nonlinear)

See `docs/guide/cli-architecture.md` for feature combinations and dependency details.

## Common Workflows

### Import & Explore
```bash
gat import matpower ieee14.raw --out grid.arrow
gat graph stats grid.arrow
gat graph islands grid.arrow
```

### Single Power Flow
```bash
gat pf dc grid.arrow --out flows.parquet
```

### Dispatch with Costs
```bash
gat opf dc grid.arrow \
  --cost costs.csv \
  --limits limits.csv \
  --branch-limits branch_limits.csv \
  --out dispatch.parquet
```

### N-1 Screening at Scale
```bash
gat scenarios materialize \
  --spec contingency_spec.yaml \
  --grid-file grid.arrow \
  --out-dir runs/scenarios

gat batch pf \
  --manifest runs/scenarios/scenario_manifest.json \
  --threads 8 \
  --max-jobs 100 \
  --out runs/batch
```

### Reliability Assessment
```bash
gat analytics reliability \
  --grid grid.arrow \
  --outages contingencies.yaml \
  --out metrics.parquet
```

## Documentation

**Getting Started:**
- `docs/guide/overview.md` — CLI structure and command organization
- `docs/guide/pf.md` — Power-flow examples and troubleshooting
- `docs/guide/opf.md` — Optimal dispatch with solvers and strategies

**Advanced Workflows:**
- `docs/guide/dist.md` — Distribution system analysis
- `docs/guide/adms.md` — Distribution automation (FLISR, VVO)
- `docs/guide/derms.md` — DER management and pricing

**Reference:**
- `docs/cli/gat.md` — Full generated command reference
- `docs/guide/cli-architecture.md` — Dispatcher and module organization

## Testing

```bash
# Quick check (minimal features)
cargo check -p gat-cli --no-default-features --features minimal-io

# Run tests (feature matrix)
cargo test -p gat-cli --features "minimal-io all-backends"

# Full CI matrix (see `.github/workflows/cli-feature-matrix.yml`)
# Runs against: minimal, minimal+full-io, minimal+full-io+viz, all-backends
```

## Related Crates

- **gat-core** — Grid types and solvers
- **gat-io** — I/O, schemas, data formats
- **gat-tui** — Interactive terminal dashboard
- **gat-dist**, **gat-adms**, **gat-derms** — Domain-specific solvers
- **gat-scenarios** — Scenario templating and materialization
- **gat-batch** — Parallel job orchestration

## See Also

- [GAT Main README](../../README.md) for project overview
- [AGENTS.md](../../AGENTS.md) for agent integration and MCP setup
- [RELEASE_PROCESS.md](../../RELEASE_PROCESS.md) for contributing changes
