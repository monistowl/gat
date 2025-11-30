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

$$\min \sum_i C_i(P_i)$$

subject to:

$$\sum_i P_i = \sum_j D_j \quad \text{(power balance)}$$

$$P_i^{\min} \leq P_i \leq P_i^{\max} \quad \text{(generator limits)}$$

where:
- $C_i(P_i)$ is the cost function for generator i
- $D_j$ is demand at bus j

### Solution

For quadratic costs $C_i(P) = a_i + b_i P + c_i P^2$:

At optimum, all generators have equal marginal cost (if unconstrained):

$$\frac{dC_i}{dP_i} = b_i + 2c_i P_i = \lambda \quad \text{(for all } i \text{ not at limits)}$$

where $\lambda$ is the system marginal price.

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
2. Small angle differences: $\sin(\theta) \approx \theta$
3. Lossless lines: $R = 0$

### Formulation

**Variables:**
- $P_i$: generator output at bus i
- $\theta_i$: voltage angle at bus i

**Objective:**

$$\min \sum_i (a_i + b_i P_i + c_i P_i^2)$$

Note: Quadratic objectives make this a QP, not LP. For true LP, use piecewise-linear cost approximation.

**Constraints:**

Power balance at each bus:

$$P_i^{\text{gen}} - P_i^{\text{load}} = \sum_j B_{ij}(\theta_i - \theta_j) \quad \forall i$$

Generator limits:

$$P_i^{\min} \leq P_i \leq P_i^{\max} \quad \forall \text{ generators}$$

Branch flow limits:

$$|P_{ij}| = \left| \frac{\theta_i - \theta_j}{X_{ij}} \right| \leq P_{ij}^{\max} \quad \forall \text{ branches}$$

Reference angle:

$$\theta_{\text{ref}} = 0$$

### Matrix Form

The power balance constraint in matrix form:

$$\mathbf{P}\_{\text{gen}} - \mathbf{P}\_{\text{load}} = \mathbf{B} \cdot \boldsymbol{\theta}$$

where $\mathbf{B}$ is the susceptance matrix (imaginary part of Y-bus).

### Locational Marginal Prices (LMPs)

The dual variables of power balance constraints give LMPs:

$$\text{LMP}_i = \lambda_i = \frac{\partial(\text{Total Cost})}{\partial(\text{Demand at bus } i)}$$

LMP components:
- **Energy**: System marginal cost ($\lambda$)
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
- $w_i = |V_i|^2$: squared voltage magnitude
- $\ell_{ij} = |I_{ij}|^2$: squared current magnitude
- $P_{ij}, Q_{ij}$: branch power flows
- $P_i^{\text{gen}}, Q_i^{\text{gen}}$: generator outputs

### Formulation

**Objective:**

$$\min \sum_i (a_i + b_i P_i + c_i P_i^2)$$

**Branch flow equations** (from bus i to bus j):

$$w_j = w_i - 2(r_{ij} P_{ij} + x_{ij} Q_{ij}) + (r_{ij}^2 + x_{ij}^2)\ell_{ij}$$

**Power balance at each bus:**

$$P_i^{\text{gen}} - P_i^{\text{load}} = \sum_j P_{ij} - \sum_k (P_{ki} - r_{ki}\ell_{ki})$$

$$Q_i^{\text{gen}} - Q_i^{\text{load}} = \sum_j Q_{ij} - \sum_k (Q_{ki} - x_{ki}\ell_{ki})$$

**The SOC constraint** (relaxed from equality):

$$P_{ij}^2 + Q_{ij}^2 \leq w_i \cdot \ell_{ij}$$

This is a **second-order cone** constraint:

$$\left\| \left( P_{ij}, Q_{ij}, \frac{w\_i - \ell_{ij}}{2} \right) \right\|\_2 \leq \frac{w\_i + \ell_{ij}}{2}$$

**Voltage limits:**

$$(V_i^{\min})^2 \leq w_i \leq (V_i^{\max})^2$$

**Thermal limits:**

$$P_{ij}^2 + Q_{ij}^2 \leq (S_{ij}^{\max})^2$$

**Generator limits:**

$$P_i^{\min} \leq P_i^{\text{gen}} \leq P_i^{\max}$$

$$Q_i^{\min} \leq Q_i^{\text{gen}} \leq Q_i^{\max}$$

### Why is SOCP a Relaxation?

The exact AC power flow requires:

$$P_{ij}^2 + Q_{ij}^2 = w_i \cdot \ell_{ij} \quad \text{(equality)}$$

SOCP relaxes this to:

$$P_{ij}^2 + Q_{ij}^2 \leq w_i \cdot \ell_{ij} \quad \text{(inequality)}$$

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

