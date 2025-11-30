+++
title = "Newton-Raphson Method"
description = "The iterative algorithm that solves power flow equations"
weight = 7
+++

# Newton-Raphson Method

The **Newton-Raphson method** is the workhorse algorithm for solving AC power flow. It iteratively refines voltage guesses until the power balance equations are satisfied to within a small tolerance.

Understanding Newton-Raphson helps you:
- Interpret convergence behavior
- Debug non-converging cases
- Appreciate why power flow is "hard" computationally

<div class="grid-widget" data-network="ieee14" data-height="400" data-voltage="true" data-flow="true" data-legend="true" data-caption="Interactive: A converged power flow solution. Click V to see voltage profile (green=nominal, yellow/red=deviation). Click ⚡ to see branch loading."></div>

---

## The Problem

Power flow needs to solve a system of nonlinear equations:

$$\mathbf{f}(\mathbf{x}) = \mathbf{0}$$

where:
- $\mathbf{x}$ = unknown voltage magnitudes and angles
- $\mathbf{f}$ = power mismatches (specified minus calculated)

For $n$ buses with one slack bus and $n_G$ PV buses:
- $(n-1)$ unknown angles
- $(n - 1 - n_G)$ unknown voltage magnitudes

The mismatch functions are:

$$\Delta P_i = P_i^{\text{spec}} - P_i^{\text{calc}}(\mathbf{V}, \boldsymbol{\theta})$$

$$\Delta Q_i = Q_i^{\text{spec}} - Q_i^{\text{calc}}(\mathbf{V}, \boldsymbol{\theta})$$

These are nonlinear because the calculated powers involve products of voltages and trigonometric functions of angles.

---

## The Newton-Raphson Idea

Newton-Raphson finds roots of $\mathbf{f}(\mathbf{x}) = \mathbf{0}$ by repeatedly:

1. **Linearizing** the equations around the current guess
2. **Solving** the linear system for a correction
3. **Updating** the guess

### Single Variable Case

For a scalar equation $f(x) = 0$:

At guess $x_k$, approximate $f$ by its tangent line:

$$f(x) \approx f(x_k) + f'(x_k)(x - x_k)$$

Setting this equal to zero and solving:

