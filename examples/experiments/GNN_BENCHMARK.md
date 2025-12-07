---

## GNN Benchmarks — Power Grid Graph Neural Network Datasets

**Paper**: "A power grid benchmark dataset for graph neural networks" (NeurIPS Datasets & Benchmarks 2024) ([NeurIPS Proceedings](https://proceedings.neurips.cc/paper_files/paper/2024/file/c7caf017cbbca1f4b368ffdc7bb8f319-Paper-Datasets_and_Benchmarks_Track.pdf))

**Reference Implementation**: [PowerGraph Dataset](https://github.com/PowerGraph-Datasets)

---

### What This Provides

GAT provides infrastructure for working with power grid GNN datasets:

1. **PowerGraph Dataset Loader** — Import NeurIPS benchmark datasets
2. **GNN Featurization** — Generate physics-informed node/edge features
3. **Export Formats** — PyTorch Geometric and NeurIPS JSON formats
4. **Round-Trip Validation** — Serialize → deserialize → verify equivalence

---

### Quick Start

```bash
# Featurize a network for GNN training
gat featurize gnn network.arrow \
  --out features.json \
  --format pytorch-geometric

# Export in NeurIPS format with metadata
gat featurize gnn network.arrow \
  --out features.json \
  --format neurips \
  --graph-id 1 \
  --scenario-id "base_case"
```

---

### Feature Extraction

GAT's `featurize_gnn` module generates physics-informed features:

**Node Features (per bus):**
| Index | Feature | Description |
|-------|---------|-------------|
| 0 | Voltage magnitude | Per-unit voltage |
| 1 | Voltage angle | Radians |
| 2 | Active power injection | Per-unit P |
| 3 | Reactive power injection | Per-unit Q |
| 4 | Bus type encoding | Slack/PQ/PV |
| 5 | Shunt conductance | Per-unit G |
| 6 | Shunt susceptance | Per-unit B |

**Edge Features (per branch):**
| Index | Feature | Description |
|-------|---------|-------------|
| 0 | Series resistance | Per-unit R |
| 1 | Series reactance | Per-unit X |
| 2 | Shunt susceptance | Per-unit B/2 |
| 3 | Tap ratio | Transformer tap |
| 4 | Phase shift | Radians |
| 5 | Thermal limit | Per-unit rating |
| 6 | Branch status | 1=closed, 0=open |

---

### Export Formats

**PyTorch Geometric Format:**
```json
{
  "x": [[0.98, 0.0, 1.2, 0.3, ...], ...],
  "edge_index": [[0, 0, 1, 2, ...], [1, 2, 3, 4, ...]],
  "edge_attr": [[0.01, 0.05, 0.02, ...], ...],
  "num_nodes": 14,
  "num_edges": 26
}
```

**NeurIPS Format:**
```json
{
  "node_features": [[0.98, 0.0, 1.2, 0.3, ...], ...],
  "edge_features": [[0.01, 0.05, 0.02, ...], ...],
  "edge_index": [[0, 0, 1, 2, ...], [1, 2, 3, 4, ...]],
  "num_nodes": 14,
  "num_edges": 26,
  "metadata": {
    "graph_id": 1,
    "scenario_id": "base_case",
    "time": "2024-01-01T00:00:00Z"
  }
}
```

---

### Round-Trip Validation

GAT supports full round-trip validation to ensure data integrity:

```rust
use gat_algo::featurize_gnn::{GnnGraphSample, PytorchGeometricJson, NeuripsGraphJson};

// Create sample from network
let sample = GnnGraphSample::from_network(&network, graph_id, scenario_id, time);

// Export to PyTorch Geometric format
let pyg_json = sample.to_pytorch_geometric_json();

// Import back
let restored = GnnGraphSample::from_pytorch_geometric_json(&pyg_json);

// Validate integrity
restored.validate()?;

// Verify equivalence
assert_eq!(sample.num_nodes, restored.num_nodes);
assert_eq!(sample.node_features, restored.node_features);
```

**Validation checks:**
- Node/edge count consistency
- Edge index bounds (no out-of-range references)
- Feature width consistency across all nodes/edges

---

### PowerGraph Dataset Integration

To load NeurIPS benchmark datasets:

```rust
use gat_io::sources::powergraph::PowerGraphLoader;

// Load dataset
let loader = PowerGraphLoader::new("path/to/powergraph_case118")?;

// Iterate over graph samples
for sample in loader.iter() {
    let features = GnnGraphSample::from_network(&sample.network, sample.id, None, None);
    // ... use for training
}
```

---

### Using with PyTorch Geometric

Export GAT features and load in Python:

```python
import json
import torch
from torch_geometric.data import Data

# Load GAT-exported features
with open("features.json") as f:
    gat_data = json.load(f)

# Convert to PyG Data object
data = Data(
    x=torch.tensor(gat_data["x"], dtype=torch.float),
    edge_index=torch.tensor(gat_data["edge_index"], dtype=torch.long),
    edge_attr=torch.tensor(gat_data["edge_attr"], dtype=torch.float),
)

# Use in GNN model
output = model(data)
```

---

### Batch Processing

Generate features for multiple scenarios:

```bash
# Create feature manifest
gat batch featurize gnn \
  --manifest scenarios.json \
  --out features/ \
  --format pytorch-geometric \
  --max-jobs 8
```

Or programmatically:

```rust
use rayon::prelude::*;

let samples: Vec<GnnGraphSample> = networks
    .par_iter()
    .enumerate()
    .map(|(i, net)| GnnGraphSample::from_network(net, i as i64, None, None))
    .collect();

// Export all
for sample in &samples {
    let json = sample.to_pytorch_geometric_json();
    // ... write to file
}
```

---

### Extending Features

Add custom features by extending the featurization:

```rust
impl GnnGraphSample {
    pub fn add_custom_node_features(&mut self, network: &Network) {
        // Example: add generator capacity features
        for (i, bus) in network.buses().iter().enumerate() {
            let gen_capacity = network.generators()
                .filter(|g| g.bus_id == bus.id)
                .map(|g| g.pmax)
                .sum::<f64>();
            self.node_features[i].push(gen_capacity);
        }
    }
}
```

---

### References

1. Liao, Q., et al. (2024). A power grid benchmark dataset for graph neural networks. *NeurIPS Datasets & Benchmarks Track*.

2. Donon, B., et al. (2019). Graph Neural Solver for Power Systems. *IJCNN*.

3. Pagnier, L., & Chertkov, M. (2021). Physics-Informed Graphical Neural Network for Parameter & State Estimations in Power Systems. *arXiv:2102.06349*.
