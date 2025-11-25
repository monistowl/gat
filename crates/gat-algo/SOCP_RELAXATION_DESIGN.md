# SOCP Relaxation Solver Implementation Guide

This document describes how to implement the `OpfMethod::SocpRelaxation` path in
`gat_algo`. It refines the earlier blueprint into concrete data structures,
solver construction steps, mapping logic, and testing guidance needed to deliver
production-quality support for the convex AC branch-flow relaxation.

## Goals and Non-Goals
- **Goals**: provide a numerically robust SOCP relaxation for AC OPF that fits
the existing `OpfSolver` interface, emits informative telemetry, and produces
outputs compatible with downstream CLI reporting (flows, voltages, losses, LMPs,
binding constraints).
- **Non-goals**: exact AC feasibility recovery, meshed network angle recovery,
or topology-reconfiguration heuristics. These can be layered later but should
not block initial delivery.

## High-Level Execution Flow
1. Extract network data from Arrow tables into solver-friendly structs.
2. Build a branch-flow SOCP with `good_lp` + Clarabel, capturing variable indices
   for solution mapping.
3. Solve and translate the result into `OpfSolution`, including duals for LMPs
   and metadata for binding constraints.
4. Expose the solver through `OpfSolver::solve` behind `OpfMethod::SocpRelaxation`
   and surface user-facing flags in the CLI.

## Data Extraction Layer
### New/Updated Structs
- `SocpBus` { `bus_id`, `v_min`, `v_max`, `pd`, `qd`, `is_slack`, `profile_index` }
- `SocpGen` { `gen_id`, `bus_index`, `p_min`, `p_max`, `q_min`, `q_max`,
  `cost_c0`, `cost_c1`, `cost_c2` }
- `SocpBranch` { `branch_id`, `from`, `to`, `r`, `x`, `b_shunt`, `rate_mva`,
  `tap_ratio` (default 1.0), `phase_shift` (radians), `is_in_service` }

### Extraction Steps
- Aggregate loads per bus (P/Q) and map generator rows to buses using existing
  indexing helpers in `opf/common.rs` (mirroring `dc_opf.rs`).
- Validate:
  - Non-zero `r + jx` magnitude and finite shunt values.
  - Voltage bounds with `v_min > 0` and `v_max >= v_min`.
  - At least one slack/reference bus; if multiple, pick the first and log a
    warning.
  - Warn when the graph is meshed; relaxation is tightest on radial systems.
- Precompute `s_max_sq = rate_mva.powi(2)` and `z_sq = r*r + x*x` to avoid repeat
  work during model build.

## Optimization Model
### Variables
- Per-bus: `v[i]` (squared voltage magnitude).
- Per-generator: `p_g[g]`, `q_g[g]`.
- Per-branch (directed from `from -> to`): `p_ij[b]`, `q_ij[b]`, `l_ij[b]`
  (squared current magnitude).

### Objective
- Sum quadratic generation costs: `c0 + c1 * p_g + c2 * p_g^2` using
  `good_lp::variables!` quadratic support with Clarabel backend.
- Optional loss penalty: `loss_weight * sum(r_ij * l_ij)` to encourage tightness
  (default `loss_weight = 0.0`).
- Telemetry: record objective contributions separately for costs and losses.

### Constraints
- **Slack Voltage Fixing**: `v[slack] = v_nom^2` (use per-bus nominal if
  available; default 1.0 pu).
- **Voltage Bounds**: `v_min^2 <= v[i] <= v_max^2`.
- **Generator Bounds**: `p_min <= p_g <= p_max`, `q_min <= q_g <= q_max`.
- **Power Balance (per bus)**:
  - `sum_out(p_ij) - sum_in(p_ij - r_ij * l_ij) + p_load - sum_gen(p_g) = 0`.
  - `sum_out(q_ij) - sum_in(q_ij - x_ij * l_ij) + q_load - sum_gen(q_g) = 0`.
- **Voltage Drop (per branch)**:
  - `v[to] = v[from] - 2*(r*p_ij + x*q_ij) + (r^2 + x^2) * l_ij`.
  - Apply tap ratio and phase shift by scaling flows/voltages on the sending
    side before substitution.
- **Thermal Limits**: `l_ij <= s_max_sq / v_base^2` (match existing per-unit
  convention in `dc_opf`).
- **SOCP**: `p_ij^2 + q_ij^2 <= v[from] * l_ij` via Clarabel’s cone interface
  (available through `good_lp::constraint!`).

### Numerical Safeguards
- Add small `eps_v = 1e-6` lower bound to `v` to avoid degeneracy when no voltage
  min is provided.
- Clamp shunt and tap values to reasonable ranges and emit warnings when
  corrected.
- Prefer `f64` throughout; ensure all costs and impedances are finite before
  model build.

## Solution Mapping
- Extract primal values into `OpfSolution`:
  - `generator_p/q` from `p_g`, `q_g`.
  - `bus_voltage_mag` = `sqrt(max(v[i], 0.0))`.
  - `branch_p_flow/q_flow` from `p_ij`, `q_ij` (respect direction stored in
    `SocpBranch`).
  - `losses_mw` = `sum(r_ij * l_ij)` and per-branch losses.
- Duals/LMPs:
  - Read duals of active power balance constraints for each bus; store in
    `bus_lmp` (convert Clarabel dual sign to economic LMP convention if needed).
  - Record `binding_constraints` when a bound is within tolerance `1e-4` and
    attach the corresponding dual.
- Status mapping:
  - Map Clarabel `SolverStatus` to `OpfStatus::{Optimal, Infeasible, Unbounded,
    Error}` with human-readable messages and solver runtime.

## Integration Points
- Add `mod socp_relaxation;` in `opf/mod.rs` and implement
  `OpfMethod::SocpRelaxation` branch to call `SocpRelaxationSolver::solve`.
- Place the implementation in `src/opf/socp_relaxation.rs` alongside `dc_opf.rs`
  to reuse shared utilities.
- Extend CLI (`crates/gat-cli`) to expose `--method socp-relaxation` and optional
  `--loss-weight`, `--socp-max-iter`, `--socp-tolerance` flags. Ensure default is
  backwards-compatible (keep existing methods unchanged).
- Emit telemetry via existing `OpfTelemetry` hooks: iterations, solve time,
  status, primal/dual residuals from Clarabel.

## Testing Plan
- **Unit Tests** (`crates/gat-algo/src/opf/socp_relaxation.rs`):
  - Radial 3-bus feeder with known feasible solution; assert voltage bounds and
    monotonic losses when `loss_weight` increases.
  - Generator limit saturation: enforce `p_max` tightness and verify binding flag
    is set with non-zero dual.
  - Slack handling: multiple slack rows → first chosen; verify fixed voltage.
- **Integration Tests** (`crates/gat-algo/tests/`):
  - Compare objective and losses against DC-OPF on the same case; assert SOCP
    objective <= DC objective + tolerance.
  - PGLib small cases (e.g., `case3`, `case14`): require solve status optimal and
    voltage within bounds; record snapshots of key outputs.
- **Property Tests** (proptest): removing all reactive data should reduce the
  SOCP solution close to DC-OPF within tolerance for flows and objective.

## Delivery Checklist
1. Implement extraction structs and validation helpers.
2. Build the SOCP model with variable/constraint bookkeeping and telemetry.
3. Map solutions (primal + dual) into `OpfSolution` with binding detection.
4. Wire into `OpfSolver` and CLI, feature-gate Clarabel if necessary.
5. Add tests (unit, integration, property) and update documentation/examples.
