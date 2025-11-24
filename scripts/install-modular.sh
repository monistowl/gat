#!/usr/bin/env bash
#
# GAT Modular Installer
#
# This script installs GAT using the new modular installation system.
# It downloads and installs components on-demand:
#  - gat (core CLI always installed)
#  - gat-tui (optional TUI interface)
#  - gat-gui (optional GUI dashboard)
#  - solvers (optional additional solver support)
#
# Usage: curl -fsSL https://github.com/monistowl/gat/releases/download/latest/install-modular.sh | bash
#        Or: bash install-modular.sh [OPTIONS]
#
# Options:
#   --prefix DIR        Install to directory (default: ~/.gat)
#   --version VERSION   Install specific version (default: latest)
#   --components COMP   Comma-separated list of components to install
#                      Options: cli, tui, gui, solvers (default: cli)
#   --help             Show this help message

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
  local os
  case "$(uname -s)" in
    Linux)
      os="linux"
      ;;
    Darwin)
      os="macos"
      ;;
    *)
      echo "Unsupported operating system: $(uname -s)" >&2
      exit 1
      ;;
  esac
  echo "$os"
}

detect_arch() {
  local arch
  case "$(uname -m)" in
    x86_64|amd64)
      arch="x86_64"
      ;;
    arm64|aarch64)
      arch="arm64"
      ;;
    *)
      echo "Unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
  echo "$arch"
}

# Resolve version
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

  # Strip 'v' prefix from version tag to match artifact naming convention
  if [[ "$VERSION" == v* ]]; then
    VERSION="${VERSION#v}"
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
}

# Download and install a component
install_component() {
  local component="$1"
  local os="$2"
  local arch="$3"

  echo "Installing $component..."

  # Build artifact name based on component
  local artifact_name
  case "$component" in
    cli)
      artifact_name="gat-${VERSION}-${os}-${arch}.tar.gz"
      ;;
    tui)
      artifact_name="gat-tui-${VERSION}-${os}-${arch}.tar.gz"
      ;;
    gui)
      artifact_name="gat-gui-${VERSION}-${os}-${arch}.tar.gz"
      ;;
    solvers)
      artifact_name="gat-solvers-${VERSION}-${os}-${arch}.tar.gz"
      ;;
    *)
      echo "Unknown component: $component" >&2
      return 1
      ;;
  esac

  local url="$RELEASE_BASE/v${VERSION}/$artifact_name"
  local tmpdir
  tmpdir=$(mktemp -d)
  trap "rm -rf '$tmpdir'" EXIT

  echo "  Downloading from $url"
  if ! curl -fsSL "$url" -o "$tmpdir/$artifact_name"; then
    echo "  Failed to download $component" >&2
    return 1
  fi

  # Extract to appropriate location
  case "$component" in
    solvers)
      # Extract solvers to lib directory
      tar -xzf "$tmpdir/$artifact_name" -C "$PREFIX/lib/" 2>/dev/null || true
      ;;
    *)
      # Extract binaries to bin directory
      tar -xzf "$tmpdir/$artifact_name" -C "$tmpdir/"
      # Find and copy the binary
      if [[ -f "$tmpdir/$component" ]]; then
        cp "$tmpdir/$component" "$PREFIX/bin/"
        chmod +x "$PREFIX/bin/$component"
      elif [[ -f "$tmpdir/bin/$component" ]]; then
        cp "$tmpdir/bin/$component" "$PREFIX/bin/"
        chmod +x "$PREFIX/bin/$component"
      else
        echo "  Warning: Binary $component not found in archive" >&2
        return 1
      fi
      ;;
  esac

  echo "  ✓ $component installed"
}

main() {
  parse_args "$@"

  echo "GAT Modular Installer"
  echo "====================="

  resolve_version
  echo "Version: $VERSION"

  local os arch
  os=$(detect_os)
  arch=$(detect_arch)
  echo "Platform: $os/$arch"
  echo "Prefix: $PREFIX"
  echo "Components: $COMPONENTS"
  echo

  ensure_dirs

  # Install each component
  IFS=',' read -ra comp_array <<< "$COMPONENTS"
  for component in "${comp_array[@]}"; do
    component=$(echo "$component" | xargs)  # trim whitespace
    if ! install_component "$component" "$os" "$arch"; then
      echo "Warning: Failed to install $component" >&2
    fi
  done

  echo
  echo "Installation complete!"
  echo "Add $PREFIX/bin to your PATH:"
  echo "  export PATH=\"$PREFIX/bin:\$PATH\""
}

main "$@"
