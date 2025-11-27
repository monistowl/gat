+++
title = "OPF Formulations"
description = "Mathematical formulations of DC-OPF, SOCP, and AC-OPF"
weight = 6
+++

# OPF Formulations

This reference presents the mathematical formulations of Optimal Power Flow (OPF) variants implemented in GAT, from simplest to most accurate.

---

## The OPF Problem

Optimal Power Flow minimizes generation cost while satisfying physical and operational constraints:

```
minimize    Total generation cost
subject to  Power flow equations (physics)
            Generator limits (equipment)
            Voltage limits (equipment protection)
            Thermal limits (line ratings)
```

The challenge: power flow equations are **nonlinear and non-convex**, making global optimization difficult.

---

## Economic Dispatch (Copper Plate)

The simplest "OPF" ignores the network entirely.

### Formulation

```
minimize    Σᵢ Cᵢ(Pᵢ)

subject to  Σᵢ Pᵢ = Σⱼ Dⱼ           (power balance)
            Pᵢ_min ≤ Pᵢ ≤ Pᵢ_max    (generator limits)
```

where:
- Cᵢ(Pᵢ) is the cost function for generator i
- Dⱼ is demand at bus j

### Solution

For quadratic costs Cᵢ(P) = aᵢ + bᵢP + cᵢP²:

At optimum, all generators have equal marginal cost (if unconstrained):

```
dCᵢ/dPᵢ = bᵢ + 2cᵢPᵢ = λ  (for all i not at limits)
```

where λ is the system marginal price.

### Limitations

- No network constraints → can dispatch infeasible solutions
- No losses → underestimates cost
- Single price → no locational pricing (LMP)

### GAT Implementation

```rust
OpfSolver::new().with_method(OpfMethod::EconomicDispatch)
```

---

## DC-OPF (Linear Program)

DC-OPF adds linearized network constraints to economic dispatch.

### Assumptions

Same as DC power flow:
1. Voltage magnitudes = 1.0 p.u.
2. Small angle differences: sin(θ) ≈ θ
3. Lossless lines: R = 0

### Formulation

**Variables:**
- Pᵢ: generator output at bus i
- θᵢ: voltage angle at bus i

**Objective:**
```
minimize    Σᵢ (aᵢ + bᵢPᵢ + cᵢPᵢ²)
```

Note: Quadratic objectives make this a QP, not LP. For true LP, use piecewise-linear cost approximation.

**Constraints:**

Power balance at each bus:
```
Pᵢ_gen - Pᵢ_load = Σⱼ Bᵢⱼ(θᵢ - θⱼ)    ∀i
```

Generator limits:
```
Pᵢ_min ≤ Pᵢ ≤ Pᵢ_max    ∀ generators
```

Branch flow limits:
```
|Pᵢⱼ| = |(θᵢ - θⱼ)/Xᵢⱼ| ≤ Pᵢⱼ_max    ∀ branches
```

Reference angle:
```
θ_ref = 0
```

### Matrix Form

The power balance constraint in matrix form:

```
P_gen - P_load = B · θ
```

where B is the susceptance matrix (imaginary part of Y-bus).

### Locational Marginal Prices (LMPs)

The dual variables of power balance constraints give LMPs:

```
LMPᵢ = λᵢ = ∂(Total Cost)/∂(Demand at bus i)
```

LMP components:
- **Energy**: System marginal cost (λ)
- **Congestion**: Shadow price of binding flow limits
- **Losses**: Zero in DC-OPF (lossless assumption)

### GAT Implementation

```bash
gat opf dc network.arrow --out results.parquet
```

Uses sparse LP/QP solver. Complexity: O(n) to O(n^1.5) depending on network structure.

---

## SOCP Relaxation (Convex)

Second-Order Cone Programming relaxes the AC-OPF into a convex problem.

### Branch Flow Model

Instead of bus injection formulation, SOCP uses branch flows as variables:

**Variables:**
- wᵢ = |Vᵢ|²: squared voltage magnitude
- ℓᵢⱼ = |Iᵢⱼ|²: squared current magnitude
- Pᵢⱼ, Qᵢⱼ: branch power flows
- Pᵢ_gen, Qᵢ_gen: generator outputs

### Formulation

**Objective:**
```
minimize    Σᵢ (aᵢ + bᵢPᵢ + cᵢPᵢ²)
```

**Branch flow equations** (from bus i to bus j):
```
wⱼ = wᵢ - 2(rᵢⱼPᵢⱼ + xᵢⱼQᵢⱼ) + (rᵢⱼ² + xᵢⱼ²)ℓᵢⱼ
```

**Power balance at each bus:**
```
Pᵢ_gen - Pᵢ_load = Σⱼ Pᵢⱼ - Σₖ (Pₖᵢ - rₖᵢℓₖᵢ)
Qᵢ_gen - Qᵢ_load = Σⱼ Qᵢⱼ - Σₖ (Qₖᵢ - xₖᵢℓₖᵢ)
```

