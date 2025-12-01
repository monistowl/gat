+++
title = "Optimal Power Flow"
description = "Complete guide to GAT's four-tier OPF solver hierarchy"
weight = 11
+++

# Optimal Power Flow (OPF)

GAT provides a **four-tier solver hierarchy** for optimal power flow, from sub-millisecond economic dispatch to production-grade nonlinear optimization. Each tier offers a different accuracy/speed tradeoff, letting you choose the right tool for each task.

<div class="grid-widget" data-network="ieee14" data-height="420" data-flow="true" data-lmp="true" data-legend="true" data-caption="Interactive: Click ⚡ to see power flow direction and loading. Click $ to visualize Locational Marginal Prices (LMPs) from OPF results."></div>

> **Validated Performance**: GAT's SOCP solver achieves 100% convergence across all 67 PGLib-OPF benchmark cases.
> See the [complete benchmark results](@/internals/benchmarks.md) for details.

## Choosing the Right Solver

```
                    Speed vs. Accuracy Tradeoff

  Fast  ──────────────────────────────────────────►  Accurate

  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
  │     ED      │   │   DC-OPF    │   │    SOCP     │   │   AC-OPF    │
  │   < 1ms     │   │   ~10ms     │   │   ~100ms    │   │    ~1s      │
  │  No network │   │  LP approx  │   │ Convex relax│   │  Full NLP   │
  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
       │                  │                  │                  │
       ▼                  ▼                  ▼                  ▼
  Feasibility      N-1 screening       Production        Final
  checks           Planning studies    dispatch          validation
```

### Quick Reference

| Tier | Command | Speed | Accuracy | Best For |
|------|---------|-------|----------|----------|
| 1 | `gat opf ed` | < 1ms | ~20% gap | Feasibility checks, generation scheduling |
| 2 | `gat opf dc` | ~10ms | ~3-5% gap | N-1 screening, transmission planning |
| 3 | `gat opf socp` | ~100ms | ~1-3% gap | Production dispatch, voltage-aware |
| 4 | `gat opf ac` | ~1s | < 0.01% gap | Final validation, full physics |

### Decision Tree

**Use Economic Dispatch when:**
- You need instant results (< 1ms)
- Network constraints don't matter yet
- Quick "can we meet demand?" checks

**Use DC-OPF when:**
- You're screening thousands of contingencies
- Planning studies where speed matters
- Real power flows are sufficient

**Use SOCP when:**
- You need voltage information
- Production dispatch decisions
- Tight bounds on generation cost

**Use AC-OPF when:**
- Final operational validation
- Full physics accuracy required
- Regulatory compliance

## Architecture Overview (v0.5.0)

GAT provides a unified `OpfSolver` supporting multiple solution methods:

| Method | Accuracy | Speed | Status | Use Case |
|--------|----------|-------|--------|----------|
| `EconomicDispatch` | ~20% gap | Fastest | ✅ Implemented | Quick estimates, screening |
| `DcOpf` | ~3-5% gap | Fast | ✅ Implemented | Planning studies |
| `SocpRelaxation` | ~1-3% gap | Moderate | ✅ Implemented | Research benchmarking |
| `AcOpf` (L-BFGS) | ~2-3% gap | Moderate | ✅ Implemented | Pure Rust deployment |
| `AcOpf` (IPOPT) | **<0.01% gap** | Fast | ✅ **Validated** | High-fidelity analysis |

### Benchmark Results

The IPOPT backend with analytical Jacobian and Hessian achieves exact agreement with PGLib reference values:

| Case | GAT Objective | Reference | Gap |
|------|---------------|-----------|-----|
| case14_ieee | $2,178.08/hr | $2,178.10/hr | **-0.00%** |
| case118_ieee | $97,213.61/hr | $97,214.00/hr | **-0.00%** |

The SOCP solver has been validated against all 67 PGLib-OPF cases:

| Metric | Value |
|--------|-------|
| Cases Tested | 67 |
| Convergence Rate | 100% |
| Largest System | 78,484 buses |
| Median Objective Gap | < 1% |

→ [View complete benchmark results](@/internals/benchmarks.md)

### What's New in 0.5.0

- **Full nonlinear AC-OPF** reproduces **all 68 PGLib benchmark cases with <0.01% gap** using IPOPT backend.
- **Multi-period dispatch** with generator ramp constraints for day-ahead scheduling.
- **IPOPT solver backend** with analytical Jacobian and Hessian — matches commercial solver precision.
- **Native solver plugin system** with automatic fallback to pure-Rust solvers.
- **Warm-start options** from DC or SOCP solutions for improved convergence.
- **Native piecewise-linear cost support** for bid curves.
- **Generator capability curves** (Q limits as function of P).
- **Angle difference constraints** for stability enforcement.
- **Sparse Y-bus** with O(nnz) storage for efficient large-network handling.
- Robust **Y-bus construction** with transformer taps, phase shifters, shunts, and π-model line charging.
- **Shunt support** for exact power flow agreement with external tools.

## Solver Backends

GAT provides a **native solver plugin system** that automatically selects the best available backend:

| Backend | Type | Best For | Availability |
|---------|------|----------|--------------|
| L-BFGS (default) | Pure Rust | General AC-OPF, portability | Always available |
| Clarabel | Pure Rust | SOCP, LP problems | Always available |
| IPOPT | Native (C++) | Large NLP, high accuracy | Optional installation |
| HiGHS | Native (C++) | LP/MIP, high performance | Optional installation |
| CBC | Native (C) | MIP problems | Optional installation |

