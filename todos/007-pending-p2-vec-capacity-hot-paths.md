---
status: completed
priority: p2
issue_id: "007"
tags: [performance, code-review]
dependencies: []
---

# Vec Allocation Without Capacity in Hot Paths

## Problem Statement

Multiple hot paths create vectors without pre-allocated capacity despite knowing the final size, causing unnecessary reallocations.

**Why it matters:** With 10k measurements: ~15 reallocations per vector = 165 total reallocations per analysis.

## Resolution

Added `with_capacity()` hints to hot path allocations in:

1. **`alloc_rents.rs:231-238`** - Congestion rent calculation (7 vectors)
   - Used `opf_df.height()` for capacity

2. **`alloc_kpi.rs:321-333`** - KPI contribution computation
   - Outer `contributions` vector uses `kpi_columns.len() * control_columns.len()`
   - Inner partition vectors use `row_count`

3. **`alloc_kpi.rs:384-392`** - Result DataFrame assembly (9 vectors)
   - Used `contributions.len()` for capacity

4. **`featurize_gnn.rs:425-428`** - GNN feature batching (3 vectors)
   - `all_nodes`: `num_graphs * node_features.len()`
   - `all_edges`: `num_graphs * edge_features.len()`
   - `all_graphs`: `num_graphs`

5. **`featurize_gnn.rs:438, 458-460`** - Flow map construction
   - `flows_map`: `groups.len()`
   - `flow_maps`: `flows_by_graph.len()`
   - `flow_map` HashMap: `group_df.height()`

**Note:** `power_flow.rs:980-987` was already fixed in a previous session.

## Acceptance Criteria

- [x] Hot path Vec allocations use with_capacity
- [x] Compilation verified
- [x] No functional changes

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | Pre-allocation prevents reallocation overhead |
| 2025-12-06 | Added capacity hints | Focus on hot paths with predictable sizes |
