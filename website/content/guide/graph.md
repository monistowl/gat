+++
title = "Graph Topology"
description = "Network topology analysis, island detection, and graph operations"
weight = 26
+++

# Graph Topology

The `gat graph` commands analyze power network topology: detect islands, compute graph statistics, find shortest paths, and export to graph formats for visualization.

<div class="grid-widget" data-network="ieee14" data-height="380" data-flow="false" data-voltage="false" data-legend="true" data-caption="Interactive: The IEEE 14-bus test case as a graph. Buses are nodes, branches are edges."></div>

## Overview

| Command | Purpose |
|---------|---------|
| `gat graph stats` | Compute graph statistics (nodes, edges, density) |
| `gat graph islands` | Detect electrically isolated islands |
| `gat graph path` | Find shortest path between buses |
| `gat graph export` | Export to DOT/GraphML for visualization |
| `gat graph neighbors` | List buses connected to a given bus |

## Quick Start

### Basic Statistics

```bash
gat graph stats grid.arrow
```

**Output:**
```
Graph Statistics
────────────────
Nodes (buses):     118
Edges (branches):  186
Density:           0.027
Average degree:    3.15
Max degree:        9 (bus 69)
Diameter:          14
Connected:         Yes (1 island)
```

### Detect Islands

```bash
gat graph islands grid.arrow
```

**Output:**
```
Island Detection
────────────────
Total islands: 2

Island 1: 115 buses
  Buses: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, ...]

Island 2: 3 buses (ISOLATED)
  Buses: [116, 117, 118]
  Warning: Disconnected from main grid
```

### Export for Visualization

```bash
# Export to DOT format (Graphviz)
gat graph export grid.arrow --format dot --out grid.dot

# Render with Graphviz
dot -Tpng grid.dot -o grid.png
```

## Graph Statistics

### `gat graph stats`

Compute topological properties of the network graph:

```bash
gat graph stats grid.arrow --format json
```

**Output columns:**

| Metric | Description |
|--------|-------------|
| `nodes` | Number of buses |
| `edges` | Number of branches (including transformers) |
| `density` | Edge density: 2E / (N × (N-1)) |
| `avg_degree` | Average connections per bus |
| `max_degree` | Most connected bus |
| `min_degree` | Least connected bus |
| `diameter` | Longest shortest path |
| `radius` | Minimum eccentricity |
| `num_islands` | Number of connected components |

### Options

```bash
gat graph stats grid.arrow \
  --include-open-branches   # Count open branches as edges
  --weighted               # Use impedance as edge weight
  --format json            # Output format: table, json, csv
```

### Use Cases

**Pre-analysis validation:**
```bash
# Verify network connectivity before OPF
gat graph stats grid.arrow --format json | jq '.num_islands'
# Should be 1 for a valid network
```

**Compare network variants:**
```bash
# Analyze topology changes
gat graph stats base_case.arrow --format csv > base_stats.csv
gat graph stats upgraded.arrow --format csv > upgraded_stats.csv
diff base_stats.csv upgraded_stats.csv
```

## Island Detection

### `gat graph islands`

Identify electrically isolated subnetworks:

```bash
gat graph islands grid.arrow --out islands.json
```

**Output format:**

```json
{
  "total_islands": 2,
  "islands": [
    {
      "id": 1,
      "bus_count": 115,
      "buses": [1, 2, 3, ...],
      "has_slack": true,
      "total_generation_mw": 4500,
      "total_load_mw": 4200
    },
    {
      "id": 2,
      "bus_count": 3,
      "buses": [116, 117, 118],
      "has_slack": false,
      "total_generation_mw": 0,
      "total_load_mw": 50
    }
  ]
}
```

### Options

```bash
gat graph islands grid.arrow \
  --respect-status        # Treat open branches as disconnected
  --min-size 2            # Only report islands with >= N buses
  --out islands.json      # Output file
  --format json           # Format: table, json, csv
```

### Workflows

