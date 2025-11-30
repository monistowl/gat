+++
title = "Reference & Theory"
description = "Power systems theory, algorithms, and technical reference"
template = "section.html"
sort_by = "weight"
+++

# Reference & Theory

Technical reference for power systems students, researchers, and practitioners.

## Quick Reference

- **[Glossary](glossary/)** — A-Z of power systems terminology
- **[Units & Conventions](units-conventions/)** — Per-unit system, sign conventions, base values

## Foundations

Start here if you're new to power systems:

- **[Complex Power](complex-power/)** — Real, reactive, and apparent power (P, Q, S)
- **[Impedance & Admittance](impedance-admittance/)** — R, X, Z, G, B, Y explained
- **[Bus Types](bus-types/)** — Slack, PV, and PQ buses
- **[Y-Bus Matrix](ybus-matrix/)** — Building the network admittance matrix

## Analysis Methods

- **[Power Flow Theory](power-flow/)** — Mathematical foundations of AC/DC power flow
- **[Newton-Raphson Method](newton-raphson/)** — The iterative solver for power flow
- **[OPF Formulations](opf-formulations/)** — DC-OPF, SOCP, and AC-OPF mathematics
- **[State Estimation Theory](state-estimation/)** — WLS estimation, bad data detection, observability

## Planning & Markets

- **[Contingency Analysis](contingency-analysis/)** — N-1 security and reliability
- **[LMP Pricing](lmp-pricing/)** — Locational marginal prices in electricity markets
- **[Reliability Theory](reliability-theory/)** — LOLE, EUE, Monte Carlo methods

## Algorithms

GAT implements established power systems algorithms:

### Power Flow
- **Newton-Raphson** — Standard AC power flow
- **Fast Decoupled** — P-θ/Q-V decoupled iteration
- **DC Power Flow** — Linearized real power flow

### Optimal Power Flow
- **DC-OPF** — Linear economic dispatch
- **SOCP Relaxation** — Convex branch-flow model (Farivar-Low)
- **AC-OPF** — Full nonlinear OPF (penalty L-BFGS + IPOPT backends)

### State Estimation
- **Weighted Least Squares** — Classical SE formulation
- **Bad Data Detection** — Chi-squared and largest normalized residual

### Reliability
- **Monte Carlo** — Sequential/non-sequential simulation
- **Analytical** — LOLE, EENS, EIR calculation
- **Multi-area** — Corridor-constrained adequacy

## Data Formats

- **[Arrow schema](../guide/arrow-schema/)** — Canonical folder layout with `system`, `buses`, `generators`, `loads`, and `branches`.
- **[gat convert format](../guide/convert/)** — Command-line helper that auto-detects imports and converts via Arrow when crossing MATPOWER, PSS/E, CIM, or PandaPower boundaries.
- **MATPOWER** — `.m` file format
- **PSS/E** — RAW file support

## Literature

Key references for implemented algorithms:

### Textbooks
- Grainger & Stevenson, *Power System Analysis*
- Wood, Wollenberg & Sheblé, *Power Generation, Operation and Control*
- Kundur, *Power System Stability and Control*

### Papers
- Zimmerman, Murillo-Sánchez, Thomas (2011) — MATPOWER
- Billinton & Allan — *Reliability Evaluation of Power Systems*
- NERC TPL Standards — Transmission Planning

## FAQ

See the **[FAQ](@/faq.md)** for common questions about power systems analysis and GAT usage.
