# GAT for AI Agents & RAG Integration

GAT (Grid Analysis Toolkit) provides a modern command-line interface for power systems analysis, optimized for integration with AI agents and LLM-based RAG (Retrieval-Augmented Generation) systems.

## Installation

### From Source
```bash
git clone https://github.com/monistowl/gat.git
cd gat
cargo build -p gat-cli --release
# Binary at: target/release/gat-cli (or symlink as `gat`)
```

### Requirements
- Rust 1.75+
- Linux, macOS, or Windows
- ~2GB disk for compilation

## Quick Start: First 5 Minutes

```bash
# 1. Import a grid model
gat import matpower --m ieee14.m -o grid.arrow

# 2. Run a power flow analysis
gat pf dc grid.arrow -o flows.parquet

# 3. Inspect results with Python/DuckDB/Polars
duckdb
SELECT * FROM read_parquet('flows.parquet');
```

All outputs are **Parquet** (columnar format, widely compatible with Python/R/SQL tools).

## GAT Toolkit Functions (Machine-Parsable)

### Data Import & Validation

| Function | Command | Purpose |
|----------|---------|---------|
| **Import MATPOWER** | `gat import matpower --m CASE.m -o grid.arrow` | Load .m/.case files (most common in academia) |
| **Import PSS/E** | `gat import psse --raw CASE.raw -o grid.arrow` | Load PSS/E RAW format |
| **Import CIM** | `gat import cim --rdf network.rdf -o grid.arrow` | Load IEC 61970 CIM-RDF files |
| **Validate Dataset** | `gat validate grid.arrow` | Check grid topology for errors |

### Network Analysis

| Function | Command | Purpose |
|----------|---------|---------|
| **Network Stats** | `gat graph stats grid.arrow` | Count buses, lines, generators, etc. |
| **Detect Islands** | `gat graph islands grid.arrow` | Find disconnected network components |
| **Connectivity Check** | `gat graph connectivity grid.arrow` | Verify radial/meshed topology |

### Power Flow (Single Case)

| Function | Command | Purpose |
|----------|---------|---------|
| **DC Power Flow** | `gat pf dc grid.arrow -o flows.parquet` | Fast, linear approximation |
| **AC Power Flow** | `gat pf ac grid.arrow -o flows.parquet` | Full nonlinear solution |
| **With Limits** | `gat pf dc grid.arrow --limits branch_limits.csv -o flows.parquet` | Enforce thermal constraints |

### Optimal Power Flow & Dispatch

| Function | Command | Purpose |
|----------|---------|---------|
| **DC Optimal Dispatch** | `gat opf dc grid.arrow --cost costs.csv -o dispatch.parquet` | Minimize generation cost (linear) |
| **AC Optimal Dispatch** | `gat opf ac grid.arrow --cost costs.csv -o dispatch.parquet` | Minimize cost with AC constraints |
| **With Solver Selection** | `gat opf dc grid.arrow --solver highs --cost costs.csv -o dispatch.parquet` | Use HiGHS/Clarabel/IPOPT backends |
| **Unit Commitment** | `gat opf dc grid.arrow --cost costs.csv --ramping ramp_limits.csv -o dispatch.parquet` | Multi-period commitment problem |

### Contingency Analysis (N-1)

| Function | Command | Purpose |
|----------|---------|---------|
| **N-1 Screening** | `gat nminus1 dc grid.arrow --spec contingencies.yaml -o nminus1.parquet` | Fast outage enumeration |
| **With Limits** | `gat nminus1 dc grid.arrow --limits branch_limits.csv -o nminus1.parquet` | Find violations post-contingency |

### Scenario-Based Analysis (What-If)

| Function | Command | Purpose |
|----------|---------|---------|
| **Validate Scenario Spec** | `gat scenarios validate --spec scenarios.yaml` | Check YAML template for errors |
| **Materialize Scenarios** | `gat scenarios materialize --spec scenarios.yaml --grid grid.arrow -o-dir runs/` | Generate case files for multiple scenarios |
| **Batch Power Flow** | `gat batch pf --manifest runs/manifest.json --max-jobs 100 --threads 8 -o runs/results` | Parallel execution of scenario PF analysis |
| **Batch OPF** | `gat batch opf --manifest runs/manifest.json --solver highs -o runs/results` | Parallel optimal dispatch |

### Time-Series Operations

