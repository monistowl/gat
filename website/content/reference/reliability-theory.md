+++
title = "Reliability Theory"
description = "Mathematical foundations of power system reliability assessment"
weight = 7
+++

# Reliability Theory

This reference explains the mathematics behind power system reliability assessment — quantifying the risk of not meeting electricity demand.

---

## What is Reliability?

Power system reliability has two aspects:

1. **Adequacy**: Are there enough resources (generation, transmission) to meet demand?
2. **Security**: Can the system withstand disturbances without cascading failures?

This reference focuses on **adequacy assessment** — the probability-based analysis of supply sufficiency.

---

## Fundamental Concepts

### Loss of Load

**Loss of Load (LOL)** occurs when available generation cannot meet demand:

```
LOL event: Available Capacity < Demand
```

This can happen due to:
- Generator outages (forced or planned)
- Transmission constraints
- Demand exceeding forecasts
- Renewable shortfalls

### Capacity States

Each generator has two states:
- **Available**: Operating or ready to operate
- **Unavailable**: Failed or under maintenance

For a system with n generators, there are 2ⁿ possible **capacity states**.

### Forced Outage Rate (FOR)

The probability a unit is unavailable due to unplanned failure:

```
FOR = (Forced Outage Hours) / (Service Hours + Forced Outage Hours)
```

Typical values:
- Coal/gas units: 2-10%
- Nuclear: 3-8%
- Hydro: 1-3%
- Wind/solar: Captured via capacity factor, not FOR

---

## Reliability Indices

### LOLE — Loss of Load Expectation

The expected number of hours (or days) per year when load exceeds available capacity:

```
LOLE = Σᵢ pᵢ · tᵢ
```

where:
- pᵢ = probability of capacity state i
- tᵢ = duration of load loss in state i (hours)

**Planning standard**: LOLE ≤ 0.1 days/year (2.4 hours/year) is common in North America.

**Interpretation**: On average, there will be 2.4 hours per year when some load cannot be served. This does NOT mean 2.4 hours of actual blackout — it's a probabilistic expectation.

### LOLP — Loss of Load Probability

The probability that load exceeds capacity at any given time:

```
LOLP = Σᵢ pᵢ    (for all states where capacity < demand)
```

Related to LOLE:
```
LOLE = LOLP × 8760 hours/year
```

### EUE/ENS — Expected Unserved Energy

The expected energy (MWh) not delivered per year:

```
EUE = Σᵢ pᵢ · (Dᵢ - Cᵢ) · tᵢ    (for states where Cᵢ < Dᵢ)
```

where:
- Dᵢ = demand in state i
- Cᵢ = available capacity in state i

**Interpretation**: Average annual energy shortfall. More meaningful than LOLE for economic analysis.

### LOLH — Loss of Load Hours

Same as LOLE but explicitly in hours:

```
LOLH = Σᵢ pᵢ · (hours in state i with LOL)
```

### Normalized Indices

For comparing systems of different sizes:

```
LOLE/peak = LOLE / Peak Demand
EUE% = EUE / Annual Energy Demand × 100%
```

---

## Analytical Methods

### Capacity Outage Probability Table (COPT)

For small systems, enumerate all capacity states:

**Example:** Two 100 MW units, each with FOR = 0.05

| State | Capacity | Probability |
|-------|----------|-------------|
| Both up | 200 MW | 0.95 × 0.95 = 0.9025 |
| Unit 1 down | 100 MW | 0.05 × 0.95 = 0.0475 |
| Unit 2 down | 100 MW | 0.95 × 0.05 = 0.0475 |
| Both down | 0 MW | 0.05 × 0.05 = 0.0025 |

### Convolution Method

For larger systems, build COPT incrementally using **convolution**:

Starting with capacity probability distribution p(C), add unit k with capacity cₖ and FOR qₖ:

```
p_new(C) = (1-qₖ)·p_old(C-cₖ) + qₖ·p_old(C)
```

This builds up the distribution one unit at a time without enumerating 2ⁿ states.

### Load Duration Curve

Load varies throughout the year. The **Load Duration Curve (LDC)** shows load ranked from highest to lowest:

```
     Load
     (MW)
       │     ╭──────╮
  Peak │─────╯       ╲
       │              ╲
       │               ╲
  Base │────────────────╲──────
       └──────────────────────── Hours
       0                    8760
```

