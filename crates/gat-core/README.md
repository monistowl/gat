# gat-core — Core Grid Types & Solvers

Fundamental data structures and algorithms for power-system modeling, power-flow analysis, and optimization.

## Quick Overview

**Core Grid Representation:**
- Bus/branch network topology via `petgraph`
- Generator dispatch limits and cost models
- Load profiles and demand
- Transmission and distribution networks

**Solvers:**
- DC/AC power flow (feasibility checking, Q-limit enforcement)
- DC/AC optimal power flow (cost minimization)
- **Full nonlinear AC-OPF** (v0.3.4) — penalty-based L-BFGS solver
- SOCP relaxation for fast convex approximation
- N-1/N-2 contingency analysis
- State estimation (weighted least squares)

**Outputs:**
- Voltages, phase angles, line flows
- Generation dispatch, constraint violations
- Scenarios and sensitivity analyses

## Grid Data Structures

```rust
use gat_core::{Network, Node, Edge, Bus, BusId, Branch, BranchId, Gen, GenId, Load, LoadId, CostModel};

// Network uses petgraph for topology
let mut network = Network::new();

// Add buses as nodes
let bus1_idx = network.graph.add_node(Node::Bus(Bus {
    id: BusId::new(0),
    name: "Bus1".into(),
    voltage_kv: 138.0,
}));

// Add generators with cost models and limits
let gen = Gen::new(GenId::new(0), "Gen1".into(), BusId::new(0))
    .with_p_limits(10.0, 100.0)      // Pmin=10 MW, Pmax=100 MW
    .with_q_limits(-50.0, 50.0)      // Qmin=-50 MVAr, Qmax=50 MVAr
    .with_cost(CostModel::quadratic(100.0, 20.0, 0.01));  // $100 + $20/MWh + $0.01/MW²h
network.graph.add_node(Node::Gen(gen));

// Add loads
network.graph.add_node(Node::Load(Load {
    id: LoadId::new(0),
    name: "Load1".into(),
    bus: BusId::new(0),
    active_power_mw: 50.0,
    reactive_power_mvar: 10.0,
}));

// Add branches as edges
network.graph.add_edge(bus1_idx, bus2_idx, Edge::Branch(Branch {
    id: BranchId::new(0),
    name: "Line1-2".into(),
    from_bus: BusId::new(0),
    to_bus: BusId::new(1),
    resistance: 0.01,
    reactance: 0.1,
    ..Branch::default()
}));
```

## Generator Cost Models

Generators support polynomial and piecewise-linear cost functions:

```rust
use gat_core::CostModel;

// Quadratic: cost = c0 + c1*P + c2*P²
let quadratic = CostModel::quadratic(100.0, 20.0, 0.01);
assert!((quadratic.evaluate(50.0) - 1125.0).abs() < 1e-6);
assert!((quadratic.marginal_cost(50.0) - 21.0).abs() < 1e-6);

// Linear: cost = c0 + c1*P
let linear = CostModel::linear(50.0, 25.0);

// Piecewise linear: Vec<(MW, $/hr)>
let piecewise = CostModel::PiecewiseLinear(vec![
    (0.0, 0.0),
    (50.0, 1000.0),
    (100.0, 2500.0),
]);
```

## Core APIs

### Optimal Power Flow (via gat-algo)
```rust
use gat_algo::{OpfSolver, OpfMethod};

// DC economic dispatch
let solver = OpfSolver::new()
    .with_method(OpfMethod::DcOpf);
let solution = solver.solve(&network)?;

// SOCP relaxation (fast, convex)
let solver = OpfSolver::new()
    .with_method(OpfMethod::SocpRelaxation);

// Full nonlinear AC-OPF (v0.3.4)
let solver = OpfSolver::new()
    .with_method(OpfMethod::AcOpf)
    .with_max_iterations(200)
    .with_tolerance(1e-4);
let solution = solver.solve(&network)?;
println!("Objective: ${:.2}/hr", solution.objective_value);
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
- **argmin L-BFGS** (full AC-OPF nonlinear optimization, v0.3.4)

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

**Website Guides:**
- [Power Flow Guide](https://monistowl.github.io/gat/guide/pf/) — DC/AC power flow theory and examples
- [Optimal Power Flow](https://monistowl.github.io/gat/guide/opf/) — Optimization formulations and solvers
- [State Estimation](https://monistowl.github.io/gat/guide/se/) — Weighted least squares estimation

**Local Files:**
- `examples/` — Standalone solver examples
- `tests/` — Test cases with MATPOWER models
- `docs/guide/pf.md` — Local power flow documentation

## See Also

- [Full Documentation](https://monistowl.github.io/gat/) — GAT website
- [GAT Main README](../../README.md)
- [gat-cli README](../gat-cli/README.md)
- [AGENTS.md](../../AGENTS.md)
