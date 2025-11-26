+++
title = "Reference & Theory"
description = "Power systems theory, algorithms, and technical reference"
template = "section.html"
sort_by = "weight"
+++

# Reference & Theory

Technical reference for power systems students, researchers, and practitioners.

## Algorithms

GAT implements established power systems algorithms:

### Power Flow
- **Newton-Raphson** — Standard AC power flow
- **Fast Decoupled** — P-θ/Q-V decoupled iteration
- **DC Power Flow** — Linearized real power flow

### Optimal Power Flow
- **DC-OPF** — Linear economic dispatch
- **AC-OPF** — Full nonlinear OPF (in development)

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

See the **[FAQ](/faq/)** for common questions about power systems analysis and GAT usage.
