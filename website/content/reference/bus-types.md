+++
title = "Bus Types: Slack, PV, and PQ"
description = "Understanding what's known vs. solved at each bus in power flow"
weight = 4
+++

# Bus Types: Slack, PV, and PQ

Every bus in a power flow study is classified as one of three types: **Slack**, **PV**, or **PQ**. This classification determines which quantities you specify as inputs and which the solver computes.

Understanding bus types is essential for setting up power flow cases correctly and interpreting results.

<div class="grid-widget" data-network="three-bus" data-height="350" data-caption="Interactive: Click nodes to see bus properties. Green diamond = Slack, Red triangle = PV, Blue circle = PQ."></div>

---

## The Core Idea

At each bus, there are four electrical quantities:
- $P$ — real power injection (MW)
- $Q$ — reactive power injection (MVAR)
- $|V|$ — voltage magnitude (p.u. or kV)
- $\theta$ — voltage angle (degrees or radians)

Power flow is a system of $2n$ equations (for $n$ buses), so we need $2n$ unknowns. At each bus, we must specify **two** quantities and solve for the other **two**.

| Bus Type | Specified (Known) | Solved (Unknown) |
|----------|-------------------|------------------|
| **Slack (Ref)** | $\|V\|$, $\theta$ | $P$, $Q$ |
| **PV (Generator)** | $P$, $\|V\|$ | $Q$, $\theta$ |
| **PQ (Load)** | $P$, $Q$ | $\|V\|$, $\theta$ |

---

## PQ Bus: The Load Bus

**Most buses are PQ buses.** These represent:
- Load points (substations, industrial customers)
- Buses with no generation or voltage control

### What's Specified

You tell the solver:
- $P$ — real power consumed (negative injection)
- $Q$ — reactive power consumed

### What's Solved

The solver finds:
- $|V|$ — resulting voltage magnitude
- $\theta$ — resulting voltage angle

### Physical Interpretation

PQ buses model **constant power loads** — loads that draw a fixed amount of P and Q regardless of voltage (within reason). This is realistic for most loads over normal voltage ranges.

### Example

A substation with 50 MW load and 20 MVAR reactive consumption:
- $P = -50$ MW (negative = consuming)
- $Q = -20$ MVAR
- $|V|$ and $\theta$ = solved by power flow

---

## PV Bus: The Generator Bus

**Generator buses with voltage control are PV buses.** These represent:
- Power plants with automatic voltage regulators (AVRs)
- Synchronous condensers
- STATCOMs and other voltage-controlling devices

### What's Specified

You tell the solver:
- $P$ — real power output (dispatch setpoint)
- $|V|$ — voltage magnitude setpoint (what the AVR maintains)

### What's Solved

The solver finds:
- $Q$ — reactive power needed to maintain voltage
- $\theta$ — voltage angle

### Physical Interpretation

Generators have governors that control $P$ (real power output) and exciters/AVRs that control $|V|$ (terminal voltage). The reactive power $Q$ adjusts automatically to maintain the voltage setpoint.

**Key insight**: At a PV bus, reactive power is a *result*, not an input. The generator produces whatever $Q$ is needed to hold voltage at the setpoint.

### Reactive Limits

Real generators have reactive capability limits:

$$Q_{\min} \leq Q \leq Q_{\max}$$

If the solved $Q$ exceeds these limits, the bus **converts to PQ** at the violated limit:
- If $Q > Q_{\max}$: Fix $Q = Q_{\max}$, let $|V|$ drop
- If $Q < Q_{\min}$: Fix $Q = Q_{\min}$, let $|V|$ rise

This is called **PV-PQ switching** and is handled automatically by power flow solvers.

### Example

A 200 MW generator with voltage setpoint 1.02 p.u. and reactive limits ±100 MVAR:
- $P = 200$ MW
- $|V| = 1.02$ p.u.
- $Q$ = solved (say, 45 MVAR needed to maintain voltage)
- If $Q$ would need to be 150 MVAR, the bus switches to PQ with $Q = 100$ MVAR

---

## Slack Bus: The Reference

**Every power flow needs exactly one slack bus** (per synchronous island). This special bus:
- Sets the angle reference ($\theta = 0$)
- Absorbs power mismatch (losses + any imbalance)

### What's Specified

- $|V|$ — voltage magnitude (like a PV bus)
- $\theta$ — voltage angle (typically set to 0°)

### What's Solved

