# gat-core — Core Grid Types & Solvers

Fundamental data structures and algorithms for power-system modeling, power-flow analysis, and optimization.

## Quick Overview

**Core Grid Representation:**
- Bus/branch network topology
- Generator dispatch limits and costs
- Load profiles and demand
- Transmission and distribution networks

**Solvers:**
- DC/AC power flow (feasibility checking)
- DC/AC optimal power flow (cost minimization)
- N-1 contingency analysis
- State estimation (weighted least squares)

**Outputs:**
- Voltages, phase angles, line flows
- Generation dispatch, constraint violations
- Scenarios and sensitivity analyses

## Grid Data Structures

```rust
// Main grid model
pub struct Grid {
    pub buses: Vec<Bus>,
    pub branches: Vec<Branch>,
    pub generators: Vec<Generator>,
    pub loads: Vec<Load>,
}

// Power flow solution
pub struct PFSolution {
    pub vm: Vec<f64>,      // Voltage magnitudes
    pub va: Vec<f64>,      // Voltage angles
    pub pf: Vec<f64>,      // Branch power flows
    pub violations: Vec<ViolationFlag>,
}
```

## Core APIs

### Power Flow
```rust
// DC power flow
let solution = dc_power_flow(&grid, &loads)?;

// AC power flow with Newton-Raphson
let solution = ac_power_flow(&grid, &loads, options)?;
```

### Optimal Power Flow
```rust
// DC OPF (linear)
let dispatch = dc_opf(&grid, &loads, &costs, &limits)?;

// AC OPF (nonlinear with solver selection)
let dispatch = ac_opf(&grid, &loads, &costs, &limits, solver)?;
```

### Contingency Analysis
```rust
// Test each branch outage
for branch in &grid.branches {
    let mut test_grid = grid.clone();
    test_grid.remove_branch(branch.id)?;
    let solution = dc_power_flow(&test_grid, &loads)?;
    violations.extend(solution.violations);
}
```

### State Estimation
```rust
let measurements = vec![
    Measurement::BranchFlow(branch_id, flow_value),
    Measurement::BusInjection(bus_id, injection_value),
];
let state = weighted_least_squares(&grid, &measurements)?;
```

## Solver Backends

Pluggable LP/QP solver integration via `good_lp`:
- **Clarabel** (default, pure Rust, open-source)
- **HiGHS** (dual simplex, high performance)
- **CBC** (COIN-OR, robust and mature)
- **IPOPT** (interior-point, nonlinear OPF)

Select solver at build time or runtime:
```bash
cargo build -p gat-core --features "all-backends"
```

## Key Types

**Network Model:**
- `Bus` — Voltage node with generation/load injection
- `Branch` — Transmission/distribution line with impedance and limits
- `Generator` — Dispatchable unit with cost curves and limits
- `Load` — Demand at buses (fixed or variable)

**Results:**
- `PFSolution` — Power flow results (voltages, angles, flows)
- `OPFSolution` — Dispatch with costs and constraint status
- `ContingencyResult` — Per-outage violation summary

**Constraints:**
- Voltage magnitude limits
- Branch flow limits
- Generator dispatch limits
- Ramping constraints (dynamic)

## Common Use Cases

### Load Flow Analysis
```rust
let grid = load_grid_from_arrow(path)?;
let loads = read_demand_profile(demand_file)?;
let solution = dc_power_flow(&grid, &loads)?;
save_solution(solution, output_path)?;
```

### Economic Dispatch
```rust
let costs = read_cost_curves(cost_file)?;
let limits = read_dispatch_limits(limits_file)?;
let dispatch = dc_opf(&grid, &loads, &costs, &limits)?;
assert!(dispatch.is_feasible());
```

### Reliability Metrics
```rust
let contingencies = enumerate_nminus1(&grid);
let mut lole_hours = 0.0;
for contingency in contingencies {
    let test_grid = grid.with_outage(contingency)?;
    let solution = dc_power_flow(&test_grid, &loads)?;
    if !solution.is_feasible() {
        lole_hours += time_slice_duration;
    }
}
```

## Dependencies

**Essential:**
- `ndarray` — Numerical linear algebra
- `sparse-linalg` — Sparse matrix operations
- `itertools` — Iterator utilities

**Optional (feature-gated):**
- `good_lp` — LP/QP solver abstraction
- `clarabel` — Default open-source solver
- `highs-sys` — HiGHS bindings
- `ipopt-sys` — IPOPT bindings

## Testing

```bash
# Quick check
cargo check -p gat-core

# Unit tests
cargo test -p gat-core --lib

# Integration tests (with solvers)
cargo test -p gat-core

# Specific test
cargo test -p gat-core test_dc_pf_ieee14 -- --nocapture
```

## Benchmarking

```bash
cargo bench -p gat-core
```

Typical performance on IEEE test cases:
- IEEE 14-bus: 5-10 ms per solve
- IEEE 118-bus: 50-100 ms per solve
- Large distribution (1000+ nodes): 200-500 ms per solve

## Related Crates

- **gat-io** — Data import/export (Arrow, Parquet, CSV)
- **gat-batch** — Parallel solve orchestration
- **gat-dist** — Distribution system specializations
- **gat-adms** — Distribution automation features
- **gat-scenarios** — Scenario generation for batch runs

## Documentation

**Examples:**
- `examples/` — Standalone solver examples
- `tests/` — Test cases with MATPOWER models

**Guides:**
- `docs/guide/pf.md` — Power flow theory and examples
- `docs/guide/opf.md` — Optimization formulations and solvers
- `docs/guide/se.md` — State estimation

## See Also

- [GAT Main README](../../README.md)
- [gat-cli README](../gat-cli/README.md)
- [AGENTS.md](../../AGENTS.md)
