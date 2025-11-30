+++
title = "Impedance and Admittance"
description = "Understanding R, X, Z, G, B, Y — how transmission lines and transformers are characterized"
weight = 3
+++

# Impedance and Admittance

Every transmission line, transformer, and cable in the power grid is characterized by its **impedance** (how much it resists current) or equivalently its **admittance** (how easily current flows). These parameters determine how power flows through the network.

<div class="grid-widget" data-network="three-bus" data-height="350" data-ybus="true" data-legend="true" data-caption="Interactive: Hover over branches to see Z = R + jX values. Click Y to view the computed admittance matrix."></div>

---

## Starting Simple: Resistance

In DC circuits, **resistance** $R$ relates voltage and current via Ohm's Law:

$$V = I \cdot R$$

Resistance dissipates energy as heat. A conductor with resistance $R$ carrying current $I$ loses power:

$$P_{\text{loss}} = I^2 R$$

This is why high-voltage transmission exists — raising voltage lets us reduce current, cutting $I^2R$ losses.

**Units**: Ohms ($\Omega$)

---

## AC Circuits: Enter Reactance

AC circuits have inductors and capacitors that store energy in fields rather than dissipating it. These elements have **reactance** $X$:

### Inductive Reactance

Inductors (coils of wire) store energy in magnetic fields. Current through an inductor creates a magnetic field that opposes changes in current.

$$X_L = \omega L = 2\pi f L$$

where:
- $L$ is inductance in Henries (H)
- $f$ is frequency (60 Hz in North America, 50 Hz in Europe)
- $\omega = 2\pi f$ is angular frequency

**Physical intuition**: Inductors "resist" current changes. At higher frequencies, this resistance increases.

### Capacitive Reactance

Capacitors store energy in electric fields between plates. Voltage across a capacitor opposes changes in voltage.

$$X_C = \frac{1}{\omega C} = \frac{1}{2\pi f C}$$

where $C$ is capacitance in Farads (F).

**Physical intuition**: Capacitors "resist" voltage changes. At higher frequencies, this resistance *decreases*.

**Units**: Ohms ($\Omega$), same as resistance

---

## Impedance: R and X Together

**Impedance** $\mathbf{Z}$ combines resistance and reactance into a single complex number:

$$\mathbf{Z} = R + jX$$

where:
- $R$ is resistance (real part) — dissipates energy
- $X$ is reactance (imaginary part) — stores energy
- $j = \sqrt{-1}$ (engineers use $j$ instead of $i$ to avoid confusion with current)

### Ohm's Law for AC

$$\mathbf{V} = \mathbf{I} \cdot \mathbf{Z}$$

Now voltage and current are complex phasors, and impedance is complex.

### Magnitude and Angle

$$|\mathbf{Z}| = \sqrt{R^2 + X^2}$$

$$\angle \mathbf{Z} = \arctan\left(\frac{X}{R}\right)$$

A transmission line with $\mathbf{Z} = 5 + j20$ $\Omega$ has:
- $|\mathbf{Z}| = \sqrt{25 + 400} = 20.6$ $\Omega$
- $\angle \mathbf{Z} = \arctan(20/5) = 76°$ (mostly reactive)

---

## Transmission Line Parameters

A transmission line has both series impedance and shunt admittance. The standard **$\pi$-model** looks like:

```
      R + jX
 ───┬──/\/\/──┬───
    │         │
   jB/2     jB/2
    │         │
   ─┴─       ─┴─
```

### Series Elements: R and X

**Resistance** $R$: Comes from conductor material (aluminum, copper). Causes $I^2R$ losses.

$$R = \rho \frac{\ell}{A}$$

where $\rho$ is resistivity, $\ell$ is length, $A$ is cross-sectional area.

**Reactance** $X$: Dominated by the magnetic field around and between conductors (inductance). Transmission lines are almost purely inductive.

$$X = \omega L \approx 0.3 \text{ to } 0.5 \text{ } \Omega/\text{km (typical)}$$

**Key insight**: For transmission lines, $X >> R$ (typically 3-10× larger). This is why DC power flow ignores resistance — reactance dominates.

### Shunt Elements: B (Line Charging)

Long transmission lines have capacitance between conductors and to ground. This creates **line charging** — the line generates reactive power even with no load.

$$B = \omega C$$

The $\pi$-model splits this capacitance equally at both ends: $B/2$ at each bus.

**Physical intuition**: Line charging is why lightly-loaded transmission lines can cause overvoltage — they're pumping reactive power into the system.

