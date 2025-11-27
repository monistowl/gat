+++
title = "Units & Conventions"
description = "Per-unit system, sign conventions, and base values in power systems"
weight = 2
+++

# Units & Conventions

Power systems use specialized unit systems and conventions that can confuse newcomers. This reference explains the per-unit system, sign conventions, and standards used in GAT.

---

## The Per-Unit System

### Why Per-Unit?

Power systems span enormous ranges:
- Voltages from 120 V to 765,000 V
- Powers from kW to GW
- Impedances from milliohms to thousands of ohms

The **per-unit (p.u.) system** normalizes all quantities to dimensionless ratios, providing:

1. **Simplified calculations**: Transformer ratios disappear; values are comparable across voltage levels
2. **Numerical stability**: All quantities are O(1), avoiding floating-point issues
3. **Quick sanity checks**: Normal voltages are ~1.0 p.u., impedances are ~0.01-0.1 p.u.
4. **Standard data formats**: MATPOWER, PSS/E, and GAT all use per-unit

### Base Quantities

The per-unit system requires choosing **base values**. Once you pick two, the rest are determined:

| Base Quantity | Symbol | Typical Choice |
|---------------|--------|----------------|
| Base power | S_base | 100 MVA (system-wide) |
| Base voltage | V_base | Nominal voltage at each bus |

**Derived bases:**

```
I_base = S_base / (√3 · V_base)     [for three-phase]
Z_base = V_base² / S_base
Y_base = S_base / V_base² = 1/Z_base
```

### Converting to Per-Unit

```
X_pu = X_actual / X_base
```

**Example:** A 345 kV line with X = 50 Ω on a 100 MVA base:

```
Z_base = (345 kV)² / 100 MVA = 1190.25 Ω
X_pu = 50 / 1190.25 = 0.042 p.u.
```

### Converting from Per-Unit

```
X_actual = X_pu · X_base
```

**Example:** A generator producing 0.8 p.u. real power on a 100 MVA base:

```
P_actual = 0.8 × 100 MVA = 80 MW
```

### Multi-Voltage Networks

Each voltage level has its own V_base (the nominal voltage), but S_base is the same everywhere. This makes transformer modeling elegant:

- Impedances referred to either side use that side's Z_base
- Ideal transformers have 1:1 ratio in per-unit (tap ratio handles off-nominal)

---

## GAT Base Conventions

### System Base

GAT reads base MVA from the network's `system.arrow` table:

| Column | Description | Default |
|--------|-------------|---------|
| `base_mva` | System MVA base | 100.0 |
| `base_frequency_hz` | System frequency | 60.0 |

All per-unit quantities in GAT use this base.

### Voltage Bases

Each bus has a nominal voltage in `buses.arrow`:

| Column | Description |
|--------|-------------|
| `voltage_kv` | Nominal voltage (kV) — this is V_base for the bus |
| `voltage_pu` | Actual voltage magnitude in per-unit |
| `angle_rad` | Voltage angle in radians |

### Impedance Convention

Branch impedances in `branches.arrow` are in per-unit on the system base:

| Column | Description |
|--------|-------------|
| `resistance` | R in p.u. on S_base |
| `reactance` | X in p.u. on S_base |
| `charging_b_pu` | Total line charging B in p.u. |

For transformers, impedances are typically given on the transformer's own MVA rating. GAT expects them converted to the system base:

```
Z_pu_system = Z_pu_nameplate × (S_base / S_nameplate)
```

---

## Sign Conventions

### Generator Convention (Source)

Generators use **generator convention**: positive P and Q mean power flowing **out** of the device into the network.

```
P_gen > 0 → producing real power (normal operation)
Q_gen > 0 → producing reactive power (overexcited, exporting VARs)
Q_gen < 0 → absorbing reactive power (underexcited)
```

### Load Convention (Sink)

Loads use **load convention**: positive P and Q mean power flowing **into** the device from the network.

```
P_load > 0 → consuming real power (normal operation)
Q_load > 0 → consuming reactive power (inductive load)
Q_load < 0 → producing reactive power (capacitive load, rare)
```

### Net Injection

Power flow equations use **net injection** at each bus:

```
P_inj = P_gen - P_load
Q_inj = Q_gen - Q_load
```

Positive injection = net generation at the bus.

### Branch Flow Direction

Branch flows use the **from-to convention**:

```
P_ij > 0 → power flowing from bus i to bus j
P_ij < 0 → power flowing from bus j to bus i
```

Due to losses, P_ij + P_ji ≠ 0 (the difference is I²R loss).

---

## Angle Conventions

### Reference Angle

The **slack bus** (reference bus) has angle θ = 0. All other angles are relative to this reference.

- Positive angles: bus leads the reference
- Negative angles: bus lags the reference

### Units

GAT stores angles in **radians** internally (`angle_rad` column). Output may be converted to degrees for display.

```
θ_deg = θ_rad × (180/π)
```

Typical transmission angles: ±30° (±0.52 rad) under normal operation.

### Angle Differences

Power flow depends on angle **differences**, not absolute angles:

```
P_ij ∝ sin(θ_i - θ_j)
```

