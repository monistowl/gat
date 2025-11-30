+++
title = "Locational Marginal Pricing (LMP)"
description = "How electricity prices vary by location due to losses and congestion"
weight = 9
+++

# Locational Marginal Pricing (LMP)

In wholesale electricity markets, prices aren't uniform — they vary by location. The **Locational Marginal Price (LMP)** at each bus is the cost of serving one additional megawatt of load at that location.

LMP is the economic signal that drives efficient generation dispatch, transmission investment, and demand response.

<div class="grid-widget" data-network="lmp-example" data-height="380" data-lmp="true" data-flow="true" data-legend="true" data-caption="Interactive: Click $ to see LMP prices (green=low, red=high). Red branch shows congestion. Click ⚡ to see power flow."></div>

---

## Why Prices Differ by Location

Two physical phenomena cause price separation:

### 1. Transmission Losses

Delivering power costs energy — wires have resistance, and $I^2R$ losses dissipate power as heat.

**Example**: To deliver 100 MW to a distant load when losses are 5%, you need to generate 105 MW. The price at the load bus should reflect these losses.

### 2. Transmission Congestion

When a transmission line reaches its thermal limit, cheap generation on one side can't reach load on the other side. More expensive local generation must run instead.

**Example**: A 50 MW line connects cheap coal ($20/MWh) to expensive gas ($60/MWh). When flow hits 50 MW:
- On the coal side: additional load could use more cheap coal → LMP ≈ $20
- On the gas side: additional load needs expensive gas → LMP ≈ $60

Congestion creates **price separation** — different LMPs on either side of a congested line.

---

## The LMP Formula

LMP at bus $i$ decomposes into three components:

$$\text{LMP}_i = \lambda + \lambda_{\text{loss},i} + \lambda_{\text{cong},i}$$

where:
- $\lambda$ = **Energy component** (system marginal cost)
- $\lambda_{\text{loss},i}$ = **Loss component** (marginal cost of losses to serve bus $i$)
- $\lambda_{\text{cong},i}$ = **Congestion component** (shadow prices of binding constraints)

### Energy Component

The base cost of energy, equal to the most expensive generator currently dispatched (the **marginal unit**). Same at all buses.

### Loss Component

Reflects how losses change when load at bus $i$ increases:

$$\lambda_{\text{loss},i} = \lambda \times \frac{\partial P_{\text{loss}}}{\partial P_i}$$

Buses electrically distant from generation have higher loss components.

### Congestion Component

Reflects binding transmission constraints:

$$\lambda_{\text{cong},i} = \sum_k \mu_k \times \text{PTDF}_{k,i}$$

where:
- $\mu_k$ = shadow price (dual variable) of constraint $k$
- $\text{PTDF}_{k,i}$ = sensitivity of flow on line $k$ to injection at bus $i$

---

## LMP from Optimal Power Flow

LMP emerges naturally from the OPF problem. Consider the simplified DC-OPF:

$$\min \sum_g c_g P_g$$

Subject to:

$$\sum_g P_g = \sum_d P_d + P_{\text{loss}} \quad (\lambda)$$
$$P_{\text{line},k} \leq P_{\max,k} \quad (\mu_k)$$
$$P_g^{\min} \leq P_g \leq P_g^{\max}$$

The **Lagrange multipliers** (dual variables) in parentheses have economic meaning:
- $\lambda$ = marginal cost of serving one more MW of total load
- $\mu_k$ = marginal cost of one more MW of transmission capacity on line $k$

**LMP at bus $i$** equals the change in total cost if load at bus $i$ increases by 1 MW:

$$\text{LMP}_i = \frac{\partial \text{Cost}}{\partial P_{d,i}} = \lambda + \sum_k \mu_k \cdot \text{PTDF}_{k,i}$$

---

## A Simple Example

Three buses, two generators, one constrained line:

```
   Gen A ($30/MWh)          Gen B ($50/MWh)
        [1]───────────────────[2]
                100 MW limit   │
                              [3] Load: 150 MW
```

**Uncongested case** (load = 80 MW):
- Gen A supplies all 80 MW (cheapest)
- LMP everywhere = $30/MWh

