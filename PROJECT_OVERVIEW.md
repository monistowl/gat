# GAT Project Overview - Complete Analysis

**Generated:** 2025-11-26  
**Version:** 0.4.0  
**Total Lines of Code:** ~75,000 lines of Rust

---

## Executive Summary

**Grid Analysis Toolkit (GAT)** is a comprehensive Rust-based power systems analysis toolkit providing industrial-grade tools for grid modeling, power flow analysis, optimal dispatch, and time-series operations. The project emphasizes:

- **Performance**: Rust-native execution with C-like speed
- **Modularity**: 17 crates organized by domain
- **Accessibility**: CLI, TUI, and planned GUI interfaces
- **Interoperability**: Arrow/Parquet outputs for Python/R/SQL integration
- **Reproducibility**: All operations emit manifests for exact replay

---

## Project Structure

### Workspace Organization

```
gat/
├── crates/          # 17 Rust crates (workspace members)
├── docs/            # User guides, CLI reference, schemas
├── test_data/       # Test fixtures (MATPOWER, PSS/E, CIM, etc.)
├── scripts/         # Installation, packaging, CI/CD
├── examples/        # Usage examples
├── .beads/          # Issue tracking database (bd)
└── site/            # Documentation website
```

### Crate Breakdown (by LOC)

| Crate | LOC | Purpose | Status |
|-------|-----|---------|--------|
| **gat-tui** | ~25,000 | Interactive terminal UI (7 panes) | ✅ Complete |
| **gat-algo** | ~15,000 | Power flow, OPF, reliability algorithms | ✅ Stable |
| **gat-io** | ~10,000 | Data import/export (MATPOWER, PSS/E, CIM, Arrow) | ⚠️ Schema refactor in progress |
| **gat-cli** | ~8,000 | Command-line interface and dispatcher | ✅ Stable |
| **gat-core** | ~2,000 | Core types, graph utilities, diagnostics | ✅ Stable |
| **gat-scenarios** | ~600 | Scenario definition and materialization | ✅ Complete |
| **gat-batch** | ~400 | Parallel job orchestration | ✅ Complete |
| **gat-ts** | ~500 | Time-series operations | ✅ Complete |
| **gat-adms** | ~1,500 | Distribution automation (FLISR/VVO) | ✅ Complete |
| **gat-derms** | ~600 | DER management and aggregation | ✅ Complete |
| **gat-dist** | ~450 | Distribution system modeling | ✅ Complete |
| **gat-notebook** | ~1,000 | Jupyter integration | 🚧 Experimental |
| **gat-viz** | ~100 | Visualization helpers | 🚧 Minimal |
| **gat-gui** | ~200 | Web dashboard | 🚧 Stub (Horizon 7) |
| **gat-schemas** | ~20 | Schema helpers | ✅ Utility |
| **gat-mcp-docs** | ~350 | MCP server for agent integration | ✅ Complete |
| **iocraft** | ~250 | Custom UI framework | ✅ Utility |

---

## Architecture

### Core Design Principles

1. **Single Responsibility**: Each crate has a focused domain
2. **Lossless Roundtrips**: Import/export preserves all data
3. **Error Recovery**: Partial imports with diagnostics
4. **Reproducibility**: All operations emit manifests
5. **Performance**: Rust-native with parallel execution

### Data Flow

```
Input Formats          GAT Core              Output Formats
─────────────         ──────────            ──────────────
MATPOWER (.m)    →                    →    Arrow/Parquet
PSS/E (.raw)     →    Network Graph   →    CSV tables
CIM RDF/XML      →    + Algorithms    →    MATPOWER
PandaPower JSON  →                    →    PandaPower JSON
                                            PSS/E RAW
```

### Key Abstractions

#### Network Graph (gat-core)
- Nodes: Buses, Generators, Loads
- Edges: Branches, Transformers
- Petgraph-based directed graph
- Validation: Referential integrity, topology checks

#### Solver Backends (gat-algo)
- **DC Power Flow**: Linear approximation (fast)
- **AC Power Flow**: Newton-Raphson with Q-limits
- **DC OPF**: Linear programming (Clarabel/HiGHS/CBC)
- **AC OPF**: Nonlinear L-BFGS penalty method
- **SOCP**: Second-order cone programming

#### Import/Export Pipeline (gat-io)
```
Format Detection → Parse → Validate → Build Graph → Export
     ↓                ↓         ↓           ↓           ↓
  .m/.raw/.rdf    Structs   Diagnostics  Network    Arrow/CSV
```

---

## Current State & Active Work

### ✅ Completed Features

1. **CLI Interface** (gat-cli)
   - 50+ commands across 10 categories
   - Shell completions (bash/zsh/fish/powershell)
   - Modular installation system

2. **Terminal UI** (gat-tui)
   - 7-pane dashboard (Dashboard, Commands, Datasets, Pipeline, Operations, Analytics, Settings)
   - 536+ tests
   - Real-time batch job monitoring
   - Interactive command execution

