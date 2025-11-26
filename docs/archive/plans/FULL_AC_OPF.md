---

# Roadmap: Full Nonlinear AC-OPF Solver

The SOCP path above delivers a convex relaxation; the next milestone is a
full-space nonlinear AC-OPF that shares data structures and CLI ergonomics with
the rest of `gat_algo`.

## Goals and Scope
- Solve full AC power flow equations with generator cost curves, voltage limits,
  thermal limits, and shunts.
- Support both polar and rectangular formulations (start with polar for clarity,
  enable rectangular for robustness on meshed/ill-conditioned cases).
- Provide warm-start hooks from PF/SOCP solutions and export LMPs, constraint
  binding info, and iteration telemetry for observability.

## Architecture Decisions
- **Backend:** Integrate IPOPT via `ipopt` crate (or `nlopt` fallback if IPOPT is
  unavailable). Keep the solver interface thin so future backends (e.g., HiGHS
  nonlinear extensions) can be swapped.
- **Model builder:** Implement a reusable `AcNlpBuilder` that consumes the same
  `BusData/BranchData/GenData` structs used by DC/SOCP paths and produces
  variable/constraint layouts plus Jacobian/Hessian callbacks.
- **Differentiation:** Use manual Jacobians for performance and numerical
  stability; keep an optional `finite_diff` mode for debugging.

## Mathematical Formulation (polar start)
- **Variables:** `V_i` magnitude, `theta_i` angle, `P_g/Q_g` for each generator,
  `P_ij/Q_ij` branch flows if using current-based constraints.
- **Power balance:** Standard AC nodal balance using bus admittance matrix with
  shunts; include load `P_d/Q_d` and fixed shunt susceptance.
- **Voltage limits:** `V_min <= V_i <= V_max` (box constraints); optionally
  relax bounds to avoid infeasibility and add penalties.
- **Thermal limits:** Enforce `|S_ij| <= S_max` (either as magnitude inequality
  or squared form) on both directions when limits are asymmetric.
- **Generator limits/costs:** Box constraints on `P/Q`; objective uses quadratic
  or piecewise-linear cost curves mapped from existing cost structs.

## Model Construction Pipeline
1. **Data prep:** Reuse DC/SOCP data aggregation; compute Y-bus and branch
   admittances once and cache for solver callbacks.
2. **Variable indexing:** Deterministic ordering for `V`, `theta`, `P_g`, `Q_g`,
   optional branch currents; store slices to reconstruct solutions quickly.
3. **Objective assembly:** Linear/quadratic generator costs + optional soft
   penalties (voltage slack, thermal slack) with configurable weights.
4. **Constraints:**
   - Equality: nodal power balance (active/reactive).
   - Inequality: voltage bounds, thermal bounds, generator limits, angle
     reference fixing (slack bus `theta=0`).
5. **Callbacks:** Implement `eval_f`, `eval_grad_f`, `eval_g`, `eval_jac_g`, and
   `eval_h` with sparse structures matching IPOPT expectations.

## Initialization and Warm Starts
- Default initialization: flat start (`V=1.0`, `theta=0`) and generator setpoints
  from dispatch defaults.
- Warm-start paths: accept prior AC PF solution, SOCP relaxation outputs, or DC
  OPF results; project onto feasibility before handing to IPOPT.
- Angle reference: explicitly pin slack angle to zero and shift all angles in
  warm-start vectors accordingly.

## Numerical Stability and Limits Handling
- Detect ill-conditioned branches (very low `x` or `r`) and either regularize or
  drop into a safeguarded rectangular formulation for those subgraphs.
- Add optional current-limit slack variables to avoid hard infeasibility and
  surface violations via penalties and binding metadata.
- Scale objective and constraint residuals (per-unit normalization) before
  handing them to IPOPT to reduce line search failures.

## Telemetry and Outputs
- Record iteration counts, convergence flags, primal/dual infeasibilities, and
  IPOPT termination reason; surface in `OpfSolution::metadata` and CLI output.
- Compute LMPs from duals of power-balance constraints; map to buses in the same
  shape as DC/SOCP results for downstream parity.
- Mark `binding_constraints` for voltage, thermal, and generator limits using
  dual magnitudes and residual checks.

## Testing and Validation
- **Unit tests:** small feeders/meshed cases with known AC PF solutions; verify
  feasibility, KCL residuals, and voltage bounds.
- **Regression:** compare objectives and dispatch vs. PGLib-OPF AC benchmarks;
  assert optimality gap vs. published solutions within tolerance.
- **Stress tests:** random load scaling, shunt toggles, and line outages to check
  solver robustness and warm-start pathways.
- **Cross-check:** confirm LMPs match duals from SOCP relaxation on cases where
  relaxation is tight.

## Milestones
1. Land `AcNlpBuilder` with Jacobian/Hessian plumbing and IPOPT wiring behind a
   feature flag.
2. Add solution mapping, telemetry, and CLI surface (`gat opf ac --solver
   ipopt`).
3. Implement warm-start ingestion from PF/SOCP results and soft-limit handling.
4. Harden against numerical issues (scaling, rectangular fallback).
5. Deliver PGLib regression suite and publish performance/accuracy metrics.
