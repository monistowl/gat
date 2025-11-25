# Optimal Power Flow (OPF)

This reference describes the OPF solver architecture, solution methods, and CLI commands.

## Architecture Overview (v0.3.2)

GAT provides a unified `OpfSolver` supporting multiple solution methods with varying accuracy/speed tradeoffs:

| Method | Accuracy | Speed | Use Case |
|--------|----------|-------|----------|
| `EconomicDispatch` | ~20% gap | Fastest | Quick estimates, screening |
| `DcOpf` | ~3-5% gap | Fast | Planning studies |
| `SocpRelaxation` | ~1-3% gap | Moderate | Research benchmarking |
| `AcOpf` | <1% gap | Slowest | High-fidelity analysis |

**Current Status:** Economic dispatch is fully implemented. DC-OPF, SOCP, and AC-OPF methods return `NotImplemented` errors (planned for future releases).

## Rust API

### OpfSolver

```rust
use gat_algo::{OpfSolver, OpfMethod, OpfSolution, OpfError};
use gat_core::Network;

// Create solver with method selection
let solver = OpfSolver::new()
    .with_method(OpfMethod::EconomicDispatch)
    .with_tolerance(1e-6)
    .with_max_iterations(100);

// Solve
let solution: OpfSolution = solver.solve(&network)?;

println!("Converged: {}", solution.converged);
println!("Objective: ${:.2}/hr", solution.objective_value);
println!("Method: {}", solution.method_used);
```

### OpfMethod Enum

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OpfMethod {
    /// Merit-order economic dispatch (no network constraints)
    EconomicDispatch,
    /// DC optimal power flow (LP with B-matrix)
    DcOpf,
    /// Second-order cone relaxation of AC-OPF
    #[default]
    SocpRelaxation,
    /// Full nonlinear AC-OPF (interior point)
    AcOpf,
}
```

### OpfSolution

```rust
pub struct OpfSolution {
    // Status
    pub converged: bool,
    pub method_used: OpfMethod,
    pub iterations: usize,
    pub solve_time_ms: u128,

    // Objective
    pub objective_value: f64,  // Total cost ($/hr)

    // Primal variables
    pub generator_p: HashMap<String, f64>,      // Active power (MW)
    pub generator_q: HashMap<String, f64>,      // Reactive power (MVAr)
    pub bus_voltage_mag: HashMap<String, f64>,  // |V| in p.u.
    pub bus_voltage_ang: HashMap<String, f64>,  // θ in radians
    pub branch_p_flow: HashMap<String, f64>,    // MW flow
    pub branch_q_flow: HashMap<String, f64>,    // MVAr flow

    // Dual variables
    pub bus_lmp: HashMap<String, f64>,          // $/MWh at each bus

    // Constraints
    pub binding_constraints: Vec<ConstraintInfo>,
    pub total_losses_mw: f64,
}
```

**Note:** Not all fields are populated by all methods. Economic dispatch provides generator outputs, objective, and estimated losses. LMPs and voltage angles require DC-OPF or higher.

## Generator Cost Models

Generators support polynomial and piecewise-linear cost functions via the `CostModel` enum:

```rust
use gat_core::{Gen, GenId, BusId, CostModel};

// Quadratic cost: $100 + $20/MWh + $0.01/MW²h
let gen = Gen::new(GenId::new(0), "Gen1".into(), BusId::new(0))
    .with_p_limits(10.0, 100.0)    // Pmin=10 MW, Pmax=100 MW
    .with_q_limits(-50.0, 50.0)    // Qmin=-50 MVAr, Qmax=50 MVAr
    .with_cost(CostModel::quadratic(100.0, 20.0, 0.01));

// Linear cost: $50 + $25/MWh
let gen2 = Gen::new(GenId::new(1), "Gen2".into(), BusId::new(1))
    .with_p_limits(0.0, 200.0)
    .with_cost(CostModel::linear(50.0, 25.0));

// Piecewise linear: [(MW, $/hr), ...]
let gen3 = Gen::new(GenId::new(2), "Gen3".into(), BusId::new(2))
    .with_p_limits(0.0, 100.0)
    .with_cost(CostModel::PiecewiseLinear(vec![
        (0.0, 0.0),
        (50.0, 1000.0),
        (100.0, 2500.0),
    ]));
```

### CostModel Methods

```rust
impl CostModel {
    /// Evaluate cost at given power output ($/hr)
    pub fn evaluate(&self, p_mw: f64) -> f64;

    /// Get marginal cost at given power ($/MWh)
    pub fn marginal_cost(&self, p_mw: f64) -> f64;

    /// Check if this cost model has actual cost data
    pub fn has_cost(&self) -> bool;
}
```

## CLI Commands

### DC OPF (`gat opf dc`)

Solves a linear dispatch problem with generator costs, limits, and demand.

```bash
gat opf dc grid.arrow \
  --cost test_data/opf/costs.csv \
  --limits test_data/opf/limits.csv \
  --out results/dc-opf.parquet \
  [--branch-limits test_data/opf/branch_limits.csv] \
  [--piecewise test_data/opf/piecewise.csv]
```

#### Inputs

* `--cost` (required): CSV with `bus_id,marginal_cost`. Missing rows default to `1.0`.
* `--limits` (required): CSV with `bus_id,pmin,pmax,demand`. Defines dispatch bounds and local load.
* `--branch-limits` (optional): CSV with `branch_id,flow_limit`. Rejects solutions violating limits.
* `--piecewise` (optional): CSV with `bus_id,start,end,slope` for piecewise linear costs.

#### Output

* `--out` writes a Parquet table with `branch_id`, `from_bus`, `to_bus`, and `flow_mw`.

### AC OPF (`gat opf ac`)

Runs a Newton–Raphson solve over the AC equations.

```bash
gat opf ac grid.arrow \
  --out results/ac-opf.parquet \
  [--tol 1e-6] \
  [--max-iter 20]
```

* `--tol`: convergence tolerance (default `1e-6`).
* `--max-iter`: maximum Newton iterations (default `20`).

## Test Fixtures

`test_data/opf` provides reusable CSVs for local experiments:

* `costs.csv`: sample marginal costs for buses `0` and `1`.
* `limits.csv`: matching `pmin`, `pmax`, and `demand` entries.
* `branch_limits.csv`: tight limits for violation testing.
* `piecewise.csv`: two-piece segments for piecewise cost testing.

## Related Documentation

* Power flow: `docs/guide/pf.md`
* State estimation: `docs/guide/se.md`
* Benchmarking: `docs/guide/benchmark.md`
* Reliability: `docs/guide/reliability.md`