**Pre-power-flow validation:**
```bash
#!/bin/bash
# Check for islands before running power flow

ISLANDS=$(gat graph islands grid.arrow --format json | jq '.total_islands')

if [ "$ISLANDS" -gt 1 ]; then
    echo "ERROR: Network has $ISLANDS islands"
    echo "Power flow will fail. Check branch statuses."
    gat graph islands grid.arrow --verbose
    exit 1
fi

echo "Network is connected, running power flow..."
gat pf ac grid.arrow --out pf_results.parquet
```

**Contingency analysis:**
```bash
# Find contingencies that island the network
for branch in $(gat inspect branches grid.arrow --format json | jq -r '.[].id'); do
    # Temporarily open branch
    gat graph islands grid.arrow \
      --open-branch "$branch" \
      --format json > /tmp/islands.json

    islands=$(jq '.total_islands' /tmp/islands.json)
    if [ "$islands" -gt 1 ]; then
        echo "Branch $branch outage creates $islands islands"
    fi
done
```

## Path Finding

### `gat graph path`

Find shortest path between two buses:

```bash
gat graph path grid.arrow --from 1 --to 118
```

**Output:**
```
Shortest Path: Bus 1 → Bus 118
──────────────────────────────
Hops: 7
Path: 1 → 2 → 5 → 6 → 11 → 12 → 117 → 118

Path details:
  1 → 2:   Branch L1-2 (line, 10.5 km)
  2 → 5:   Branch L2-5 (line, 25.0 km)
  5 → 6:   Branch T5-6 (transformer)
  6 → 11:  Branch L6-11 (line, 15.2 km)
  11 → 12: Branch L11-12 (line, 8.3 km)
  12 → 117: Branch L12-117 (line, 42.1 km)
  117 → 118: Branch L117-118 (line, 5.0 km)

Total impedance: 0.0234 + j0.1567 pu
```

### Options

```bash
gat graph path grid.arrow \
  --from 1 \
  --to 118 \
  --weight impedance    # Weight by: hops (default), impedance, length
  --all-paths           # Find all paths (not just shortest)
  --max-hops 10         # Limit path length
  --format json
```

### Use Cases

**Trace power flow path:**
```bash
# Find path from generator to load
gat graph path grid.arrow --from 1 --to 14 --weight impedance
```

**Identify critical paths:**
```bash
# Find all paths between major substations
gat graph path grid.arrow \
  --from 1 --to 100 \
  --all-paths \
  --max-hops 8 \
  --out paths.json
```

## Neighbor Analysis

### `gat graph neighbors`

List buses directly connected to a given bus:

```bash
gat graph neighbors grid.arrow --bus 5
```

**Output:**
```
Neighbors of Bus 5
──────────────────
Direct connections: 4

Bus 2:  Branch L2-5 (line)
Bus 4:  Branch L4-5 (line)
Bus 6:  Branch T5-6 (transformer)
Bus 7:  Branch L5-7 (line)
```

### Options

```bash
gat graph neighbors grid.arrow \
  --bus 5 \
  --depth 2             # Include 2-hop neighbors
  --include-status      # Show branch status
  --format json
```

### Use Cases

**Local topology analysis:**
```bash
# Analyze substation connectivity
gat graph neighbors grid.arrow --bus 69 --depth 2 --format json | \
  jq '.neighbors | length'
```

**Find isolated buses:**
```bash
# Buses with only one connection (radial endpoints)
gat graph stats grid.arrow --format json | \
  jq '.degree_distribution | to_entries | map(select(.value == 1)) | .[].key'
```

## Graph Export

### `gat graph export`

Export network topology for visualization tools:

```bash
# DOT format (Graphviz)
gat graph export grid.arrow --format dot --out grid.dot

# GraphML format (Gephi, yEd)
gat graph export grid.arrow --format graphml --out grid.graphml

# JSON format (D3.js, vis.js)
gat graph export grid.arrow --format json --out grid.json
```

### DOT Format Options

