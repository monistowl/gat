# Optimal Power Flow (OPF)

This reference describes the OPF solver architecture, solution methods, and CLI commands.

## Architecture Overview (v0.4.0)

GAT provides a unified `OpfSolver` supporting multiple solution methods with varying accuracy/speed tradeoffs:

| Method | Accuracy | Speed | Use Case | Status |
|--------|----------|-------|----------|--------|
| `EconomicDispatch` | ~20% gap | Fastest | Quick estimates, screening | ✅ Implemented |
| `DcOpf` | ~3-5% gap | Fast | Planning studies | ✅ Implemented |
| `SocpRelaxation` | ~1-3% gap | Moderate | Research, convex lower bounds | ✅ Implemented |
| `AcOpf` | <1% gap | Slowest | High-fidelity analysis | ✅ **Implemented** |

**Current Status:** All four methods are fully implemented. The full nonlinear AC-OPF solver passes 65/68 PGLib benchmark cases with a median 2.9% objective gap.

---

## Full Nonlinear AC-OPF (AcOpf)

The crown jewel of GAT's optimization suite is the **full-space nonlinear AC-OPF solver**. Unlike convex relaxations (SOCP, SDP), this solver handles the exact nonlinear AC power flow equations to find optimal generator dispatch while respecting all physical constraints.

### Why AC-OPF Matters

The AC Optimal Power Flow problem answers: *"Given a network and loads, what's the cheapest way to dispatch generators while respecting all physical constraints?"*

Real-world applications include:

- **Day-ahead markets**: Setting generator schedules and locational marginal prices (LMPs)
- **Real-time dispatch**: 5-minute economic adjustments
- **Planning studies**: Transmission expansion, renewable integration
- **Voltage/VAR optimization**: Minimizing losses in distribution networks

### Mathematical Formulation

The AC-OPF is a **nonlinear program (NLP)** in polar coordinates:

```
┌─────────────────────────────────────────────────────────────────────────┐
│  DECISION VARIABLES                                                      │
│  ─────────────────                                                       │
│  V_i ∈ [V_min, V_max]     Voltage magnitude at bus i (p.u.)             │
│  θ_i ∈ [-π/2, π/2]        Voltage angle at bus i (radians)              │
│  P_g ∈ [P_min, P_max]     Real power output of generator g (MW)         │
│  Q_g ∈ [Q_min, Q_max]     Reactive power output of generator g (MVAr)   │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  OBJECTIVE: Minimize total generation cost                               │
│  ─────────                                                               │
│  min  Σ_g [ c₀_g + c₁_g · P_g + c₂_g · P_g² ]                           │
│                                                                          │
│  where c₀, c₁, c₂ are polynomial cost coefficients ($/hr, $/MWh, $/MW²h)│
│                                                                          │
│  For thermal generators, this models the heat-rate curve:                │
│    - c₀: No-load cost (fuel burned at minimum stable output)            │
│    - c₁: Incremental cost (marginal fuel per MW)                        │
│    - c₂: Curvature (efficiency decreases at high/low output)            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  EQUALITY CONSTRAINTS: Power Balance (Kirchhoff's Laws)                  │
│  ────────────────────                                                    │
│                                                                          │
│  At each bus i, power in = power out:                                    │
│                                                                          │
│  P_i^inj = P_i^gen - P_i^load = Σⱼ V_i V_j [G_ij cos(θ_ij) + B_ij sin(θ_ij)]
│  Q_i^inj = Q_i^gen - Q_i^load = Σⱼ V_i V_j [G_ij sin(θ_ij) - B_ij cos(θ_ij)]
│                                                                          │
│  where θ_ij = θ_i - θ_j, and G_ij + jB_ij = Y_ij (admittance matrix)    │
│                                                                          │
│  Reference bus: θ_ref = 0 (arbitrary angle reference)                    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  INEQUALITY CONSTRAINTS: Physical Limits                                 │
│  ──────────────────────                                                  │
│                                                                          │
│  Voltage limits:     V_min ≤ V_i ≤ V_max    (equipment protection)      │
│  Generator P limits: P_min ≤ P_g ≤ P_max    (capability curve)          │
│  Generator Q limits: Q_min ≤ Q_g ≤ Q_max    (field heating limits)      │
│  Thermal limits:     S_ij ≤ S_max           (conductor/transformer)     │
│                                                                          │
│  where S_ij = √(P_ij² + Q_ij²) is apparent power flow                   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Why Is AC-OPF Hard?

The power flow equations create a **non-convex** feasible region:

1. **Bilinear terms**: V_i · V_j · cos(θ_ij) couples voltage and angle
2. **Trigonometric functions**: sin/cos create multiple local minima
3. **Product of variables**: P_ij² + Q_ij² ≤ S_max² is non-convex

This makes AC-OPF **NP-hard** in general. No known algorithm guarantees a global optimum in polynomial time.

### Solver Backends

GAT provides a **native solver plugin system** that automatically selects the best available backend:

| Backend | Type | Best For | Availability |
|---------|------|----------|--------------|
| L-BFGS (default) | Pure Rust | General AC-OPF, portability | Always available |
| Clarabel | Pure Rust | SOCP, LP problems | Always available |
| IPOPT | Native (C++) | Large NLP, high accuracy | Optional installation |
| HiGHS | Native (C++) | LP/MIP, high performance | Optional installation |
| CBC | Native (C) | MIP problems | Optional installation |

**Solver Selection:**
```rust
use gat_algo::opf::{SolverDispatcher, DispatchConfig, ProblemClass};