### LOLE Calculation with LDC

For each capacity state, find hours where demand exceeds capacity:

```
LOLE = Σᵢ pᵢ · H(Cᵢ)
```

where H(C) = hours on LDC where load > C.

### EUE Calculation

```
EUE = Σᵢ pᵢ · E(Cᵢ)
```

where E(C) = area under LDC above capacity level C (MWh).

---

## Monte Carlo Simulation

For complex systems with dependencies, analytical methods become intractable. **Monte Carlo simulation** samples random scenarios.

### Sequential Monte Carlo

Simulates system operation chronologically:

1. **Initialize**: All units available, time = 0
2. **Sample next event**: Unit failure or repair (exponential distribution)
3. **Update state**: Mark unit up/down
4. **Check adequacy**: If capacity < demand, record LOL
5. **Advance time**: Move to next event or hour
6. **Repeat**: Until end of year
7. **Average**: Over many year replications

**Advantages:**
- Captures chronological effects (ramp limits, storage)
- Models dependent failures
- Handles maintenance schedules

**Disadvantages:**
- Computationally expensive
- Requires many samples for convergence

### Non-Sequential Monte Carlo

Samples independent hourly snapshots:

1. **For each hour** in the year:
   - Sample each unit state (Bernoulli with FOR)
   - Sum available capacity
   - Compare to demand
   - Record LOL if capacity < demand
2. **Repeat**: Many times (1000+ replications)
3. **Average**: Compute expected values

**Advantages:**
- Much faster than sequential
- Easily parallelizable
- Sufficient for adequacy assessment

**Disadvantages:**
- Ignores chronological dependencies
- Can't model storage or ramps

### Convergence

Standard error decreases as 1/√N:

```
SE(LOLE) ≈ σ / √N
```

For 1% relative error with LOLE ≈ 2.4 hours:
- Need ~10,000 samples
- Or use variance reduction techniques

### GAT Implementation

```rust
use gat_algo::reliability::{MonteCarlo, ReliabilityMetrics};

let mc = MonteCarlo::new(num_scenarios);
let metrics: ReliabilityMetrics = mc.compute_reliability(&network, seed)?;

println!("LOLE: {:.2} hours/year", metrics.lole);
println!("EUE: {:.0} MWh/year", metrics.eue);
```

---

## Multi-Area Reliability

Real power systems span multiple interconnected areas.

### Corridor Constraints

Areas are connected by **tie-lines (corridors)** with limited transfer capacity:

```
Area A ═══════╦═══════ Area B
              ║
         Transfer limit
           (e.g., 500 MW)
```

Power can flow between areas, but only up to corridor limits.

### Multi-Area LOLE

Each area has its own LOLE, affected by:
- Local generation adequacy
- Import capability from neighbors
- Neighbor's adequacy (can they export?)

```
LOLE_A = f(local capacity, import limit, availability of imports)
```

### Coordinated Assessment

The multi-area problem considers:
1. Sample capacity states in all areas
2. Compute optimal power transfers (respecting limits)
3. Determine LOL in each area after transfers
4. Aggregate across scenarios

### GAT Multi-Area Implementation

```rust
use gat_algo::{MultiAreaSystem, MultiAreaMonteCarlo};

let mut system = MultiAreaSystem::new();
system.add_area(AreaId(0), network_a)?;
system.add_area(AreaId(1), network_b)?;
system.add_corridor(Corridor::new(0, AreaId(0), AreaId(1), 500.0))?;

let mc = MultiAreaMonteCarlo::new(1000);
let metrics = mc.compute_multiarea_reliability(&system)?;

println!("Area A LOLE: {:.2}", metrics.area_lole[&AreaId(0)]);
println!("Area B LOLE: {:.2}", metrics.area_lole[&AreaId(1)]);
```

---

## ELCC — Effective Load Carrying Capability

ELCC measures the capacity value of variable resources.

### Definition

**ELCC** = Additional load the system can serve at the same reliability level when a resource is added.

```
ELCC(resource) = Load_with_resource - Load_without_resource
                 (at constant LOLE)
```

### Calculation Method

1. Compute base case LOLE with existing system
2. Add new resource (e.g., 100 MW wind farm)
3. Increase load until LOLE returns to base level
4. ELCC = load increase amount

### Capacity Credit

```
Capacity Credit = ELCC / Nameplate Capacity × 100%
```

