# Transmission Expansion Planning (TEP)

The `gat-algo` crate provides a Transmission Expansion Planning (TEP) solver that determines which candidate transmission lines to build to minimize total system cost while meeting demand.

## Overview

TEP is a classic power system planning problem:

```
Given:
  - Existing network with generators and loads
  - Set of candidate transmission lines with investment costs
  - Operating scenarios (load levels, generation availability)

Decide:
  - Which candidate lines to build (binary decisions)
  - Generator dispatch (continuous)

Minimize:
  Total cost = Investment cost + Operating cost

Subject to:
  - Power balance at each bus
  - Generator capacity limits
  - Branch flow limits (existing and candidates)
  - DC power flow physics
```

## MILP Formulation

GAT implements a DC-based Mixed-Integer Linear Programming formulation:

```
minimize    Σ_k c_k·x_k + Σ_g c_g·P_g
            └──────────┘   └─────────┘
            investment     operating

subject to:
  Σ P_gen(i) - Σ P_load(i) = Σ_j P_ij      (power balance)
  P_ij = b_ij · (θ_i - θ_j)                (DC power flow)
  -M(1-x_k) ≤ P_k - b_k·Δθ ≤ M(1-x_k)     (Big-M disjunctive)
  |P_ij| ≤ P_ij^max                        (flow limits)
  |P_k| ≤ P_k^max · x_k                    (candidate limits)
  x_k ∈ {0,1}                              (binary decisions)
```

### Big-M Constraints

The key modeling challenge is that candidate line flow should:
- Follow physics (`P = b·Δθ`) when the line is **built** (x=1)
- Be zero when the line is **not built** (x=0)

Big-M formulation achieves this:
- When x=1: `-M·0 ≤ P - b·Δθ ≤ M·0` → `P = b·Δθ` (physics enforced)
- When x=0: `-M ≤ P - b·Δθ ≤ M` (relaxed, but P=0 from flow limit)

## API Usage

### Creating a TEP Problem

```rust
use gat_algo::{TepProblemBuilder, solve_tep, TepSolverConfig};
use gat_core::{Network, BusId};

// Load or create base network
let network = load_network("case14.arrow")?;

// Build TEP problem with candidate lines
let problem = TepProblemBuilder::new(network)
    .big_m(10000.0)  // Big-M value for disjunctive constraints
    .planning_params(
        8760.0,      // Operating hours per year
        0.10,        // Discount rate (10%)
        10,          // Planning horizon (years)
    )
    // Add candidate lines: (name, from, to, reactance, capacity, cost)
    .candidate("Line 1-2", BusId::new(1), BusId::new(2), 0.10, 100.0, 40_000_000.0)
    .candidate("Line 2-3", BusId::new(2), BusId::new(3), 0.15, 80.0, 30_000_000.0)
    .candidate("Line 3-4", BusId::new(3), BusId::new(4), 0.08, 150.0, 50_000_000.0)
    .build();
```

### Solving the Problem

```rust
let config = TepSolverConfig::default();
let solution = solve_tep(&problem, &config)?;

println!("{}", solution.summary());
```

### Solution Output

```
TEP Solution Summary
========================================
Status: Optimal
Total Cost: $157,372,812.51
  Investment: $15,460,812.51
  Operating: $141,912,000.00
Lines Built: 3 (3 circuits)
Total Generation: 760.00 MW
Solve Time: 1.43ms

Build Decisions:
  [SKIP]  Line 1-2
  [BUILD] Line 2-3 (x1) - $30,000,000.00
  [BUILD] Line 3-4 (x1) - $50,000,000.00
  ...
```

## Data Structures

### CandidateLine

```rust
pub struct CandidateLine {
    pub id: CandidateId,
    pub name: String,
    pub from_bus: BusId,
    pub to_bus: BusId,
    pub reactance_pu: f64,      // Per-unit reactance
    pub capacity_mw: f64,       // Flow capacity (MW)
    pub investment_cost: f64,   // Capital cost ($)
    pub max_circuits: Option<usize>,  // Optional parallel circuits
}
```

### TepProblem

```rust
pub struct TepProblem {
    pub network: Network,       // Base network
    pub candidates: Vec<CandidateLine>,
    pub base_mva: f64,         // System base (100 MVA default)
    pub big_m: f64,            // Big-M value (10,000 default)
    pub operating_hours: f64,  // Annual hours (8760 default)
    pub discount_rate: f64,    // For CRF calculation
    pub planning_years: usize, // Planning horizon
}
```

### TepSolution

```rust
pub struct TepSolution {
    pub optimal: bool,
    pub total_cost: f64,
    pub investment_cost: f64,
    pub operating_cost: f64,
    pub build_decisions: Vec<LineBuildDecision>,
    pub generator_dispatch: HashMap<String, f64>,
    pub bus_angles: HashMap<String, f64>,
    pub solve_time: Duration,
}
```

## Investment Cost Annualization

Investment costs are annualized using the Capital Recovery Factor (CRF):

```
CRF = r(1+r)^n / ((1+r)^n - 1)

where:
  r = discount rate (e.g., 0.10 for 10%)
  n = planning years (e.g., 10)
```

For 10% discount rate over 10 years: CRF ≈ 0.1627

This means a $1M investment costs ~$162,745/year when annualized.

## Solver Implementation

The current solver uses **LP relaxation**:
- Binary variables x_k ∈ {0,1} are relaxed to continuous x_k ∈ [0,1]
- Often yields integer solutions for well-structured problems
- Falls back to rounding heuristics when needed

**Solver backend**: Clarabel (interior-point convex optimizer)

## Example: Garver 6-Bus System

The classic Garver (1970) benchmark:

```rust
// 6 buses, 3 generators, 5 loads
// ~760 MW total load
// Generator costs: $10, $20, $30 per MWh

let problem = TepProblemBuilder::new(garver_network)
    .candidate("Line 1-2", bus(1), bus(2), 0.10, 200.0, 40e6)
    .candidate("Line 2-3", bus(2), bus(3), 0.10, 200.0, 40e6)
    .candidate("Line 3-5", bus(3), bus(5), 0.05, 300.0, 20e6)
    .candidate("Line 3-6", bus(3), bus(6), 0.05, 300.0, 30e6)
    .candidate("Line 4-6", bus(4), bus(6), 0.10, 250.0, 35e6)
    // ... more candidates
    .build();

let solution = solve_tep(&problem, &TepSolverConfig::default())?;
// Optimal: builds 3 lines for $157M total cost
```

## References

- **Garver (1970)**: "Transmission Network Estimation Using Linear Programming" - Classic disjunctive formulation with 6-bus benchmark
- **Romero et al. (2002)**: "Analysis of heuristic algorithms for transmission network expansion planning" - MILP vs. heuristics comparison
- **Alguacil et al. (2003)**: "Transmission network expansion planning: A mixed-integer LP approach" - Modern MILP techniques

## Future Enhancements

- True MILP solver (branch-and-bound for guaranteed integer solutions)
- Multi-scenario planning (uncertainty in load/generation)
- N-1 security constraints
- Multi-year staging (sequential investment decisions)
- AC power flow constraints (voltage, reactive power)
