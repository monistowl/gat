+++
title = "The Y-Bus Matrix"
description = "Building and understanding the network admittance matrix"
weight = 6
+++

# The Y-Bus Matrix

The **bus admittance matrix** (Y-bus) is the fundamental data structure of power flow analysis. It encodes the entire network topology and parameters in a single sparse matrix that relates bus voltages to bus currents:

$$\mathbf{I} = \mathbf{Y} \cdot \mathbf{V}$$

Every power flow solver builds the Y-bus as its first step.

---

## Why a Matrix?

Consider a simple 3-bus network:

```
     Bus 1 ───────── Bus 2
       │               │
       │               │
       └───── Bus 3 ───┘
```

At each bus, Kirchhoff's Current Law says: current injected = sum of currents flowing out on branches.

We could write this as three separate equations, but it's much cleaner as a matrix equation:

$$\begin{pmatrix} I_1 \\ I_2 \\ I_3 \end{pmatrix} = \mathbf{Y} \cdot \begin{pmatrix} V_1 \\ V_2 \\ V_3 \end{pmatrix}$$

The Y-bus matrix $\mathbf{Y}$ captures all branch admittances and how buses connect.

<div class="grid-widget" data-network="three-bus" data-height="380" data-ybus="true" data-legend="true" data-caption="Interactive: Click 'Y' to view the computed Y-bus matrix. Hover matrix cells to highlight branches."></div>

---

## Building the Y-Bus

For a network with $n$ buses, the Y-bus is an $n \times n$ complex matrix built from branch admittances.

### Off-Diagonal Elements

For $i \neq j$, the element $Y_{ij}$ equals the **negative of the admittance** connecting buses $i$ and $j$:

$$Y_{ij} = -y_{ij}$$

If buses $i$ and $j$ are not directly connected, $Y_{ij} = 0$.

**Why negative?** Current flowing from bus $i$ to bus $j$ is:

$$I_{i \to j} = y_{ij}(V_i - V_j)$$

This contributes $+y_{ij}V_i$ to bus $i$'s current (leaving) and $-y_{ij}V_i$ to bus $j$'s current equation. The negative sign in $Y_{ij}$ captures this relationship.

### Diagonal Elements

The diagonal element $Y_{ii}$ equals the **sum of all admittances connected to bus $i$**:

$$Y_{ii} = \sum_{j \neq i} y_{ij} + y_i^{\text{shunt}}$$

This includes:
- Series admittances of all branches connected to bus $i$
- Shunt admittances at bus $i$ (capacitors, reactors, line charging)

### The Admittance of a Branch

For a branch with impedance $\mathbf{Z} = R + jX$:

$$y = \frac{1}{\mathbf{Z}} = \frac{1}{R + jX} = \frac{R - jX}{R^2 + X^2}$$

In power systems notation, we write $y = g + jb$ where:
- $g = \frac{R}{R^2 + X^2}$ (series conductance)
- $b = \frac{-X}{R^2 + X^2}$ (series susceptance)

---

## Example: Building a 3-Bus Y-Bus

Consider this network:

```
       y₁₂ = 0.5 - j2.0
  [1] ────────────────── [2]
   │                      │
   │ y₁₃ = 0.3 - j1.5     │ y₂₃ = 0.4 - j1.8
   │                      │
  [3] ────────────────────┘
```

(All values in per-unit. No shunts for simplicity.)

**Off-diagonal elements:**

$$Y_{12} = Y_{21} = -y_{12} = -(0.5 - j2.0) = -0.5 + j2.0$$

$$Y_{13} = Y_{31} = -y_{13} = -(0.3 - j1.5) = -0.3 + j1.5$$

$$Y_{23} = Y_{32} = -y_{23} = -(0.4 - j1.8) = -0.4 + j1.8$$

**Diagonal elements:**

$$Y_{11} = y_{12} + y_{13} = (0.5 - j2.0) + (0.3 - j1.5) = 0.8 - j3.5$$

$$Y_{22} = y_{12} + y_{23} = (0.5 - j2.0) + (0.4 - j1.8) = 0.9 - j3.8$$

$$Y_{33} = y_{13} + y_{23} = (0.3 - j1.5) + (0.4 - j1.8) = 0.7 - j3.3$$

**Full Y-bus:**

$$\mathbf{Y} = \begin{pmatrix}
0.8 - j3.5 & -0.5 + j2.0 & -0.3 + j1.5 \\
-0.5 + j2.0 & 0.9 - j3.8 & -0.4 + j1.8 \\
-0.3 + j1.5 & -0.4 + j1.8 & 0.7 - j3.3
\end{pmatrix}$$

**Notice:**
- Symmetric (for networks without phase shifters)
- Diagonal elements have negative imaginary parts (typical transmission lines are inductive)
- Each row sums to zero if there are no shunts (current conservation)

---

## The π-Model and Line Charging

Real transmission lines have distributed capacitance to ground, modeled as shunt elements at each end (the **π-model**):

```
         y_series
    ──┬───/\/\/───┬──
      │           │
    jB/2        jB/2
      │           │
     ─┴─         ─┴─
```

