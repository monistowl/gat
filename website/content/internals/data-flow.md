+++
title = "Data Flow & Design"
description = "How data moves through GAT, from input to solution"
template = "page.html"
weight = 39
[extra]
toc = true
+++

# Data Flow & Design

This document describes GAT's internal architecture: how data flows from input files through analysis algorithms to results, and the design principles that guide the implementation.

## High-Level Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              USER INPUT                                      │
│  MATPOWER (.m) │ PSS/E (.raw) │ CIM (.rdf) │ pandapower (.json) │ Arrow     │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              gat-io                                          │
│                                                                              │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐     │
│  │  MATPOWER   │   │   PSS/E     │   │    CIM      │   │ pandapower  │     │
│  │   Parser    │   │   Parser    │   │   Parser    │   │   Parser    │     │
│  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘   └──────┬──────┘     │
│         │                 │                 │                 │             │
│         └─────────────────┴─────────────────┴─────────────────┘             │
│                                    │                                         │
│                                    ▼                                         │
│                          ┌─────────────────┐                                │
│                          │  Diagnostics &  │                                │
│                          │   Validation    │                                │
│                          └────────┬────────┘                                │
└───────────────────────────────────┼─────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                             gat-core                                         │
│                                                                              │
│                    ┌──────────────────────────────┐                         │
│                    │    Network<Node, Edge>       │                         │
│                    │  (petgraph UnDiGraph)        │                         │
│                    │                              │                         │
│                    │  Nodes: Bus, Gen, Load       │                         │
│                    │  Edges: Branch, Transformer  │                         │
│                    └──────────────┬───────────────┘                         │
│                                   │                                          │
└───────────────────────────────────┼──────────────────────────────────────────┘
                                    │
          ┌─────────────────────────┼─────────────────────────┐
          │                         │                         │
          ▼                         ▼                         ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│    gat-algo     │    │    gat-dist     │    │   gat-adms/     │
│                 │    │                 │    │   gat-derms     │
│  • Power Flow   │    │  • Distribution │    │                 │
│  • OPF (DC/AC)  │    │    Analysis     │    │  • Domain-      │
│  • SOCP         │    │  • Radial       │    │    specific     │
│  • Reliability  │    │    Networks     │    │    analytics    │
│  • Contingency  │    │                 │    │                 │
└────────┬────────┘    └────────┬────────┘    └────────┬────────┘
         │                      │                      │
         └──────────────────────┴──────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SOLVER DISPATCH                                     │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    Pure-Rust Solvers (always available)              │   │
│   │  • Clarabel (SOCP)  • L-BFGS (NLP)  • Clarabel (LP fallback)        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    Native Solvers (optional, subprocess IPC)         │   │
│   │  • IPOPT (NLP)  • CBC (MIP)  • CLP (LP)  • HiGHS (LP/MIP)           │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              OUTPUT                                          │
│  Solution JSON │ Arrow tables │ Visualization │ Reports │ MCP responses     │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Core Abstractions

### Network Model (`gat-core`)

The `Network` type is the central data structure, built on petgraph's undirected multigraph:

```rust
pub struct Network {
    pub graph: UnDiGraph<Node, Edge>,
    pub metadata: NetworkMetadata,
}

pub enum Node {
    Bus(Bus),
    Gen(Gen),
    Load(Load),
}

pub enum Edge {
    Branch(Branch),
    // Transformers are branches with special tap/shift parameters
}
```

**Design Rationale:**
- **Graph-based**: Enables fast topological queries (connectivity, island detection, path finding)
- **Multigraph**: Supports parallel branches between same buses (common in real networks)
- **Undirected**: Power flows bidirectionally; direction determined by solution
- **Type-safe IDs**: `BusId`, `GenId`, `LoadId`, `BranchId` prevent mixing element types

### I/O Layer (`gat-io`)

Format parsers convert heterogeneous input formats to the common `Network` model:

```rust
pub fn parse_matpower(path: &str) -> Result<ImportResult> {
    // Parse MATLAB syntax → Network + Diagnostics
}

pub struct ImportResult {
    pub network: Network,
    pub diagnostics: ImportDiagnostics,
}

pub struct ImportDiagnostics {
    pub warnings: Vec<DiagnosticMessage>,
    pub errors: Vec<DiagnosticMessage>,
    pub skipped_elements: usize,
}
```

**Design Rationale:**
- **Single responsibility**: Each parser handles only its format's quirks
- **Error recovery**: Partial imports continue, collecting diagnostics
- **Lossless roundtrips**: Preserve all fields (e.g., MATPOWER gencost models)

### Algorithm Layer (`gat-algo`)

Algorithms operate on `Network` and produce typed solutions:

```rust
pub struct OpfSolver {
    method: OpfMethod,
    options: SolverOptions,
}

impl OpfSolver {
    pub fn solve(&self, network: &Network) -> Result<OpfSolution> {
        match self.method {
            OpfMethod::DcOpf => self.solve_dc(network),
            OpfMethod::SocpRelaxation => self.solve_socp(network),
            OpfMethod::AcNlp => self.solve_ac_nlp(network),
        }
    }
}
```

**Design Rationale:**
- **Unified interface**: Same API regardless of solution method
- **Progressive refinement**: Start with fast DC, refine with SOCP, validate with AC-NLP
- **Pluggable solvers**: Backend selection is transparent to calling code

## Solver Integration Patterns

GAT uses two distinct patterns for solver integration:

### Pattern 1: Compiled-In FFI (`solver-ipopt` feature)

The solver library is linked directly into the binary:

```
┌─────────────────────────────────────────┐
│              gat-cli binary             │
│  ┌────────────────────────────────────┐ │
│  │        gat-algo                    │ │
│  │  ┌──────────────────────────────┐  │ │
│  │  │    IPOPT FFI bindings        │  │ │
│  │  │    (gat-ipopt-sys)           │  │ │
│  │  └──────────────────────────────┘  │ │
│  └────────────────────────────────────┘ │
│                   │                     │
│                   ▼                     │
│  ┌────────────────────────────────────┐ │
│  │  libipopt.so (dynamically linked)  │ │
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**Pros:** Lower latency, no IPC overhead
**Cons:** Crashes can take down main process

### Pattern 2: Subprocess IPC (`native-dispatch` feature)

The solver runs in a separate process, communicating via Arrow IPC:

```
┌──────────────────┐         Arrow IPC        ┌──────────────────┐
│   gat-cli        │ ◄──────────────────────► │   gat-ipopt      │
│                  │                          │   (subprocess)   │
│  Problem data    │        stdin/stdout      │                  │
│  in Network form │ ──────────────────────── │  Links against   │
│                  │                          │  libipopt.so     │
│  Solution back   │ ◄─────────────────────── │                  │
└──────────────────┘                          └──────────────────┘
```

**Pros:** Crash isolation, version flexibility, clean resource cleanup
**Cons:** ~50-100ms startup overhead per solve

See [Solver Architecture](../solver-architecture/) for detailed documentation.

## Crate Dependencies

```
                     ┌─────────────────┐
                     │    gat-cli      │
                     │  (user entry)   │
                     └────────┬────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│   gat-algo    │    │   gat-dist    │    │  gat-adms/    │
│  (algorithms) │    │ (distribution)│    │  gat-derms    │
└───────┬───────┘    └───────┬───────┘    └───────┬───────┘
        │                    │                    │
        └────────────────────┴────────────────────┘
                             │
                             ▼
                    ┌───────────────┐
                    │   gat-core    │
                    │ (data model)  │
                    └───────────────┘
                             ▲
                             │
                    ┌───────────────┐
                    │    gat-io     │
                    │  (parsers)    │
                    └───────────────┘
