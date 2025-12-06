---
status: completed
priority: p3
issue_id: "013"
tags: [simplicity, code-review, dead-code]
dependencies: []
---

# Audit and Remove #[allow(dead_code)] Items

## Problem Statement

25+ instances of `#[allow(dead_code)]` across the codebase indicate unused code that should be removed or implemented.

**Why it matters:** Dead code increases maintenance burden and obscures what's actually used.

## Resolution

Audited all `#[allow(dead_code)]` annotations and categorized them:

### Removed (truly dead)
- `power_flow.rs`: `YBusComponents`, `build_y_bus`, `compute_p`, `Complex64` import (~75 lines)
- `power_flow/legacy.rs`: Same functions duplicated (~75 lines)
- `ac_opf.rs`: `PenaltyFormulation` struct and impl (~25 lines)
- `opf/mod.rs`: `solve_with_dispatcher` method (~35 lines)
- `opf/socp.rs`: `SocpInitialPoint` and `warm_start_from_dc` (~110 lines)
- `reliability_monte_carlo.rs`: `calculate_deliverable_generation` non-arena version (~110 lines)
- `featurize_gnn.rs`: `GraphMeta` struct (~12 lines)
- `benchmark/common.rs`: Entire file deleted (~80 lines)

### Kept (legitimate)
- **FFI bindings** (`gat-clp`, `gat-cbc`): Standard pattern for C bindings
- **Public API fields** (`AcOpfSolver::max_iterations`, `tolerance`): Builder methods use them
- **Debugging aids** (`NkScreener::ptdf`, `IncrementalSolver::reduced_b_prime`): Explicitly marked for debugging
- **Internal structs used by public functions** (`VariableBounds`, `QcEnvelope`): Used by `solve_enhanced`

### Deferred (GUI/TUI)
- `gat-ui-common/src/jobs.rs` - Deferred per user request
- `gat-tui/src/ui/ansi.rs` - Deferred per user request
- `gat-gui/src-tauri/src/service.rs` - Deferred per user request

**Total LOC removed:** ~520 lines

## Acceptance Criteria

- [x] No `#[allow(dead_code)]` in non-FFI code (remaining are legitimate)
- [x] All tests pass after removal (141 tests pass)
- [x] Code compiles without warnings

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | Dead code clutters codebase |
| 2025-12-06 | Audit completed | Categorize before deleting - some "dead" code is actually legitimate |
