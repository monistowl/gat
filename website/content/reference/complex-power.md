+++
title = "Complex Power: Real, Reactive, and Apparent"
description = "Understanding P, Q, and S — the foundation of power systems analysis"
weight = 2
+++

# Complex Power: Real, Reactive, and Apparent

If you remember one thing from power systems, make it this: **power isn't just watts**. There are three types of power, and understanding them is the key to everything else.

---

## The Big Picture

In a DC circuit, power is simple: $P = V \times I$, measured in watts.

AC circuits are more interesting. Because voltage and current are sinusoids that can be out of phase with each other, power comes in three flavors:

| Type | Symbol | Unit | What It Does |
|------|--------|------|--------------|
| **Real Power** | $P$ | Watts (W) | Does useful work |
| **Reactive Power** | $Q$ | VAR | Sustains magnetic/electric fields |
| **Apparent Power** | $S$ | VA | Total power the system must deliver |

These three form a right triangle — the **power triangle**:

```
            S (Apparent)
           /|
          / |
         /  | Q (Reactive)
        /   |
       /θ___|
         P (Real)
```

The angle $\theta$ is the **power factor angle** — the phase difference between voltage and current.

---

## Real Power (P): The Useful Stuff

Real power is the power that does actual work: spinning motors, heating elements, running computers. It's what you pay for on your electricity bill.

$$P = V \cdot I \cdot \cos(\theta)$$

where:
- $V$ is RMS voltage magnitude
- $I$ is RMS current magnitude
- $\theta$ is the angle between voltage and current phasors

**Physical intuition**: Real power represents energy flowing *one way* — from generator to load — doing useful work along the way.

**Units**: Watts (W), kilowatts (kW), megawatts (MW)

---

## Reactive Power (Q): The Oscillating Energy

Here's where AC gets interesting. Inductors (motors, transformers) and capacitors temporarily store energy in magnetic and electric fields. This energy sloshes back and forth between the source and these elements 120 times per second (at 60 Hz).

$$Q = V \cdot I \cdot \sin(\theta)$$

**Reactive power does no useful work** — it just oscillates. But it's essential:
- Motors need reactive power to create magnetic fields
- Transmission lines have natural reactive losses
- Voltage levels depend on reactive power balance

**Sign convention**:
- $Q > 0$: **Lagging** (inductive) — current lags voltage. Motors, transformers, transmission lines.
- $Q < 0$: **Leading** (capacitive) — current leads voltage. Capacitor banks, lightly-loaded cables.

**Units**: VAR (volt-ampere reactive), kVAR, MVAR

### Why Care About Reactive Power?

Even though reactive power doesn't do work, it creates real current that:
1. **Heats up wires** (conductors see the full current, reactive or not)
2. **Uses up transformer capacity** (rated in MVA, not MW)
3. **Affects voltage** — too little reactive support causes voltage to sag

This is why utilities install capacitor banks and why generators have reactive power limits.

---

## Apparent Power (S): The Total Package

Apparent power is the magnitude of complex power — what the equipment must be rated to handle:

$$S = \sqrt{P^2 + Q^2} = V \cdot I$$

**Why it matters**: A transformer rated 100 MVA can deliver any combination of P and Q that satisfies $\sqrt{P^2 + Q^2} \leq 100$. If you're delivering 60 MW and 80 MVAR, that's 100 MVA — fully loaded even though only 60% is "real" power.

**Units**: VA, kVA, MVA

---

## Complex Power: Putting It Together

Engineers work with **complex power** $\mathbf{S}$, which elegantly captures both P and Q:

$$\mathbf{S} = P + jQ = \mathbf{V} \cdot \mathbf{I}^*$$

where $\mathbf{V}$ and $\mathbf{I}$ are complex phasors, and $*$ denotes complex conjugate.

In rectangular form:
$$\mathbf{S} = P + jQ$$

In polar form:
$$\mathbf{S} = |S| \angle \theta$$

where $|S| = \sqrt{P^2 + Q^2}$ and $\theta = \arctan(Q/P)$.

### The Complex Conjugate — Why?

You might wonder why $\mathbf{I}^*$ instead of $\mathbf{I}$. This convention ensures:
- Positive $P$ means power delivered from source to load
- Positive $Q$ means inductive (lagging) load

If voltage leads current by angle $\theta$: $\mathbf{V} = V\angle 0°$, $\mathbf{I} = I\angle(-\theta)$

Then: $\mathbf{S} = V\angle 0° \cdot I\angle(+\theta) = VI\angle\theta = VI\cos\theta + jVI\sin\theta = P + jQ$ ✓

---

## Power Factor

**Power factor** measures how effectively current delivers real power:

$$\text{pf} = \cos(\theta) = \frac{P}{S}$$

| Power Factor | Meaning |
|--------------|---------|
| 1.0 (unity) | All power is real; current perfectly in phase |
| 0.85 lagging | Typical industrial load (motors) |
| 0.95 leading | Capacitor-compensated load |
| 0.0 | Pure reactive; no useful work |

**Why utilities care**: Poor power factor means more current for the same real power, causing:
- Higher line losses ($I^2R$ losses)
- Larger conductor and transformer requirements
- Voltage drops

Many industrial customers pay **power factor penalties** if their pf drops below 0.9.

---

## Per-Unit Power

In power systems, we often express power in **per-unit** (p.u.):

$$S_{\text{p.u.}} = \frac{S_{\text{actual}}}{S_{\text{base}}}$$

Typical base: 100 MVA for transmission systems.

If $S_{\text{base}} = 100$ MVA, then:
- 50 MW = 0.5 p.u. real power
- 30 MVAR = 0.3 p.u. reactive power
- $\mathbf{S} = 0.5 + j0.3$ p.u.

See [Units & Conventions](/reference/units-conventions/) for details.

---

## Complex Power in Power Flow

The power flow equations express complex power balance at each bus:

$$S_i = V_i \sum_{k=1}^{n} Y_{ik}^* V_k^*$$

This expands to the real and reactive power equations you'll see in [Power Flow Theory](/reference/power-flow/):

$$P_i = \sum_{k=1}^{n} |V_i||V_k|(G_{ik}\cos\theta_{ik} + B_{ik}\sin\theta_{ik})$$

$$Q_i = \sum_{k=1}^{n} |V_i||V_k|(G_{ik}\sin\theta_{ik} - B_{ik}\cos\theta_{ik})$$

where $\theta_{ik} = \theta_i - \theta_k$ is the angle difference between buses.

---

## Key Takeaways

1. **Real power (P)** does useful work; **reactive power (Q)** sustains fields but doesn't work
2. **Apparent power (S)** determines equipment ratings: $S = \sqrt{P^2 + Q^2}$
3. **Complex power** $\mathbf{S} = P + jQ = \mathbf{V}\mathbf{I}^*$ elegantly captures both
4. **Power factor** $\cos\theta = P/S$ measures delivery efficiency
5. Poor power factor wastes current capacity and causes voltage problems

---

## See Also

- [Power Flow Theory](/reference/power-flow/) — How complex power flows through networks
- [Units & Conventions](/reference/units-conventions/) — Per-unit system
- [Impedance & Admittance](/reference/impedance-admittance/) — Circuit parameters affecting power flow
- [Glossary](/reference/glossary/) — Quick definitions
