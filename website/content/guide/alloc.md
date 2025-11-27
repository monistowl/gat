+++
title = "Settlement & Allocation"
description = "Congestion rent analysis and KPI attribution"
weight = 62
+++

# Settlement & Allocation

The `gat alloc` commands analyze OPF results to compute congestion rents, surplus decomposition, and KPI attribution — essential for market settlement and policy analysis.

## Overview

| Command | Purpose |
|---------|---------|
| `alloc rents` | Compute congestion rents from OPF |
| `alloc kpi` | Attribute KPI changes to control actions |

## Congestion Rent Analysis

### `gat alloc rents`

Decomposes system surplus into congestion rents, generator revenues, and load payments from OPF results.

```bash
gat alloc rents \
  --opf-results opf_output.parquet \
  --grid-file grid.arrow \
  --out congestion_rents.parquet
```

**Required Arguments:**
- `--opf-results` — OPF output (must have `bus_id`, `lmp`, `injection_mw`, `flow_mw`)
- `--grid-file` — Grid topology (Arrow format)
- `--out` — Output file

**Options:**
- `--tariffs` — Optional tariff parameters CSV (`resource_id`, `tariff_rate`)
- `--out-partitions` — Partition output by columns

**Output Columns:**
- `branch_id` — Transmission element
- `congestion_rent` — Revenue from price differences ($/hr)
- `flow_mw` — Power flow on branch
- `lmp_from`, `lmp_to` — Nodal prices at endpoints
- `binding` — Whether flow limit is binding

### Surplus Decomposition

The total system surplus decomposes as:

```
Total Surplus = Generator Revenue - Load Payments + Congestion Rents
```

Where:
- **Generator Revenue** = Σ (LMP_i × P_gen_i)
- **Load Payments** = Σ (LMP_i × P_load_i)
- **Congestion Rents** = Σ (LMP_to - LMP_from) × Flow_ij

**Reference:** [LMP-Based Congestion Analysis](https://doi.org/10.1109/TPWRS.2003.820692)

### Example: Market Settlement

```bash
# Run OPF
gat opf dc grid.arrow \
  --cost costs.csv \
  --limits limits.csv \
  --out opf_results.parquet

# Compute settlement
gat alloc rents \
  --opf-results opf_results.parquet \
  --grid-file grid.arrow \
  --out settlement.parquet

# Analyze in Python
import polars as pl
settlement = pl.read_parquet("settlement.parquet")
total_rents = settlement["congestion_rent"].sum()
print(f"Total congestion rents: ${total_rents:.2f}/hr")
```

## KPI Attribution

### `gat alloc kpi`

Approximates the contribution of control actions to KPI improvements using gradient-based sensitivity.

```bash
gat alloc kpi \
  --kpi-results kpi_output.parquet \
  --scenario-meta scenarios.parquet \
  --out contributions.parquet
```

**Required Arguments:**
- `--kpi-results` — KPI values by scenario (`scenario_id`, `kpi_value`)
- `--scenario-meta` — Scenario metadata (control flags, policy settings)
- `--out` — Output contribution table

**Output Columns:**
- `control_variable` — Name of the control action
- `contribution` — Estimated contribution to KPI change
- `direction` — Positive/negative effect
- `confidence` — Confidence level of estimate

### Example: Policy Impact Analysis

```bash
# Run scenarios with different policies
gat batch opf grid.arrow \
  --scenarios policy_scenarios.yaml \
  --out batch_results/

# Extract KPIs
gat analytics kpi batch_results/ --out kpi_results.parquet

# Attribute contributions
gat alloc kpi \
  --kpi-results kpi_results.parquet \
  --scenario-meta policy_scenarios.parquet \
  --out policy_impact.parquet
```

This enables questions like:
- "How much did DER dispatch reduce congestion?"
- "What's the reliability impact of the new line?"
- "Which control action contributed most to loss reduction?"

**Reference:** [SHAP for Model Explanations](https://doi.org/10.1038/s42256-019-0138-9)

## Use Cases

### Market Operations
- Compute congestion rents for ISO settlement
- Decompose uplift charges by cause
- Analyze FTR (Financial Transmission Rights) values

### Regulatory Analysis
- Attribute reliability improvements to investments
- Quantify policy impacts on system costs
- Support rate case analysis

### Research
- Validate market clearing algorithms
- Study LMP behavior under scenarios
- Develop attribution methods for grid KPIs

## Related Commands

- [OPF](/guide/opf/) — Optimal power flow analysis
- [Batch Analysis](/guide/batch/) — Multi-scenario analysis
- [Reliability](/guide/reliability/) — Reliability metrics
