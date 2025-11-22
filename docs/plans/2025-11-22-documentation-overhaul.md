# Documentation Overhaul Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Consolidate GAT documentation by updating the user-facing README.md to reflect TUI, service layer, and new domain features (ADMS/DERMS/DIST), while creating focused guides that users can navigate to from the main README.

**Architecture:**
- README.md becomes the authoritative entry point with brief feature highlights and links to `docs/guide/`
- Each domain gets a dedicated guide in `docs/guide/` with examples and CLI recipes
- Navigation is clear: users read README for overview, click through for deep dives
- Auto-doc targets stay synchronized via `cargo xtask doc all`

**Tech Stack:** Markdown, Rust docstrings (for auto-doc), `cargo xtask` build system

---

### Task 1: Update README.md Section 1 - Feature Highlights

**Files:**
- Modify: `README.md:10-35` (What Makes GAT Worth Learning section)

**Step 1: Read current section**

Run: `head -35 /home/tom/Code/gat/README.md | tail -26`

Review what's currently highlighted and note gaps (no mention of TUI, ADMS/DERMS/DIST, service layer).

**Step 2: Write updated section in editor**

Update the "For advanced users" section to include:
- TUI for interactive workflows (instead of just CLI)
- Distribution automation (ADMS/FLISR/VVO)
- DER management and aggregation (DERMS)
- Distribution system analysis (DIST)

Replace lines 19-34 with:

```markdown
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
```

**Step 3: Read and verify the change**

Run: `sed -n '19,34p' /home/tom/Code/gat/README.md`

Verify new features are listed and accurate.

**Step 4: Commit**

```bash
git add README.md
git commit -m "docs: Add TUI, ADMS/DERMS/DIST features to highlights"
```

---

### Task 2: Create New "Interface Options" Section in README

**Files:**
- Modify: `README.md` (add new section after "What Makes GAT Worth Learning")

**Step 1: Identify insertion point**

Run: `grep -n "^---$" /home/tom/Code/gat/README.md | head -1`

This tells us where to insert. Should be after line 35 (after the highlights section).

**Step 2: Write new section**

Insert between current lines 35-36 (after first `---`):

```markdown
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
```

**Step 3: Verify insertion point in file**

Run: `sed -n '33,40p' /home/tom/Code/gat/README.md`

Check it looks correct (should see "---" and start of installation section).

**Step 4: Edit the file**

Use Edit tool to insert after line 35:

Old string (lines 33-36):
```
---

# 📦 Installation
```

New string:
```
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
```

**Step 5: Commit**

```bash
git add README.md
git commit -m "docs: Add interface options section (CLI, TUI, GUI)"
```

---

### Task 3: Consolidate and Update CLI Reference Section

**Files:**
- Modify: `README.md:224-279` (CLI Reference section)

**Step 1: Read current CLI reference**

Run: `sed -n '224,279p' /home/tom/Code/gat/README.md`

Note: Currently lists basic commands but misses:
- ADMS/DERMS/DIST command families
- Featurization, allocation analytics
- Scenario/batch orchestration details

**Step 2: Expand CLI reference with all command families**

Replace the simplified reference with comprehensive but concise listing:

Old content (lines 226-279):
```
gat <category> <subcommand> [options]

### **Importers**
...
gat gui run
```

New content:
```
gat <category> <subcommand> [options]

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

```

**Step 3: Verify expanded reference is clear**

Read it mentally to ensure:
- All major command families are covered
- Examples show both category and subcommand
- Descriptions are concise (one line each)

**Step 4: Commit**

```bash
git add README.md
git commit -m "docs: Expand CLI reference to cover all command families"
```

---

### Task 4: Create New "Workflow Quick-Start" Section

**Files:**
- Modify: `README.md` (add new section after CLI reference)

**Step 1: Identify insertion point**

Current CLI reference ends at line 279. Insert new section before "Scenario definitions" (line 283).