**The SOC constraint** (relaxed from equality):
```
Pᵢⱼ² + Qᵢⱼ² ≤ wᵢ · ℓᵢⱼ
```

This is a **second-order cone** constraint:
```
||(Pᵢⱼ, Qᵢⱼ, (wᵢ - ℓᵢⱼ)/2)||₂ ≤ (wᵢ + ℓᵢⱼ)/2
```

**Voltage limits:**
```
Vᵢ_min² ≤ wᵢ ≤ Vᵢ_max²
```

**Thermal limits:**
```
Pᵢⱼ² + Qᵢⱼ² ≤ Sᵢⱼ_max²
```

**Generator limits:**
```
Pᵢ_min ≤ Pᵢ_gen ≤ Pᵢ_max
Qᵢ_min ≤ Qᵢ_gen ≤ Qᵢ_max
```

### Why is SOCP a Relaxation?

The exact AC power flow requires:
```
Pᵢⱼ² + Qᵢⱼ² = wᵢ · ℓᵢⱼ    (equality)
```

SOCP relaxes this to:
```
Pᵢⱼ² + Qᵢⱼ² ≤ wᵢ · ℓᵢⱼ    (inequality)
```

This makes the feasible region **convex**, enabling global optimization.

### Exactness Conditions

The SOCP relaxation is **exact** (inequality is tight at optimum) when:
- Network is radial (tree topology)
- Voltage upper bounds are not binding
- Objective is strictly increasing in power injection

For meshed networks, the relaxation may be loose (gap between SOCP and true AC solution).

### GAT Implementation

```rust
OpfSolver::new().with_method(OpfMethod::SocpRelaxation)
```

Uses Clarabel interior-point solver. Typical convergence: 15-30 iterations.

### References

- Farivar & Low (2013), "Branch Flow Model: Relaxations and Convexification"
- Gan, Li, Topcu & Low (2015), "Exact Convex Relaxation of OPF for Radial Networks"

---

## AC-OPF (Full Nonlinear)

The exact AC-OPF is a nonlinear, non-convex optimization problem.

### Variables

- Vᵢ: voltage magnitude at bus i (p.u.)
- θᵢ: voltage angle at bus i (radians)
- Pᵢ_gen, Qᵢ_gen: generator real and reactive power

### Formulation

**Objective:**
```
minimize    Σᵢ (aᵢ + bᵢPᵢ + cᵢPᵢ²)
```

**Power balance (equality constraints):**
```
Pᵢ_gen - Pᵢ_load = Σⱼ VᵢVⱼ(Gᵢⱼcos(θᵢⱼ) + Bᵢⱼsin(θᵢⱼ))
Qᵢ_gen - Qᵢ_load = Σⱼ VᵢVⱼ(Gᵢⱼsin(θᵢⱼ) - Bᵢⱼcos(θᵢⱼ))
```

where θᵢⱼ = θᵢ - θⱼ and Gᵢⱼ + jBᵢⱼ = Yᵢⱼ.

**Voltage limits:**
```
Vᵢ_min ≤ Vᵢ ≤ Vᵢ_max
```

**Generator limits:**
```
Pᵢ_min ≤ Pᵢ_gen ≤ Pᵢ_max
Qᵢ_min ≤ Qᵢ_gen ≤ Qᵢ_max
```

**Thermal limits:**
```
Pᵢⱼ² + Qᵢⱼ² ≤ Sᵢⱼ_max²
```

where branch flows are:
```
Pᵢⱼ = Vᵢ²Gᵢⱼ - VᵢVⱼ(Gᵢⱼcos(θᵢⱼ) + Bᵢⱼsin(θᵢⱼ))
Qᵢⱼ = -Vᵢ²Bᵢⱼ - VᵢVⱼ(Gᵢⱼsin(θᵢⱼ) - Bᵢⱼcos(θᵢⱼ))
```

**Reference angle:**
```
θ_ref = 0
```

### Solution Methods

AC-OPF is **NP-hard** in general. No polynomial-time algorithm guarantees the global optimum. Practical approaches:

#### Penalty Method (GAT Default)

Convert constrained problem to unconstrained via penalty:

```
minimize f(x) + μ·||g(x)||² + μ·||max(0, h(x))||²
```

where:
- f(x) is the objective
- g(x) = 0 are equality constraints (power balance)
- h(x) ≤ 0 are inequality constraints

Algorithm:
1. Start with small μ
2. Minimize penalized objective using L-BFGS
3. If constraints violated, increase μ and repeat
4. Stop when constraints satisfied within tolerance

**Pros:** Simple, handles infeasible starts
**Cons:** First-order convergence, ill-conditioning at large μ

#### Interior Point Method (IPOPT)

Barrier method staying strictly inside inequality constraints:

```
minimize f(x) - μ·Σ log(-hⱼ(x))
subject to g(x) = 0
```

Uses Newton steps with:
- Gradient of Lagrangian
- Hessian of Lagrangian (requires second derivatives)

