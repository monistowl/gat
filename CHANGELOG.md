# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.3.4] - 2025-11-25

### Added

#### Full Nonlinear AC-OPF Solver

- **Y-bus construction** (`crates/gat-algo/src/opf/ac_nlp/ybus.rs`)
  - Complex admittance matrix from network topology
  - Series admittance from branch resistance and reactance
  - Tap ratio and phase shift support for transformers
  - Line charging (shunt susceptance) from π-model
  - Dense matrix storage with bus ID to index mapping

- **AC power flow equations** (`crates/gat-algo/src/opf/ac_nlp/power_equations.rs`)
  - Active and reactive power injection calculations: P_i and Q_i
  - Full Jacobian computation with partial derivatives (∂P/∂θ, ∂P/∂V, ∂Q/∂θ, ∂Q/∂V)
  - Support for polar formulation with voltage magnitude and angle variables

- **NLP problem formulation** (`crates/gat-algo/src/opf/ac_nlp/problem.rs`)
  - Variable layout: [V, θ, P_g, Q_g]
  - Quadratic objective function with polynomial cost curves
  - Equality constraints for power balance at each bus
  - Bound constraints for voltage limits and generator limits
  - Flat start initialization for warm starting the solver

- **Penalty-method solver** (`crates/gat-algo/src/opf/ac_nlp/solver.rs`)
  - L-BFGS quasi-Newton optimizer from argmin crate
  - Penalty function approach for handling equality constraints
  - Iterative penalty increase until constraint feasibility achieved
  - Finite difference gradient computation
  - LMP approximation from marginal generator costs

- **OpfMethod::AcOpf** now routes to the new ac_nlp solver module instead of returning NotImplemented

- **Comprehensive test suite** (`crates/gat-algo/tests/ac_opf.rs`)
  - Basic convergence tests on 2-bus network
  - Comparison with SOCP relaxation (validates AC cost >= SOCP bound)
  - Economic dispatch verification on 3-bus meshed network
  - Merit order dispatch validation (cheaper generators prioritized)

#### Documentation

- **Updated `docs/guide/opf.md`** with Full AC-OPF section
  - Feature matrix showing implemented and planned capabilities
  - Usage examples with code snippets
  - Mathematical formulation with power flow equations
  - Solver backend explanation (L-BFGS penalty method)

### Dependencies

- Added `argmin = "0.10"` for L-BFGS optimization
- Added `argmin-math = "0.4"` with vec feature for vector operations

---

## [0.3.3] - 2025-11-25

### Added

#### SOCP Relaxation Solver

- **Full SOCP relaxation implementation** in `gat-algo` for AC Optimal Power Flow
  - Baran-Wu / Farivar-Low branch-flow model with squared voltage/current variables
  - Convex second-order cone constraints for global optimality guarantees
  - Clarabel interior-point solver backend (15-30 iterations typical)

- **Quadratic cost support**
  - Full polynomial cost curves: `cost = c₀ + c₁·P + c₂·P²`
  - Proper marginal cost computation: `MC = c₁ + 2·c₂·P`
  - Quadratic objective matrix construction for Clarabel

- **Phase-shifting transformer support**
  - Angle variables (θ) for each bus with linearized coupling
  - Phase shift angle (φ) applied in voltage drop equations
  - Reference bus angle fixed to zero

- **Comprehensive transformer modeling**
  - Off-nominal tap ratios with τ² voltage transformation
  - Line charging via π-model (half-shunt susceptance at each end)
  - Thermal limits from `s_max_mva` or `rating_a_mva`

- **LMP and dual variable extraction**
  - Locational Marginal Prices from power balance constraint duals
  - Binding constraint identification with shadow prices
  - Voltage, thermal, and generator limit tracking

- **Extensive test suite** (`crates/gat-algo/tests/socp.rs`)
  - 3-bus and 10-bus meshed network tests
  - Quadratic cost optimization verification
  - Phase-shifting transformer tests
  - Tap ratio transformer tests
  - Thermal limit binding tests
  - Line charging tests