**Step 2: Write workflow examples section**

Add between lines 279-283:

```markdown
---

## 🚀 Common Workflows

### Import a grid and run a quick power flow

```bash
gat import matpower case9.raw --out grid.arrow
gat pf dc grid.arrow --out flows.parquet
```

### Explore a grid interactively

```bash
cargo run -p gat-tui --release
# Then: Browse datasets, check pipeline status, view reliability metrics
```

### Run N-1 contingency analysis at scale

```bash
gat scenarios materialize --spec rts_nminus1.yaml --grid-file grid.arrow --out-dir runs/scenarios
gat batch opf --manifest runs/scenarios/rts_nminus1/scenario_manifest.json --out runs/batch/rts_opf
gat runs describe $(gat runs list --root runs --format json | jq -r '.[0].id')
```

### Analyze DER hosting capacity

```bash
gat dist hosting --grid grid.arrow --der-file ders.csv --out hosting_curves.parquet
```

### Extract reliability metrics

```bash
gat analytics reliability --grid grid.arrow --outages contingencies.yaml --out results.parquet
```

For more detailed walkthroughs, see `docs/guide/`.

---
```

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: Add common workflows section with CLI examples"
```

---

### Task 5: Update Documentation Links Section

**Files:**
- Modify: `README.md:399-410` (Documentation & Workflows section)

**Step 1: Read current section**

Run: `sed -n '399,410p' /home/tom/Code/gat/README.md`

Note: References ADMS/DERMS/DIST guides that don't exist yet.

**Step 2: Reorganize with clear subsections**

Replace lines 399-410 with organized guide structure:

Old:
```markdown
# 🗂 Documentation & Workflows

All curated docs now live under `docs/guide/` and the generated assets live under...
* `docs/guide/doc-workflow.md` lays out...
```

New:
```markdown
# 📚 Documentation & Guides

**Getting Started:**
- `docs/guide/overview.md` — CLI architecture, command organization, and xtask workflow
- `docs/guide/pf.md` — Power flow (DC/AC) examples and troubleshooting
- `docs/guide/opf.md` — Optimal power flow with costs, limits, and solver selection

**Advanced Domains:**
- `docs/guide/adms.md` — Distribution automation (FLISR, VVO, outage coordination)
- `docs/guide/derms.md` — DER management (envelope aggregation, pricing, stress testing)
- `docs/guide/dist.md` — Distribution system analysis (AC flows, hosting capacity)

**Common Tasks:**
- `docs/guide/ts.md` — Time-series operations (resample, join, aggregate)
- `docs/guide/se.md` — State estimation (weighted least squares)
- `docs/guide/graph.md` — Network topology tools (stats, islands, visualization)
- `docs/guide/datasets.md` — Public dataset fetching and caching
- `docs/guide/gat-tui.md` — Terminal UI architecture and pane navigation

**Infrastructure & Workflows:**
- `docs/guide/doc-workflow.md` — Integration with `bd` issue tracker and auto-doc system
- `docs/guide/cli-architecture.md` — Dispatcher, command modules, telemetry
- `docs/guide/feature-matrix.md` — CI/CD matrix testing with solver combinations
- `docs/guide/mcp-onboarding.md` — MCP server setup for agent integration
- `docs/guide/packaging.md` — Binary distribution and installation
- `docs/guide/scaling.md` — Multi-horizon scaling roadmap and performance tuning

**Auto-Generated Documentation:**
- `docs/cli/gat.md` — Full CLI command reference (generated)
- `docs/schemas/` — JSON schema for manifests and outputs
- `docs/ROADMAP.md` — Project plan with milestones and acceptance criteria