```

| Crate | Responsibility |
|-------|----------------|
| `gat-core` | Network graph, base types, topology operations |
| `gat-io` | File format parsers, validation, Arrow schema |
| `gat-algo` | OPF, power flow, reliability, contingency analysis |
| `gat-dist` | Distribution network analysis (radial, unbalanced) |
| `gat-adms` | ADMS domain features |
| `gat-derms` | DERMS domain features |
| `gat-ts` | Time series operations |
| `gat-viz` | Visualization helpers |
| `gat-cli` | Command-line interface |
| `gat-gui` | Native GUI (egui) |
| `gat-tui` | Terminal UI (ratatui) |

## Build Infrastructure

### Vendored COIN-OR Stack

GAT vendors the complete COIN-OR solver stack for fully offline, reproducible builds:

```
vendor/
├── CoinUtils-master.zip    # Base utilities
├── Osi-master.zip          # Open Solver Interface
├── Clp-master.zip          # Simplex LP solver
├── Cgl-master.zip          # Cut Generator Library
├── Cbc-master.zip          # Branch & Cut MIP solver
├── metis-4.0.3.tar.gz      # Graph partitioning
├── MUMPS_5.8.1.tar.gz      # Parallel sparse solver
└── Ipopt-stable-3.14/      # Interior point optimizer
```

Build scripts in `scripts/` handle the compilation:

```bash
./scripts/build-clp.sh   # CoinUtils → Osi → Clp
./scripts/build-cbc.sh   # Cgl → Cbc (requires CLP)
./scripts/build-ipopt.sh # Metis → MUMPS → IPOPT
```

### Feature Flags

Key cargo features control compilation:

| Feature | Effect |
|---------|--------|
| `solver-ipopt` | Enable IPOPT FFI bindings |
| `solver-clarabel` | Enable Clarabel SOCP solver |
| `solver-coin_cbc` | Enable CBC MIP solver |
| `native-dispatch` | Enable subprocess solver dispatch |
| `full-io` | All file format parsers |
| `minimal-io` | Basic format support only |
| `viz` | Visualization support |
| `wasm` | WASM-compatible build |

## Error Handling Philosophy

GAT uses a layered approach to error handling:

1. **Parse errors**: Collected as diagnostics, partial import continues
2. **Validation errors**: Warnings vs. errors, configurable strictness
3. **Solver errors**: Automatic fallback to alternative solvers
4. **Numerical errors**: Graceful degradation with informative messages

```rust
// Example: solver fallback chain
match ipopt_solver.solve(&network) {
    Ok(solution) => return Ok(solution),
    Err(SolverError::Crash) => {
        warn!("IPOPT crashed, falling back to L-BFGS");
        return lbfgs_solver.solve(&network);
    }
}
```

## Performance Considerations

### Memory Layout

- Network uses petgraph's CSR-based storage for cache efficiency
- Arrow columnar format enables zero-copy reads where possible
- Large matrices use sparse representations (CsrMatrix, triplet format)

### Parallelism

- Topology operations use rayon for parallel iteration
- Monte Carlo reliability uses parallel scenario evaluation
- MUMPS (IPOPT's linear solver) uses OpenMP for factorization

### Typical Problem Sizes

| Scale | Buses | Branches | Solve Time (IPOPT) |
|-------|-------|----------|-------------------|
| Small | < 100 | < 200 | < 1s |
| Medium | 100-1000 | 200-2000 | 1-10s |
| Large | 1000-10000 | 2000-20000 | 10-60s |
| Very Large | > 10000 | > 20000 | > 60s |

## Related Documentation

- [CLI Architecture](../cli-architecture/) — Command module structure
- [Solver Architecture](../solver-architecture/) — Native solver plugin system
- [CI/CD Workflows](../ci-cd/) — Build and release process
- [Feature Matrix](../feature-matrix/) — Feature flag combinations