| Function | Command | Purpose |
|----------|---------|---------|
| **Time-Series Power Flow** | `gat ts solve --grid grid.arrow --timeseries loads.parquet -o ts_results.parquet` | Multi-period OPF |
| **Forecasting** | `gat ts forecast --grid grid.arrow --historical hist.parquet -o forecast.parquet` | Load/wind prediction |
| **Statistical Summary** | `gat ts stats --timeseries data.parquet --window 24h -o stats.parquet` | Rolling aggregates |

### Reliability & Deliverability

| Function | Command | Purpose |
|----------|---------|---------|
| **Reliability Metrics** | `gat analytics reliability --grid grid.arrow -oages contingencies.yaml -o metrics.parquet` | LOLE, EUE, severity |
| **Deliverability Index** | `gat analytics deliverability --grid grid.arrow --assets critical.csv -o deliverability.parquet` | Critical load accessibility |
| **ELCC (Effective Load Carrying Capacity)** | `gat analytics elcc --grid grid.arrow --scenarios 1000 -o elcc.parquet` | Renewable capacity value |

### Spatial Analysis (Geo)

| Function | Command | Purpose |
|----------|---------|---------|
| **Spatial Join** | `gat geo join --grid grid.arrow --polygons tracts.parquet --method point_in_polygon -o mapping.parquet` | Map buses to geographic regions (census tracts, zip codes, utility districts) |
| **Spatial Featurization** | `gat geo featurize --mapping mapping.parquet --timeseries ts_results.parquet --lags 1,24,168 --windows 24,168 --seasonal true -o features.parquet` | Aggregate time-series to polygon-level with lags/rolling stats |

### Distribution Domain

| Function | Command | Purpose |
|----------|---------|---------|
| **Distribution Power Flow** | `gat dist pf --grid dist_grid.arrow --demand demand.csv -o dist_flows.parquet` | Radial distribution analysis |
| **Voltage Control (VVO)** | `gat dist vvo --grid dist_grid.arrow --demand demand.csv -o vvo_setpoints.parquet` | Volt-var optimization |
| **Fused Recloser Logic (FLISR)** | `gat adms flisr --grid dist_grid.arrow --fault-location fault_loc.csv -o restoration.parquet` | Automatic restoration sequences |

### DER Management

| Function | Command | Purpose |
|----------|---------|---------|
| **DER Aggregation** | `gat derms aggregate --assets ders.csv --pricing prices.csv -o aggregated.parquet` | Virtual power plant coordination |
| **Hosting Capacity** | `gat derms hosting-capacity --grid grid.arrow --der-type solar -o hosting.parquet` | Max DER penetration analysis |
| **Revenue Stacking** | `gat derms revenue-stack --assets ders.csv --energy prices.csv --ancillary prices.csv -o revenue.parquet` | Multi-service optimization |

### State Estimation & Observability

| Function | Command | Purpose |
|----------|---------|---------|
| **State Estimation** | `gat se estimate --grid grid.arrow --measurements meas.csv -o state_estimates.parquet` | SCADA data reconciliation |
| **Observability Check** | `gat se observability --grid grid.arrow --measurements meas.csv -o observable.parquet` | Identify unobserved regions |

### Run Management & Reproducibility

| Function | Command | Purpose |
|----------|---------|---------|
| **List Past Runs** | `gat runs list --root results/` | Show all saved analyses |
| **Resume Run** | `gat runs resume run.json --execute` | Re-run previous analysis with same parameters |
| **Export Run Metadata** | `gat runs show run.json` | View command, solver, timings, all parameters |

---

## Integration with AI Agents

### Option 1: CLI Direct Invocation

Agents can call GAT directly:

```bash
gat import matpower --m case.m -o grid.arrow
gat pf dc grid.arrow -o flows.parquet
```

**Advantages:**
- No external dependencies beyond GAT binary
- Fast (single binary, no Python/Conda overhead)
- Full reproducibility (all parameters in command)

**Best for:** Automated workflows, batch scripting, CI/CD pipelines

### Option 2: MCP Server (Claude, Cline, other LLM clients)

**MCP (Model Context Protocol)** provides structured function signatures to LLMs.

#### Setup

1. **Install MCP bridge** (Python):
   ```bash
   pip install gat-mcp
   ```

2. **Configure Claude/Cline** (`~/.config/claude/config.json`):
   ```json
   {
     "mcpServers": {
       "gat": {
         "command": "gat-mcp",
         "args": ["--host", "127.0.0.1", "--port", "9000"]
       }
     }
   }
   ```

