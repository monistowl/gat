# Milestone 3 Implementation Plan

## Summary
Milestone 3 is about delivering the core power-flow solvers (DC + AC) and ensuring they hook cleanly into the CLI and the new solver registry. The CLI should solve standard MATPOWER grids with both Gauss/Faer backends, expose register/test hooks, and provide clear tolerance/iteration controls.

## Deliverables
1. DC power flow solver that builds B′ from the topology, assembles the right-hand side with injections, solves the system via the registry-backed solver, and emits branch flows in Parquet.
2. AC Newton–Raphson driver with configurable tolerance/iterations plus the ability to fall back to DC when AC fails; reuse core solver backends where dense solves are needed.
3. CLI commands: `gat pf dc` and `gat pf ac` with solver selection (`--solver`), threading hints, and logging, plus manifest recording/test harness guaranteeing MATPOWER cases converge.
4. Tests: unit tests for matrix assembly, regression against at least one small IEEE MATPOWER case (compare flows/angles with reference values), and CLI regression via `assert_cmd` for simple runs.
5. Documentation updates: describe CLI parameters, solver selection, tolerance guidance, and mention Milestone 3 completion in the roadmap/docs.

## Steps
1. **Matrix assembly helpers**
   * Extend `gat-algo` (or `gat-core`) with functions that build B′, branch susceptance, and injection vectors from `Network` and generator/load data.
   * Provide helpers to convert branch flows into arrow/parquet outputs.

2. **DC solver pipeline**
   * Implement `pf::dc_solve` that dispatches to `SolverKind::build_solver`, solves B′ x = P, and returns structured branch results.
   * Add configurable thread building via Rayon matching CLI hints.
   * Emit results (flows/angles) to Parquet via existing IO helpers (reuse `gat-io` exports?).

3. **AC Newton pack**
   * Create `pf::ac_solve` that iterates Newton steps with tolerance + max_iter, logs residuals via `tracing`, and falls back to DC or reports failure.
   * Capture voltages/angles, branch flows, and convergence info.

4. **CLI wiring**
   * In `gat-cli/src/cli.rs` and `main.rs`, plug `PowerFlowCommands::Dc`/`::Ac` to the new solver functions, pass solver names, tolerances, logging.
   * Record manifests per run and ensure CLI prints success/failure details.

5. **Testing and docs**
   * Add unit tests for the matrix builder + Newton solver (hard-coded small cases).
   * CLI regression: use `assert_cmd` to run `gat pf dc --grid test_data/matpower/case9.arrow --out tmp` verifying exit code.
   * Document CLI options, mention solver registry and tolerance settings in `docs/guide/pf.md` (or CLI doc) plus update `docs/ROADMAP.md` under M3.

## Risks & notes
- AC Newton may need more numeric work; start with DC and extend. Use `ndarray`/`faer` for LU as needed.
- CLI needs safe defaults (gauss, tolerance 1e-6, max_iter 20). Provide clear error messages when solvers fail.
