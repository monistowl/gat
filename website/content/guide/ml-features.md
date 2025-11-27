+++
title = "ML Feature Extraction"
description = "Extract features from power systems data for machine learning models"
weight = 60
+++

# ML Feature Extraction

GAT provides tools for extracting features from power system analysis results, enabling integration with machine learning pipelines for GNN-based modeling, KPI prediction, and spatial forecasting.

## Overview

The `gat featurize` command transforms power flow and OPF outputs into ML-ready feature tables:

| Command | Purpose | Output |
|---------|---------|--------|
| `featurize gnn` | Graph features for GNNs | Node/edge/graph features |
| `featurize kpi` | Tabular features for KPI prediction | Wide feature tables |

## GNN Feature Extraction

### `gat featurize gnn`

Converts power grid data into graph-structured features compatible with PyTorch Geometric, DGL, and other GNN frameworks.

```bash
gat featurize gnn grid.arrow \
  --flows pf_results.parquet \
  --out features/
```

**Arguments:**
- `<GRID_FILE>` — Grid topology in Arrow format
- `--flows` — Power flow results (must have `branch_id`, `flow_mw`)
- `--out` — Output directory for feature tables

**Options:**
- `--out-partitions <cols>` — Partition output by columns (e.g., `graph_id,scenario_id`)
- `--group-by-scenario` — Group flows by `scenario_id`
- `--group-by-time` — Group flows by time column

**Output Structure:**
```
features/
├── nodes.parquet      # Bus features: topology + injections
├── edges.parquet      # Branch features: impedance + flows
└── graphs.parquet     # Graph-level metadata
```

**Node Features:**
- Bus ID, voltage magnitude, angle
- Active/reactive injection
- Load demand, generation dispatch
- Bus type (PQ/PV/slack)

**Edge Features:**
- Branch impedance (R, X, B)
- Power flow (MW, MVAr)
- Thermal loading percentage
- Tap ratio (transformers)

### Example: Training Data Pipeline

```bash
# 1. Run batch power flow for multiple scenarios
gat batch pf grid.arrow --scenarios scenarios.yaml --out batch_results/

# 2. Extract GNN features
gat featurize gnn grid.arrow \
  --flows batch_results/flows.parquet \
  --out gnn_features/ \
  --group-by-scenario

# 3. Load in Python
import torch_geometric
# ... load from gnn_features/
```

**Reference:** [GNNs for Power Systems](https://doi.org/10.1109/TPWRS.2020.3041234)

## KPI Feature Tables

### `gat featurize kpi`

Aggregates batch analysis outputs into wide feature tables for training probabilistic KPI predictors.

```bash
gat featurize kpi \
  --batch-root batch_results/ \
  --reliability reliability.parquet \
  --out kpi_features.parquet
```

**Options:**
- `--batch-root` — Directory with batch PF/OPF outputs
- `--reliability` — Optional reliability metrics file
- `--scenario-meta` — Optional scenario metadata (YAML/JSON)
- `--out` — Output Parquet file
- `--out-partitions` — Partition columns

**Output Features:**
- System stress indicators (loading, voltage margins)
- Policy/control flags from scenario metadata
- Aggregated reliability metrics (LOLE, EUE)
- Keyed by `(scenario_id, time, zone)`

### Use Case: Reliability Prediction

Build models to predict reliability KPIs from operating conditions:

```bash
# 1. Run reliability analysis
gat adms reliability --grid grid.arrow --scenarios 500 --out reliability.parquet

# 2. Generate feature tables
gat featurize kpi \
  --batch-root batch_results/ \
  --reliability reliability.parquet \
  --scenario-meta scenarios.yaml \
  --out training_data.parquet

# 3. Train model (Python)
import lightgbm as lgb
# ... train on training_data.parquet
```

**Supported ML Frameworks:**
- TabNet, NGBoost
- LightGBM, XGBoost
- scikit-learn gradient boosting

## Related Commands

- [Batch Analysis](/guide/batch/) — Run scenarios for training data
- [Reliability](/guide/reliability/) — Generate reliability metrics
- [Geo Features](/guide/geo/) — Spatial feature aggregation