---

## Admittance: The Inverse

**Admittance** $\mathbf{Y}$ is the reciprocal of impedance:

$$\mathbf{Y} = \frac{1}{\mathbf{Z}} = G + jB$$

where:
- $G$ is **conductance** (real part) — how easily real current flows
- $B$ is **susceptance** (imaginary part) — how easily reactive current flows

### Why Use Admittance?

For network analysis, admittances are easier to work with:

**Impedances in series add**: $\mathbf{Z}_{\text{total}} = \mathbf{Z}_1 + \mathbf{Z}_2$

**Admittances in parallel add**: $\mathbf{Y}_{\text{total}} = \mathbf{Y}_1 + \mathbf{Y}_2$

Since buses connect multiple branches in parallel, building the network matrix (Y-bus) is simpler with admittances.

### Converting Z to Y

$$\mathbf{Y} = \frac{1}{\mathbf{Z}} = \frac{1}{R + jX} = \frac{R - jX}{R^2 + X^2}$$

So:

$$G = \frac{R}{R^2 + X^2}$$

$$B = \frac{-X}{R^2 + X^2}$$

**Note the sign**: Positive $X$ (inductive) gives negative $B$.

**Units**: Siemens (S), the reciprocal of ohms. $1 \text{ S} = 1/\Omega$

---

## Transformer Impedance

Transformers also have impedance, primarily reactive (leakage inductance). The **per-unit impedance** is typically 5-15% for power transformers:

$$Z_{\text{p.u.}} = 0.05 \text{ to } 0.15$$

This means if you short-circuit the secondary, the fault current is limited to:

$$I_{\text{fault}} = \frac{1}{Z_{\text{p.u.}}} = 6.7 \text{ to } 20 \times I_{\text{rated}}$$

### Tap Ratio

Transformers also have a **tap ratio** $t$ (turns ratio):

$$\frac{V_1}{V_2} = t$$

Tap changers adjust voltage ±10% in discrete steps. Off-nominal taps affect the admittance matrix — see [Y-Bus Matrix](@/reference/ybus-matrix.md).

---

## Per-Unit Impedance

Transmission studies use **per-unit** values to normalize across voltage levels:

$$Z_{\text{p.u.}} = \frac{Z_{\text{actual}}}{Z_{\text{base}}}$$

where:

$$Z_{\text{base}} = \frac{V_{\text{base}}^2}{S_{\text{base}}}$$

**Example**: 345 kV system, 100 MVA base:

$$Z_{\text{base}} = \frac{(345 \times 10^3)^2}{100 \times 10^6} = 1190 \text{ } \Omega$$

A line with $Z = 11.9 + j119$ $\Omega$ becomes $Z_{\text{p.u.}} = 0.01 + j0.1$ p.u.

**Advantage**: Per-unit values are similar across voltage levels, making it easy to spot unusual values.

See [Units & Conventions](@/reference/units-conventions.md) for details.

---

## Summary Table

| Quantity | Symbol | Formula | Unit | Physical Meaning |
|----------|--------|---------|------|------------------|
| Resistance | $R$ | — | $\Omega$ | Energy dissipation |
| Reactance | $X$ | $\omega L$ or $-1/\omega C$ | $\Omega$ | Energy storage |
| Impedance | $\mathbf{Z}$ | $R + jX$ | $\Omega$ | Opposition to current |
| Conductance | $G$ | $R/(R^2+X^2)$ | S | Ease of real current |
| Susceptance | $B$ | $-X/(R^2+X^2)$ | S | Ease of reactive current |
| Admittance | $\mathbf{Y}$ | $G + jB = 1/\mathbf{Z}$ | S | Ease of current flow |

---

## Key Takeaways

1. **Impedance** $\mathbf{Z} = R + jX$ combines resistance (losses) and reactance (storage)
2. Transmission lines have $X >> R$ — reactance dominates
3. **Admittance** $\mathbf{Y} = 1/\mathbf{Z} = G + jB$ is used for network analysis
4. Line charging $B$ causes reactive power generation on lightly-loaded lines
5. Per-unit values normalize across voltage levels

---

## See Also

- [Y-Bus Matrix](@/reference/ybus-matrix.md) — Building the network admittance matrix
- [Power Flow Theory](@/reference/power-flow.md) — How impedance affects power flow
- [Complex Power](@/reference/complex-power.md) — Power flowing through impedances
- [Units & Conventions](@/reference/units-conventions.md) — Per-unit system
