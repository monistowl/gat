# GAT Architecture Overview

## Crate Structure

```
gat/
├── gat-core        # Core data structures, graph model
├── gat-algo        # Power flow, OPF, reliability algorithms
├── gat-io          # File format import/export
├── gat-solver-common # Solver IPC protocol
├── gat-clp/cbc/ipopt # Native solver binaries
├── gat-cli         # Command-line interface
├── gat-tui         # Terminal UI
└── gat-gui         # Desktop GUI (future)
```

## Dependency Graph

```
gat-core (foundation)
    ↑
gat-io  →  gat-algo  →  gat-solver-common
    ↑          ↑
gat-batch, gat-scenarios, gat-ts
    ↑
gat-cli, gat-tui, gat-gui
```

## Key Design Decisions

1. **Graph-Based Network Model**: Uses petgraph for topology representation. Buses, generators, loads, and shunts are nodes; branches and transformers are edges.

2. **Type-Safe IDs**: Newtype wrappers (`BusId`, `GenId`, `BranchId`) prevent ID mixing at compile time.

3. **Subprocess Solver Isolation**: Native solvers (CLP, CBC, IPOPT) run as separate processes, communicating via Arrow IPC. This provides:
   - Memory isolation (solver crashes don't bring down the main process)
   - License compliance (some solvers have restrictive licenses)
   - Easy version management

4. **Arrow IPC Protocol**: Zero-copy data transfer to solvers using Apache Arrow format.

5. **LinearSystemBackend Trait**: Abstract interface for dense linear system solvers (Ax = b), currently implemented by:
   - `GaussSolver`: Simple Gaussian elimination
   - `FaerSolver`: High-performance LU via the `faer` crate

6. **Sparse Matrix Support**: Uses `sprs` for sparse matrices in DC power flow and OPF formulations, critical for large networks (10,000+ buses).

## Extension Points

- **Add new file formats**: Implement in `gat-io/src/importers/`
- **Add new algorithms**: Add to `gat-algo/src/`
- **Add new solvers**: Create `gat-<solver>` crate with IPC protocol
- **Add new CLI commands**: Extend `gat-cli/src/commands/`

## Performance Considerations

- **Sparse matrices**: Power grids are ~0.05% dense; sparse representations are essential
- **HashMap pre-allocation**: Use `with_capacity()` when size is known
- **Bus lookup caching**: In Monte Carlo, cache `BusId → NodeIndex` maps across scenarios
- **Parallel scenarios**: Use rayon for embarrassingly parallel scenario analysis