3. **Power Flow Solvers** (gat-algo)
   - DC/AC power flow
   - DC/AC optimal power flow
   - N-1/N-2 contingency analysis
   - State estimation (WLS)

4. **Distribution Tools** (gat-adms, gat-derms, gat-dist)
   - FLISR (Fault Location, Isolation, Service Restoration)
   - VVO (Volt-Var Optimization)
   - DER aggregation and scheduling
   - Hosting capacity analysis

5. **Analytics** (gat-algo)
   - Reliability metrics (LOLE, EUE)
   - Deliverability scores
   - ELCC (Effective Load Carrying Capability)
   - PTDF (Power Transfer Distribution Factors)

### ⚠️ In Progress

#### 1. **Normalized Arrow Schema Refactor** (gat-181, gat-j17)
**Status:** 60% complete, compilation errors blocking

**Goal:** Lossless roundtrips for all formats

**Changes:**
- Multi-file Arrow directory format (buses.arrow, generators.arrow, etc.)
- Expanded schema with all MATPOWER/PandaPower fields
- Manifest with checksums and provenance
- Atomic writes via temp directory

**Blockers:**
- 39 compilation errors in gat-io
- Arrow importer/exporter need updates
- Other importers (CIM, PandaPower, PSS/E) need schema updates

**Files Modified:**
- `crates/gat-io/src/helpers/network_builder.rs` - Added new fields
- `crates/gat-io/src/importers/matpower.rs` - Populate all fields
- `crates/gat-io/src/exporters/arrow_directory_writer.rs` - Needs fixes
- `crates/gat-io/src/importers/arrow.rs` - Needs major updates

#### 2. **Unified Exporter Interface** (gat-pmw)
**Status:** Planned, blocked by schema refactor

**Goal:** Mirror importer pattern for exports

**Phases:**
1. Foundation: ExportFormat enum, CLI integration
2. MATPOWER export with roundtrip tests
3. CSV export for spreadsheet tools
4. Optional: PSS/E, PandaPower exports

**Dependencies:**
- Requires gat-181 (MATPOWER importer) complete
- Requires gat-wao (PandaPower importer) complete

### 🚧 Experimental

1. **Jupyter Integration** (gat-notebook)
   - Python bindings via PyO3
   - Notebook-friendly APIs
   - Status: Experimental, not in default build

2. **GUI Dashboard** (gat-gui)
   - Web-based interface
   - Status: Stub, planned for Horizon 7

---

## Testing & Quality

### Test Coverage

| Crate | Tests | Coverage | Notes |
|-------|-------|----------|-------|
| gat-tui | 536+ | High | Visual, interactive, integration tests |
| gat-algo | 50+ | High | AC/DC OPF, reliability, SOCP |
| gat-io | 30+ | Medium | Import/export roundtrips |
| gat-cli | 20+ | Medium | CLI integration, benchmarks |
| gat-core | 15+ | Medium | Graph utilities, diagnostics |

### Benchmark Suites

1. **PGLib** - 68 MATPOWER cases (14 to 13,659 buses)
   - AC-OPF: 65/68 passing, median 2.9% gap
   - Command: `gat benchmark pglib`

2. **OPFData** - Public OPF test cases
   - Command: `gat benchmark opfdata`

3. **PFDelta** - Power flow validation
   - Command: `gat benchmark pfdelta`

4. **Arrow I/O Performance** (gat-3of)
   - Status: Benchmarks implemented, blocked by compilation errors
   - Tests: Load, write, roundtrip across 6 network sizes

---

## Documentation

### User Documentation

```
docs/
├── guide/
│   ├── overview.md          # CLI architecture
│   ├── pf.md                # Power flow guide
│   ├── opf.md               # Optimal power flow
│   ├── adms.md              # Distribution automation
│   ├── derms.md             # DER management
│   ├── dist.md              # Distribution systems
│   ├── ts.md                # Time-series operations
│   ├── se.md                # State estimation
│   ├── graph.md             # Network topology
│   ├── datasets.md          # Public datasets
│   ├── gat-tui.md           # Terminal UI guide
│   ├── cli-architecture.md  # CLI design
│   ├── feature-matrix.md    # CI/CD testing
│   ├── mcp-onboarding.md    # Agent integration
│   ├── packaging.md         # Binary distribution
│   └── scaling.md           # Performance tuning
├── cli/
│   └── gat.md               # Auto-generated CLI reference
├── schemas/
│   └── *.json               # JSON schemas for manifests
└── plans/
    ├── 2025-11-26-normalized-arrow-schema-design.md
    └── 2025-11-26-unified-exporter-design.md
```

### Developer Documentation

- **AGENTS.md** - Agent integration guide
- **RELEASE_PROCESS.md** - Branch strategy (experimental → staging → main)
- **README.md** - User-facing overview
- **CHANGELOG.md** - Version history
- Per-crate READMEs in `crates/*/README.md`

---

## Issue Tracking

### System: Beads (bd)

**Database:** `.beads/*.db` (SQLite + JSONL)

**Current Issues:** 50+ tracked