#### Documentation

- **Comprehensive didactic comments** in `socp.rs` (~1600 lines)
  - Full mathematical derivations with ASCII diagrams
  - Per-unit system explanation (IEEE Std 141-1993)
  - Branch-flow model fundamentals
  - SOC constraint transformation (rotated to standard cone)
  - Academic citations with DOIs:
    - Baran & Wu (1989): DOI:10.1109/61.25627
    - Farivar & Low (2013): DOI:10.1109/TPWRS.2013.2255317
    - Gan, Li, Topcu & Low (2015): DOI:10.1109/TAC.2014.2332712
    - Low (2014): DOI:10.1109/TCNS.2014.2309732
    - Jabr (2006): DOI:10.1109/TPWRS.2006.879234
    - Schweppe et al. (1988): DOI:10.1007/978-1-4613-1683-1

- **Updated `docs/guide/opf.md`**
  - SOCP feature matrix with supported capabilities
  - Usage examples and solver backend details
  - Mathematical foundation summary

### Changed

- **`OpfMethod::SocpRelaxation`** now routes to the SOCP solver instead of returning `NotImplemented`
- **Variable layout** in SOCP solver adds angle variables between voltage and generator variables
- **Objective computation** now includes full polynomial (c₀ + c₁·P + c₂·P²) instead of linear only

### Fixed

- **Dead code warnings** for `base_kv` and `phase_shift_rad` fields
  - `base_kv` now used in `compute_system_base_kv()` for multi-voltage scaling
  - `phase_shift_rad` now applied in angle-coupled voltage drop equations

---

## [0.3.1] - 2025-11-24

### Added

#### Modular Installation System

- **New `install-modular.sh` script** for on-demand component installation
  - Select components at install time: `cli`, `tui`, `gui`, `solvers`
  - Automatic OS/architecture detection (Linux, macOS; x86_64, ARM64)
  - Version resolution from GitHub API using `jq`
  - Environment variable support: `GAT_PREFIX`, `GAT_VERSION`, `GAT_COMPONENTS`
  - Comprehensive error handling with helpful fallback to source builds

- **Rust-based Modular Install Module** (`crates/gat-cli/src/install/`)
  - `component.rs` — Component registry with enum-based architecture
    - Supports `Tui`, `Gui`, `Solvers` component types
    - Binary name and artifact prefix mapping
    - Installation status checking with proper directory logic
  - `github.rs` — GitHub API client for release fetching and download URLs
    - `fetch_latest_release()` using curl + jq for JSON parsing
    - OS/architecture detection functions
    - Cross-platform binary naming conventions
  - `installer.rs` — Main installation logic with fallback pattern
    - Binary download from GitHub releases with automatic extraction
    - Fallback to source build when binary unavailable
    - Special handling for Solvers component (extract to lib/solvers, no source build)
    - `find_binary_in_dir()` helper for handling various tarball structures
  - `gat_home.rs` — Directory structure management
    - Automatic creation of `~/.gat/{bin,config,lib,cache}` directories
    - Centralized configuration directory management
  - `config.rs` — Configuration file management
    - Default templates for `gat.toml`, `tui.toml`, `gui.toml`
    - Configuration file auto-creation with sensible defaults

#### Installation Structure

- **New directory layout** `~/.gat/`
  - `bin/` — Executable binaries (gat, gat-cli, gat-tui, gat-gui)
  - `config/` — Configuration files (gat.toml, tui.toml, gui.toml)
  - `lib/solvers/` — Solver binaries and data packages
  - `cache/` — Dataset cache and run history

- **Updated `install.sh`**
  - Changed default prefix from `~/.local` to `~/.gat`
  - Maintains compatibility with bundle variant installation
  - Supports fallback to source build when binaries unavailable

- **Enhanced `package.sh`**
  - Conditional inclusion of modular binaries (gat-tui, gat-gui)
  - Separate packaging for headless, analyst, and full variants
  - Proper artifact naming for modular components

