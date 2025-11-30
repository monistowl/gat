#!/usr/bin/env bash
#
# GAT Modular Installer
#
# This script installs GAT components on-demand by downloading the full bundle
# and extracting only the requested components:
#  - cli (core CLI, always installed with gat-cli and gat symlink)
#  - tui (optional TUI interface)
#  - gui (optional GUI dashboard)
#  - solvers (optional native solver binaries + shared libraries)
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/monistowl/gat/main/scripts/install-modular.sh | bash
#   Or: bash install-modular.sh [OPTIONS]
#
# Examples:
#   # CLI only (default)
#   bash install-modular.sh
#
#   # CLI + TUI
#   bash install-modular.sh --components cli,tui
#
#   # Everything
#   bash install-modular.sh --components cli,tui,gui,solvers
#
set -euo pipefail

usage() {
  cat <<'USAGE'
GAT Modular Installer

Usage: bash install-modular.sh [OPTIONS]

Options:
  --prefix DIR         Install prefix (default: ~/.gat)
  --version VERSION    Version to install (default: latest)
  --components COMPS   Comma-separated components to install
                       Available: cli, tui, gui, solvers
                       Default: cli
  --help              Show this help message

Environment variables:
  GAT_PREFIX          Install prefix (overridden by --prefix)
  GAT_VERSION         Version to install (overridden by --version)
  GAT_COMPONENTS      Components to install (overridden by --components)

Examples:
  # Install CLI only
  bash install-modular.sh

  # Install CLI + TUI
  bash install-modular.sh --components cli,tui

  # Install everything
  bash install-modular.sh --components cli,tui,gui,solvers

  # Install to custom location
  bash install-modular.sh --prefix /opt/gat --components cli,tui
USAGE
}

# Configuration
PREFIX="${GAT_PREFIX:-$HOME/.gat}"
VERSION="${GAT_VERSION:-latest}"
COMPONENTS="${GAT_COMPONENTS:-cli}"
GITHUB_REPO="monistowl/gat"
RELEASE_BASE="https://github.com/$GITHUB_REPO/releases/download"
GITHUB_API="https://api.github.com/repos/$GITHUB_REPO/releases/latest"

# Detect OS and architecture
detect_os() {
  case "$(uname -s)" in
    Linux) echo "linux" ;;
    Darwin) echo "macos" ;;
    *)
      echo "Unsupported operating system: $(uname -s)" >&2
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo "x86_64" ;;
    arm64|aarch64) echo "arm64" ;;
    *)
      echo "Unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

# Resolve version from GitHub API
resolve_version() {
  if [[ "$VERSION" == "latest" ]]; then
    if command -v jq &>/dev/null; then
      VERSION=$(curl -fsSL "$GITHUB_API" | jq -r '.tag_name // empty')
      if [[ -z "$VERSION" ]]; then
        echo "Failed to resolve latest version from GitHub API" >&2
        exit 1
      fi
    else
      echo "ERROR: jq is required to resolve the latest version." >&2
      echo "Please install jq or specify a version with --version" >&2
      exit 1
    fi
  fi
}

# Parse command-line arguments
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --prefix)
        PREFIX="$2"
        shift 2
        ;;
      --version)
        VERSION="$2"
        shift 2
        ;;
      --components)
        COMPONENTS="$2"
        shift 2
        ;;
      --help)
        usage
        exit 0
        ;;
      *)
        echo "Unknown option: $1" >&2
        usage
        exit 1
        ;;
    esac
  done
}

# Ensure directory structure exists
ensure_dirs() {
  mkdir -p "$PREFIX/bin"
  mkdir -p "$PREFIX/config"
  mkdir -p "$PREFIX/lib"
  mkdir -p "$PREFIX/cache"
  mkdir -p "$PREFIX/solvers"
}

# Check if a component is requested
wants_component() {
  local component="$1"
  [[ ",$COMPONENTS," == *",$component,"* ]]
}

