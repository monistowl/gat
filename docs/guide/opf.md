# Optimal Power Flow (OPF)

This reference describes the OPF solver architecture, solution methods, and CLI commands.

## Architecture Overview (v0.3.2)

GAT provides a unified `OpfSolver` supporting multiple solution methods with varying accuracy/speed tradeoffs:

| Method | Accuracy | Speed | Use Case | Status |
|--------|----------|-------|----------|--------|
| `EconomicDispatch` | ~20% gap | Fastest | Quick estimates, screening | Implemented |
| `DcOpf` | ~3-5% gap | Fast | Planning studies | Implemented |
| `SocpRelaxation` | ~1-3% gap | Moderate | Research, convex lower bounds | **Implemented** |
| `AcOpf` | <1% gap | Slowest | High-fidelity analysis | Planned |

**Current Status:** Economic dispatch, DC-OPF, and SOCP relaxation are fully implemented. AC-OPF (nonlinear interior point) is planned for a future release.

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

## SOCP Relaxation Details

The SOCP (Second-Order Cone Programming) relaxation provides a convex approximation to AC-OPF that:

- **Guarantees global optimality** within the relaxed problem
- **Provides valid lower bounds** on the true AC-OPF objective
- **Often yields AC-feasible solutions** directly (exactness for radial networks)
- **Runs in polynomial time** via interior-point methods

### Mathematical Foundation

The solver implements the Baran-Wu / Farivar-Low branch-flow model:

```
Variables per branch:
  P_ij, Q_ij  = real/reactive power flow
  ℓ_ij        = |I_ij|² (squared current magnitude)
  v_i         = |V_i|² (squared voltage magnitude)

Key constraint (relaxed):
  P² + Q² ≤ v · ℓ    (SOCP relaxation of P² + Q² = v · ℓ)
```

**References:**
- Farivar & Low (2013): [DOI:10.1109/TPWRS.2013.2255317](https://doi.org/10.1109/TPWRS.2013.2255317)
- Low (2014): [DOI:10.1109/TCNS.2014.2309732](https://doi.org/10.1109/TCNS.2014.2309732)

### Supported Features

| Feature | Support |
|---------|---------|
| Quadratic cost curves | ✅ `c₀ + c₁·P + c₂·P²` |
| Piecewise-linear costs | ✅ Approximated at midpoint |
| Voltage magnitude bounds | ✅ Default [0.9, 1.1] p.u. |
| Thermal limits (MVA) | ✅ From `s_max_mva` or `rating_a_mva` |
| Tap-changing transformers | ✅ Off-nominal tap ratios |
| Phase-shifting transformers | ✅ Angle-coupled formulation |
| Line charging (π-model) | ✅ Half-line shunt susceptance |
| LMP computation | ✅ From dual variables |
| Binding constraint reporting | ✅ With shadow prices |

### Usage

```rust
use gat_algo::{OpfSolver, OpfMethod};

let solver = OpfSolver::new()
    .with_method(OpfMethod::SocpRelaxation)
    .with_tolerance(1e-6)
    .with_max_iterations(100);

let solution = solver.solve(&network)?;

// Access results
println!("Total cost: ${:.2}/hr", solution.objective_value);
println!("System losses: {:.2} MW", solution.total_losses_mw);

for (bus, lmp) in &solution.bus_lmp {
    println!("LMP at {}: ${:.2}/MWh", bus, lmp);
}
```

### Solver Backend

SOCP uses [Clarabel](https://github.com/oxfordcontrol/Clarabel.rs), a high-performance interior-point solver for conic programs. Typical convergence is 15-30 iterations.

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