**Key Epics:**
1. **gat-j17** - Normalized Arrow Schema (in_progress)
2. **gat-pmw** - Unified Exporter Interface (open)
3. **gat-2u9** - Integrate CLI features into TUI (open)
4. **gat-9uc** - Ground-up TUI replacement (in_progress)
5. **gat-e6s** - Complete TUI implementation (open)

**Priority Distribution:**
- P1 (High): 30+ issues
- P2 (Medium): 10+ issues
- P3 (Low): 5+ issues

**Status Distribution:**
- Open: 25+
- In Progress: 5+
- Closed: 20+

---

## Dependencies

### Core Dependencies

**Rust Toolchain:** 1.75+

**Key Crates:**
- `petgraph` - Graph data structures
- `polars` - DataFrame operations (Arrow/Parquet)
- `good_lp` - Linear programming abstraction
- `clarabel` - Default SOCP solver
- `ratatui` - Terminal UI framework
- `clap` - CLI argument parsing
- `serde` - Serialization
- `anyhow` - Error handling

**Optional Solvers:**
- HiGHS - LP/MIP solver
- CBC - LP/MIP solver
- IPOPT - Nonlinear solver (future)

### Build Variants

1. **Full** (default)
   - CLI + TUI + all features
   - ~20 MB binary

2. **Headless**
   - CLI only, minimal I/O
   - ~5 MB binary

3. **Analyst**
   - CLI + visualization + all solvers
   - ~25 MB binary

---

## Installation & Distribution

### Installation Methods

1. **Modular Installer** (recommended)
   ```bash
   curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.4.0/scripts/install-modular.sh | bash
   ```
   - Installs to `~/.gat/`
   - Component selection: cli, tui, gui, solvers
   - Binary-first, falls back to source build

2. **Bundle Variants**
   - Full tarball with docs
   - Headless variant for servers

3. **Build from Source**
   ```bash
   cargo build -p gat-cli --release --all-features
   ```

### Directory Structure

```
~/.gat/
├── bin/           # Executables (gat, gat-tui, gat-gui)
├── config/        # Configuration (gat.toml, tui.toml, gui.toml)
├── lib/solvers/   # Solver binaries
└── cache/         # Dataset cache, run history
```

---

## Performance Characteristics

### Benchmarks

| Operation | 14 bus | 118 bus | 1354 bus | Target |
|-----------|--------|---------|----------|--------|
| DC PF | ~50ms | ~200ms | ~1s | <2s |
| AC PF | ~500ms | ~2s | ~10s | <30s |
| DC OPF | ~100ms | ~500ms | ~3s | <10s |
| AC OPF | ~2s | ~10s | ~60s | <120s |
| N-1 (100 contingencies) | ~5s | ~20s | ~120s | <300s |

### Scalability

- **Parallel Execution**: Rayon-based parallelism
- **Batch Jobs**: Fan-out across machines with GNU parallel
- **Memory**: Efficient Arrow columnar format
- **I/O**: Parquet compression (35-45% size reduction)

---

## Roadmap

### Horizon 6 (Current - v0.4.x)

- ✅ Modular installation
- ✅ TUI dashboard
- ✅ Distribution tools (ADMS/DERMS)
- ⚠️ Normalized Arrow schema (in progress)
- 🔜 Unified exporter interface

### Horizon 7 (Planned - v0.5.0)

- GUI dashboard (web-based)
- Enhanced visualization
- Real-time data integration
- Advanced reliability metrics
- Multi-area coordination

### Future Considerations

- Python SDK (PyO3 bindings)
- Cloud deployment (Kubernetes)
- Real-time SCADA integration
- Machine learning integration
- Multi-objective optimization

---

## Contributing

### Development Setup

1. Install Rust: https://rustup.rs
2. Clone repository
3. Install `bd` for issue tracking
4. Run `bd ready` to see available tasks
5. See `RELEASE_PROCESS.md` for branch strategy

### Branch Strategy

- **experimental** - Active development
- **staging** - Pre-release testing
- **main** - Stable releases

### Code Quality

- Clippy lints enforced
- Rustfmt for formatting
- Comprehensive test coverage
- Documentation required for public APIs

---

## License

See `LICENSE.txt` in repository root.

---

## Contact & Support

- **Issues**: Use `bd` issue tracker (`.beads/`)
- **Documentation**: `docs/guide/`
- **Examples**: `examples/`, `test_data/`
- **MCP Server**: `gat-mcp-docs` for agent integration

---

## Summary Statistics

- **Total LOC**: ~75,000 lines of Rust
- **Crates**: 17 workspace members
- **Commands**: 50+ CLI commands
- **Tests**: 600+ across all crates
- **Documentation**: 20+ guide documents
- **Supported Formats**: MATPOWER, PSS/E, CIM, PandaPower, Arrow, CSV
- **Solver Backends**: Clarabel, HiGHS, CBC
- **Platforms**: Linux, macOS (x86_64, ARM64)

---

**Last Updated:** 2025-11-26  
**Project Status:** Active Development  
**Current Focus:** Arrow schema refactor, unified exporter interface