After documentation changes, run `cargo xtask doc all` to regenerate CLI reference, schemas, and website. Expose the tree to agents with `gat-mcp-docs --docs docs --addr 127.0.0.1:4321`.
```

**Step 3: Verify new structure is clear**

Check that:
- Getting Started guides are accessible
- Advanced domains are grouped
- Common tasks are easy to find
- Infrastructure section explains automation

**Step 4: Commit**

```bash
git add README.md
git commit -m "docs: Reorganize documentation links by category"
```

---

### Task 6: Create docs/guide/adms.md

**Files:**
- Create: `docs/guide/adms.md`

**Step 1: Write ADMS guide**

Create with content:

```markdown
# ADMS (Automatic Distribution Management System)

ADMS tools coordinate reliability and efficiency on distribution networks through automated switching and voltage control.

## Key Concepts

**FLISR** — Fault Location, Isolation, and Service Restoration
- Detects faults on a feeder
- Isolates the faulted section
- Restores service to affected loads via alternate feeder paths

**VVO** — Volt-Var Optimization
- Adjusts voltage setpoints on regulators and capacitors
- Minimizes losses while maintaining voltage limits
- Reactive power dispatch

**Outage Coordination** — Manages multiple simultaneous outages or planned maintenance
- Evaluates restoration priorities
- Coordinates tie-line usage
- Estimates service duration

## Usage Examples

### Run FLISR for a specific fault

```bash
gat adms flisr \
  --grid distribution_network.arrow \
  --fault-location feeder_1/branch_42 \
  --tie-lines tie_config.yaml \
  --out flisr_result.parquet
```

Output includes:
- Isolation sequence (which switches to open)
- Restored load (MW/MVAr)
- Restoration time estimate
- Backup sources used

### VVO optimization

```bash
gat adms vvo \
  --grid distribution_network.arrow \
  --voltage-limits voltage_bands.yaml \
  --regulator-deadbands deadbands.yaml \
  --max-iterations 10 \
  --out vvo_dispatch.parquet
```

Output:
- Optimal voltage setpoints (per regulator)
- Capacitor switching commands
- Estimated loss reduction
- Peak voltage violations (if any)

### Outage impact analysis

```bash
gat adms outage \
  --grid distribution_network.arrow \
  --outage-scenario outages.yaml \
  --out outage_impact.parquet
```

## Integration with Reliability Metrics

ADMS commands output islanded/unsupplied load estimates that feed into:
- Energy Not Served (ENS)
- Loss-of-Load Expectation (LOLE)
- Customer Average Interruption Frequency Index (CAIFI)

See `docs/guide/analytics.md` for reliability aggregation.

## References

- **crate**: `crates/gat-adms/README.md`
- **CLI**: `gat adms --help`
- **Schema**: `docs/schemas/adms_output.json`
```

**Step 2: Verify file is readable**

Run: `wc -l docs/guide/adms.md`

Expected: ~80 lines

**Step 3: Commit**

```bash
git add docs/guide/adms.md
git commit -m "docs: Add ADMS guide (FLISR, VVO, outage coordination)"
```

---

### Task 7: Create docs/guide/derms.md

**Files:**
- Create: `docs/guide/derms.md`

**Step 1: Write DERMS guide**

Create with content:

```markdown
# DERMS (Distributed Energy Resource Management System)

DERMS tools help aggregators and utilities manage portfolios of solar, storage, EV chargers, and demand response.

## Key Concepts

**DER Envelope Aggregation**
- Aggregates individual DER capabilities (power, energy, ramp rate)
- Produces a dispatch envelope: at each time step, what power ranges are achievable?
- Accounts for:
  - Battery state-of-charge constraints
  - Ramp limits
  - Reserve margins
  - Device availability

**Pricing-Based Scheduling**
- Given dynamic electricity prices (or locational prices)
- Optimizes individual and portfolio charging/discharging
- Maximizes arbitrage profit while respecting envelope limits

**Stress Testing**
- Simulates response to scenarios (peak demand, low wind, sudden outages)
- Verifies that aggregated DERs meet reliability targets (e.g., minimum reserve, ramp capability)

## Usage Examples

### Aggregate a DER portfolio