### Installing Native Solvers

Native solvers provide better performance for large networks but require system dependencies:

```bash
# Build and install IPOPT wrapper (requires libipopt-dev)
cargo xtask solver build ipopt --install

# List installed native solvers
gat solver list

# Uninstall a native solver
gat solver uninstall ipopt
```

### Architecture Benefits

Native solvers run as **isolated subprocesses** communicating via Arrow IPC:

- **Crash isolation**: Native library issues don't crash the main process
- **Version flexibility**: Different solver versions can coexist
- **Portability**: Pure-Rust fallbacks always available when native solvers aren't installed

The solver dispatcher automatically selects the best available backend based on problem class (LP, SOCP, NLP, MIP).

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

## SOCP Relaxation (v0.4.0)

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

## Full AC-OPF (v0.4.0)

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
| Thermal limits (branch flow) | ✅ |
| IPOPT backend | ✅ (`solver-ipopt` feature) |

Key components in `gat_algo::opf::ac_nlp`:

- `ybus.rs`: builds the complex admittance matrix with tap ratios, phase shifters, and shunt line charging.
- `sparse_ybus.rs`: O(nnz) sparse Y-bus storage for large networks.
- `power_equations.rs`: evaluates P/Q injections and full Jacobians (∂P/∂θ, ∂P/∂V, ∂Q/∂θ, ∂Q/∂V) in polar form.
- `branch_flow.rs`: computes branch apparent power flows for thermal limit enforcement.
- `hessian.rs`: second-derivative computation for interior-point methods (IPOPT).
- `solver.rs`: wraps argmin's L-BFGS optimizer with a penalty ramp until equality constraints reach feasibility.
- `ipopt_solver.rs`: full-featured interior-point solver via IPOPT (requires `solver-ipopt` feature).

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

GAT provides four OPF commands matching the solver hierarchy:

```bash
# Tier 1: Economic Dispatch (< 1ms)
gat opf ed grid.arrow --out dispatch.parquet

# Tier 2: DC-OPF (~10ms for 118-bus)
gat opf dc grid.arrow --out flows.parquet

# Tier 3: SOCP Relaxation (~100ms for 118-bus)
gat opf socp grid.arrow --out solution.parquet

# Tier 4: AC-OPF (~1s for 118-bus)
gat opf ac grid.arrow --out optimal.parquet
```

### Economic Dispatch (`gat opf ed`)

Merit-order dispatch ignoring network constraints. Fastest option for "can we meet demand?" checks.

```bash
gat opf ed grid.arrow --out dispatch.parquet
```

**Output:** Generator dispatch (P) ordered by marginal cost.

### DC-OPF (`gat opf dc`)

Linear approximation with B-matrix flow constraints. Standard for transmission planning.

```bash
gat opf dc grid.arrow \
  --out results/dc-opf.parquet \
  [--branch-limits limits.csv]
```

**Features:**
- Real power flow on branches
- Generator cost minimization
- Optional branch flow limits
- LMP extraction from duals

**Output:** Branch flows, generator dispatch, bus LMPs.

### SOCP Relaxation (`gat opf socp`)

Second-order cone relaxation with voltage magnitude modeling. Production-ready for most applications.

```bash
gat opf socp grid.arrow \
  --out results/socp.parquet \
  [--tol 1e-6]
```

**Features:**
- Squared voltage/current variables
- Quadratic generator costs
- Transformer tap ratios and phase shifters
- Thermal limits (apparent power)
- LMP extraction from duals

**Backend:** Clarabel (pure Rust, no external dependencies)

**Output:** Branch flows, voltages, generator dispatch, LMPs.

### AC-OPF (`gat opf ac`)

Full nonlinear AC-OPF with polar formulation. Maximum accuracy for final validation.

```bash
gat opf ac grid.arrow \
  --out results/ac-opf.parquet \
  [--tol 1e-4] \
  [--max-iter 200] \
  [--warm-start socp]
```

**Options:**
- `--tol`: convergence tolerance (default `1e-4`)
- `--max-iter`: maximum iterations (default `200`)
- `--warm-start`: initialization — `flat`, `dc`, or `socp` (recommended)

**Backend:** IPOPT with analytical Jacobian and Hessian (if installed), otherwise L-BFGS

**Output:** Full voltage profile (V, θ), generator P/Q, branch flows, losses, LMPs.

## Test Fixtures

`test_data/opf` provides reusable CSVs for local experiments:

* `costs.csv`: sample marginal costs for buses `0` and `1`.
* `limits.csv`: matching `pmin`, `pmax`, and `demand` entries.
* `branch_limits.csv`: tight limits for violation testing.
* `piecewise.csv`: two-piece segments for piecewise cost testing.

## Related Documentation

* **Benchmarks**: [Complete PGLib-OPF validation results](@/internals/benchmarks.md) — 67 cases, 100% convergence
* **Power Flow**: [Power Flow Guide](@/guide/pf.md)
* **State Estimation**: [State Estimation Guide](@/guide/se.md)
* **Reliability**: [Reliability Guide](@/guide/reliability.md)
* **Solver Architecture**: [Native Solver Plugin System](@/internals/solver-architecture.md)