The choice of reference only affects absolute values, not flows.

---

## Power Conventions

### Three-Phase vs. Single-Phase

Power system quantities are typically **three-phase totals** unless noted:

```
S_3φ = √3 · V_LL · I_L
```

GAT uses three-phase quantities throughout. Single-phase equivalents (common in textbooks) differ by factors of 3 or √3.

### Complex Power

Complex power S combines real (P) and reactive (Q):

```
S = P + jQ
```

| Component | Symbol | Units | Physical Meaning |
|-----------|--------|-------|------------------|
| Apparent | S, \|S\| | VA, MVA | Total current-carrying requirement |
| Real | P | W, MW | Useful work, energy transfer |
| Reactive | Q | VAR, MVAR | Energy oscillation, no net transfer |

### Power Factor

```
pf = P / |S| = cos(φ)
```

where φ is the angle between voltage and current.

- **Lagging pf**: Current lags voltage (inductive load, Q > 0)
- **Leading pf**: Current leads voltage (capacitive load, Q < 0)
- **Unity pf**: P = |S|, Q = 0

---

## Impedance Conventions

### Series vs. Shunt

**Series elements** (lines, transformers) have:
- R: resistance (causes real power loss)
- X: reactance (limits power transfer)
- Combined: Z = R + jX

**Shunt elements** (capacitors, reactors, line charging) have:
- G: conductance (rare, represents corona/leakage)
- B: susceptance (main component)
- Combined: Y = G + jB

### Inductive vs. Capacitive

| Element | Reactance | Susceptance |
|---------|-----------|-------------|
| Inductor | X > 0 | B < 0 |
| Capacitor | X < 0 | B > 0 |

Lines and transformers are inductive (X > 0). Line charging is capacitive (B > 0).

### The π-Model

Transmission lines use the **π-equivalent circuit**:

```
    Bus i                    Bus j
      o──────┬──[R+jX]──┬──────o
             │          │
            jB/2       jB/2
             │          │
            ═╧═        ═╧═
```

- Series impedance: Z = R + jX
- Shunt admittance: B/2 at each end (line charging)

B is the **total** line charging; each end gets half.

---

## Transformer Conventions

### Tap Ratio

The tap ratio `a` relates primary and secondary voltages:

```
V_primary = a · V_secondary (ideal transformer)
```

- `a = 1.0`: Nominal tap position
- `a > 1.0`: Step-up (or boost on regulated side)
- `a < 1.0`: Step-down (or buck on regulated side)

GAT's `tap_ratio` in `branches.arrow` uses this convention.

### Off-Nominal Taps in Y-bus

For a transformer from bus i to bus j with tap ratio a and impedance Z:

```
Y_ii += y/a²
Y_jj += y
Y_ij = Y_ji = -y/a
```

where y = 1/Z.

Note: Off-nominal taps make Y-bus asymmetric if the tap is not at 1.0.

### Phase Shifters

Phase-shifting transformers add an angle shift:

```
V_i = a·e^(jφ) · V_j
```

The phase shift `φ` (in radians) controls real power flow direction. GAT stores this in `phase_shift_rad`.

---

## Common Pitfalls

### Mixing Bases

**Problem:** Combining data from different sources with different S_base.

**Solution:** Always convert to a common base:
```
Z_new_base = Z_old_base × (S_new / S_old)
```

### Forgetting √3

**Problem:** Using single-phase formulas for three-phase systems.

**Solution:** Remember:
- Line-to-line voltage = √3 × line-to-neutral voltage
- Three-phase power = 3 × single-phase power

### Sign Errors

**Problem:** Confusing generator and load convention.

**Solution:**
- Generators: positive = producing
- Loads: positive = consuming
- Injections = generation - load

### Angle Units

**Problem:** Mixing radians and degrees.

**Solution:** GAT uses radians internally. Convert explicitly:
```
radians = degrees × π/180
degrees = radians × 180/π
```

---

## Quick Reference Tables

### Unit Prefixes

| Prefix | Symbol | Factor |
|--------|--------|--------|
| kilo | k | 10³ |
| mega | M | 10⁶ |
| giga | G | 10⁹ |

### Common Units

| Quantity | SI Unit | Power System Unit |
|----------|---------|-------------------|
| Voltage | V | kV |
| Current | A | A or kA |
| Power | W | MW, MVAR, MVA |
| Impedance | Ω | Ω or p.u. |
| Frequency | Hz | Hz |
| Angle | rad | rad or degrees |

### Typical Per-Unit Values

| Quantity | Normal Range | Alarm Range |
|----------|--------------|-------------|
| Voltage magnitude | 0.95 - 1.05 p.u. | < 0.90 or > 1.10 |
| Line reactance | 0.01 - 0.30 p.u. | — |
| Transformer reactance | 0.05 - 0.15 p.u. | — |
| Generator output | 0.3 - 1.0 p.u. of rating | — |

---

## See Also

- [Glossary](/reference/glossary/) — Term definitions
- [Power Flow Theory](/reference/power-flow/) — Equations using these conventions
- [Arrow Schema](/guide/arrow-schema/) — How GAT stores these values