// Configure dispatcher (native solvers enabled if installed)
let config = DispatchConfig {
    native_enabled: true,
    ..Default::default()
};

let dispatcher = SolverDispatcher::with_config(config);

// Automatic selection based on problem class
let solver = dispatcher.select(ProblemClass::NonlinearProgram)?;
// Returns IPOPT if installed, otherwise L-BFGS
```

**Installing Native Solvers:**
```bash
# Build and install IPOPT wrapper
cargo xtask solver build ipopt --install

# List installed solvers
gat solver list

# Uninstall a solver
gat solver uninstall ipopt
```

Native solvers run as **isolated subprocesses** communicating via Arrow IPC, ensuring:
- Crash isolation (native library issues don't crash the main process)
- Version flexibility (different solver versions can coexist)
- Portability (pure-Rust fallbacks always available)

### Solver Architecture

GAT's AC-OPF uses a **penalty method with L-BFGS** quasi-Newton optimization:

```
┌─────────────────────────────────────────────────────────────────────────┐
│  PENALTY METHOD                                                          │
│  ──────────────                                                          │
│                                                                          │
│  Convert constrained NLP to unconstrained:                               │
│                                                                          │
│  min  f(x) + μ · Σ g_i(x)²  +  μ · Σ max(0, h_j(x))²                    │
│       ├───┘   └──────────┘      └───────────────────┘                    │
│       original  equality         inequality constraint                   │
│       objective penalty          penalty                                 │
│                                                                          │
│  The penalty parameter μ starts small and increases iteratively until    │
│  constraints are satisfied within tolerance.                             │
└─────────────────────────────────────────────────────────────────────────┘
```

**Advantages:**
- **Simplicity**: No need for barrier functions or constraint Jacobians
- **Robustness**: Works even when starting far from feasible region
- **Scalability**: L-BFGS has O(n) memory and O(n²) per-iteration cost

**Optional IPOPT Backend:**

For faster convergence on large networks, GAT supports the IPOPT (Interior Point OPTimizer) backend with analytical Hessians:

```rust
// Enable with feature flag
#[cfg(feature = "solver-ipopt")]
use gat_algo::opf::ac_nlp::solve_with_ipopt;
```

IPOPT provides:
- **Second-order Newton convergence** (vs first-order L-BFGS)
- **Better constraint handling** via barrier methods
- **Sparse linear algebra** efficient for large networks

### Supported Features

| Feature | Status | Notes |
|---------|--------|-------|
| Quadratic cost curves | ✅ | `c₀ + c₁·P + c₂·P²` |
| Piecewise-linear costs | ✅ | Native support with breakpoints |
| Polynomial costs | ✅ | Up to degree 3 |
| Voltage magnitude bounds | ✅ | Per-bus V_min/V_max |
| Generator P limits | ✅ | P_min ≤ P_g ≤ P_max |
| Generator Q limits | ✅ | Q_min ≤ Q_g ≤ Q_max |
| Capability curves | ✅ | Q limits as function of P |
| Thermal limits | ✅ | Branch MVA ratings |
| Tap-changing transformers | ✅ | Off-nominal tap ratios |
| Phase-shifting transformers | ✅ | Phase angle coupling |
| Line charging (π-model) | ✅ | Shunt susceptance |
| Warm-start (DC/SOCP) | ✅ | Faster convergence |
| Multi-period dispatch | ✅ | Ramp constraints |
| LMP estimation | ✅ | From marginal generators |
| IPOPT backend | ✅ | With analytical Hessian |

### Performance on PGLib Benchmarks

Tested on the industry-standard PGLib-OPF test suite (v23.07):

| Metric | Result |
|--------|--------|
| Cases tested | 68 |
| Convergence rate | 95.6% (65/68) |
| Cases with <5% gap | 76% (48/68) |
| Median objective gap | 2.91% |
| Network sizes | 14 - 13,659 buses |

The three non-converging cases are large stressed networks (3000+ buses) that require more sophisticated initialization.

### CLI Usage

```bash
# Basic usage - full nonlinear AC-OPF
gat opf ac-nlp grid.arrow -o result.json

