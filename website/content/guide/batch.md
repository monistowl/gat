+++
title = "Batch Analysis"
description = "Run power flow and OPF across multiple scenarios"
weight = 55
+++

# Batch Analysis

The `gat batch` command enables running power flow and OPF analysis across multiple scenarios in parallel, following the CANOS (Coordinated Automatic Network Operating System) framework for reliability studies.

## Overview

| Command | Purpose |
|---------|---------|
| `batch pf` | Run DC/AC power flow for every scenario |
| `batch opf` | Run DC/AC OPF for every scenario |

Batch analysis is essential for:
- **Reliability studies** — Monte Carlo simulation across outage scenarios
- **Sensitivity analysis** — Varying load, generation, or topology
- **Training data generation** — Creating datasets for ML models

## Power Flow Batch

### `gat batch pf`

Run power flow for every scenario defined in a manifest:

```bash
gat batch pf grid.arrow \
  --scenarios scenarios.yaml \
  --method dc \
  --out batch_results/
```

**Arguments:**
- `<GRID_FILE>` — Base grid topology (Arrow format)
- `--scenarios` — Scenario manifest (YAML or JSON)
- `--method` — Power flow method: `dc` or `ac`
- `--out` — Output directory

**Scenario Manifest Format:**

```yaml
# scenarios.yaml
scenarios:
  - id: base_case
    description: "Normal operating conditions"

  - id: high_load
    description: "Summer peak"
    load_scale: 1.2

  - id: gen_outage_1
    description: "Generator G1 offline"
    offline_generators: [G1]

  - id: line_outage_5_7
    description: "Line 5-7 outage"
    offline_branches: [branch_5_7]
```

**Output Structure:**
```
batch_results/
├── base_case/
│   └── flows.parquet
├── high_load/
│   └── flows.parquet
├── gen_outage_1/
│   └── flows.parquet
└── summary.parquet
```

## OPF Batch

### `gat batch opf`

Run optimal power flow for every scenario with reliability statistics:

```bash
gat batch opf grid.arrow \
  --scenarios scenarios.yaml \
  --method dc \
  --out batch_opf_results/
```

**Additional Output:**
- Generator dispatch per scenario
- LMPs (Locational Marginal Prices)
- Binding constraints
- Objective value (total cost)

## CANOS Framework

GAT's batch analysis follows the CANOS framework for coordinated network analysis:

1. **Scenario Definition** — Define operating conditions, outages, load variations
2. **Fan-Out** — Distribute analysis across scenarios (parallelized)
3. **Aggregation** — Collect results and compute statistics
4. **Reliability Metrics** — Derive LOLE, EUE, and other indices

**Reference:** [CANOS Framework](https://doi.org/10.1109/TPWRS.2007.899019)

## Example: N-1 Contingency Analysis

```bash
# Generate N-1 scenarios
cat > n1_scenarios.yaml << 'EOF'
scenarios:
{% for branch in branches %}
  - id: outage_{{ branch.id }}
    offline_branches: [{{ branch.id }}]
{% endfor %}
EOF

# Run batch power flow
gat batch pf grid.arrow \
  --scenarios n1_scenarios.yaml \
  --method dc \
  --out n1_results/

# Analyze results
gat analytics contingency n1_results/ --out contingency_report.parquet
```

## Example: Monte Carlo Reliability

```bash
# Generate random outage scenarios
gat scenarios generate \
  --grid grid.arrow \
  --count 500 \
  --outage-rate 0.02 \
  --out mc_scenarios.yaml

# Run batch OPF
gat batch opf grid.arrow \
  --scenarios mc_scenarios.yaml \
  --method dc \
  --out mc_results/

# Compute reliability metrics
gat analytics reliability mc_results/ --out reliability.parquet
```

## Performance

Batch analysis leverages Rayon for parallel execution:

| Grid Size | Scenarios | DC PF Time | AC PF Time |
|-----------|-----------|------------|------------|
| 118 buses | 100 | ~2s | ~10s |
| 118 buses | 1000 | ~15s | ~90s |
| 2000 buses | 100 | ~10s | ~60s |

Times on 16-core system. Actual performance varies by hardware.

## Integration with ML Pipelines

Batch results feed into ML feature extraction:

```bash
# 1. Run batch analysis
gat batch pf grid.arrow --scenarios scenarios.yaml --out batch/

# 2. Extract GNN features
gat featurize gnn grid.arrow --flows batch/*/flows.parquet --out features/

# 3. Train model
python train_gnn.py --features features/
```

## Related Commands

- [Power Flow](/guide/pf/) — Single power flow analysis
- [OPF](/guide/opf/) — Single OPF analysis
- [Reliability](/guide/reliability/) — Reliability metrics
- [ML Features](/guide/ml-features/) — Feature extraction