```bash
gat derms aggregate \
  --devices device_catalog.csv \
  --ders_metadata der_list.yaml \
  --start 2024-01-01 \
  --end 2024-01-31 \
  --out ders_envelope.parquet
```

Output (time-indexed):
- Available power (MW)
- Available energy (MWh)
- Maximum charge rate (MW)
- Maximum discharge rate (MW)
- Reserve requirement (MW)

### Schedule DERs for price response

```bash
gat derms schedule \
  --envelope ders_envelope.parquet \
  --prices spot_prices.csv \
  --strategy max_arbitrage \
  --soc_bounds "[0.1,0.9]" \
  --out dispatch_schedule.parquet
```

Output:
- Dispatch setpoint (MW) per device per interval
- Predicted profit
- Battery state-of-charge trajectory
- Constraint violations (if any)

### Run stress tests

```bash
gat derms stress \
  --envelope ders_envelope.parquet \
  --scenarios stress_test_matrix.yaml \
  --out stress_results.parquet
```

## Integration with Distribution Analysis

DER aggregates can be incorporated into DIST hosting-capacity and ADMS voltage-support workflows:

```bash
gat dist hosting \
  --grid dist_network.arrow \
  --ders ders_envelope.parquet \
  --out hosting_curves.parquet
```

See `docs/guide/dist.md` for full workflow.

## References

- **crate**: `crates/gat-derms/README.md`
- **CLI**: `gat derms --help`
- **Schema**: `docs/schemas/derms_output.json`
```

**Step 2: Verify file is readable**

Run: `wc -l docs/guide/derms.md`

Expected: ~90 lines

**Step 3: Commit**

```bash
git add docs/guide/derms.md
git commit -m "docs: Add DERMS guide (aggregation, scheduling, stress testing)"
```

---

### Task 8: Create docs/guide/dist.md

**Files:**
- Create: `docs/guide/dist.md`

**Step 1: Write DIST guide**

Create with content:

```markdown
# DIST (Distribution System Analysis)

DIST tools analyze and optimize distribution networks: power flows, optimal dispatch, and hosting capacity for renewable energy.

## Key Concepts

**Distribution Network Modeling**
- Imported from MATPOWER (.raw, .m files)
- AC power flow with transformer impedances, load models, capacitor banks
- Voltage regulators and switching devices
- Integration points to transmission (substation).

**AC Optimal Power Flow**
- Minimizes loss or cost on distribution system
- Respects voltage bands, thermal limits, reactive capability
- Can incorporate DER dispatch (via `gat derms`)

**Hosting Capacity**
- Maximum amount of solar/wind/storage that can be connected at each bus
- Constrained by:
  - Voltage rise (from increased injection)
  - Thermal overloads on feeder sections
  - Reverse power flow limits

## Usage Examples

### Import a distribution system from MATPOWER

```bash
gat import matpower ieee13_feeder.raw --out dist_network.arrow
```

Output: `dist_network.arrow` (Arrow format with bus, branch, generator data).

### Run AC power flow

```bash
gat dist pf \
  --grid dist_network.arrow \
  --out dist_pf.parquet
```

Output:
- Bus voltage (pu and angle)
- Branch flow (MW, MVAr, I)
- Loss summary
- Voltage violations (if any)

### Optimal power flow with cost minimization

```bash
gat dist opf \
  --grid dist_network.arrow \
  --costs gen_costs.csv \
  --limits dispatch_limits.csv \
  --out dist_opf.parquet
```

Output:
- Optimal dispatch (MW per generator)
- Resulting power flow
- Total cost
- Constraint violations (if binding)

### Hosting capacity analysis

```bash
gat dist hosting \
  --grid dist_network.arrow \
  --type solar \
  --voltage-band "[0.95,1.05]" \
  --thermal-margin 0.1 \
  --out hosting_solar.parquet
```