# With options
gat opf ac-nlp grid.arrow \
  -o result.json \
  --tol 1e-4 \
  --max-iter 200 \
  --warm-start dc
```

**Options:**
- `--tol`: Convergence tolerance (default: 1e-4)
- `--max-iter`: Maximum iterations (default: 200)
- `--warm-start`: Initialization method: `flat`, `dc`, `socp` (default: `flat`)

**Output:**

The JSON output includes:
```json
{
  "converged": true,
  "method_used": "AcOpf",
  "iterations": 47,
  "objective_value": 5296.68,
  "generator_p": { "Gen1": 89.3, "Gen2": 163.4 },
  "generator_q": { "Gen1": 12.8, "Gen2": -4.1 },
  "bus_voltage_mag": { "Bus1": 1.04, "Bus2": 0.98 },
  "bus_voltage_ang": { "Bus1": 0.0, "Bus2": -0.07 },
  "bus_lmp": { "Bus1": 35.2, "Bus2": 36.8 }
}
```

### Rust API

```rust
use gat_algo::opf::ac_nlp::{AcOpfProblem, solve_ac_opf};
use gat_core::Network;

// Build problem from network
let problem = AcOpfProblem::from_network(&network)?;

// Solve with penalty method + L-BFGS
let solution = solve_ac_opf(&problem, 200, 1e-4)?;

if solution.converged {
    println!("Total cost: ${:.2}/hr", solution.objective_value);

    for (gen, mw) in &solution.generator_p {
        let mvar = solution.generator_q.get(gen).unwrap_or(&0.0);
        println!("  {}: {:.1} MW, {:.1} MVAr", gen, mw, mvar);
    }
}
```

### Warm-Start from DC/SOCP

For faster convergence, initialize from a convex relaxation:

```rust
use gat_algo::opf::ac_nlp::{AcOpfProblem, solve_ac_opf_warm_start};
use gat_algo::opf::OpfSolution;

// Solve DC-OPF first
let dc_solution: OpfSolution = dc_opf_solver.solve(&network)?;

// Warm-start AC-OPF from DC solution
let problem = AcOpfProblem::from_network(&network)?;
let ac_solution = solve_ac_opf_warm_start(&problem, 200, 1e-4, Some(&dc_solution))?;
```

### Multi-Period Dispatch

For day-ahead scheduling with ramp constraints:

```rust
use gat_algo::opf::ac_nlp::{
    AcOpfProblem, MultiPeriodProblem, PeriodData, RampConstraint,
    solve_multiperiod_sequential,
};

