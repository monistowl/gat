---
status: completed
priority: p1
issue_id: "003"
tags: [performance, code-review, algo]
dependencies: []
---

# Dense Matrix Allocation in DC Power Flow

## Problem Statement

The `build_bus_susceptance` function at `crates/gat-algo/src/power_flow.rs:1266` allocates a dense n×n matrix for the susceptance matrix, despite power grids being extremely sparse (typically 0.03% density for 10,000-bus networks).

**Why it matters:** This causes 40-4000x memory overhead and prevents analysis of large networks.

## Resolution

Implemented sparse pathway for base case DC power flow:

1. Added `compute_dc_angles_sparse()` function that uses `SparseSusceptance` and `IncrementalSolver`
2. Added `get_bus_ids()` helper function to avoid building full matrix just for bus enumeration
3. Modified `branch_flow_dataframe()` to route:
   - **Base case (no skip_branch)**: Uses sparse solver for memory efficiency
   - **Contingency analysis (with skip_branch)**: Uses dense solver for Woodbury updates

**Changes:**
- `crates/gat-algo/src/power_flow.rs`:
  - Added `compute_dc_angles_sparse()` (~40 lines)
  - Added `get_bus_ids()` helper (~10 lines)
  - Modified `branch_flow_dataframe()` routing logic

**Memory improvement:** For 10,000-bus networks, reduced from 800 MB to ~2 MB for base case.

## Acceptance Criteria

- [x] Dense matrix replaced with sparse representation (for base case)
- [x] N-1 contingency analysis uses appropriate solver
- [x] Tests pass with sparse implementation (141 tests pass)

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | Dense matrix dominates memory for large networks |
| 2025-12-06 | Implemented sparse pathway | Leverage existing SparseSusceptance infrastructure |