Output (per bus):
- Maximum solar (MW)
- Limiting constraint (voltage rise vs. thermal)
- Incremental capacity vs. penetration

### With DER integration

```bash
gat dist hosting \
  --grid dist_network.arrow \
  --ders ders_envelope.parquet \
  --type wind \
  --out hosting_wind_with_ders.parquet
```

## Integration with Other Tools

- **Featurization**: Generate network features for ML:
  ```bash
  gat featurize gnn --grid dist_network.arrow --out features.parquet
  ```

- **Reliability**: Evaluate CAIFI, SAIDI under outage scenarios:
  ```bash
  gat analytics reliability --grid dist_network.arrow --outages contingencies.yaml --out reliability.parquet
  ```

- **ADMS coordination**: Combine VVO and hosting capacity:
  ```bash
  gat adms vvo --grid dist_network.arrow --regulator-deadbands vvo_config.yaml --out vvo_dispatch.parquet
  ```

## References

- **crate**: `crates/gat-dist/README.md`
- **CLI**: `gat dist --help`
- **Schema**: `docs/schemas/dist_output.json`
```

**Step 2: Verify file is readable**

Run: `wc -l docs/guide/dist.md`

Expected: ~120 lines

**Step 3: Commit**

```bash
git add docs/guide/dist.md
git commit -m "docs: Add DIST guide (distribution modeling, OPF, hosting capacity)"
```

---

### Task 9: Create docs/guide/analytics.md

**Files:**
- Create: `docs/guide/analytics.md`

**Step 1: Write analytics guide**

Create with content:

```markdown
# Analytics & Reliability Metrics

Analytics commands extract grid insights: power transfer distribution, reliability indices, and interconnection limits.

## Key Metrics

**PTDF** — Power Transfer Distribution Factors
- Sensitivity: 1 MW injection at bus A → how much MW flows on each branch?
- Use case: Determine congestion risks for renewable injection
- Linear analysis (post-contingency or steady-state)

**Reliability Metrics**
- **ENS** — Energy Not Served (MWh unserved per year)
- **LOLE** — Loss-of-Load Expectancy (hours/year at risk)
- **CAIFI** — Customer Average Interruption Frequency Index
- Based on outage scenarios (N-1, N-2) and restoration times

**ELCC** — Effective Load Carrying Capability
- How much load can be served with a new resource (wind/solar/battery)?
- Incorporates weather, demand, and existing resources

**Deliverability** — Transmission hosting capacity
- How much renewable energy can be delivered to load?
- Limited by transmission thermal ratings and voltage stability

**DS** — Demand Served
- Fraction of load met after N-1 screening
- Input to reliability index calculations

## Usage Examples

### Compute PTDF for a source-sink pair

```bash
gat analytics ptdf \
  --grid transmission_network.arrow \
  --source bus_1 \
  --sink bus_2 \
  --transfer 1.0 \
  --solver gauss \
  --out ptdf_1_2.parquet
```

Output:
- Branch ID, flow (MW), PTDF (fraction per MW)
- Summary: max PTDF, min PTDF, branches above threshold

### Reliability analysis

```bash
gat analytics reliability \
  --grid network.arrow \
  --outages contingency_scenarios.yaml \
  --restoration-times outage_mttr.csv \
  --demand demand_profile.csv \
  --out reliability_indices.parquet
```

Output:
- Outage ID
- Peak unserved load (MW)
- Energy not served (MWh)
- Estimated LOLE, CAIFI contributions

### ELCC for solar

```bash
gat analytics elcc \
  --grid network.arrow \
  --weather solar_irradiance.csv \
  --demand load_profile.csv \
  --existing-resources existing_gens.yaml \
  --candidate-capacity 50 \
  --candidate-type solar \
  --out elcc_solar.parquet
```

Output:
- Effective load carrying capability (MW)
- Margin above nameplate (if any)
- Sensitivity to weather data

### Deliverability screening

