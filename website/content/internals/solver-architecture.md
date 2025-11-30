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

GAT supports two methods for installing native solvers: **system packages** (quickest) or **vendored builds** (fully offline, reproducible).

### Method 1: System Packages (Quick)

```bash
# Ubuntu/Debian - IPOPT
sudo apt install libipopt-dev coinor-libipopt1v5

# Ubuntu/Debian - CBC/CLP
sudo apt install coinor-libcbc-dev coinor-libclp-dev

# macOS
brew install ipopt coin-or-tools/coinor/cbc
```

### Method 2: Vendored Build (Offline, Reproducible)

GAT includes vendored sources for the complete COIN-OR solver stack. This enables fully offline builds with reproducible results.

**Prerequisites:**
```bash
# Ubuntu/Debian
sudo apt install build-essential gfortran libblas-dev liblapack-dev libbz2-dev zlib1g-dev pkg-config

# macOS
brew install gcc lapack pkg-config
```

**Build from vendored sources:**
```bash
# LP/SOCP solver stack: CoinUtils → Osi → Clp
./scripts/build-clp.sh

# MIP solver stack: Cgl → Cbc (requires CLP)
./scripts/build-cbc.sh

# NLP solver stack: Metis → MUMPS → IPOPT
./scripts/build-ipopt.sh
```

**What gets built:**

| Stack | Script | Components | Output Libraries |
|-------|--------|------------|------------------|
| LP | `build-clp.sh` | CoinUtils, Osi, Clp | `libCoinUtils.a`, `libOsi.a`, `libClp.a` |
| MIP | `build-cbc.sh` | Cgl, Cbc | `libCgl.a`, `libCbc.a` |
| NLP | `build-ipopt.sh` | Metis 4.0, MUMPS 5.8, IPOPT 3.14 | `libcoinmetis.a`, `libcoinmumps.a`, `libipopt.so` |

All libraries install to `vendor/local/`. MUMPS is built with OpenMP for parallel factorization; Metis provides graph-based ordering for better fill-in reduction.

**Use with Cargo:**
```bash
# Set paths and build
export PKG_CONFIG_PATH="$PWD/vendor/local/lib/pkgconfig:$PKG_CONFIG_PATH"
export LD_LIBRARY_PATH="$PWD/vendor/local/lib:$LD_LIBRARY_PATH"
cargo build --release --features solver-ipopt

# Or use the wrapper script
./scripts/with-ipopt.sh cargo test --features solver-ipopt
```

### IPOPT (Recommended for AC-OPF)

IPOPT provides the fastest and most accurate AC-OPF solutions:

```bash
# From vendored sources (recommended for CI/reproducibility)
./scripts/build-ipopt.sh
./scripts/with-ipopt.sh cargo build --features solver-ipopt

# Or system package (quick setup)
sudo apt install libipopt-dev coinor-libipopt1v5
cargo build --features solver-ipopt
```

### CBC/CLP (Recommended for LP/MIP)

CBC provides branch-and-cut MIP solving; CLP provides simplex LP:

```bash
# From vendored sources
./scripts/build-clp.sh
./scripts/build-cbc.sh
cargo build -p gat-cbc -p gat-clp

# Or system package
sudo apt install coinor-libcbc-dev
cargo build -p gat-cbc
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

**Vendored Build Chain:**

The shell scripts in `scripts/` handle the native solver build:

1. Extract vendored source archives from `vendor/`
2. Apply COIN-OR patches for compatibility
3. Configure with proper dependency ordering
4. Build with parallel make (`-j$(nproc)`)
5. Install static libraries to `vendor/local/`
6. Generate pkg-config files for Cargo build.rs detection

**Cargo Build Integration:**

Each native solver crate (`gat-cbc`, `gat-clp`, `gat-ipopt`) has a `build.rs` that:

1. Tries system pkg-config first (fastest)
2. Falls back to `vendor/local/` pre-built libraries
3. Can build from source as last resort (slowest)

```rust
// Priority order in gat-cbc/build.rs
fn main() {
    if try_system_cbc() { return; }           // System pkg-config
    if try_prebuilt("vendor/local") { return; } // Vendored build
    build_from_source();                       // Fallback compile
}
```

**CI Integration:**

The `.github/workflows/native-solvers.yml` workflow:
- Caches `vendor/local/` between builds
- Runs vendored build scripts if cache misses
- Tests all solver features with proper `LD_LIBRARY_PATH`

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

- [OPF Guide](@/guide/opf.md) — Choosing the right solver tier
- [Benchmarks](@/internals/benchmarks.md) — Complete PGLib validation results
- [Building from Source](@/guide/install-verify.md) — Compiling with native solver support