Typical values:
- Thermal units: 90-95%
- Wind: 10-30%
- Solar: 30-70% (depends on peak timing)
- Storage: Varies with duration

### Why ELCC < Nameplate for Renewables

Variable resources aren't always available when needed:
- Wind may be calm during peak demand
- Solar unavailable at night (evening peaks)
- Correlation with system stress matters

---

## Distribution Reliability Indices

For distribution systems serving end customers:

### SAIDI — System Average Interruption Duration Index

Average outage duration per customer:

```
SAIDI = Σ(Customer Interruption Durations) / Total Customers
```

Units: minutes or hours per customer per year.

### SAIFI — System Average Interruption Frequency Index

Average number of outages per customer:

```
SAIFI = Σ(Customer Interruptions) / Total Customers
```

Units: interruptions per customer per year.

### CAIDI — Customer Average Interruption Duration Index

Average duration of an interruption:

```
CAIDI = SAIDI / SAIFI = Σ(Durations) / Σ(Interruptions)
```

Units: minutes or hours per interruption.

### CAIFI — Customer Average Interruption Frequency Index

Average interruption frequency for affected customers:

```
CAIFI = Σ(Interruptions) / Customers Affected
```

---

## N-1 and N-2 Criteria

Deterministic security standards complement probabilistic reliability.

### N-1 Criterion

The system must survive any single contingency:
- Loss of one generator
- Loss of one transmission line
- Loss of one transformer

Without:
- Overloads on remaining elements
- Voltage violations
- Loss of load

### N-2 Criterion

For critical facilities, survive two simultaneous failures:
- Double-circuit tower collapse
- Substation busbar fault
- Common-mode failures

### Relationship to LOLE

N-1/N-2 are **deterministic** (pass/fail).
LOLE is **probabilistic** (expected value).

Both are needed:
- N-1 ensures immediate security
- LOLE ensures long-term adequacy

---

## Practical Considerations

### Data Requirements

Reliability assessment requires:
- Generator capacities and FOR values
- Load forecast (hourly for 8760 hours)
- Transmission limits (for multi-area)
- Maintenance schedules (planned outages)
- Renewable profiles (for ELCC)

### Sensitivity Analysis

Key sensitivities to test:
- FOR uncertainty (±20% typical)
- Load forecast error
- Renewable correlation assumptions
- Transmission limit changes

### Computational Efficiency

For large systems:
- Use variance reduction (importance sampling, control variates)
- Parallel Monte Carlo across scenarios
- Smart sampling (focus on high-impact states)

GAT uses non-sequential Monte Carlo with parallel scenario evaluation.

---

## Mathematical Appendix

### Exponential Failure Model

Time to failure follows exponential distribution:

```
P(failure before time t) = 1 - e^(-λt)
```

where λ = failure rate (failures/hour).

Mean time to failure:
```
MTTF = 1/λ
```

FOR relationship:
```
FOR = MTTR / (MTTF + MTTR)
```

where MTTR = mean time to repair.

### Markov Model for Two-State Unit

States: Up (1), Down (0)

Transition rates:
- λ: failure rate (Up → Down)
- μ: repair rate (Down → Up)

Steady-state probabilities:
```
P(Up) = μ / (λ + μ)
P(Down) = λ / (λ + μ) = FOR
```

### Convolution Formula Derivation

If C₁ has distribution p₁(c) and C₂ has distribution p₂(c), the sum C = C₁ + C₂ has:

```
p(c) = Σₓ p₁(x) · p₂(c - x)
```

This is the discrete convolution of p₁ and p₂.

---

## References

### Textbooks

- **Billinton & Allan**, *Reliability Evaluation of Power Systems* — The standard reference
- **Billinton & Allan**, *Reliability Evaluation of Engineering Systems* — General reliability theory
- **Endrenyi**, *Reliability Modeling in Electric Power Systems* — Advanced topics

### Standards

- **IEEE 762**: Standard Definitions for Use in Reporting Electric Generating Unit Reliability
- **NERC TPL**: Transmission Planning Standards
- **NERC MOD**: Modeling Standards

### GAT Documentation

- [Reliability Guide](/guide/reliability/) — Practical usage
- [Analytics Reference](/reference/analytics/) — Reliability metrics in GAT
- [Glossary](/reference/glossary/) — Term definitions
