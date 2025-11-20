# DERMS/ADMS Distribution Workflows – Agent Playbook

This document refines the proposed DERMS/ADMS/distribution workflows into explicit, small-step instructions for coding agents working on the **testing** branch. Treat DERMS/ADMS as named, parameterized workflows layered on existing PF/OPF/SE/TS kernels, with added distribution-specific modeling and a few new crates/solvers.

## Guiding intent
- Focus on offline/planning "digital twin" analytics (not realtime control).
- Surface everything via CLI namespaces that emit Parquet plus `run.json` manifests.
- Reuse PF/OPF/SE/TS kernels; wrap them in workflow-specific commands that accept configs + tables + scenarios and fan out runs.

## CLI namespaces to add
- `gat dist` – distribution-network modeling and OPF.
- `gat derms` – DER portfolio flexibility and scheduling workflows.
- `gat adms` – reliability, FLISR, Volt/VAR, and outage simulations.

## Data schemas to introduce
- `dist_nodes.parquet`: `node_id`, `phase` (A/B/C/ABC), `type` (load/source/slack/DER_agg), `v_min`, `v_max`, `load_p`, `load_q`, `feeder_id`.
- `dist_branches.parquet`: `branch_id`, `from_node`, `to_node`, `r`, `x`, `b`, `tap`, `status`, `thermal_limit`.
- Optional: `dist_caps.parquet` (capacitors/regulators), `dist_switches.parquet` (switching elements).
- `der_assets.parquet`: `asset_id`, `bus_id`, `phase`, `asset_type`, `p_min/p_max`, `q_min/q_max`, `ramp_up/ramp_down`, `energy_cap`, `soc_min/soc_max`, `efficiency`, `owner_id`, `agg_id`, `priority`, `cost_curve_id`, optional `telemetry_id`.
- Reliability tables: `reliability.parquet` (failure/repair rates), `switching_devices.parquet`, `outage_scenarios.parquet`.

## Stage-by-stage implementation plan
Each stage lists small, explicit tasks agents can execute sequentially. Keep changes scoped; land increments to the **testing** branch.

### Stage 0 – Plumbing and naming
1. Add crates `crates/gat-dist`, `crates/gat-derms`, `crates/gat-adms` with minimal lib + Cargo metadata.
2. Wire the new crates into `gat-cli` via Clap: add top-level subcommands `dist`, `derms`, `adms` parallel to `ts`, `pf`, `opf`.
3. Add shared Arrow/Polars schema module (e.g., `gat-schemas`) with the dist/DER/reliability tables; expose schema validation helpers.
4. Document namespace intent in `README` or `docs/` stub pages.

### Stage 1 – Minimal `gat dist` (balanced PF/OPF + host capacity)
1. Implement schema structs and validators for `dist_nodes`/`dist_branches`.
2. Add `gat dist import matpower` that reuses existing import path; allow feeder filtering/renaming.
3. Implement `gat dist pf` using existing balanced AC PF engine; default to radial/weakly meshed assumptions.
4. Implement `gat dist opf` as a wrapper over existing DC/AC OPF with added voltage and thermal constraints where available.
5. Implement `gat dist hostcap` that sweeps DER injections at selected nodes: incrementally call `dist opf` until infeasible; emit summary + per-scenario Parquets.
6. Add `docs/guide/dist.md` with sample commands and expected outputs.

### Stage 2 – `gat derms` core (assets, envelopes, scheduling)
1. Finalize `der_assets` schema + `gat derms validate-assets` command.
2. Implement `gat derms envelope`: for each grouping (`bus_id`, `agg_id`, or feeder), solve OPF feasibility at extremal (P, Q) combinations; collect vertices and optional convex hull; write `der_envelopes.parquet`.
3. Implement `gat derms schedule`: multi-period LP/QP with variables (P, Q, SOC) per asset/time, constraints from asset bounds + SOC dynamics + network limits via linearized OPF/PTDF; objective flag for cost vs. curtailment.
4. Implement `gat derms stress-test`: generate random/scripted price/load scenarios, call `schedule` repeatedly, and record violation/curtailment metrics.
5. Add `docs/guide/derms.md` and synthetic DER test datasets under `test_data/derms/`.

### Stage 3 – `gat adms` reliability and VVO/FLISR
1. Implement reliability table schemas and validators.
2. Implement `gat adms flisr-sim`: single-fault loop that runs PF/OPF, applies rule-based/MILP switching to restore service under radial constraints, and writes `flisr_runs` + `reliability_indices` (SAIDI/SAIFI/CAIDI).
3. Implement `gat adms outage-mc`: Poisson-sample outages from reliability data, run `flisr-sim` per sample, accumulate unserved energy and interruption metrics.
4. Implement `gat adms vvo-plan`: run `dist opf --objective vvo` on representative day-types; optimize taps/caps/DER Q; emit settings + performance metrics.
5. Optional: `gat adms state-estimation` wrapper that maps distribution measurements to existing WLS SE and optionally runs corrective `dist opf`.
6. Add `docs/guide/adms.md` with usage examples.

### Stage 4 – TUI integration and datasets
1. Extend `gat-tui` with feeder visualization (nodes/branches), DERMS dashboards (flexibility envelopes, schedules), and ADMS reliability panels (SAIDI/SAIFI, outage maps).
2. Register public datasets (`gat dataset public`) for IEEE 13/34-node feeders with synthetic DER assets.
3. Expand tutorials/cookbooks showing end-to-end flows (e.g., hosting-capacity sweep, DERMS envelopes, VVO planning).

## Execution patterns for workflows
- All new commands must follow existing GAT patterns: thin CLI over library code, Parquet outputs plus resumable `run.json`, and fan-out-friendly design for batch execution.
- Prefer incremental delivery: land CLI stubs with schema validation before deep solver work, then wire solvers.
- Leverage existing solvers (Clarabel + good_lp; PF/OPF kernels) before adding new dependencies; only introduce new crates when needed for unbalanced or SOCP distribution OPF.

## Notes for coding agents
- Keep planning docs inside `history/` (per repo guidance).
- Avoid adding TODO lists; if new work is discovered, create beads issues when available.
- Ensure new schemas and commands have minimal integration tests (CLI-level) and sample data under `test_data/`.
