#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/release-utils.sh" ]]; then
  source "$SCRIPT_DIR/release-utils.sh"
else
  ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
  source "$ROOT_DIR/scripts/release-utils.sh"
fi

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  echo "This script is intended to be sourced, not executed." >&2
  exit 1
fi

# Solver discovery and build setup for GAT packaging
#
# Native solvers (IPOPT, CBC, HiGHS) are built from vendored sources via xtask.
# This script installs build toolchain dependencies only - no solver packages.

install_solver_deps() {
  # Install build toolchain for compiling vendored solver sources
  case "$(uname -s)" in
    Linux)
      if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y \
          build-essential \
          cmake \
          ninja-build \
          gfortran \
          jq \
          pkg-config
      fi
      ;;
    Darwin)
      if command -v brew >/dev/null 2>&1; then
        brew install cmake ninja pkg-config jq || true
      fi
      ;;
  esac
}

ensure_solvers_available() {
  # With native-dispatch, solvers are built from vendored sources and
  # discovered at runtime via ~/.gat/solvers/. No system binaries required.
  return 0
}