This adds $jB/2$ to the diagonal elements at each end:

$$Y_{ii} \mathrel{+}= jB_{ik}/2 \quad \text{(for each branch } ik \text{ connected to bus } i)$$

Line charging is significant for long high-voltage lines and causes lightly-loaded lines to generate reactive power.

---

## Transformers in the Y-Bus

Transformers with tap ratio $t$ require special treatment. For a transformer between buses $i$ and $j$ with:
- Series admittance $y$
- Tap ratio $t$ (typically near 1.0)

The Y-bus contributions are **asymmetric**:

$$Y_{ii} \mathrel{+}= \frac{y}{t^2}$$

$$Y_{jj} \mathrel{+}= y$$

$$Y_{ij} = Y_{ji} = -\frac{y}{t}$$

**Physical interpretation**: The tap ratio changes the effective impedance seen from each side. Off-nominal taps (t ≠ 1) make the Y-bus asymmetric.

### Phase Shifters

Phase-shifting transformers add a complex tap ratio $t \angle \phi$:

$$Y_{ij} = -\frac{y}{t e^{-j\phi}}, \quad Y_{ji} = -\frac{y}{t e^{j\phi}}$$

This creates asymmetry: $Y_{ij} \neq Y_{ji}$, enabling active power flow control.

---

## Y-Bus Properties

### Sparsity

Real power networks are sparse — each bus connects to only a few others. The Y-bus inherits this sparsity:
- A 1000-bus network might have 1,500 branches
- Y-bus has 1,000,000 elements, but only ~3,000 are non-zero

GAT stores Y-bus in sparse format (CSR/CSC), making operations efficient.

### Symmetry

For networks with only transmission lines and transformers (no phase shifters):
- $Y_{ij} = Y_{ji}$ — the matrix is symmetric
- This allows efficient storage and computation

Phase shifters break symmetry.

### Row Sums

For a network without shunts, each row of Y-bus sums to zero:

$$\sum_{j} Y_{ij} = 0$$

With shunts, row sums equal the total shunt admittance at that bus.

### Singularity

The Y-bus is singular (determinant = 0) for any connected network. This reflects the fact that we can add any constant to all voltages without changing currents — there's no absolute voltage reference.

Power flow handles this by fixing the slack bus angle.

---

## From Y-Bus to Power Flow

The power flow equations come from combining $\mathbf{I} = \mathbf{Y} \cdot \mathbf{V}$ with the complex power relationship $S = V \cdot I^*$:

$$S_i = V_i \cdot I_i^* = V_i \sum_{k=1}^{n} Y_{ik}^* V_k^*$$

Separating into real and imaginary parts:

$$P_i = \sum_{k=1}^{n} |V_i||V_k|(G_{ik}\cos\theta_{ik} + B_{ik}\sin\theta_{ik})$$

$$Q_i = \sum_{k=1}^{n} |V_i||V_k|(G_{ik}\sin\theta_{ik} - B_{ik}\cos\theta_{ik})$$

where:
- $Y_{ik} = G_{ik} + jB_{ik}$
- $\theta_{ik} = \theta_i - \theta_k$

These are the nonlinear equations solved by Newton-Raphson.

---

## Z-Bus: The Inverse

The **bus impedance matrix** is $\mathbf{Z} = \mathbf{Y}^{-1}$. Unlike the sparse Y-bus, Z-bus is **fully dense**.

Z-bus is used for:
- **Fault analysis**: $I_{\text{fault}} = V / Z_{kk}$ for a fault at bus $k$
- **Sensitivity analysis**: How voltage at bus $i$ changes with injection at bus $j$

Because Z-bus is dense, it's rarely computed for large networks. Instead, we solve $\mathbf{Y} \cdot \mathbf{x} = \mathbf{b}$ directly using sparse LU factorization.

---

## Building Y-Bus in GAT

GAT builds the Y-bus in the `ybus.rs` module:

```rust
let ybus = build_ybus(&buses, &branches);
```

This:
1. Creates a sparse matrix structure
2. Iterates through branches, adding series and shunt elements
3. Handles transformer tap ratios
4. Returns a sparse complex matrix ready for power flow

You can inspect the Y-bus with `gat inspect --ybus`.

---

## Key Takeaways

1. **Y-bus encodes the network**: $\mathbf{I} = \mathbf{Y} \cdot \mathbf{V}$
2. **Off-diagonal**: $Y_{ij} = -y_{ij}$ (negative branch admittance)
3. **Diagonal**: $Y_{ii} = \sum y_{ij} + \text{shunts}$ (sum of connected admittances)
4. **Sparse and symmetric** for typical networks (phase shifters break symmetry)
5. Foundation for power flow, fault analysis, and sensitivity studies

---

## See Also

- [Impedance & Admittance](@/reference/impedance-admittance.md) — Branch parameters
- [Power Flow Theory](@/reference/power-flow.md) — Using Y-bus in power flow
- [Bus Types](@/reference/bus-types.md) — How buses are classified
- [Newton-Raphson Method](@/reference/newton-raphson.md) — Solving the power flow equations
