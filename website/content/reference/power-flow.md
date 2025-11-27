+++
title = "Power Flow Theory"
description = "Mathematical foundations of AC and DC power flow analysis"
weight = 5
+++

# Power Flow Theory

This reference explains the physics and mathematics behind power flow analysis — the foundational calculation for all grid studies.

## Why Power Flow Matters

Power flow analysis answers the fundamental question: *Given generation and load at each bus, what are the voltages and line flows throughout the network?*

Every other grid analysis builds on power flow:
- **OPF** optimizes generation subject to power flow equations
- **Contingency analysis** runs power flow for each outage scenario
- **State estimation** reconciles measurements with power flow physics
- **Planning studies** evaluate future scenarios via power flow

Understanding power flow deeply is essential for interpreting results and debugging convergence issues.

---

## Physical Intuition

### Kirchhoff's Laws

Power flow is governed by two physical principles:

1. **Conservation of charge** (Kirchhoff's Current Law): Current entering a node equals current leaving
2. **Conservation of energy** (Kirchhoff's Voltage Law): Voltage drops around any closed loop sum to zero

In power systems terms:
- Power injected at each bus must equal power flowing out on connected branches
- Voltage differences across branches drive power flow

### Why Does Power Flow?

Consider two buses connected by a transmission line:

```
    Bus 1              Bus 2
    V₁∠θ₁ ────────── V₂∠θ₂
           R + jX
```

Power flows from higher to lower voltage angle. The real power transfer is approximately:

```
P₁₂ ≈ (V₁ · V₂ / X) · sin(θ₁ - θ₂)
```

Key insights:
- **Angle difference drives real power**: Larger θ₁ - θ₂ means more MW flowing
- **Reactance limits flow**: Higher X means less power for the same angle difference
- **Voltage magnitudes matter**: Lower voltages reduce power transfer capability

For reactive power, voltage magnitude difference is the driver:

```
Q₁₂ ≈ (V₁ / X) · (V₁ - V₂·cos(θ₁ - θ₂))
```

This is why generators raise voltage to export reactive power and lower it to absorb.

---

## AC Power Flow

AC power flow solves the exact nonlinear equations relating voltages, angles, and power injections.

### Complex Power

At each bus, complex power injection is:

```
S_i = P_i + jQ_i = V_i · I_i*
```

where V_i is complex voltage and I_i* is the complex conjugate of current.

### The Admittance Matrix (Y-bus)

The network is characterized by the admittance matrix Y, where:

```
I = Y · V
```

For a network with n buses, Y is an n×n sparse complex matrix:

**Off-diagonal elements** (i ≠ j):
```
Y_ij = -y_ij = -1/(R_ij + jX_ij)
```
where y_ij is the series admittance of the branch connecting buses i and j.

**Diagonal elements**:
```
Y_ii = Σⱼ y_ij + y_i^shunt
```
Sum of all admittances connected to bus i, plus any shunt elements (capacitors, reactors).

### Power Balance Equations

Substituting I = Y·V into S = V·I* gives the power flow equations:

**Real power at bus i:**
```
P_i = Σⱼ |V_i| · |V_j| · (G_ij · cos(θ_i - θ_j) + B_ij · sin(θ_i - θ_j))
```

**Reactive power at bus i:**
```
Q_i = Σⱼ |V_i| · |V_j| · (G_ij · sin(θ_i - θ_j) - B_ij · cos(θ_i - θ_j))
```

where G_ij + jB_ij = Y_ij (conductance and susceptance).

These 2n equations (n real power, n reactive power) must be satisfied simultaneously.

### Bus Classifications

Not all 4n variables (P, Q, |V|, θ at each bus) are unknowns. Bus types determine what's specified:

| Bus Type | Known | Unknown | Purpose |
|----------|-------|---------|---------|
| **Slack (Ref)** | \|V\|, θ | P, Q | Reference angle, absorbs mismatch |
| **PV (Generator)** | P, \|V\| | Q, θ | Voltage-controlled generation |
| **PQ (Load)** | P, Q | \|V\|, θ | Fixed demand |

With one slack bus and assuming n_pv PV buses:
- Unknowns: (n-1) angles + (n - n_pv - 1) voltage magnitudes
- Equations: (n-1) real power + (n - n_pv - 1) reactive power

The system is square and (usually) solvable.

### The Jacobian Matrix

The power flow equations are nonlinear, so we solve iteratively. Newton-Raphson linearizes around the current estimate:

```
[ΔP]   [J₁  J₂] [Δθ ]
[  ] = [      ] [   ]
[ΔQ]   [J₃  J₄] [Δ|V|]
```

The Jacobian submatrices contain partial derivatives:

- **J₁ = ∂P/∂θ**: How real power changes with angle
- **J₂ = ∂P/∂|V|**: How real power changes with voltage magnitude
- **J₃ = ∂Q/∂θ**: How reactive power changes with angle
- **J₄ = ∂Q/∂|V|**: How reactive power changes with voltage magnitude

For example, the off-diagonal element of J₁:

```
∂P_i/∂θ_j = |V_i| · |V_j| · (G_ij · sin(θ_i - θ_j) - B_ij · cos(θ_i - θ_j))
```

GAT computes the full Jacobian in `power_equations.rs`.

### Newton-Raphson Algorithm

1. **Initialize**: Set all |V| = 1.0 p.u., all θ = 0 (flat start)

2. **Compute mismatches**: Calculate P_calc and Q_calc from current V, θ
   ```
   ΔP = P_specified - P_calc
   ΔQ = Q_specified - Q_calc
   ```

3. **Check convergence**: If max(|ΔP|, |ΔQ|) < tolerance, stop

4. **Compute Jacobian**: Build J from current V, θ

5. **Solve linear system**:
   ```
   [Δθ, Δ|V|]ᵀ = J⁻¹ · [ΔP, ΔQ]ᵀ
   ```

6. **Update**: θ ← θ + Δθ, |V| ← |V| + Δ|V|

7. **Repeat** from step 2

Typical convergence: 3-10 iterations for well-conditioned cases.

### Why Newton-Raphson?

Newton-Raphson has **quadratic convergence** near the solution — errors decrease as ε² each iteration. This means:
- 6 correct digits → 12 correct digits in one iteration
- Very fast once "close" to the answer

The cost is computing and factoring the Jacobian each iteration (O(n²) to O(n³) depending on sparsity).

---

## DC Power Flow

DC power flow is a linear approximation enabling much faster solutions.

### Assumptions

1. **Voltage magnitudes ≈ 1.0 p.u.**: |V_i| ≈ 1 for all buses
2. **Small angle differences**: sin(θ_i - θ_j) ≈ θ_i - θ_j, cos(θ_i - θ_j) ≈ 1
3. **Lossless lines**: R << X, so G ≈ 0

### Simplified Equations

Under these assumptions, the real power equation becomes:

```
P_i = Σⱼ B_ij · (θ_i - θ_j)
```

In matrix form:

```
P = B · θ
```

where B is the susceptance matrix (imaginary part of Y-bus, negated).

This is a **linear system** — no iteration needed! One matrix solve gives the answer.

### Solving DC Power Flow

1. Remove the slack bus row/column from B (it has θ = 0)
2. Solve the reduced system: θ = B_reduced⁻¹ · P
3. Compute branch flows: P_ij = (θ_i - θ_j) / X_ij

### What DC Power Flow Ignores

- **Reactive power**: No Q equations, no voltage magnitude results
- **Losses**: I²R losses are zero (R = 0 assumption)
- **Voltage constraints**: Cannot check if |V| stays within limits
- **VAR limits**: Generator reactive limits don't apply

### When to Use DC vs. AC

| Use DC Power Flow | Use AC Power Flow |
|-------------------|-------------------|
| Screening studies | Final verification |
| Contingency ranking | Voltage analysis |
| Market clearing (LMPs) | Reactive planning |
| Transmission planning | Loss calculation |
| Large-scale studies | Distribution networks |

DC power flow typically underestimates congestion and overestimates transfer capability.

---

## Fast Decoupled Power Flow

A middle ground between full Newton-Raphson and DC approximation.

### P-θ / Q-V Decoupling

In transmission networks:
- Real power P depends mainly on angles θ
- Reactive power Q depends mainly on voltage magnitudes |V|

This suggests solving two smaller systems instead of one large one:

```
ΔP = B' · Δθ      (P-θ subproblem)
ΔQ = B'' · Δ|V|   (Q-V subproblem)
```

### Algorithm

1. Solve P-θ: Update angles using B' (constant matrix)
2. Solve Q-V: Update voltages using B'' (constant matrix)
3. Repeat until converged

**Advantage**: B' and B'' are factored once and reused (no Jacobian rebuild)

**Disadvantage**: Slower convergence than Newton-Raphson (linear, not quadratic)

Typically useful for real-time applications where speed matters more than iteration count.

---

## Convergence Issues

Power flow doesn't always converge. Common causes and remedies:

### Heavy Loading

The system may have no solution if load exceeds transfer capability.

**Symptoms**: Mismatch oscillates or grows
**Remedy**: Reduce load, add generation, or check for data errors

### Reactive Power Limits

Generators hitting Q limits switch from PV to PQ buses, potentially causing voltage collapse.

**Symptoms**: Voltages drop progressively
**Remedy**: Add reactive support, relax limits for debugging

### Bad Initial Guess

Flat start may be far from the solution for heavily loaded or unusual systems.

**Symptoms**: Divergence from the first iteration
**Remedy**: Use warm start from similar case, or solve a relaxed problem first

### Data Errors

Incorrect impedances, missing buses, or topology errors.

**Symptoms**: Immediate divergence or nonsensical results
**Remedy**: Check input data, run `gat graph islands` to verify connectivity

### Ill-Conditioning

Very long or very short lines create numerical issues.

**Symptoms**: Slow convergence, sensitivity to tolerance
**Remedy**: Check per-unit values, consider network reduction

---

## Power Flow in GAT

GAT implements power flow in `gat-algo`:

### DC Power Flow

```bash
gat pf dc network.arrow --out flows.parquet
```

Uses sparse LU factorization of the B matrix. O(n) for radial networks, O(n^1.5) for meshed.

### AC Power Flow

```bash
gat pf ac network.arrow --out flows.parquet --tol 1e-6 --max-iter 20
```

Full Newton-Raphson with:
- Sparse Jacobian computation
- LU factorization with AMD ordering
- Automatic PV→PQ switching at reactive limits

### Implementation Details

| Component | Location | Purpose |
|-----------|----------|---------|
| Y-bus construction | `ybus.rs` | Build admittance matrix |
| Sparse Y-bus | `sparse_ybus.rs` | O(nnz) storage |
| Power equations | `power_equations.rs` | P, Q, and Jacobian |
| Newton-Raphson | `solver.rs` | Iteration loop |

---

## Mathematical Derivations

### Derivation of Power Flow Equations

Starting from S = V · I* and I = Y · V:

```
S_i = V_i · (Σⱼ Y_ij · V_j)*
    = V_i · Σⱼ Y_ij* · V_j*
    = Σⱼ V_i · V_j* · Y_ij*
```

Writing V_i = |V_i| · e^(jθ_i) and Y_ij = G_ij + jB_ij:

```
S_i = Σⱼ |V_i| · |V_j| · e^(j(θ_i - θ_j)) · (G_ij - jB_ij)
```

Taking real and imaginary parts gives the P and Q equations.

### Derivation of DC Approximation

Starting from:
```
P_i = Σⱼ |V_i| · |V_j| · (G_ij · cos(θ_ij) + B_ij · sin(θ_ij))
```

Apply assumptions:
1. |V_i| = |V_j| = 1
2. G_ij = 0 (lossless)
3. sin(θ_ij) ≈ θ_ij for small angles

Result:
```
P_i = Σⱼ B_ij · θ_ij = Σⱼ B_ij · (θ_i - θ_j)
```

---

## Further Reading

### Textbooks

- **Grainger & Stevenson**, *Power System Analysis* — Standard undergraduate text
- **Glover, Sarma & Overbye**, *Power System Analysis and Design* — Comprehensive coverage
- **Kundur**, *Power System Stability and Control* — Advanced dynamics

### Papers

- **Tinney & Hart (1967)**, "Power Flow Solution by Newton's Method" — Foundational paper on sparse techniques
- **Stott & Alsaç (1974)**, "Fast Decoupled Load Flow" — The FDLF algorithm
- **Van Amerongen (1989)**, "A General-Purpose Version of the Fast Decoupled Load Flow" — Modern FDLF

### GAT Documentation

- [Power Flow Guide](/guide/pf/) — Practical usage
- [OPF Guide](/guide/opf/) — Optimization with power flow constraints
- [Glossary](/reference/glossary/) — Term definitions
