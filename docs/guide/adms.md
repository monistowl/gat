# ADMS workflows (`gat adms`)

The `gat adms` namespace provides reliability-facing utilities that align with the ADMS feature list (FLISR, outage Monte Carlo, Volt/VAR planning, and SE wrappers). The current implementation focuses on schema validation and lightweight analytics stubs suitable for experimentation.

## Schemas
- `reliability.parquet`: `element_id`, `type`, `lambda`, `r`, `switchable`.
- Additional switching/outage tables can be layered later without breaking compatibility.

## Commands
- `gat adms validate-reliability reliability.parquet`
  - Ensures reliability metadata includes failure and repair rates.
- `gat adms flisr-sim --reliability reliability.parquet --out flisr_runs.parquet`
  - Emits per-scenario interruption counts and durations.
- `gat adms outage-mc --reliability reliability.parquet --out outage_stats.parquet [--samples 100]`
  - Samples outages using a Poisson approximation and reports event counts and unserved energy.
- `gat adms vvo-plan --nodes dist_nodes.parquet --branches dist_branches.parquet --out vvo_settings.parquet`
  - Writes placeholder tap/cap settings derived from the supplied feeders.
- `gat adms state-estimation --out adms_state.parquet`
  - Produces a stub state-estimation result for downstream prototyping.

## Notes
- All outputs are Parquet and can be cataloged alongside other `gat` run manifests.
- Reliability table validation is a prerequisite for FLISR and outage Monte Carlo helpers.
