# Solver Plugin Protocol (IPC v2)

## Overview

Native solvers (CLP, CBC, IPOPT) run as isolated subprocesses communicating via Arrow IPC over stdin/stdout. This architecture provides:

- **Memory isolation**: Solver crashes don't affect the main process
- **License compliance**: Solvers with restrictive licenses run in separate address spaces
- **Version flexibility**: Different solver versions can coexist

## Relationship to Strategy Pattern

The OPF system uses a Strategy Pattern with two abstraction levels:

```
┌─────────────────────────────────────────────────────────────────┐
│                        OpfDispatcher                             │
│  (orchestrates solving, manages fallback chains)                 │
└──────────────────────┬──────────────────────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ DcOpf        │ │ Socp         │ │ AcOpf        │  ◄─ OpfFormulation
│ Formulation  │ │ Formulation  │ │ Formulation  │     (what to solve)
└──────┬───────┘ └──────┬───────┘ └──────┬───────┘
       │                │                │
       ▼                ▼                ▼
  ProblemClass:    ProblemClass:    ProblemClass:
  LinearProgram    ConicProgram     NonlinearProgram
       │                │                │
       └────────────────┼────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                      SolverRegistry                              │
│  (selects best available backend for problem class)              │
└──────────────────────┬──────────────────────────────────────────┘
                       │
       ┌───────────────┼───────────────┐
       ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ Clarabel     │ │ L-BFGS       │ │ IPOPT        │  ◄─ OpfBackend
│ Backend      │ │ Backend      │ │ Backend      │     (how to solve)
│ (pure Rust)  │ │ (pure Rust)  │ │ (via IPC)    │
└──────────────┘ └──────────────┘ └──────────────┘
                                         │
                                         ▼ Arrow IPC
                                  ┌──────────────┐
                                  │ gat-ipopt    │
                                  │ subprocess   │
                                  └──────────────┘
```

**Pure-Rust backends** (Clarabel, L-BFGS) run in-process.
**Native backends** (IPOPT, CLP, CBC) use the IPC protocol below.

## Message Format

```
[4-byte length prefix (little-endian)][Arrow IPC message]
```

All messages use Apache Arrow IPC format for zero-copy serialization.

## Problem Batch Schema

```
message ProblemBatch {
  num_vars: i64
  num_constraints: i64
  objective: float64[]
  constraint_matrix: SparseMatrix
  lower_bounds: float64[]
  upper_bounds: float64[]
}
```

## Solution Batch Schema

```
message SolutionBatch {
  status: SolverStatus
  objective_value: float64
  primal_solution: float64[]
  dual_solution: float64[]
  iterations: i64
  solve_time_ms: i64
}
```

## Solver Status Values

| Status | Meaning |
|--------|---------|
| 0 | Optimal |
| 1 | Infeasible |
| 2 | Unbounded |
| 3 | Iteration limit |
| 4 | Time limit |
| 5 | Numerical error |
| 6 | Unknown |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Invalid input |
| 2 | Solver error |
| 3 | Timeout |
| 139 | SIGSEGV (solver crash) |

## Implementation

See `gat-solver-common/src/subprocess.rs` for the protocol implementation.

### Spawning a Solver

```rust
use gat_solver_common::SolverProcess;

let mut solver = SolverProcess::spawn("gat-clp")?;
solver.send_problem(&problem)?;
let solution = solver.receive_solution()?;
```

### Implementing a New Solver

1. Create `gat-<solver>/src/main.rs`
2. Parse Arrow IPC from stdin
3. Convert to native solver format
4. Solve and serialize result to stdout
5. Handle SIGTERM for graceful shutdown

See `gat-clp/src/main.rs` for a reference implementation.

## Debugging

Set `GAT_SOLVER_DEBUG=1` to log IPC messages to stderr.

```bash
GAT_SOLVER_DEBUG=1 gat opf dc case9.arrow
```