#### Shell Scripts & Testing

- **New `test-modular-install.sh`** comprehensive test suite
  - Validates script existence and syntax
  - Tests directory structure creation
  - Verifies component parsing and environment variables
  - Color-coded test output with pass/fail tracking
  - 9 total test cases covering core workflows

#### Documentation

- **Updated README.md Installation section**
  - Modular installer featured as primary installation method
  - One-liner curl command for quick start
  - Component selection examples (CLI, CLI+TUI, full, everything)
  - Bundle variant installation as alternative
  - Installation directory structure documentation
  - Source build instructions with feature flag explanations
  - Development setup guide

- **New FAQ & Migration Guide section in README.md**
  - Upgrade instructions from v0.1 to v0.3.1
  - Side-by-side installation guide
  - Component explanation (TUI, solvers, variants)
  - Configuration location and customization
  - Data storage and environment variables
  - Troubleshooting common issues
  - Piping and data format information

#### GitHub Actions

- **Updated `.github/workflows/create-release.yml`**
  - Release notes feature modular installation as primary method
  - Includes component selection examples in release notes
  - Documents PATH setup for installed binaries
  - Bundle variant alternatives for direct tarball installation

### Changed

- **Version bumped to 0.3.1** across all 16 crates in workspace
  - Core crates: gat-core, gat-io, gat-cli, gat-tui
  - Domain crates: gat-adms, gat-derms, gat-dist, gat-algo
  - Support crates: gat-batch, gat-scenarios, gat-schemas, gat-ts, gat-viz
  - Plus CLI-specific crates

- **Default installation location** changed from `~/.local/` to `~/.gat/`
  - More discoverable and user-friendly naming
  - Aligns with other tools (pyenv, nvm style)
  - Avoids conflicts with system package managers

- **Binary naming consistency**
  - `gat-cli` remains the primary executable
  - `gat` symlink/copy for shorter command invocation
  - Modular components: `gat-tui`, `gat-gui`, `gat-cli`

- **GitHub API integration pattern**
  - Switched from Python heredoc JSON parsing to `jq` command-line tool
  - More reliable and cross-platform compatible
  - Eliminates stdin consumption issues

- **TUI launch in documentation**
  - Primary command now: `gat-tui` (post-installation)
  - Development fallback: `cargo run -p gat-tui --release`

### Fixed

- **Install script stdin consumption issue**
  - Root cause: Python heredoc was consuming stdin instead of piped curl output
  - Solution: Replaced with `jq -r '.tag_name // empty'` for JSON parsing

- **Tarball extraction path handling**
  - Release tarballs can have various directory structures
  - Added `find_binary_in_dir()` helper to search both root and subdirectories
  - Ensures binaries are found regardless of tarball structure

- **Solvers component special handling**
  - Recognized Solvers as data package (not binary-only)
  - Extracts to `lib/solvers/` instead of `bin/`
  - Rejects source builds for Solvers component with clear error message

- **Borrow checker lifetime issues** in installer.rs
  - Fixed temporary value drops with explicit variable separation
  - Changed method chains to preserve references across statements

- **Code quality**
  - Removed unused imports (`Path` from gat_home.rs, `serde_json::Value` from github.rs)
  - Renamed `Component::from_str()` to `Component::parse()` to avoid trait conflicts
  - Applied consistent formatting with `cargo fmt`
  - Addressed clippy warnings about standard library method naming

### Test Coverage

- All 16 crates successfully compile and pass tests
- GitHub Actions CI validates across Linux and macOS platforms
- `test-modular-install.sh` provides end-to-end validation of install workflows
- Manual testing confirms binary download, fallback builds, and component selection

### Known Limitations

- GUI component stub (releases with placeholder, full implementation planned for Horizon 7)
- Solvers component requires binary distribution (source build not supported)
- ARM64 macOS builds available but less frequently tested

---

## [0.1.0] - Previous Release

See git history for changes prior to v0.3.1.
