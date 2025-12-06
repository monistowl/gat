# gat-solver-common

Common types, IPC protocol, and plugin infrastructure for GAT solver plugins.

## Overview

This crate provides the foundation for GAT's native solver plugin system, enabling
process-isolated C/C++ optimization solvers (IPOPT, CBC, HiGHS) to communicate
with the main `gat` binary via Arrow IPC.

## Architecture

```
gat (main) ──stdin──> gat-ipopt (subprocess)
           <─stdout──
           <─stderr── (logs/errors)
```

## Key Components

### Plugin Harness (`plugin.rs`)

The `SolverPlugin` trait and `run_solver_plugin()` function eliminate boilerplate
for solver binaries:

```rust
use gat_solver_common::{run_solver_plugin, SolverPlugin, ProblemBatch, SolutionBatch};
use anyhow::Result;

struct MySolver;

impl SolverPlugin for MySolver {
    fn name(&self) -> &'static str { "gat-mysolver" }

    fn solve(&self, problem: &ProblemBatch) -> Result<SolutionBatch> {
        // Your solver implementation here
    }
}

fn main() {
    run_solver_plugin(MySolver);
}
```

The harness handles:
- Tracing initialization (respects `RUST_LOG`)
- Version/protocol logging
- Arrow IPC problem reading from stdin
- Arrow IPC solution writing to stdout
- Error handling and exit codes

### IPC Protocol (`ipc.rs`)

Arrow IPC schemas for problem and solution data:

- `ProblemBatch`: Network data (buses, generators, branches) + solver parameters
- `SolutionBatch`: Results (voltages, power outputs, LMPs) + solve status

Protocol versions:
- v1: Single-batch (legacy)
- v2: Multi-batch with length-prefixed frames (current)

### Subprocess Management (`subprocess.rs`)

`SolverProcess` spawns solver binaries and handles IPC:

```rust
use gat_solver_common::{SolverProcess, SolverId};

let solver = SolverProcess::find_binary(SolverId::Ipopt)?;
let solution = solver.solve_blocking(&problem)?;
```

### Error Types (`error.rs`)

- `ExitCode`: Standardized exit codes (0=success, 1=solver error, etc.)
- `SolverError`: Error enum for IPC, process, and solver failures

## Supported Solvers

### Native Solvers (require installation)

| Solver | Problem Type | Binary |
|--------|--------------|--------|
| IPOPT | NLP (AC-OPF) | `gat-ipopt` |
| CLP | LP (DC-OPF) | `gat-clp` |
| CBC | MIP (unit commitment) | `gat-cbc` |
| HiGHS | LP/MIP | `gat-highs` |
| Bonmin | MINLP | `gat-bonmin` |

### Pure-Rust Solvers (always available)

| Solver | Problem Type |
|--------|--------------|
| Clarabel | SOCP/SDP |
| L-BFGS | NLP (penalty method) |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Solver error |
| 2 | IPC/protocol error |
| 3 | Timeout |

## License

MIT OR Apache-2.0
