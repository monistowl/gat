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

SOLVER_COMMANDS=(cbc highs)

install_solver_deps() {
  case "$(uname -s)" in
    Linux)
      if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y coinor-cbc coinor-libcbc-dev jq cmake ninja-build build-essential
      fi
      install_highs_from_source
      ;;
    Darwin)
      if command -v brew >/dev/null 2>&1; then
        brew tap coin-or-tools/coinor
        brew install coin-or-tools/coinor/cbc highs pkg-config jq || true
        setup_brew_solver_env
      fi
      ;;
  esac
}

setup_brew_solver_env() {
  local prefix
  prefix="$(brew --prefix coin-or-tools/coinor/cbc 2>/dev/null || true)"
  if [[ -n "$prefix" ]]; then
    export PATH="$prefix/bin:$PATH"
    export LDFLAGS="${LDFLAGS:-} -L$prefix/lib"
    export CPPFLAGS="${CPPFLAGS:-} -I$prefix/include"
    export PKG_CONFIG_PATH="$prefix/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
    export LIBRARY_PATH="$prefix/lib:${LIBRARY_PATH:-}"
    export DYLD_LIBRARY_PATH="$prefix/lib:${DYLD_LIBRARY_PATH:-}"
  fi

  prefix="$(brew --prefix highs 2>/dev/null || true)"
  if [[ -n "$prefix" ]]; then
    export PATH="$prefix/bin:$PATH"
  fi
}

ensure_solvers_available() {
  local missing=()
  for solver in "${SOLVER_COMMANDS[@]}"; do
    if ! command -v "$solver" >/dev/null 2>&1; then
      missing+=("$solver")
    fi
  done

  if (( ${#missing[@]} > 0 )); then
    echo "Missing required solver binaries: ${missing[*]}" >&2
    return 1
  fi

  return 0
}

install_highs_from_source() {
  if command -v highs >/dev/null 2>&1; then
    return
  fi
  local temp_dir
  temp_dir="$(mktemp -d)"
  git clone --depth 1 https://github.com/ERGO-Code/HiGHS "$temp_dir/highs"
  mkdir -p "$temp_dir/highs/build"
  cmake -S "$temp_dir/highs" -B "$temp_dir/highs/build" -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr/local
  cmake --build "$temp_dir/highs/build"
  sudo cmake --install "$temp_dir/highs/build"
  rm -rf "$temp_dir"
}
