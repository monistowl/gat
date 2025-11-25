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
