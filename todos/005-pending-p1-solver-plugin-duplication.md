---
status: completed
priority: p1
issue_id: "005"
tags: [architecture, code-review, solvers]
dependencies: []
---

# Solver Plugin IPC Protocol Duplication

## Problem Statement

Three solver binaries (IPOPT, CBC, CLP) have nearly identical boilerplate code for IPC handling, main entry, logging setup, and error handling. Bug fixes must be applied to 3 locations.

**Why it matters:** Code duplication leads to inconsistent fixes and maintenance burden.

## Resolution

Created `SolverPlugin` trait and `run_solver_plugin()` harness in `gat-solver-common`:

**New files:**
- `crates/gat-solver-common/src/plugin.rs` - Plugin harness implementation
- `crates/gat-solver-common/README.md` - Documentation

**API:**
```rust
pub trait SolverPlugin {
    fn name(&self) -> &'static str;
    fn solve(&self, problem: &ProblemBatch) -> Result<SolutionBatch>;
    fn use_v2_protocol(&self) -> bool { true }
    fn init(&self) -> Result<()> { Ok(()) }
}

pub fn run_solver_plugin<P: SolverPlugin>(plugin: P) -> !
```

**Harness handles:**
- Tracing initialization (respects `RUST_LOG`)
- Version/protocol logging
- Arrow IPC problem reading from stdin
- Arrow IPC solution writing to stdout
- Error handling and standardized exit codes

**Dependencies added:**
- `tracing = "0.1"`
- `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`

## Acceptance Criteria

- [x] Common IPC handling in gat-solver-common
- [x] Solver binaries can be reduced to solve logic only
- [x] Protocol version handling consistent

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | 3 identical implementations = maintenance burden |
| 2025-12-06 | Created plugin harness | Template Method pattern reduces boilerplate to ~10 lines |