**Pros:** Superlinear convergence, handles many constraints efficiently
**Cons:** Requires Hessian computation, needs feasible start

GAT provides IPOPT backend via the `solver-ipopt` feature.

### KKT Conditions

At a local optimum, the Karush-Kuhn-Tucker conditions hold:

```
∇f(x*) + Σλᵢ∇gᵢ(x*) + Σμⱼ∇hⱼ(x*) = 0   (stationarity)
gᵢ(x*) = 0                               (primal feasibility)
hⱼ(x*) ≤ 0                               (primal feasibility)
μⱼ ≥ 0                                   (dual feasibility)
μⱼhⱼ(x*) = 0                             (complementarity)
```

The dual variables λᵢ and μⱼ provide sensitivity information and LMPs.

### GAT Implementation

**L-BFGS Penalty Method:**
```bash
gat opf ac-nlp network.arrow --out results.json
```

**IPOPT (if compiled with feature):**
```rust
// Requires solver-ipopt feature
solve_with_ipopt(&problem, max_iter, tolerance)
```

Key source files:
- `problem.rs`: Problem construction
- `solver.rs`: L-BFGS penalty loop
- `ipopt_solver.rs`: IPOPT interface
- `hessian.rs`: Second derivatives for IPOPT

---

## Comparison of Methods

| Method | Accuracy | Speed | Guarantees | Use Case |
|--------|----------|-------|------------|----------|
| Economic Dispatch | ~20% gap | Fastest | Global (convex) | Screening |
| DC-OPF | 3-5% gap | Fast | Global (LP/QP) | Planning |
| SOCP | 1-3% gap | Moderate | Global (convex) | Research |
| AC-OPF | Exact | Slowest | Local only | Operations |

### Accuracy vs. Problem Size

For the PGLib-OPF benchmark suite:

| Method | Median Gap | 95th Percentile |
|--------|------------|-----------------|
| DC-OPF | 4.2% | 12% |
| SOCP | 1.8% | 8% |
| AC-OPF (L-BFGS) | 2.9% | 15% |
| AC-OPF (IPOPT) | 0.5% | 3% |

*Gap = (method cost - best known) / best known × 100%*

---

## Cost Function Details

### Quadratic Cost Model

```
C(P) = c₀ + c₁·P + c₂·P²
```

- c₀: No-load cost ($/hr) — fuel burned at minimum output
- c₁: Linear cost ($/MWh) — marginal fuel cost
- c₂: Quadratic cost ($/MW²h) — efficiency curve

Marginal cost: dC/dP = c₁ + 2c₂P

### Piecewise Linear Cost

```
C(P) = Σₖ mₖ·max(0, P - Pₖ)
```

Linear segments approximate any convex cost curve. Requires additional variables in LP formulation.

### Multi-Segment Representation in GAT

```rust
CostModel::Polynomial { coefficients: vec![c0, c1, c2] }
CostModel::PiecewiseLinear(vec![(P1, C1), (P2, C2), ...])
```

---

## Constraint Handling

### Active vs. Inactive Constraints

At optimum:
- **Active (binding)**: Constraint is exactly satisfied (equality)
- **Inactive**: Constraint has slack (strict inequality)

Only active constraints affect the solution. Dual variables (shadow prices) are non-zero only for active constraints.

### Soft Constraints via Penalty

Some constraints can be relaxed with penalty:

```
minimize f(x) + ρ·(violation)²
```

Useful for:
- Detecting infeasibility
- Trading off constraint satisfaction vs. cost
- Load shedding in emergency scenarios

### GAT Constraint Implementation

| Constraint | Hard/Soft | Location |
|------------|-----------|----------|
| Power balance | Hard (penalty) | `solver.rs` |
| Voltage limits | Soft (bounds) | `problem.rs` |
| Generator limits | Hard (bounds) | `problem.rs` |
| Thermal limits | Hard (penalty) | `branch_flow.rs` |

---

## Warm Starting

Using a previous solution accelerates convergence:

```rust
solve_ac_opf_warm_start(&problem, &initial_solution, max_iter, tol)
```

Effective for:
- Contingency analysis (N-1 cases differ slightly)
- Time-series studies (sequential hours)
- Sensitivity analysis (parameter sweeps)

Typical speedup: 2-5x fewer iterations.

---

## Further Reading

### Foundational Papers

- **Carpentier (1962)**: Original OPF formulation
- **Dommel & Tinney (1968)**: Newton-based OPF solution

### Convex Relaxations

- **Jabr (2006)**: Radial network convexification
- **Farivar & Low (2013)**: Branch flow SOCP relaxation
- **Lavaei & Low (2012)**: SDP relaxation and exactness

### Software

- **MATPOWER**: Reference OPF implementation
- **PowerModels.jl**: Julia OPF with multiple formulations
- **IPOPT**: Interior-point NLP solver

### GAT Documentation

- [OPF Guide](/guide/opf/) — Practical usage
- [Power Flow Theory](/reference/power-flow/) — Underlying equations
- [Glossary](/reference/glossary/) — Term definitions