3. **In Claude/Cline, call functions like:**
   ```
   gat_import_matpower(file="case9.raw", out="grid.arrow")
   gat_pf_dc(grid="grid.arrow", out="flows.parquet")
   gat_analytics_reliability(grid="grid.arrow", outages="contingencies.yaml")
   ```

**Advantages:**
- Type-safe function signatures
- LLM understands parameter docs
- Automatic result parsing

**Best for:** Interactive AI assistants, code generation, natural language interfaces

### Option 3: Python SDK (Future)

For Jupyter notebooks and Python data pipelines:

```python
import gat

grid = gat.import_matpower("case9.raw")
flows = gat.pf_dc(grid, out="flows.parquet")
metrics = gat.analytics_reliability(grid, outages="contingencies.yaml")
```

*(In development)*

---

## Output Formats & Querying

All results are **Apache Parquet** (columnar, compressed):

```bash
# Option 1: DuckDB (SQL)
duckdb "SELECT branch_id, power_mw, limit_mva FROM 'flows.parquet' WHERE power_mw > limit_mva"

# Option 2: Polars (Python)
import polars as pl
flows = pl.read_parquet("flows.parquet")
violations = flows.filter(pl.col("power_mw") > pl.col("limit_mva"))

# Option 3: Pandas
import pandas as pd
flows = pd.read_parquet("flows.parquet")
print(flows[flows['power_mw'] > flows['limit_mva']])

# Option 4: Apache Spark
spark.read.parquet("flows.parquet").show()
```

---

## Common Workflow Examples

### Scenario-Based N-1 Analysis at Scale

```bash
# 1. Materialize 1000 N-1 contingencies from template
gat scenarios materialize \
  --spec n1_contingencies.yaml \
  --grid grid.arrow \
  -o-dir runs/n1

# 2. Run DC power flow for all scenarios in parallel
gat batch pf \
  --manifest runs/n1/scenario_manifest.json \
  --threads 16 \
  --max-jobs 1000 \
  -o runs/n1_results

# 3. Identify violations in Python
python << 'EOF'
import polars as pl
results = pl.read_parquet("runs/n1_results/flows.parquet")
violations = results.filter(pl.col("power_mw") > pl.col("limit_mva"))
print(f"Total violations: {len(violations)}")
print(violations.group_by("contingency_id").count())
EOF
```

### Spatial Analysis (Equity-Focused Reliability)

```bash
# 1. Import grid and geographic data
gat import matpower --m grid.m -o grid.arrow
# (assume tracts.parquet is from Census TIGER/Line via GDAL)

# 2. Map buses to census tracts
gat geo join \
  --grid grid.arrow \
  --polygons tracts.parquet \
  --method point_in_polygon \
  -o bus_to_tract.parquet

# 3. Run time-series OPF to get hourly flows
gat ts solve \
  --grid grid.arrow \
  --timeseries loads_2024.parquet \
  -o ts_results.parquet

# 4. Featurize: aggregate loads and flows to tract level
gat geo featurize \
  --mapping bus_to_tract.parquet \
  --timeseries ts_results.parquet \
  --lags 1,24,168 \
  --windows 24,168 \
  --seasonal true \
  -o tract_features.parquet

# 5. Join with socioeconomic data in Python
python << 'EOF'
import polars as pl
features = pl.read_parquet("tract_features.parquet")
census = pl.read_csv("census_data.csv")
equity = features.join(census, on="tract_id", how="left")
print(equity.select(["tract_id", "load_mw", "median_income", "dem_pct"]))
EOF
```

### DER Hosting Capacity for Planning

```bash
gat derms hosting-capacity \
  --grid grid.arrow \
  --der-type solar \
  --voltage-band 0.95,1.05 \
  --penetration-min 0 \
  --penetration-max 5.0 \
  -o hosting.parquet

# Query in DuckDB
duckdb "SELECT bus_id, hosting_capacity_mw FROM 'hosting.parquet' ORDER BY hosting_capacity_mw DESC LIMIT 10"
```

---

## For RAG Systems & Documentation

### Function Signature Format (For Embeddings & Retrieval)

Each GAT function should be indexed with:

```
Category: Power Flow
Function: gat_pf_dc
Signature: gat pf dc <GRID.ARROW> [-o OUTPUT.parquet] [--limits LIMITS.csv]
Description: Compute DC power flow on a grid
Inputs: Grid model (Arrow), optional thermal limits (CSV)
Outputs: Branch flows, bus angles, generation (Parquet)
Example: gat pf dc ieee14.arrow -o flows.parquet
```

