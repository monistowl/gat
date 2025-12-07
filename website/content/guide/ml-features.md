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
- `--format <FORMAT>` — Output format (see below)
- `--out-partitions <cols>` — Partition output by columns (e.g., `graph_id,scenario_id`)
- `--group-by-scenario` — Group flows by `scenario_id`
- `--group-by-time` — Group flows by time column

### Output Formats

The `--format` option controls the output structure:

| Format | Description | Use Case |
|--------|-------------|----------|
| `arrow` | GAT native Parquet tables (default) | Production pipelines, large datasets |
| `neurips-json` | NeurIPS PowerGraph benchmark format | Academic benchmarks, paper reproduction |
| `pytorch-geometric` | PyTorch Geometric JSON format | Direct PyG integration |

#### Arrow Format (default)

```bash
gat featurize gnn grid.arrow --flows flows.parquet --out features/ --format arrow
```

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

#### NeurIPS JSON Format

Compatible with the [NeurIPS 2024 PowerGraph benchmark](https://openreview.net/forum?id=xyz) format:

```bash
gat featurize gnn grid.arrow --flows flows.parquet --out graphs/ --format neurips-json
```

**Output:** One JSON file per graph instance:
```
graphs/
├── graph_0.json
├── graph_1.json
└── ...
```

**JSON Schema:**
```json
{
  "graph_id": "case14_scenario_0",
  "num_nodes": 14,
  "num_edges": 20,
  "node_features": [[1.0, 100.0, 50.0, ...], ...],
  "edge_index": [[0, 1, 2, ...], [1, 2, 3, ...]],
  "edge_features": [[0.01, 0.05, 10.0], ...],
  "y": 0,
  "task": "classification"
}
```

#### PyTorch Geometric Format

Direct-loadable format for [PyTorch Geometric](https://pytorch-geometric.readthedocs.io/):

```bash
gat featurize gnn grid.arrow --flows flows.parquet --out graphs/ --format pytorch-geometric
```

**JSON Schema:**
```json
{
  "x": [[1.0, 100.0, ...], ...],
  "edge_index": [[0, 1, 2], [1, 2, 3]],
  "edge_attr": [[0.01, 0.05], ...],
  "y": 0,
  "num_nodes": 14
}
```

**Python Loading Example:**
```python
import json
import torch
from torch_geometric.data import Data

with open("graphs/graph_0.json") as f:
    d = json.load(f)

data = Data(
    x=torch.tensor(d["x"], dtype=torch.float),
    edge_index=torch.tensor(d["edge_index"], dtype=torch.long),
    edge_attr=torch.tensor(d["edge_attr"], dtype=torch.float),
    y=torch.tensor([d["y"]], dtype=torch.long),
)
```

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

## PowerGraph Benchmark Dataset (NeurIPS 2024)

GAT includes a loader for the **PowerGraph** benchmark dataset from NeurIPS 2024, enabling reproducible GNN research on power systems. This loader requires the `powergraph` feature flag.

### Dataset Overview

PowerGraph provides standardized GNN benchmarks for power grid analysis:

| Task | Description | Label Type |
|------|-------------|------------|
| Cascading Failure | Predict if outage triggers cascade | Binary classification |
| Voltage Stability | Predict voltage collapse risk | Regression |
| Optimal Dispatch | Predict generation schedule | Multi-output regression |

### Loading PowerGraph Data

```rust
use gat_io::sources::powergraph::{load_powergraph_dataset, list_powergraph_datasets};

// List available datasets
let datasets = list_powergraph_datasets("/path/to/powergraph")?;
for info in &datasets {
    println!("{}: {} samples, task={:?}", info.name, info.num_samples, info.task);
}

// Load a specific dataset
let samples = load_powergraph_dataset("/path/to/powergraph/cascading_failure.mat")?;
for sample in &samples {
    println!(
        "Graph: {} nodes, {} edges, label={:?}",
        sample.num_nodes, sample.num_edges, sample.label
    );
}
```

### Converting to PyTorch Geometric

```rust
use gat_io::sources::powergraph::sample_to_pytorch_geometric_json;

let json = sample_to_pytorch_geometric_json(&sample);
std::fs::write("graph.json", json)?;
```

### Python Integration

```python
import json
import torch
from torch_geometric.data import Data, InMemoryDataset

class PowerGraphDataset(InMemoryDataset):
    def __init__(self, root, transform=None):
        super().__init__(root, transform)
        self.data, self.slices = torch.load(self.processed_paths[0])

    @property
    def processed_file_names(self):
        return ['data.pt']

    def process(self):
        data_list = []
        for path in (self.root / 'graphs').glob('*.json'):
            with open(path) as f:
                d = json.load(f)
            data_list.append(Data(
                x=torch.tensor(d['x'], dtype=torch.float),
                edge_index=torch.tensor(d['edge_index'], dtype=torch.long),
                edge_attr=torch.tensor(d['edge_attr'], dtype=torch.float),
                y=torch.tensor([d['y']], dtype=torch.float),
            ))
        torch.save(self.collate(data_list), self.processed_paths[0])

# Usage
dataset = PowerGraphDataset('/path/to/exported/graphs')
```

### Feature Specification

**Node Features** (7 dimensions):
| Index | Feature | Unit |
|-------|---------|------|
| 0 | Voltage magnitude | kV |
| 1 | Active generation | MW |
| 2 | Reactive generation | MVAr |
| 3 | Active load | MW |
| 4 | Reactive load | MVAr |
| 5 | Number of generators | count |
| 6 | Number of loads | count |

**Edge Features** (3 dimensions):
| Index | Feature | Unit |
|-------|---------|------|
| 0 | Resistance | p.u. |
| 1 | Reactance | p.u. |
| 2 | Power flow | MW |

### Building with PowerGraph Support

```bash
# Enable the powergraph feature
cargo build -p gat-io --features powergraph
cargo build -p gat-cli --features powergraph

# Run tests
cargo test -p gat-io --features powergraph
```

### References

- **PowerGraph Paper**: NeurIPS 2024 Datasets & Benchmarks Track
- **Dataset**: [OpenReview Submission](https://openreview.net/)
- **Crate**: `crates/gat-io/src/sources/powergraph.rs`

## Related Commands

- [Batch Analysis](@/guide/batch.md) — Run scenarios for training data
- [Reliability](@/guide/reliability.md) — Generate reliability metrics
- [Geo Features](@/guide/geo.md) — Spatial feature aggregation