**Congested case** (load = 150 MW):
- Gen A supplies 100 MW (line limit)
- Gen B supplies 50 MW (expensive but unconstrained)
- LMP at bus 1 = $30 (could use more cheap Gen A)
- LMP at buses 2, 3 = $50 (marginal source is Gen B)

The $20 difference is the **congestion rent** — revenue collected from the price separation.

---

## Economic Interpretation

### For Generators

LMP tells you the value of your output:
- Generate when LMP > your marginal cost
- Don't generate when LMP < your marginal cost

**Revenue** = LMP × MWh generated

### For Loads

LMP is what you pay for electricity at your location:
- **Cost** = LMP × MWh consumed
- High LMP locations pay more (incentive to reduce demand or relocate)

### For Transmission Owners

Congestion creates **financial transmission rights (FTRs)**:
- Revenue = (LMP_sink - LMP_source) × MW
- When congestion exists, transmission rights are valuable

---

## LMP Components in Action

| Bus | Energy | Loss | Congestion | Total LMP |
|-----|--------|------|------------|-----------|
| Gen A | $30 | $0 | -$5 | $25 |
| Hub | $30 | $2 | $0 | $32 |
| Load | $30 | $5 | $15 | $50 |

**Interpretation**:
- Gen A gets $25 (below system price due to negative congestion component — can't export all it wants)
- Hub is roughly at system price with small losses
- Load pays $50 — premium for losses and congestion

---

## Market Settlement

Wholesale electricity markets settle based on LMPs:

**Day-Ahead Market**:
1. Generators offer supply curves (price vs. quantity)
2. Loads bid demand (or take price)
3. Market operator runs SCOPF to clear market
4. LMPs emerge as dual variables

**Real-Time Market**:
1. Balances deviations from day-ahead schedules
2. 5-minute LMPs reflect real-time conditions
3. Deviations settled at real-time LMP

### Example Settlement

Generator at $30/MWh cost, dispatched for 100 MW:
- LMP at their bus = $35/MWh
- Revenue = 100 MW × $35 = $3,500/hour
- Profit = $3,500 - $3,000 = $500/hour

This profit margin incentivizes efficient generation.

---

## Negative Prices

LMPs can go negative when:
- Renewable generation exceeds demand
- Generators have minimum output constraints (must-run)
- Transmission is congested (can't export surplus)

**Negative LMP means**: Pay someone to take your power!

Wind farms with production tax credits may still profit at negative prices, which has led to occasional -$100 to -$300/MWh prices in windy, uncongested areas.

---

## LMP in GAT

GAT's OPF produces LMPs as part of the solution:

```bash
gat opf network.arrow --method dc
```

Output includes:
- `lmp.arrow`: LMP at each bus
- Decomposition into energy, loss, congestion components
- Binding constraints and shadow prices

You can visualize LMP patterns:
```bash
gat viz network.arrow --color-by lmp
```

---

## Geographic Patterns

In real markets, LMPs show characteristic patterns:

**Load pockets**: Urban areas with limited transmission have higher LMPs due to congestion.

**Generation pockets**: Remote wind/solar farms may have lower (even negative) LMPs when they can't export.

**Hub prices**: Major interconnection points tend toward system average.

**Seasonal variation**: Summer peaks drive congestion in AC-heavy regions; winter peaks in heating regions.

---

## Key Takeaways

1. **LMP = Energy + Losses + Congestion** — price varies by location
2. **Congestion** creates price separation across constrained lines
3. **LMPs emerge from OPF** as dual variables (shadow prices)
4. **Markets settle at LMP** — generators paid, loads charged by location
5. **Investment signal** — high LMPs indicate where generation or transmission is needed

---

## See Also

- [OPF Formulations](@/reference/opf-formulations.md) — The optimization that produces LMPs
- [Contingency Analysis](@/reference/contingency-analysis.md) — Security constraints affect LMPs
- [Power Flow Theory](@/reference/power-flow.md) — Physical basis for losses and flows
- [Glossary](@/reference/glossary.md) — PTDF, congestion, marginal cost definitions