// Define time periods with load profiles
let periods = vec![
    PeriodData::new(Duration::from_secs(3600), 0.85),  // Off-peak
    PeriodData::new(Duration::from_secs(3600), 1.00),  // Peak
    PeriodData::new(Duration::from_secs(3600), 0.90),  // Evening
];

// Generator ramp constraints (MW/hr)
let ramp_constraints = vec![
    RampConstraint::symmetric("Gen1", 50.0),  // ±50 MW/hr
    RampConstraint::new("Gen2", 30.0, 40.0),  // +30/-40 MW/hr
];

// Create and solve multi-period problem
let base_problem = AcOpfProblem::from_network(&network)?;
let mp_problem = MultiPeriodProblem::new(base_problem, periods, ramp_constraints);
let mp_solution = solve_multiperiod_sequential(&mp_problem, 200, 1e-4)?;

// Access results per period
for (i, sol) in mp_solution.period_solutions.iter().enumerate() {
    println!("Period {}: ${:.2}/hr", i, sol.objective_value);
}
```

---

## SOCP Relaxation

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

### Solver Backend

SOCP uses [Clarabel](https://github.com/oxfordcontrol/Clarabel.rs), a high-performance interior-point solver for conic programs. Typical convergence is 15-30 iterations.

---

## DC OPF

The DC approximation linearizes power flow equations by assuming:
- Flat voltage profile (|V| = 1.0 p.u.)
- Small angle differences (sin θ ≈ θ, cos θ ≈ 1)
- Lossless lines (R << X)

This yields a **linear program** solvable in polynomial time with guaranteed global optimum.

### CLI Usage

```bash
gat opf dc grid.arrow \
  --cost costs.csv \
  --limits limits.csv \
  --out dispatch.parquet \
  [--branch-limits branch_limits.csv] \
  [--piecewise piecewise.csv]
```

**Inputs:**
- `--cost`: CSV with `bus_id,marginal_cost`
- `--limits`: CSV with `bus_id,pmin,pmax,demand`
- `--branch-limits` (optional): CSV with `branch_id,flow_limit`
- `--piecewise` (optional): CSV with `bus_id,start,end,slope`

**Output:**
- Parquet table with `branch_id`, `from_bus`, `to_bus`, `flow_mw`

---

## Generator Cost Models

Generators support polynomial and piecewise-linear cost functions via the `CostModel` enum:

```rust
use gat_core::{Gen, GenId, BusId, CostModel};

// Quadratic cost: $100 + $20/MWh + $0.01/MW²h
let gen = Gen::new(GenId::new(0), "Gen1".into(), BusId::new(0))
    .with_p_limits(10.0, 100.0)
    .with_q_limits(-50.0, 50.0)
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

---

## Rust API Reference

### OpfSolver

```rust
use gat_algo::{OpfSolver, OpfMethod, OpfSolution, OpfError};
use gat_core::Network;

// Create solver with method selection
let solver = OpfSolver::new()
    .with_method(OpfMethod::AcOpf)
    .with_tolerance(1e-4)
    .with_max_iterations(200);

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
    /// Full nonlinear AC-OPF (penalty method + L-BFGS)
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

---

## Key References

- **Carpentier (1962)**: Original OPF formulation
  "Contribution à l'étude du dispatching économique"

- **Dommel & Tinney (1968)**: Newton-based OPF
  [DOI:10.1109/TPAS.1968.292150](https://doi.org/10.1109/TPAS.1968.292150)

- **Cain, O'Neill & Castillo (2012)**: Comprehensive survey
  "History of Optimal Power Flow and Formulations", FERC Technical Conference

- **Liu & Nocedal (1989)**: L-BFGS algorithm
  [DOI:10.1007/BF01589116](https://doi.org/10.1007/BF01589116)

- **Farivar & Low (2013)**: SOCP relaxation
  [DOI:10.1109/TPWRS.2013.2255317](https://doi.org/10.1109/TPWRS.2013.2255317)

---

## Related Documentation

- Power flow: `docs/guide/pf.md`
- State estimation: `docs/guide/se.md`
- Benchmarking: `docs/guide/benchmark.md`
- Reliability: `docs/guide/reliability.md`