$$x_{k+1} = x_k - \frac{f(x_k)}{f'(x_k)}$$

**Geometric interpretation**: Follow the tangent line to where it crosses zero.

```
    f(x)
     │    ╱
     │   ╱ tangent
     │  ╱
  f(xₖ)●────────┐
     │╱         │
─────┼──────────●────── x
    xₖ₊₁       xₖ
```

### Vector Case (Power Flow)

For a system of equations $\mathbf{f}(\mathbf{x}) = \mathbf{0}$:

$$\mathbf{x}_{k+1} = \mathbf{x}_k - \mathbf{J}^{-1} \mathbf{f}(\mathbf{x}_k)$$

where $\mathbf{J}$ is the **Jacobian matrix** — the matrix of partial derivatives:

$$J_{ij} = \frac{\partial f_i}{\partial x_j}$$

In practice, we solve the linear system rather than inverting:

$$\mathbf{J} \cdot \Delta\mathbf{x} = -\mathbf{f}$$

then update: $\mathbf{x}_{k+1} = \mathbf{x}_k + \Delta\mathbf{x}$

---

## Power Flow Formulation

The state vector contains angles first, then voltage magnitudes:

$$\mathbf{x} = \begin{pmatrix} \boldsymbol{\theta} \\ \mathbf{|V|} \end{pmatrix}$$

The mismatch vector contains real power first, then reactive:

$$\mathbf{f} = \begin{pmatrix} \Delta\mathbf{P} \\ \Delta\mathbf{Q} \end{pmatrix}$$

### The Jacobian Matrix

The Jacobian has a natural 2×2 block structure:

$$\mathbf{J} = \begin{pmatrix}
\dfrac{\partial \mathbf{P}}{\partial \boldsymbol{\theta}} & \dfrac{\partial \mathbf{P}}{\partial \mathbf{|V|}} \\[1em]
\dfrac{\partial \mathbf{Q}}{\partial \boldsymbol{\theta}} & \dfrac{\partial \mathbf{Q}}{\partial \mathbf{|V|}}
\end{pmatrix} = \begin{pmatrix}
\mathbf{J}_1 & \mathbf{J}_2 \\
\mathbf{J}_3 & \mathbf{J}_4
\end{pmatrix}$$

Each block has a physical interpretation:
- $\mathbf{J}_1$: How real power changes with angles (strong coupling)
- $\mathbf{J}_2$: How real power changes with voltages (weak coupling)
- $\mathbf{J}_3$: How reactive power changes with angles (weak coupling)
- $\mathbf{J}_4$: How reactive power changes with voltages (strong coupling)

### Jacobian Elements

For the off-diagonal elements ($i \neq k$):

$$\frac{\partial P_i}{\partial \theta_k} = |V_i||V_k|(G_{ik}\sin\theta_{ik} - B_{ik}\cos\theta_{ik})$$

$$\frac{\partial P_i}{\partial |V_k|} = |V_i|(G_{ik}\cos\theta_{ik} + B_{ik}\sin\theta_{ik})$$

$$\frac{\partial Q_i}{\partial \theta_k} = -|V_i||V_k|(G_{ik}\cos\theta_{ik} + B_{ik}\sin\theta_{ik})$$

$$\frac{\partial Q_i}{\partial |V_k|} = |V_i|(G_{ik}\sin\theta_{ik} - B_{ik}\cos\theta_{ik})$$

The diagonal elements involve sums over all connected buses.

---

## The Algorithm

```
1. Initialize: V = 1.0 p.u., θ = 0 for all buses (flat start)
2. Repeat:
   a. Calculate P_calc and Q_calc from current V, θ
   b. Compute mismatches: ΔP = P_spec - P_calc, ΔQ = Q_spec - Q_calc
   c. Check convergence: if max(|ΔP|, |ΔQ|) < tolerance, STOP
   d. Build Jacobian matrix J
   e. Solve J · Δx = -f for corrections Δθ, Δ|V|
   f. Update: θ ← θ + Δθ, |V| ← |V| + Δ|V|
   g. Handle PV buses: if Q exceeds limits, convert to PQ
3. Return solution or report non-convergence
```

### Convergence Criterion

Typically we check:

$$\max\left(|\Delta P_i|, |\Delta Q_i|\right) < \epsilon$$

Common tolerances:
- $\epsilon = 10^{-6}$ p.u. for high accuracy
- $\epsilon = 10^{-4}$ p.u. for practical studies

---

## Convergence Properties

### Quadratic Convergence

Near the solution, Newton-Raphson converges **quadratically** — each iteration roughly doubles the number of correct digits:

| Iteration | Max Mismatch |
|-----------|--------------|
| 1 | 0.5 |
| 2 | 0.1 |
| 3 | 0.001 |
| 4 | 0.000001 |
| 5 | 0.000000000001 |

This is why power flow typically converges in **3-7 iterations** from a flat start.

### When It Doesn't Converge

Non-convergence usually indicates one of:

1. **Infeasible operating point**: The specified generation and load cannot be satisfied (e.g., not enough reactive support)

2. **Bad initial guess**: Flat start too far from solution. Try warm-starting from a similar solved case.

3. **Numerical issues**: Very high or low impedances, islanded buses, bad data.

4. **Stressed system**: Operating near voltage collapse. The Jacobian becomes nearly singular.

**Debugging tips**:
- Check for negative resistance or zero impedance branches
- Look for isolated buses
- Verify generation equals load plus reasonable losses
- Try reducing load to find a feasible point

---

## Fast Decoupled Power Flow

The Jacobian has a special structure we can exploit:

- $\mathbf{J}_1$ (∂P/∂θ) is large — angles strongly affect real power
- $\mathbf{J}_4$ (∂Q/∂|V|) is large — voltages strongly affect reactive power
- $\mathbf{J}_2$ and $\mathbf{J}_3$ are smaller — cross-coupling is weak

**Fast decoupled** power flow ignores the off-diagonal blocks and uses constant approximations:

$$\mathbf{B}' \cdot \Delta\boldsymbol{\theta} = \frac{\Delta\mathbf{P}}{\mathbf{|V|}}$$

$$\mathbf{B}'' \cdot \Delta\mathbf{|V|} = \frac{\Delta\mathbf{Q}}{\mathbf{|V|}}$$

where $\mathbf{B}'$ and $\mathbf{B}''$ are constant (computed once).

**Advantages**:
- Faster iterations (two small systems vs. one large)
- No Jacobian rebuild each iteration
- Each half-iteration solves a symmetric positive-definite system

**Disadvantages**:
- Linear convergence (slower than quadratic)
- May not converge for ill-conditioned networks (high R/X ratios)

---

## DC Power Flow

DC power flow takes linearization to the extreme, assuming:
1. Voltage magnitudes ≈ 1.0 p.u.
2. Angle differences are small: $\sin\theta \approx \theta$, $\cos\theta \approx 1$
3. Resistance negligible: $R << X$

This yields a linear system:

$$\mathbf{B} \cdot \boldsymbol{\theta} = \mathbf{P}$$

where $B_{ik} = -1/X_{ik}$ for connected buses.

**No iteration needed** — just solve the linear system. But DC power flow ignores reactive power and losses.

---

## Computational Complexity

Each Newton-Raphson iteration involves:

1. **Power calculation**: $O(m)$ where $m$ = number of branches
2. **Jacobian formation**: $O(m)$ — sparse matrix, entries only for connected buses
3. **Linear solve**: $O(n^{1.5})$ using sparse LU factorization

For a 10,000-bus system with 5 iterations:
- Jacobian: ~15,000 × 15,000 but only ~100,000 non-zeros
- Solve time: milliseconds on modern hardware

Power flow is fast enough for real-time operations and contingency screening.

---

## GAT Implementation

GAT implements Newton-Raphson in `crates/gat-algo/src/power_flow/`:

```rust
let result = solve_ac_power_flow(&network, &options)?;
```

Options include:
- `tolerance`: Convergence criterion (default: 1e-6)
- `max_iterations`: Iteration limit (default: 50)
- `enforce_q_limits`: Enable PV→PQ switching

The result includes:
- Converged voltage magnitudes and angles
- Real and reactive power at each bus
- Slack bus generation
- Line flows and losses

---

## Key Takeaways

1. **Newton-Raphson iteratively solves** $\mathbf{J} \cdot \Delta\mathbf{x} = -\mathbf{f}$
2. **Jacobian** is the matrix of sensitivities: how power changes with voltage/angle
3. **Quadratic convergence**: typically 3-7 iterations from flat start
4. **Non-convergence** usually means infeasible case or bad data
5. **Fast decoupled** trades accuracy for speed; **DC power flow** is fully linear

---

## See Also

- [Power Flow Theory](/reference/power-flow/) — The equations being solved
- [Bus Types](/reference/bus-types/) — How buses affect the Jacobian structure
- [Y-Bus Matrix](/reference/ybus-matrix/) — Network data used in calculations
- [OPF Formulations](/reference/opf-formulations/) — Extending power flow to optimization