```bash
gat analytics deliverability \
  --grid network.arrow \
  --injection-point feeder_123 \
  --injection-ramp 100 \
  --max-penetration 30 \
  --out deliverability.parquet
```

## Integration with Planning Workflows

Use analytics in batch studies:

```bash
# Scenario-based reliability
gat scenarios materialize --spec scenarios.yaml --grid-file network.arrow --out-dir runs/scenarios
gat batch opf --manifest runs/scenarios/scenario_manifest.json --out runs/batch/opf_results

# Then compute reliability from batch results
gat analytics reliability \
  --grid network.arrow \
  --batch-root runs/batch/opf_results \
  --out reliability_summary.parquet
```

## References

- **CLI**: `gat analytics --help`
- **Schemas**: `docs/schemas/analytics_*.json`
- **Examples**: `test_data/analytics/`
```

**Step 2: Verify file is readable**

Run: `wc -l docs/guide/analytics.md`

Expected: ~120 lines

**Step 3: Commit**

```bash
git add docs/guide/analytics.md
git commit -m "docs: Add analytics guide (PTDF, reliability, ELCC, deliverability)"
```

---

### Task 10: Final README Polish and Verification

**Files:**
- Modify: `README.md` (review all changes)

**Step 1: Read the full updated README**

Run: `wc -l /home/tom/Code/gat/README.md && head -60 /home/tom/Code/gat/README.md`

Check that:
- Feature highlights include TUI/ADMS/DERMS/DIST
- Interface section explains CLI, TUI, GUI options
- CLI reference covers all command families
- Workflow section has concrete examples
- Documentation links point to new guides

**Step 2: Verify no broken links**

Run: `grep -E "docs/guide/|docs/cli/" /home/tom/Code/gat/README.md | head -20`

Ensure all referenced files exist or are auto-generated:
- `docs/guide/overview.md` ✓
- `docs/guide/pf.md` ✓
- `docs/guide/opf.md` ✓
- `docs/guide/adms.md` (just created) ✓
- `docs/guide/derms.md` (just created) ✓
- `docs/guide/dist.md` (just created) ✓
- `docs/guide/ts.md` ✓
- `docs/guide/se.md` ✓
- `docs/guide/graph.md` ✓
- `docs/guide/datasets.md` ✓
- `docs/guide/gat-tui.md` ✓
- `docs/cli/gat.md` (auto-generated) — will be created by `cargo xtask doc all`

**Step 3: Commit final README state**

```bash
git add README.md
git commit -m "docs: Polish README with guide links and updated structure"
```

**Step 4: Run auto-doc generation**

Run: `cd /home/tom/Code/gat && cargo xtask doc all 2>&1 | tail -20`

Expected output:
```
Generated docs/cli/gat.md
Updated docs/schemas/
Updated docs/man/gat.1
Updated site/book/
```

This ensures `docs/cli/gat.md` is generated and matches current CLI.

**Step 5: Verify auto-doc output**

Run: `ls -lh /home/tom/Code/gat/docs/cli/gat.md`

Should show a recent timestamp (just now).

**Step 6: Commit generated docs**

```bash
git add docs/cli/ docs/schemas/ docs/man/ site/book/
git commit -m "docs: Regenerate auto-generated documentation"
```

---

## Summary

This plan updates the user-facing README.md to reflect the full GAT feature set (TUI, ADMS, DERMS, DIST) and creates three new guides (adms.md, derms.md, dist.md) plus analytics.md. It also reorganizes the documentation links for clarity and regenerates auto-generated assets to keep everything in sync.

**Total Changes:**
- README.md: 5 sections updated (highlights, new interface section, expanded CLI ref, new workflows section, reorganized doc links)
- 4 new guide files (adms.md, derms.md, dist.md, analytics.md)
- Auto-doc regenerated (cli, schemas, man, site)
- 10 commits total

**Verification:**
- README links to all real guides
- No broken references
- Auto-doc is current
- All commits are small and focused

