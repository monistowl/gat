+++
title = "Contingency Analysis and N-1 Security"
description = "How power systems are designed to survive equipment failures"
weight = 8
+++

# Contingency Analysis and N-1 Security

The power grid must keep the lights on even when things break. **Contingency analysis** tests whether the system can survive equipment failures, and the **N-1 criterion** is the fundamental security standard.

---

## Why Contingencies Matter

Equipment fails. Transmission lines get struck by lightning. Generators trip offline. Transformers overheat. A secure power system must handle these events without:
- Cascading outages
- Widespread blackouts
- Equipment damage

The August 2003 Northeast blackout — 55 million people without power — demonstrated what happens when contingencies cascade out of control.

---

## The N-1 Criterion

**N-1 security** means the system must survive the loss of any single component:

> After any single credible contingency, the system must remain stable with no thermal overloads and voltages within limits.

If you have $N$ components (lines, generators, transformers), you must be able to lose any one and still operate safely.

### What Counts as a "Credible Contingency"?

Typically:
- Loss of any single transmission line
- Loss of any single generator
- Loss of any single transformer
- Loss of any single bus (rare, but included for critical substations)

**Not usually included**:
- Multiple simultaneous failures (covered by N-2)
- Extreme events (hurricanes, earthquakes)
- Common-mode failures (multiple lines on same tower)

### The Mathematical Statement

For all single contingencies $c \in C$:

**Before contingency (base case):**

$$P_{\text{line}} \leq P_{\max} \quad \text{(all lines)}$$
$$V_{\min} \leq V \leq V_{\max} \quad \text{(all buses)}$$

**After contingency $c$:**

$$P_{\text{line}}^{(c)} \leq P_{\max} \quad \text{(remaining lines)}$$
$$V_{\min}^{(c)} \leq V^{(c)} \leq V_{\max}^{(c)} \quad \text{(all buses)}$$

---

## How Contingency Analysis Works

### Step 1: Define Contingency List

Create a list of all contingencies to test:
- All transmission line outages
- All generator outages
- Critical transformer outages

A 1000-bus system might have 1500+ contingencies.

### Step 2: Solve Base Case

Run power flow for the intact system. Verify it's feasible (no violations).

### Step 3: Test Each Contingency

For each contingency:
1. **Remove the component** from the network model
2. **Re-solve power flow** (or use sensitivity factors)
3. **Check for violations**:
   - Line flows > thermal limit
   - Voltages outside [0.95, 1.05] p.u.
   - Generator reactive limits exceeded
4. **Record severity** of any violations

### Step 4: Report Results

List contingencies that cause violations, ranked by severity:
- Worst thermal overloads (% above limit)
- Worst voltage violations
- Number of cascading issues

---

## Screening with Sensitivity Factors

Running full AC power flow for 1500 contingencies is slow. **Sensitivity factors** provide quick approximations:

### PTDF (Power Transfer Distribution Factor)

How does power redistribute when a generator-load pattern changes?

$$\Delta P_{\text{line}} = \text{PTDF} \times \Delta P_{\text{injection}}$$

### LODF (Line Outage Distribution Factor)

If line $k$ trips, how does flow redistribute to other lines?

$$P_{\ell}^{\text{post}} = P_{\ell}^{\text{pre}} + \text{LODF}_{k \to \ell} \times P_k^{\text{pre}}$$

**LODF screening workflow**:
1. Compute LODFs once (offline)
2. For each contingency: multiply base flows by LODFs
3. Flag contingencies with potential violations
4. Run full AC power flow only for flagged cases

This reduces computation from 1500 power flows to perhaps 50.

---

## N-2 and Beyond

**N-2 security** requires surviving any two simultaneous failures:
- Two transmission lines
- One line + one generator
- Two generators

N-2 is required for:
- Extra-high voltage (EHV) lines
- Critical generation interconnections
- Major load centers

**N-k security** generalizes to $k$ simultaneous failures, but becomes combinatorially explosive:
- N-1: ~1500 contingencies
- N-2: ~1,000,000 contingencies
- N-3: ~500,000,000 contingencies

