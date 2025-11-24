# Packaging & Installation

GAT 0.3.1 introduces a modular installation system with two main approaches: **modular component selection** (on-demand installation) and **bundle variants** (pre-packaged tarballs). Both support binary-first delivery with fallback to source builds.

## Architecture Overview

### Directory Structure

Installation creates a clean directory hierarchy under `~/.gat/` (configurable):

```
~/.gat/
├── bin/           # Executables (gat, gat-cli, gat-tui, gat-gui)
├── config/        # Configuration files (gat.toml, tui.toml, gui.toml)
├── lib/
│   └── solvers/   # Solver binaries and data packages
└── cache/         # Dataset cache and run history
```

This structure enables:
- **Component isolation** — Each component installs to a specific location
- **Easy uninstall** — Remove `~/.gat/` to completely remove GAT
- **Configuration management** — Centralized config with per-user defaults
- **Clean upgrades** — New versions overwrite only what's needed

### Modular Component System

GAT now defines four installable components:

1. **`cli`** (always required)
   - Core `gat-cli` binary with power flow, OPF, time-series, and analytics
   - Minimal dependencies, ~5 MB headless variant

2. **`tui`** (optional)
   - Interactive terminal UI with 7-pane dashboard
   - For exploration, batch monitoring, and workflow visualization
   - ~10 MB additional footprint

3. **`gui`** (stub, future release)
   - Web-based dashboard planned for Horizon 7
   - Current releases include placeholder

4. **`solvers`** (optional)
   - Additional solver backends: CBC, HiGHS (beyond default Clarabel)
   - Distributed as data package extracting to `lib/solvers/`

## Installation Methods

### 1. Modular Installer (Recommended)

The `scripts/install-modular.sh` script enables on-demand component selection:

```bash
# CLI only (default)
curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh | bash

# CLI + TUI
GAT_COMPONENTS=cli,tui bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh)

# Everything
GAT_COMPONENTS=cli,tui,gui,solvers bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.3.1/scripts/install-modular.sh)
```

**Features:**
- No Rust dependency required (downloads pre-built binaries)
- Automatic OS/architecture detection (Linux, macOS; x86_64, ARM64)
- Version resolution from GitHub API
- Fallback to source build if binaries unavailable
- Configurable installation prefix via `--prefix` flag
- Environment variables: `GAT_PREFIX`, `GAT_VERSION`, `GAT_COMPONENTS`

**Implementation:**
- Shell script: `scripts/install-modular.sh` (~230 lines)
- Rust CLI module: `crates/gat-cli/src/install/`
  - `component.rs` — Component registry and metadata
  - `github.rs` — GitHub API client (curl + jq)
  - `installer.rs` — Download/extract/fallback logic
  - `gat_home.rs` — Directory structure management
  - `config.rs` — Configuration file templates

### 2. Bundle Variant Installation

For users preferring bundled releases with documentation:

```bash
# Full variant (CLI + TUI + docs, recommended)
curl -fsSL https://github.com/monistowl/gat/releases/download/v0.3.1/gat-0.3.1-linux-x86_64-full.tar.gz | tar xz
cd gat-0.3.1-linux-x86_64-full
./install.sh

# Headless variant (CLI only, minimal footprint)
curl -fsSL https://github.com/monistowl/gat/releases/download/v0.3.1/gat-0.3.1-linux-x86_64-headless.tar.gz | tar xz
cd gat-0.3.1-linux-x86_64-headless
./install.sh --variant headless

# Analyst variant (CLI + visualization/analysis tools)
./install.sh --variant analyst
```

**Bundled Artifacts:**

`scripts/package.sh` (requires `jq` and `cargo`) produces three variants:

| Variant | Contents | Use Case | Size |
|---------|----------|----------|------|
| `headless` | gat-cli only, minimal I/O | Embedded systems, servers | ~5 MB |
| `analyst` | gat-cli + visualization tools | Analysis workflows | ~15 MB |
| `full` | gat-cli + TUI + docs + all features | Default, recommended | ~20 MB |

Tarballs are named: `gat-<version>-<os>-<arch>-<variant>.tar.gz`

Each tarball includes:
- `bin/` directory with executables
- `README.md` and `LICENSE.txt`
- `scripts/` subdirectory with `install.sh` and helpers
- `docs/` folder (full variant only)

