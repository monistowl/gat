# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