- $V_i$: voltage magnitude at bus i (p.u.)
- $\theta_i$: voltage angle at bus i (radians)
- $P_i^{\text{gen}}, Q_i^{\text{gen}}$: generator real and reactive power

### Formulation

**Objective:**

$$\min \sum_i (a_i + b_i P_i + c_i P_i^2)$$

**Power balance (equality constraints):**

$$P_i^{\text{gen}} - P_i^{\text{load}} = \sum_j V_i V_j (G_{ij} \cos\theta_{ij} + B_{ij} \sin\theta_{ij})$$

$$Q_i^{\text{gen}} - Q_i^{\text{load}} = \sum_j V_i V_j (G_{ij} \sin\theta_{ij} - B_{ij} \cos\theta_{ij})$$

where $\theta_{ij} = \theta_i - \theta_j$ and $G_{ij} + jB_{ij} = Y_{ij}$.

**Voltage limits:**

$$V_i^{\min} \leq V_i \leq V_i^{\max}$$

**Generator limits:**

$$P_i^{\min} \leq P_i^{\text{gen}} \leq P_i^{\max}$$

$$Q_i^{\min} \leq Q_i^{\text{gen}} \leq Q_i^{\max}$$

**Thermal limits:**

$$P_{ij}^2 + Q_{ij}^2 \leq (S_{ij}^{\max})^2$$

where branch flows are:

$$P_{ij} = V_i^2 G_{ij} - V_i V_j (G_{ij} \cos\theta_{ij} + B_{ij} \sin\theta_{ij})$$

$$Q_{ij} = -V_i^2 B_{ij} - V_i V_j (G_{ij} \sin\theta_{ij} - B_{ij} \cos\theta_{ij})$$

**Reference angle:**

$$\theta_{\text{ref}} = 0$$

### Solution Methods

AC-OPF is **NP-hard** in general. No polynomial-time algorithm guarantees the global optimum. Practical approaches:

#### Penalty Method (GAT Default)

Convert constrained problem to unconstrained via penalty:

$$\min f(\mathbf{x}) + \mu \|g(\mathbf{x})\|^2 + \mu \|\max(0, h(\mathbf{x}))\|^2$$

where:
- $f(\mathbf{x})$ is the objective
- $g(\mathbf{x}) = 0$ are equality constraints (power balance)
- $h(\mathbf{x}) \leq 0$ are inequality constraints

Algorithm:
1. Start with small $\mu$
2. Minimize penalized objective using L-BFGS
3. If constraints violated, increase $\mu$ and repeat
4. Stop when constraints satisfied within tolerance

**Pros:** Simple, handles infeasible starts
**Cons:** First-order convergence, ill-conditioning at large $\mu$

#### Interior Point Method (IPOPT)

Barrier method staying strictly inside inequality constraints:

$$\min f(\mathbf{x}) - \mu \sum_j \log(-h_j(\mathbf{x})) \quad \text{subject to } g(\mathbf{x}) = 0$$

Uses Newton steps with:
- Gradient of Lagrangian
- Hessian of Lagrangian (requires second derivatives)

**Pros:** Superlinear convergence, handles many constraints efficiently
**Cons:** Requires Hessian computation, needs feasible start

GAT provides IPOPT backend via the `solver-ipopt` feature.

### KKT Conditions

At a local optimum, the Karush-Kuhn-Tucker conditions hold:

$$\nabla f(\mathbf{x}^*) + \sum_i \lambda_i \nabla g_i(\mathbf{x}^*) + \sum_j \mu_j \nabla h_j(\mathbf{x}^*) = 0 \quad \text{(stationarity)}$$

$$g_i(\mathbf{x}^*) = 0 \quad \text{(primal feasibility)}$$

$$h_j(\mathbf{x}^*) \leq 0 \quad \text{(primal feasibility)}$$

$$\mu_j \geq 0 \quad \text{(dual feasibility)}$$

$$\mu_j h_j(\mathbf{x}^*) = 0 \quad \text{(complementarity)}$$

The dual variables $\lambda_i$ and $\mu_j$ provide sensitivity information and LMPs.

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

$$C(P) = c_0 + c_1 P + c_2 P^2$$

- $c_0$: No-load cost ($/hr) — fuel burned at minimum output
- $c_1$: Linear cost ($/MWh) — marginal fuel cost
- $c_2$: Quadratic cost ($/MW²h) — efficiency curve

Marginal cost: $\frac{dC}{dP} = c_1 + 2c_2 P$

### Piecewise Linear Cost

$$C(P) = \sum_k m_k \cdot \max(0, P - P_k)$$

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

$$\min f(\mathbf{x}) + \rho \cdot (\text{violation})^2$$

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

- [OPF Guide](@/guide/opf.md) — Practical usage
- [Power Flow Theory](@/reference/power-flow.md) — Underlying equations
- [Glossary](@/reference/glossary.md) — Term definitions