### 3. Source Build (Fallback)

When binaries are unavailable or for development:

```bash
# Requires Rust: https://rustup.rs

# Full features (default)
cargo build -p gat-cli --release --all-features

# Headless (minimal dependencies)
cargo build -p gat-cli --release --no-default-features --features minimal-io

# Analyst (visualization + analysis)
cargo build -p gat-cli --release --no-default-features --features "minimal-io,viz,all-backends"
```

Both installers automatically fall back to source build if binaries are unavailable.

## Release Workflow

### 1. Building Release Binaries

```bash
# For each variant: headless, analyst, full
scripts/package.sh headless
scripts/package.sh analyst
scripts/package.sh full
```

Artifacts land in `dist/`:

```
dist/
├── gat-0.3.1-linux-x86_64-headless.tar.gz
├── gat-0.3.1-linux-x86_64-analyst.tar.gz
├── gat-0.3.1-linux-x86_64-full.tar.gz
├── gat-0.3.1-macos-x86_64-full.tar.gz
├── gat-0.3.1-macos-arm64-full.tar.gz
└── ...
```

### 2. GitHub Release Notes

The `.github/workflows/create-release.yml` workflow:
- Builds packages for Linux and macOS (both x86_64 and arm64)
- Generates release notes with component selection examples
- Attaches tarballs as release assets
- Provides both modular installer and bundle variant examples

Release notes feature:
- Modular installation one-liner as primary method
- Component selection examples
- Bundle variant downloads for each platform
- PATH setup instructions

### 3. Version Management

All 16 crates in the workspace maintain synchronized versions via `Cargo.toml` workspace metadata:

```toml
[workspace]
members = [ "crates/*" ]
[workspace.package]
version = "0.3.1"
```

Update with:

```bash
# Update all crate versions at once
sed -i 's/version = "0.3.0"/version = "0.3.1"/g' crates/*/Cargo.toml Cargo.toml
```

## Download URLs & Naming Convention

### GitHub Release Downloads

Artifacts are accessible via:

```
https://github.com/monistowl/gat/releases/download/v0.3.1/gat-0.3.1-<os>-<arch>-<variant>.tar.gz
```

**Platform/Architecture Codes:**
- OS: `linux`, `macos`
- Architecture: `x86_64`, `arm64`
- Variant: `headless`, `analyst`, `full`

**Examples:**
- `gat-0.3.1-linux-x86_64-full.tar.gz`
- `gat-0.3.1-macos-arm64-full.tar.gz`
- `gat-0.3.1-linux-x86_64-headless.tar.gz`

### Modular Component Artifacts

Individual component binaries for modular installation:

```
https://github.com/monistowl/gat/releases/download/v0.3.1/gat-tui-0.3.1-<os>-<arch>.tar.gz
https://github.com/monistowl/gat/releases/download/v0.3.1/gat-gui-0.3.1-<os>-<arch>.tar.gz
https://github.com/monistowl/gat/releases/download/v0.3.1/gat-solvers-0.3.1-<os>-<arch>.tar.gz
```

## Testing

`scripts/test-modular-install.sh` validates:

- Script existence and syntax
- Help output and documentation
- Directory structure creation
- Environment variable handling
- Component argument parsing
- Integration with gat CLI

Run with:

```bash
bash scripts/test-modular-install.sh        # Full build test
bash scripts/test-modular-install.sh --quick  # Fast syntax/help checks
```

## Key Files

| File | Purpose |
|------|---------|
| `scripts/install-modular.sh` | On-demand component installer |
| `scripts/install.sh` | Bundle variant installer |
| `scripts/package.sh` | Release artifact builder |
| `scripts/test-modular-install.sh` | Installation validation tests |
| `crates/gat-cli/src/install/` | Rust install module |
| `.github/workflows/create-release.yml` | Release automation |
| `docs/guide/packaging.md` | This file |

## Migration from v0.1

v0.1 users upgrading to v0.3.1:

- Installation location changed from `~/.local/` to `~/.gat/`
- New directory structure separates config, binaries, and cache
- Modular component selection available (TUI, GUI, solvers optional)
- Configuration files auto-created with sensible defaults
- All steps the same: re-run installer, update PATH

See `FAQ & Migration Guide` in `README.md` for user-facing instructions.