This enables RAG systems to:
1. **Retrieve** relevant functions by user intent ("compute power flow")
2. **Validate** user parameters ("does the grid file exist?")
3. **Generate** correct command syntax ("gat pf dc ...")
4. **Trace** results ("output is in flows.parquet, can query with DuckDB")

---

## Performance Characteristics

| Operation | Grid Size | Time (Single-Threaded) |
|-----------|-----------|----------------------|
| DC Power Flow | 1,000 buses | ~50ms |
| DC Power Flow | 10,000 buses | ~200ms |
| AC Power Flow | 1,000 buses | ~500ms |
| N-1 Screening (100 contingencies) | 1,000 buses | ~5s |
| Batch PF (1000 scenarios) | 1,000 buses | ~50s (16 threads) |
| Time-Series OPF (365 days, hourly) | 1,000 buses | ~10min |

---

## Troubleshooting & Getting Help

### Common Issues

**"Grid file not found"**
```bash
# Check file exists and is valid Arrow
file grid.arrow
ls -lh grid.arrow
```

**"Solver not available"**
```bash
# Check available solvers
gat opf dc --help | grep solver
# Build with solvers: cargo build --features all-backends
```

**"Out of memory on large batch"**
```bash
# Reduce jobs in parallel
gat batch pf --manifest manifest.json --max-jobs 50 --threads 4
```

### Getting Support

- **Bug reports**: [GitHub Issues](https://github.com/monistowl/gat/issues)
- **Documentation**: [docs/guide/](docs/guide/)
- **Example cases**: [test_data/](test_data/)

---

## Related Documentation

- **[README.md](README.md)** — Project overview, features, installation
- **[RELEASE_PROCESS.md](RELEASE_PROCESS.md)** — Contributing and release workflow
- **[crates/gat-cli/README.md](crates/gat-cli/README.md)** — Detailed CLI reference
- **[crates/gat-core/README.md](crates/gat-core/README.md)** — Core solver algorithms
- **[crates/gat-tui/README.md](crates/gat-tui/README.md)** — Interactive terminal UI

---

## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Auto-syncs to JSONL for version control
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**
```bash
bd ready --json
```

**Create new issues:**
```bash
bd create "Issue title" -t bug|feature|task -p 0-4 --json
bd create "Issue title" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**
```bash
bd update bd-42 --status in_progress --json
bd update bd-42 --priority 1 --json
```

**Complete work:**
```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`
6. **Commit together**: Always commit the `.beads/issues.jsonl` file together with the code changes so issue state stays in sync with code state

### Auto-Sync

bd automatically syncs with git:
- Exports to `.beads/issues.jsonl` after changes (5s debounce)
- Imports from JSONL when newer (e.g., after `git pull`)
- No manual export/import needed!

### MCP Server (Recommended)

If using Claude or MCP-compatible clients, install the beads MCP server:

```bash
pip install beads-mcp
```

Add to MCP config (e.g., `~/.config/claude/config.json`):
```json
{
  "beads": {
    "command": "beads-mcp",
    "args": []
  }
}
```

Then use `mcp__beads__*` functions instead of CLI commands.

### Managing AI-Generated Planning Documents

AI assistants often create planning and design documents during development:
- PLAN.md, IMPLEMENTATION.md, ARCHITECTURE.md
- DESIGN.md, CODEBASE_SUMMARY.md, INTEGRATION_PLAN.md
- TESTING_GUIDE.md, TECHNICAL_DESIGN.md, and similar files

**Best Practice: Use a dedicated directory for these ephemeral files**

**Recommended approach:**
- Create a `history/` directory in the project root
- Store ALL AI-generated planning/design docs in `history/`
- Keep the repository root clean and focused on permanent project files
- Only access `history/` when explicitly asked to review past planning

**Example .gitignore entry (optional):**
```
# AI planning documents (ephemeral)
history/
```

**Benefits:**
- ✅ Clean repository root
- ✅ Clear separation between ephemeral and permanent documentation
- ✅ Easy to exclude from version control if desired
- ✅ Preserves planning history for archeological research
- ✅ Reduces noise when browsing the project

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ✅ Store AI planning docs in `history/` directory
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems
- ❌ Do NOT clutter repo root with planning documents

For more details, see README.md and QUICKSTART.md.