- $P$ — real power injection (whatever's needed to balance the system)
- $Q$ — reactive power injection

### Why Is It Necessary?

Two fundamental reasons:

**1. Angle Reference**

Voltage angles are only meaningful relative to a reference. Without fixing one angle, the solution is not unique (all angles could shift together).

**2. Loss Absorption**

The total generation must equal total load *plus losses*. But we don't know losses until we solve the power flow! The slack bus provides the "swing" generation to cover this unknown quantity.

$$P_{\text{slack}} = \sum P_{\text{load}} + P_{\text{losses}} - \sum P_{\text{other gen}}$$

### Choosing the Slack Bus

Typically the slack bus is:
- The largest generator (provides most "swing" capacity)
- The system's major interconnection point
- A bus near the electrical center of the network

**Practical tip**: Slack bus choice affects the solution distribution of losses but not the physics. If the slack $P$ comes out unreasonably large, your input data may be unbalanced.

### Example

A large coal plant serving as slack bus:
- $|V| = 1.0$ p.u.
- $\theta = 0°$ (by definition)
- $P$ = solved (e.g., 450 MW to balance the system)
- $Q$ = solved (e.g., 120 MVAR)

---

## Summary Comparison

| Aspect | PQ Bus | PV Bus | Slack Bus |
|--------|--------|--------|-----------|
| **Typical use** | Loads | Generators | Reference generator |
| **Knowns** | $P$, $Q$ | $P$, $\|V\|$ | $\|V\|$, $\theta$ |
| **Unknowns** | $\|V\|$, $\theta$ | $Q$, $\theta$ | $P$, $Q$ |
| **Controls** | Nothing | Voltage magnitude | Angle reference, power balance |
| **Count** | Most buses | Generator buses | Exactly one per island |

<div class="grid-widget" data-network="three-bus" data-height="280" data-highlight="1" data-caption="Slack bus (green) sets the angle reference and balances power"></div>

---

## The Mathematical Picture

Power flow solves $2n$ nonlinear equations. For an $n$-bus system with:
- 1 slack bus
- $n_G$ PV buses
- $n - 1 - n_G$ PQ buses

We have:
- $(n-1)$ unknown angles (slack angle fixed)
- $(n - 1 - n_G)$ unknown voltage magnitudes (slack and PV magnitudes fixed)

Total unknowns: $(n-1) + (n - 1 - n_G) = 2n - 2 - n_G$

The power balance equations provide:
- $(n-1)$ real power equations (slack P not constrained)
- $(n - 1 - n_G)$ reactive power equations (slack and PV bus Q not constrained)

The system is square: same number of equations and unknowns. ✓

---

## GAT Bus Types

In GAT's Arrow format, bus type is stored in the `type` column of `buses.arrow`:

| Value | Type | Description |
|-------|------|-------------|
| 1 | PQ | Load bus |
| 2 | PV | Generator bus |
| 3 | Slack | Reference bus |
| 4 | Isolated | Not connected |

When you run `gat pf`, GAT:
1. Reads bus types from the network
2. Sets up equations based on types
3. Handles PV→PQ switching if generators hit limits
4. Reports slack bus P and Q in the solution

---

## Common Mistakes

### Forgetting the Slack Bus

Power flow will fail with "no slack bus" or produce nonsense. Every synchronous island needs exactly one.

### Multiple Slack Buses

Having two slack buses over-constrains the problem. Only one bus can set the angle reference and absorb mismatch.

### Ignoring Reactive Limits

If a PV bus solution shows $Q$ outside limits, the voltage setpoint cannot be maintained. Enable PV-PQ switching or adjust limits.

### Wrong Sign Convention

Remember: positive injection means generation, negative means consumption.
- Load bus: $P < 0$, $Q < 0$ typically
- Generator bus: $P > 0$, $Q$ = whatever's needed

---

## Key Takeaways

1. **PQ buses** specify $P$ and $Q$; solve for $|V|$ and $\theta$
2. **PV buses** specify $P$ and $|V|$; solve for $Q$ and $\theta$
3. **Slack bus** specifies $|V|$ and $\theta$; solves for $P$ and $Q$
4. Every synchronous island needs **exactly one** slack bus
5. PV buses can convert to PQ when reactive limits bind

---

## See Also

- [Power Flow Theory](/reference/power-flow/) — The equations solved for each bus type
- [Newton-Raphson Method](/reference/newton-raphson/) — How the solver handles bus types
- [Y-Bus Matrix](/reference/ybus-matrix/) — Network representation used in power flow
- [Glossary](/reference/glossary/) — Quick definitions
