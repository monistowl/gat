+++
title = "Optimal Power Flow"
description = "Optimal Power Flow (OPF)"
weight = 11
+++

# Optimal Power Flow (OPF)

This reference describes the OPF solver architecture, solution methods, and CLI commands.

## Architecture Overview (v0.3.4)

GAT provides a unified `OpfSolver` supporting multiple solution methods with varying accuracy/speed tradeoffs:

| Method | Accuracy | Speed | Status | Use Case |
|--------|----------|-------|--------|----------|
| `EconomicDispatch` | ~20% gap | Fastest | ✅ Implemented | Quick estimates, screening |
| `DcOpf` | ~3-5% gap | Fast | ✅ Implemented | Planning studies |
| `SocpRelaxation` | ~1-3% gap | Moderate | ✅ Implemented | Research benchmarking |
| `AcOpf` | <1% gap | Slowest | ✅ Implemented (L-BFGS penalty) | High-fidelity analysis |

### What's new in 0.3.4

- **Full nonlinear AC-OPF path is now routed to the new `ac_nlp` module** instead of returning `NotImplemented`.
- Robust **Y-bus construction** with transformer taps, phase shifters, and π-model line charging.
- Explicit **power injection equations and Jacobians** in polar form with voltage magnitude/angle variables.
- **Penalty-method solver** using argmin's L-BFGS optimizer with automatic penalty ramping and flat-start initialization.
- **LMP approximation** from marginal generator costs after the penalty solve completes.
- Expanded **test coverage** (2-bus, 3-bus, SOCP bound comparisons, merit order, and convergence safeguards).

## Rust API

### OpfSolver

```rust
use gat_algo::{OpfSolver, OpfMethod, OpfSolution, OpfError};
use gat_core::Network;

// Create solver with method selection
let solver = OpfSolver::new()
    .with_method(OpfMethod::SocpRelaxation)  // or AcOpf
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
    /// Full nonlinear AC-OPF (penalty-method L-BFGS)
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
    pub bus_voltage_ang: HashMap<String, f64>,  // θ in degrees
    pub branch_p_flow: HashMap<String, f64>,    // MW flow
    pub branch_q_flow: HashMap<String, f64>,    // MVAr flow

    // Dual variables
    pub bus_lmp: HashMap<String, f64>,          // $/MWh at each bus

    // Constraints
    pub binding_constraints: Vec<ConstraintInfo>,
    pub total_losses_mw: f64,
}
```

**Note:** Not all fields are populated by all methods. Economic dispatch provides generator outputs, objective, and estimated losses. SOCP and AC-OPF provide full voltage, angle, and LMP data; AC-OPF now backfills LMPs using marginal generator costs after the penalty loop finishes.

## SOCP Relaxation (v0.3.4)

The SOCP solver implements the Baran-Wu / Farivar-Low branch-flow model:

### Features

| Feature | Status |
|---------|--------|
| Squared voltage/current variables | ✅ |
| Quadratic costs (c₀ + c₁·P + c₂·P²) | ✅ |
| Phase-shifting transformers | ✅ |
| Off-nominal tap ratios | ✅ |
| Line charging (π-model) | ✅ |
| Thermal limits (S_max) | ✅ |
| Voltage bounds | ✅ |
| LMP extraction from duals | ✅ |

### Mathematical Formulation

Variables: w_i (squared voltage), ℓ_ij (squared current), P_ij, Q_ij (branch flows)

**Objective:**
```
minimize Σ (c₀ + c₁·P_g + c₂·P_g²)
```

**Branch-flow constraints:**
```
w_j = w_i - 2(r·P_ij + x·Q_ij) + (r² + x²)·ℓ_ij
P_ij² + Q_ij² ≤ w_i · ℓ_ij  (SOC constraint)
```

**Solver:** Clarabel interior-point conic solver (15-30 iterations typical)

### References

- Baran & Wu (1989): DOI:10.1109/61.25627
- Farivar & Low (2013): DOI:10.1109/TPWRS.2013.2255317
- Gan, Li, Topcu & Low (2015): DOI:10.1109/TAC.2014.2332712

## Full AC-OPF (v0.3.4)

The AC-OPF solver uses polar coordinates with a penalty-method L-BFGS optimizer and now ships with a complete `ac_nlp` pipeline:

### Features

| Feature | Status |
|---------|--------|
| Polar formulation (V, θ) | ✅ |
| Y-bus construction (with taps + phase shifts) | ✅ |
| Line charging / π-model support | ✅ |
| Quadratic costs | ✅ |
| Voltage bounds | ✅ |
| Generator limits | ✅ |
| Jacobian computation | ✅ |
| L-BFGS penalty optimizer | ✅ |
| Thermal limits | 🔄 Planned |
| IPOPT backend | 🔄 Planned |

Key components in `gat_algo::opf::ac_nlp`:

- `ybus.rs`: builds the complex admittance matrix with tap ratios, phase shifters, and shunt line charging.
- `power_equations.rs`: evaluates P/Q injections and full Jacobians (∂P/∂θ, ∂P/∂V, ∂Q/∂θ, ∂Q/∂V) in polar form.
- `solver.rs`: wraps argmin's L-BFGS optimizer with a penalty ramp until equality constraints reach feasibility.

### Mathematical Formulation

Variables: V_i (voltage magnitude), θ_i (angle), P_g, Q_g (generator dispatch)

**Objective:**
```
minimize Σ (c₀ + c₁·P_g + c₂·P_g²)
```

**Power flow equations:**
```
P_i = Σⱼ V_i·V_j·(G_ij·cos(θ_i - θ_j) + B_ij·sin(θ_i - θ_j))
Q_i = Σⱼ V_i·V_j·(G_ij·sin(θ_i - θ_j) - B_ij·cos(θ_i - θ_j))
```

**Solver:** argmin L-BFGS with iterative penalty method (penalty factor ramps until equality constraints are feasible).

### Usage

```rust
let solver = OpfSolver::new()
    .with_method(OpfMethod::AcOpf)
    .with_max_iterations(200)
    .with_tolerance(1e-4);

let solution = solver.solve(&network)?;
```

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

Runs a penalty-method nonlinear solve over the full AC equations (polar form) using argmin's L-BFGS optimizer.

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

* Power flow: [Power Flow Guide](/guide/pf/)
* State estimation: [State Estimation Guide](/guide/se/)
* Benchmarking: [Benchmarking Guide](/guide/benchmark/)
* Reliability: [Reliability Guide](/guide/reliability/)
