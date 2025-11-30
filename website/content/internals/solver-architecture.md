+++
title = "Solver Architecture"
description = "How GAT's native solver plugin system works"
template = "page.html"
weight = 51
[extra]
toc = true
+++

# Native Solver Plugin System

GAT uses a unique **subprocess-based plugin architecture** for native solvers. This design provides crash isolation, version flexibility, and seamless fallback to pure-Rust implementations when native solvers aren't available.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                          GAT CLI                                 │
│                    (gat opf ac grid.arrow)                       │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Solver Dispatcher                            │
│  • Detects problem type (LP, SOCP, NLP, MIP)                    │
│  • Checks installed native solvers                               │
│  • Falls back to pure-Rust if needed                            │
└──────────────────────────┬──────────────────────────────────────┘
                           │
           ┌───────────────┴───────────────┐
           │                               │
           ▼                               ▼
┌─────────────────────┐         ┌─────────────────────┐
│   Pure-Rust Path    │         │   Native Solver     │
│                     │         │   Subprocess        │
│  • L-BFGS (AC-OPF)  │         │                     │
│  • Clarabel (SOCP)  │         │  ┌───────────────┐  │
│  • Always available │         │  │ IPOPT/HiGHS   │  │
│                     │         │  │ CBC/CLP       │  │
└─────────────────────┘         │  └───────────────┘  │
                                │         │          │
                                │    Arrow IPC       │
                                │         │          │
                                └─────────┴──────────┘
```

## Key Design Principles

### 1. Crash Isolation

Native libraries (especially C/C++ solvers like IPOPT) can crash due to numerical issues, memory corruption, or library bugs. By running them in a separate process:

- **Main process stays alive**: A solver crash doesn't take down the CLI
- **Clean error recovery**: The dispatcher can retry with a different solver
- **Resource cleanup**: OS handles memory/file cleanup on subprocess exit

### 2. Version Flexibility

Different users may need different solver versions:

```bash
# Install specific IPOPT version
cargo xtask solver build ipopt --version 3.14.0 --install

# Multiple versions can coexist
~/.gat/solvers/
  ipopt-3.14.0/
  ipopt-3.13.4/
  highs-1.7.0/
```

### 3. Arrow IPC Protocol

All solver communication uses Apache Arrow IPC:

```
┌────────────────────┐    Arrow IPC     ┌────────────────────┐
│    GAT Process     │ ──────────────► │  Solver Process    │
│                    │                  │                    │
│  Problem data:     │                  │  Receives:         │
│  • Bounds          │  Multi-batch     │  • Structured data │
│  • Constraints     │  streaming       │  • Type-safe       │
│  • Objectives      │                  │  • Zero-copy       │
│                    │ ◄────────────── │                    │
│  Solution:         │                  │  Returns:          │
│  • Primal vars     │                  │  • Status code     │
│  • Dual vars       │                  │  • Iteration count │
│  • Status          │                  │  • Solve time      │
└────────────────────┘                  └────────────────────┘
```

**Benefits of Arrow IPC:**
- **Type safety**: Schema validation catches bugs early
- **Efficient**: Zero-copy reads where possible
- **Language-agnostic**: Works with C, C++, Python solvers

## Available Backends

| Backend | Type | Problem Classes | Status |
|---------|------|-----------------|--------|
| **L-BFGS** | Pure Rust | NLP | Always available |
| **Clarabel** | Pure Rust | LP, SOCP | Always available |
| **IPOPT** | Native (C++) | NLP | Optional |
| **HiGHS** | Native (C++) | LP, MIP | Optional |
| **CBC** | Native (C) | MIP | Optional |
| **CLP** | Native (C) | LP | Optional |

### Automatic Selection

The dispatcher automatically selects the best available solver:

```rust
// Problem classification
match problem_type {
    ProblemType::LP => {
        // Try: HiGHS → CLP → Clarabel
    }
    ProblemType::SOCP => {
        // Try: Clarabel (always available)
    }
    ProblemType::NLP => {
        // Try: IPOPT → L-BFGS
    }
    ProblemType::MIP => {
        // Try: HiGHS → CBC
    }
}
```

## Installing Native Solvers

### IPOPT (Recommended for AC-OPF)

IPOPT provides the fastest and most accurate AC-OPF solutions:

```bash
# Ubuntu/Debian
sudo apt install libipopt-dev coinor-libipopt1v5

# Build and install the GAT wrapper
cargo xtask solver build ipopt --install

# Verify installation
gat solver list
```

### HiGHS (Recommended for LP/MIP)

HiGHS is a high-performance open-source LP/MIP solver:

```bash
# Build from source
cargo xtask solver build highs --install

# Or use system package
sudo apt install highs
```

### Managing Solvers

```bash
# List installed solvers
gat solver list

# Uninstall a solver
gat solver uninstall ipopt

# Check solver health
gat solver check ipopt
```

## Implementation Details

### Subprocess Communication

Each native solver wrapper implements the `SolverProcess` trait:

```rust
pub trait SolverProcess {
    /// Spawn the solver subprocess
    fn spawn(&self) -> Result<Child>;

    /// Write problem data via Arrow IPC
    fn write_problem(&self, problem: &Problem) -> Result<()>;

    /// Read solution via Arrow IPC
    fn read_solution(&self) -> Result<Solution>;

    /// Clean up subprocess
    fn terminate(&mut self) -> Result<()>;
}
```

### Error Handling

The dispatcher handles solver failures gracefully:

```rust
match native_solver.solve(problem) {
    Ok(solution) => return solution,
    Err(SolverError::Crash) => {
        warn!("Native solver crashed, falling back to pure-Rust");
        return fallback_solver.solve(problem);
    }
    Err(SolverError::Timeout) => {
        warn!("Native solver timed out");
        return Err(OpfError::Timeout);
    }
    Err(e) => return Err(e.into()),
}
```

### Build System Integration

The `cargo xtask solver build` command:

1. Checks system dependencies (`libipopt-dev`, etc.)
2. Compiles the Rust wrapper crate
3. Links against system libraries
4. Installs binary to `~/.gat/solvers/`
5. Registers solver with GAT config

## Performance Characteristics

| Solver | Startup | Memory | Best For |
|--------|---------|--------|----------|
| L-BFGS | < 1ms | Low | Small NLP |
| Clarabel | < 1ms | Low | SOCP |
| IPOPT | ~50ms | Medium | Large NLP |
| HiGHS | ~10ms | Medium | LP/MIP |

**Note:** Subprocess startup overhead (~50-100ms) is amortized over solve time. For large problems (> 1000 buses), IPOPT's superior convergence more than compensates.

## Troubleshooting

### "Solver not found"

```bash
# Check if solver is installed
gat solver list

# Check system dependencies
ldd ~/.gat/solvers/ipopt/gat-ipopt-wrapper

# Rebuild solver
cargo xtask solver build ipopt --install --force
```

### "Arrow IPC error"

Usually indicates version mismatch:

```bash
# Rebuild with current GAT version
cargo xtask solver build ipopt --install --force
```

### "Solver crashed"

Check solver logs:

```bash
# Enable verbose logging
GAT_LOG=debug gat opf ac grid.arrow

# Check solver-specific logs
cat ~/.gat/logs/ipopt.log
```

## Related Documentation

- [OPF Guide](/guide/opf/) — Choosing the right solver tier
- [Benchmarks](/internals/benchmarks/) — Complete PGLib validation results
- [Building from Source](/guide/install-verify/) — Compiling with native solver support