Practical N-2 analysis screens for "credible" N-2 events (related failures, common modes).

---

## Security-Constrained OPF

Standard OPF minimizes cost subject to base case constraints only. **Security-constrained OPF (SCOPF)** adds contingency constraints:

$$\min \sum_g c_g(P_g)$$

Subject to:

$$\text{Base case power flow constraints}$$
$$P_{\text{line}} \leq P_{\max} \quad \text{(base case)}$$
$$P_{\text{line}}^{(c)} \leq P_{\max} \quad \text{(all contingencies } c)$$

This ensures the dispatch is secure even if a contingency occurs.

### The Cost of Security

SCOPF solutions cost more than standard OPF because:
- Some cheap generation must be backed off to create margin
- More expensive units may need to run for post-contingency support
- Transmission constraints bind more tightly

The difference represents the **cost of security** — what we pay to avoid blackouts.

---

## Corrective vs. Preventive Actions

Two philosophies for handling contingencies:

### Preventive (Pre-contingency)

Operate so that **no action is needed** after a contingency — the system automatically stays within limits.

**Pros**: Simple, safe
**Cons**: More conservative, higher cost

### Corrective (Post-contingency)

Operate closer to limits, but have **automatic actions** ready:
- Generator runback
- Load shedding
- Line switching

**Pros**: More efficient base case operation
**Cons**: Requires fast automation, higher risk

Modern systems use a mix: preventive for severe contingencies, corrective for less critical ones.

---

## Real-Time Contingency Analysis

Utilities run contingency analysis continuously:

1. **State estimator** provides current system state
2. **Contingency analysis** tests all N-1 events
3. **Operator displays** show any violations
4. **Alarms** trigger if security margin is low

Cycle time: every 2-5 minutes.

If a contingency shows violations, operators take action:
- Redispatch generation
- Adjust voltage setpoints
- Call for emergency procedures

---

## Example: Line Outage

Consider a three-bus system with two parallel paths:

```
    Gen ──[Line 1]──┬── Load
                    │
         ──[Line 2]──┘
```

**Base case**: Each line carries 50% of the load (100 MW each), well within 150 MW limits.

**Contingency (Line 1 trips)**: All 200 MW must flow through Line 2.

**Result**: Line 2 overloads at 200 MW vs. 150 MW limit. **N-1 violation!**

**Solution**: Either:
- Reduce load to 150 MW (load shedding)
- Build a third parallel path (expansion)
- Install flow control (phase shifter)
- Accept the risk (if contingency is rare)

---

## Cascading Failures

The real danger is when one failure triggers another:

1. **Initial contingency**: Line A trips
2. **Overload**: Lines B and C now exceed limits
3. **Protection operates**: Line B trips on overcurrent
4. **Further overload**: Line C now at 200% limit
5. **Cascade**: Line C trips, island separates
6. **Frequency collapse**: Island has generation-load mismatch

N-1 security is designed to prevent step 2 — no overloads after the first failure.

---

## GAT Contingency Analysis

GAT's `gat-algo` crate includes contingency analysis:

```bash
gat contingency network.arrow --n1
```

This:
1. Identifies all single contingencies
2. Computes LODFs for screening
3. Runs AC power flow for critical contingencies
4. Reports violations and severity

Options:
- `--n2`: Test double contingencies
- `--thermal-limit 100`: Override default limits (%)
- `--voltage-limits 0.95,1.05`: Voltage bounds

---

## Key Takeaways

1. **N-1 criterion**: Survive any single equipment failure
2. **Contingency analysis** tests all credible outages
3. **LODFs** enable fast screening without full power flow
4. **Security-constrained OPF** embeds contingency constraints in dispatch
5. **Cascading failures** are why we need security margins

---

## See Also

- [Power Flow Theory](/reference/power-flow/) — The analysis run for each contingency
- [OPF Formulations](/reference/opf-formulations/) — Security-constrained optimization
- [Reliability Theory](/reference/reliability-theory/) — Probabilistic adequacy assessment
- [Glossary](/reference/glossary/) — N-1, LODF, PTDF definitions