```bash
gat graph export grid.arrow \
  --format dot \
  --label-buses          # Show bus names
  --label-branches       # Show branch IDs
  --color-by voltage     # Color nodes by: voltage, type, island
  --highlight-path 1,5,6,11  # Highlight specific path
  --out grid.dot
```

**Rendering with Graphviz:**
```bash
# Simple layout
dot -Tpng grid.dot -o grid.png

# Force-directed layout (better for large networks)
neato -Tpng grid.dot -o grid_neato.png

# Circular layout
circo -Tpng grid.dot -o grid_circular.png

# SVG for web
dot -Tsvg grid.dot -o grid.svg
```

### GraphML Format Options

```bash
gat graph export grid.arrow \
  --format graphml \
  --include-attributes   # Include bus/branch attributes
  --out grid.graphml
```

**Opening in Gephi:**
1. Open Gephi → File → Open → grid.graphml
2. Apply layout (ForceAtlas 2 works well for power grids)
3. Color by node attribute (voltage level, bus type)

### JSON Format for Web

```bash
gat graph export grid.arrow \
  --format json \
  --include-positions    # Include x,y coordinates if available
  --out grid.json
```

**Structure:**
```json
{
  "nodes": [
    {"id": 1, "name": "Bus 1", "type": "slack", "voltage_kv": 345},
    {"id": 2, "name": "Bus 2", "type": "pv", "voltage_kv": 345}
  ],
  "edges": [
    {"source": 1, "target": 2, "type": "line", "id": "L1-2"},
    {"source": 2, "target": 3, "type": "transformer", "id": "T2-3"}
  ]
}
```

**Using with D3.js:**
```javascript
// Load and visualize
d3.json("grid.json").then(data => {
  const simulation = d3.forceSimulation(data.nodes)
    .force("link", d3.forceLink(data.edges).id(d => d.id))
    .force("charge", d3.forceManyBody().strength(-100))
    .force("center", d3.forceCenter(width/2, height/2));
  // ... render SVG
});
```

## Advanced Workflows

### Topology Validation Pipeline

```bash
#!/bin/bash
# validate_topology.sh - Check network topology before analysis

GRID=$1
REPORT="topology_report.md"

echo "# Topology Validation Report" > "$REPORT"
echo "Grid: $GRID" >> "$REPORT"
echo "Date: $(date)" >> "$REPORT"
echo "" >> "$REPORT"

# 1. Basic statistics
echo "## Graph Statistics" >> "$REPORT"
gat graph stats "$GRID" --format table >> "$REPORT"
echo "" >> "$REPORT"

# 2. Island detection
ISLANDS=$(gat graph islands "$GRID" --format json | jq '.total_islands')
echo "## Connectivity" >> "$REPORT"
if [ "$ISLANDS" -eq 1 ]; then
    echo "✓ Network is fully connected" >> "$REPORT"
else
    echo "⚠ WARNING: $ISLANDS islands detected" >> "$REPORT"
    gat graph islands "$GRID" --verbose >> "$REPORT"
fi
echo "" >> "$REPORT"

# 3. Degree analysis
echo "## Degree Distribution" >> "$REPORT"
gat graph stats "$GRID" --format json | \
  jq '.degree_distribution | to_entries | sort_by(.key) | .[] | "\(.key) connections: \(.value) buses"' >> "$REPORT"

# 4. Export for visualization
echo "## Visualization" >> "$REPORT"
gat graph export "$GRID" --format dot --out topology.dot
dot -Tpng topology.dot -o topology.png
echo "![Network Topology](topology.png)" >> "$REPORT"

echo "Report saved to $REPORT"
```

### Critical Branch Analysis