# Install components from extracted bundle
install_from_bundle() {
  local bundle_dir="$1"

  # CLI is always installed
  if [[ -f "$bundle_dir/bin/gat-cli" ]]; then
    cp "$bundle_dir/bin/gat-cli" "$PREFIX/bin/gat-cli"
    chmod +x "$PREFIX/bin/gat-cli"
    # Create gat symlink
    cp "$bundle_dir/bin/gat-cli" "$PREFIX/bin/gat"
    chmod +x "$PREFIX/bin/gat"
    echo "  Installed: gat-cli, gat"
  else
    echo "ERROR: gat-cli not found in bundle" >&2
    return 1
  fi

  # TUI (optional)
  if wants_component "tui" && [[ -f "$bundle_dir/bin/gat-tui" ]]; then
    cp "$bundle_dir/bin/gat-tui" "$PREFIX/bin/gat-tui"
    chmod +x "$PREFIX/bin/gat-tui"
    echo "  Installed: gat-tui"
  fi

  # GUI (optional)
  if wants_component "gui" && [[ -f "$bundle_dir/bin/gat-gui" ]]; then
    cp "$bundle_dir/bin/gat-gui" "$PREFIX/bin/gat-gui"
    chmod +x "$PREFIX/bin/gat-gui"
    echo "  Installed: gat-gui"
  fi

  # Solvers (optional) - includes binaries and shared libraries
  if wants_component "solvers"; then
    # Install solver binaries
    if [[ -d "$bundle_dir/solvers" ]]; then
      for solver in "$bundle_dir/solvers"/*; do
        if [[ -x "$solver" ]]; then
          local name
          name=$(basename "$solver")
          cp "$solver" "$PREFIX/solvers/$name"
          chmod +x "$PREFIX/solvers/$name"
          echo "  Installed solver: $name"
        fi
      done
    fi

    # Install shared libraries
    if [[ -d "$bundle_dir/lib" ]]; then
      # Copy all shared libraries, preserving symlinks
      # Linux uses .so*, macOS uses .dylib
      find "$bundle_dir/lib" -maxdepth 1 \( -name "*.so*" -o -name "*.dylib" \) -exec cp -P {} "$PREFIX/lib/" \;
      local lib_count
      lib_count=$(find "$PREFIX/lib" \( -name "*.so*" -o -name "*.dylib" \) 2>/dev/null | wc -l)
      if [[ "$lib_count" -gt 0 ]]; then
        echo "  Installed $lib_count shared libraries"
      fi
    fi
  fi
}

main() {
  parse_args "$@"

  echo "GAT Modular Installer"
  echo "====================="

  resolve_version
  local tag_version="$VERSION"
  # Strip 'v' prefix for artifact naming
  local clean_version="${VERSION#v}"

  local os arch
  os=$(detect_os)
  arch=$(detect_arch)

  echo "Version: $clean_version"
  echo "Platform: $os/$arch"
  echo "Prefix: $PREFIX"
  echo "Components: $COMPONENTS"
  echo

  ensure_dirs

  # Download the full bundle (we extract only what's needed)
  local artifact_name="gat-${clean_version}-${os}-${arch}-full.tar.gz"
  local url="$RELEASE_BASE/${tag_version}/$artifact_name"

  local tmpdir
  tmpdir=$(mktemp -d)
  trap "rm -rf '$tmpdir'" EXIT

  echo "Downloading $artifact_name..."
  if ! curl -fsSL "$url" -o "$tmpdir/$artifact_name"; then
    echo "ERROR: Failed to download release bundle" >&2
    echo "URL: $url" >&2
    exit 1
  fi

  echo "Extracting..."
  tar -xzf "$tmpdir/$artifact_name" -C "$tmpdir"

  # Find the extracted directory (should be gat-{version}-{os}-{arch}-full)
  local bundle_dir
  bundle_dir=$(find "$tmpdir" -maxdepth 1 -type d -name "gat-*" | head -1)

  if [[ -z "$bundle_dir" || ! -d "$bundle_dir" ]]; then
    echo "ERROR: Could not find extracted bundle directory" >&2
    exit 1
  fi

  echo "Installing components..."
  install_from_bundle "$bundle_dir"

  echo
  echo "Installation complete!"
  echo
  echo "Add $PREFIX/bin to your PATH:"
  echo "  export PATH=\"$PREFIX/bin:\$PATH\""

  if wants_component "solvers"; then
    echo
    echo "Solver binaries installed to: $PREFIX/solvers"
    if [[ "$os" == "linux" ]]; then
      echo "Note: Set LD_LIBRARY_PATH if needed:"
      echo "  export LD_LIBRARY_PATH=\"$PREFIX/lib:\$LD_LIBRARY_PATH\""
    fi
  fi
}

main "$@"
