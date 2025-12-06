+++
title = "Packaging & Installation"
description = "Building release packages and installing GAT with modular components"
weight = 150
+++

# Packaging & Installation

GAT 0.5.5 introduces a modular installation system with two main approaches: **modular component selection** (on-demand installation) and **bundle variants** (pre-packaged tarballs). Both support binary-first delivery with fallback to source builds.

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
curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.5.5/scripts/install-modular.sh \
  | bash

# CLI + TUI
GAT_COMPONENTS=cli,tui \
  bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.5.5/scripts/install-modular.sh)

# Everything
GAT_COMPONENTS=cli,tui,gui,solvers \
  bash <(curl -fsSL https://raw.githubusercontent.com/monistowl/gat/v0.5.5/scripts/install-modular.sh)
```

**Features:**
- No Rust dependency required (downloads pre-built binaries)
- Automatic OS/architecture detection (Linux, macOS; x86_64, ARM64)
- Version resolution from GitHub API
- Fallback to source build if binaries unavailable
- Configurable installation prefix via `--prefix` flag
- Environment variables: `GAT_PREFIX`, `GAT_VERSION`, `GAT_COMPONENTS`

### 2. Bundle Variant Installation

For users preferring bundled releases with documentation:

```bash
# Full variant (CLI + TUI + docs, recommended)
curl -fsSL \
  https://github.com/monistowl/gat/releases/download/v0.5.5/gat-0.5.5-linux-x86_64-full.tar.gz \
  | tar xz
cd gat-0.5.5-linux-x86_64-full && ./install.sh

# Headless variant (CLI only, minimal footprint)
curl -fsSL \
  https://github.com/monistowl/gat/releases/download/v0.5.5/gat-0.5.5-linux-x86_64-headless.tar.gz \
  | tar xz
cd gat-0.5.5-linux-x86_64-headless && ./install.sh --variant headless
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

### Building Release Binaries

```bash
# For each variant: headless, analyst, full
scripts/package.sh headless
scripts/package.sh analyst
scripts/package.sh full
```

Artifacts land in `dist/`:

```
dist/
├── gat-0.5.5-linux-x86_64-headless.tar.gz
├── gat-0.5.5-linux-x86_64-analyst.tar.gz
├── gat-0.5.5-linux-x86_64-full.tar.gz
├── gat-0.5.5-macos-x86_64-full.tar.gz
├── gat-0.5.5-macos-arm64-full.tar.gz
└── ...
```

### Download URLs & Naming Convention

Artifacts are accessible via:

```
https://github.com/monistowl/gat/releases/download/v0.5.5/gat-0.5.5-<os>-<arch>-<variant>.tar.gz
```

**Platform/Architecture Codes:**
- OS: `linux`, `macos`
- Architecture: `x86_64`, `arm64`
- Variant: `headless`, `analyst`, `full`

**Examples:**
- `gat-0.5.5-linux-x86_64-full.tar.gz`
- `gat-0.5.5-macos-arm64-full.tar.gz`
- `gat-0.5.5-linux-x86_64-headless.tar.gz`

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

## Migration from v0.1

v0.1 users upgrading to v0.5.5:

- Installation location changed from `~/.local/` to `~/.gat/`
- New directory structure separates config, binaries, and cache
- Modular component selection available (TUI, GUI, solvers optional)
- Configuration files auto-created with sensible defaults
- All steps the same: re-run installer, update PATH

See the main `README.md` for user-facing installation instructions and migration guide.