```bash
#!/bin/bash
# Find branches whose failure would island the network

GRID=$1
echo "Critical Branch Analysis for $GRID"
echo "─────────────────────────────────"

BRANCHES=$(gat inspect branches "$GRID" --format json | jq -r '.[].id')

for branch in $BRANCHES; do
    # Check if removing this branch creates islands
    result=$(gat graph islands "$GRID" --open-branch "$branch" --format json)
    islands=$(echo "$result" | jq '.total_islands')

    if [ "$islands" -gt 1 ]; then
        echo "CRITICAL: Branch $branch"
        echo "  Outage creates $islands islands"
        echo "$result" | jq -r '.islands[] | "  Island \(.id): \(.bus_count) buses"'
    fi
done
```

### Multi-Network Comparison

```bash
# Compare topology of base case vs expansion plan
gat graph stats base.arrow --format json > base_topo.json
gat graph stats expansion.arrow --format json > expansion_topo.json

python3 << 'EOF'
import json

with open("base_topo.json") as f:
    base = json.load(f)
with open("expansion_topo.json") as f:
    expansion = json.load(f)

print("Topology Comparison")
print("─" * 40)
print(f"Buses:      {base['nodes']} → {expansion['nodes']} (+{expansion['nodes'] - base['nodes']})")
print(f"Branches:   {base['edges']} → {expansion['edges']} (+{expansion['edges'] - base['edges']})")
print(f"Density:    {base['density']:.4f} → {expansion['density']:.4f}")
print(f"Avg Degree: {base['avg_degree']:.2f} → {expansion['avg_degree']:.2f}")
print(f"Diameter:   {base['diameter']} → {expansion['diameter']}")
EOF
```

## Troubleshooting

### "Network has multiple islands"

**Cause:** Open branches disconnecting parts of the network

**Solution:**
```bash
# Find open branches
gat inspect branches grid.arrow --format json | \
  jq '.[] | select(.status == false) | .id'

# Check which branches are critical
gat graph islands grid.arrow --verbose
```

### Path not found

**Cause:** Buses are in different islands

**Solution:**
```bash
# Check which island each bus belongs to
gat graph islands grid.arrow --format json | \
  jq '.islands[] | select(.buses | index(BUS_ID))'
```

### Export produces empty file

**Cause:** No buses/branches in network

**Solution:**
```bash
# Verify network loaded correctly
gat inspect summary grid.arrow
```

## Integration

### Python NetworkX Integration

```python
import subprocess
import json
import networkx as nx

def load_gat_network(grid_path):
    """Load GAT network as NetworkX graph."""
    result = subprocess.run(
        ["gat", "graph", "export", grid_path, "--format", "json"],
        capture_output=True, text=True
    )
    data = json.loads(result.stdout)

    G = nx.Graph()
    for node in data["nodes"]:
        G.add_node(node["id"], **node)
    for edge in data["edges"]:
        G.add_edge(edge["source"], edge["target"], **edge)

    return G

# Usage
G = load_gat_network("grid.arrow")
print(f"Nodes: {G.number_of_nodes()}")
print(f"Edges: {G.number_of_edges()}")
print(f"Connected: {nx.is_connected(G)}")
print(f"Diameter: {nx.diameter(G)}")
```

### Rust Integration

```rust
use gat_core::network::Network;
use gat_algo::graph::{GraphAnalyzer, IslandDetector};

let network = gat_io::load_network("grid.arrow")?;

// Compute statistics
let analyzer = GraphAnalyzer::new(&network);
println!("Nodes: {}", analyzer.node_count());
println!("Edges: {}", analyzer.edge_count());
println!("Density: {:.4}", analyzer.density());

// Detect islands
let detector = IslandDetector::new(&network);
let islands = detector.detect()?;
println!("Islands: {}", islands.len());

// Find path
let path = analyzer.shortest_path(1, 118)?;
println!("Path: {:?}", path);
```

## Related Commands

- [Network Inspection](@/guide/inspect.md) — Detailed component data
- [Contingency Analysis](@/guide/reliability.md) — N-1 security assessment
- [Convert](@/guide/convert.md) — Import/export network formats

## Theory Reference

- [Contingency Analysis Theory](@/reference/contingency-analysis.md) — Topology-based screening
- [Graph Theory](@/reference/graph.md) — Mathematical foundations
