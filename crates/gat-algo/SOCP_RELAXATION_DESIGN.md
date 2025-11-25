# SOCP Relaxation Solver Blueprint

This note captures a concrete implementation plan for the `OpfMethod::SocpRelaxation`
path in `gat_algo`. The goal is to move from the current placeholder to a production
solver that fits the existing OPF API while keeping numerical robustness and observability
for the CLI.

## Current State
- The OPF facade (`opf/mod.rs`) wires `OpfMethod::SocpRelaxation` to a
  `NotImplemented` error.
- `OpfSolution` already carries fields for P/Q injections, voltages, flows, LMPs,
  and constraint metadata that a convex AC relaxation can populate.
- The DC-OPF module demonstrates the expected control flow: extract network data,
  build a convex program with `good_lp`, solve, and map the result into `OpfSolution`.

## Target Model
Implement the standard branch-flow SOCP relaxation (Baran-Wu / Farivar-Low style)
for radial and lightly meshed networks:
- Variables per branch: real/reactive flows `(P_ij, Q_ij)`, squared current `l_ij`,
  squared sending-end voltage `v_i`.
- Variables per bus: squared voltage magnitude `v_i` and angle proxy (optional for
  reporting; not needed for constraints).
- Objective: minimize total generation cost plus optional loss penalty
  (e.g., `c0 + c1*P_g + c2*P_g^2 + w_loss * sum(r_ij * l_ij)`).
- Constraints:
  - Power balance at each bus: `P_g - P_d = sum_out P_ij - sum_in (P_ij - r_ij*l_ij)`
    (and similarly for `Q`).
  - Voltage drop: `v_j = v_i - 2*(r_ij*P_ij + x_ij*Q_ij) + (r_ij^2 + x_ij^2)*l_ij`.
  - Thermal limit: `l_ij <= (S_max_ij)^2`.
  - Second-order cone: `P_ij^2 + Q_ij^2 <= v_i * l_ij`.
  - Voltage bounds: `v_min^2 <= v_i <= v_max^2`.
  - Generator bounds for `P/Q`.

## Data Extraction Layer
Re-use the patterns from `dc_opf.rs`:
- Build `BusData`, `GenData`, and `BranchData` structs but include resistance `r`,
  reactance `x`, shunt admittance, and thermal limits.
- Precompute load aggregation per bus for both `P` and `Q`.
- Validate assumptions: non-zero impedance, voltage setpoints within limits, and
  connectivity (warn if mesh cycles appear because relaxation is tightest on radial
  graphs).

## Solver Construction (good_lp with Clarabel)
1. **Variables**
   - `v[bus]` for squared voltage.
   - `p_gen[gen]`, `q_gen[gen]` for generator setpoints.
   - `p_flow[branch]`, `q_flow[branch]`, `l_current[branch]` for branch flows.
2. **Objective**
   - Sum polynomial generator costs using linear/quadratic terms supported by
     `good_lp` (Clarabel handles convex quadratics).
   - Add optional loss penalty `sum(r_ij * l_ij)` to promote tightness.
3. **Constraints**
   - Linear equalities for nodal power balance (active/reactive).
   - Voltage drop equations along each branch.
   - Bounds for voltages, generator P/Q, and current magnitudes.
   - SOCP constraints via `constraint!(p_flow[i]*p_flow[i] + q_flow[i]*q_flow[i] <= v[from]*l_current[i])`.
4. **Reference bus handling**
   - Fix `v` at slack bus to nominal (`|V|^2 = 1.0`) and optionally fix angle to
     zero for reporting.
5. **Solve**
   - Use `clarabel()` backend; capture solve time and status. Provide graceful
     error mapping for infeasibility/unbounded cases.

## Solution Mapping
- Populate `OpfSolution` fields from variable values:
  - `generator_p/q`, `bus_voltage_mag` (sqrt of `v`), `branch_p_flow/q_flow`.
  - Approximate angles using linearized PTDF if desired for display (not required
    for feasibility).
- Compute `total_losses_mw = sum(r_ij * l_ij)`.
- Derive LMPs from dual variables on power-balance constraints (Clarabel exposes
  duals). Store under `bus_lmp`.
- Record `binding_constraints` by checking proximity to limits (e.g., 1e-4 gap) and
  carrying the dual shadow prices.

## Testing Strategy
- Unit tests on toy radial feeders to verify feasibility, voltage limits, and loss
  monotonicity relative to DC-OPF.
- Regression tests comparing objective gap vs. AC-OPF reference data (pglib). Use
  tolerance checks (e.g., <=3% gap) and ensure solver convergence.
- Property tests for invariants: zero-impedance branches rejected; removing all
  reactive loads reduces to DC-OPF within tolerance.

## Incremental Delivery Steps
1. Implement network extraction structs and validation helpers.
2. Build SOCP model with `good_lp`/Clarabel and map variables to indices.
3. Wire the solver into `opf::OpfSolver::solve` with telemetry (iterations/time).
4. Add solution-mapping utilities and LMP extraction.
5. Write integration tests in `crates/gat-algo/tests/` with small systems and
   snapshot expected outputs.
6. Expose CLI flag documentation in `docs/guide/opf.md` once the solver is live.

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
