#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: install.sh [--prefix DIR] [--variant headless|analyst|full] [--version VERSION]

Environment variables:
  GAT_PREFIX        Install prefix (defaults to ~/.local)
  GAT_VARIANT       Variant to install (headless, analyst, or full)
  GAT_VERSION       Version to install (defaults to latest release)
  GAT_RELEASE_BASE  Base URL for release artifacts
USAGE
}

PREFIX="${GAT_PREFIX:-$HOME/.local}"
VARIANT="${GAT_VARIANT:-full}"
VERSION="${GAT_VERSION:-latest}"
RELEASE_BASE="${GAT_RELEASE_BASE:-https://github.com/monistowl/gat/releases/download}"
GITHUB_LATEST_API="${GAT_GITHUB_LATEST_API:-https://api.github.com/repos/monistowl/gat/releases/latest}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/release-utils.sh"
source "$SCRIPT_DIR/solver-discovery.sh"

ensure_linux_library_paths

trim_variant() {
  VARIANT="$(echo "$VARIANT" | tr '[:upper:]' '[:lower:]')"
  if [[ "$VARIANT" != "headless" && "$VARIANT" != "analyst" && "$VARIANT" != "full" ]]; then
    echo "Invalid variant: $VARIANT (expected headless, analyst, or full)" >&2
    exit 1
  fi
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --prefix)
        PREFIX="$2"
        shift 2
        ;;
      --variant)
        VARIANT="$2"
        shift 2
        ;;
      --headless)
        VARIANT="headless"
        shift
        ;;
      --analyst)
        VARIANT="analyst"
        shift
        ;;
      --full)
        VARIANT="full"
        shift
        ;;
      --version)
        VERSION="$2"
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        PREFIX="$1"
        shift
        ;;
    esac
  done
}

resolve_version() {
  if [[ "$VERSION" != "latest" ]]; then
    return
  fi

  # Prefer GitHub release metadata when using the GitHub releases base URL.
  if [[ "$RELEASE_BASE" == *github.com* ]]; then
    if latest_json=$(curl -fsSL "$GITHUB_LATEST_API" 2>/dev/null) && [[ -n "$latest_json" ]]; then
      VERSION="$(
        printf '%s' "$latest_json" | python3 - <<'PY'
import json, sys
try:
    data = json.load(sys.stdin)
    print(data.get("tag_name", "").strip())
except json.JSONDecodeError:
    pass
PY
      )"
      VERSION="${VERSION:-}"
      if [[ -n "$VERSION" ]]; then
        return
      fi
    fi
  fi

  # Fallback to the legacy `latest.txt` when GitHub metadata can't be retrieved.
  local latest_url="$RELEASE_BASE/latest.txt"
  if VERSION_CONTENTS=$(curl -fsSL "$latest_url" 2>/dev/null); then
    VERSION="$VERSION_CONTENTS"
    return
  fi

  VERSION=""
}

install_binary() {
  local src="$1"
  local dest_name="$2"
  if [[ -x "$src" ]]; then
    cp "$src" "$BINDIR/$dest_name"
    chmod +x "$BINDIR/$dest_name"
  fi
}

install_from_dir() {
  local dir="$1"
  local bin_dir="$dir/bin"
  if [[ ! -d "$bin_dir" ]]; then
    echo "Expected bin/ directory in $dir" >&2
    return 1
  fi

  if [[ ! -x "$bin_dir/gat-cli" ]]; then
    echo "Missing gat-cli binary in $bin_dir" >&2
    return 1
  fi

  install_binary "$bin_dir/gat-cli" "gat-cli"
  install_binary "$bin_dir/gat" "gat"

  if [[ "$VARIANT" == "full" ]]; then
    install_binary "$bin_dir/gat-gui" "gat-gui"
    install_binary "$bin_dir/gat-tui" "gat-tui"
  fi
}

build_from_source() {
  echo "Falling back to building from source ($VARIANT)..."
  if [[ "$(uname -s)" == "Darwin" ]]; then
    setup_brew_solver_env
  fi
  if ! ensure_solvers_available; then
    echo "Required solver binaries (cbc/highs) missing; install them to build from source." >&2
    return 1
  fi

  local build_flags
  case "$VARIANT" in
    headless)
      build_flags="--no-default-features --features minimal-io"
      ;;
    analyst)
      build_flags="--no-default-features --features minimal-io,adms,derms,dist,analytics,featurize"
      ;;
    full)
      build_flags="--all-features"
      ;;
  esac

  pushd "$ROOT_DIR" >/dev/null
  cargo build -p gat-cli --release $build_flags
  popd >/dev/null

  mkdir -p "$BINDIR"
  install_binary "$ROOT_DIR/target/release/gat-cli" "gat-cli"
  install_binary "$ROOT_DIR/target/release/gat-cli" "gat"
  if [[ "$VARIANT" == "full" ]]; then
    install_binary "$ROOT_DIR/target/release/gat-gui" "gat-gui"
    install_binary "$ROOT_DIR/target/release/gat-tui" "gat-tui"
  fi
}

download_and_install() {
  local os arch tarball_name url tmpdir extracted_root canonical_version

  os="$(detect_os)"
  arch="$(detect_arch)"

  if [[ -z "$os" || -z "$arch" ]]; then
    echo "Unsupported platform for binary install (os=$(uname -s), arch=$(uname -m))."
    return 1
  fi

  if [[ -z "$VERSION" ]]; then
    echo "No binary version found; skipping download."
    return 1
  fi

  canonical_version="$(release_tag_to_version "$VERSION")"
  tarball_name="$(release_tarball_name "$VARIANT" "$canonical_version" "$os" "$arch")"
  url="$(release_download_url "$VARIANT" "$VERSION" "$RELEASE_BASE" "$os" "$arch")"
  tmpdir="$(mktemp -d)"

  echo "Attempting to download $url"
  if ! curl -fL "$url" -o "$tmpdir/$tarball_name"; then
    echo "Download failed; falling back to source build."
    rm -rf "$tmpdir"
    return 1
  fi

  extracted_root=$(tar -tzf "$tmpdir/$tarball_name" | head -1 | cut -d/ -f1)
  tar -xzf "$tmpdir/$tarball_name" -C "$tmpdir"
  mkdir -p "$BINDIR"
  if ! install_from_dir "$tmpdir/$extracted_root"; then
    rm -rf "$tmpdir"
    return 1
  fi

  rm -rf "$tmpdir"
  return 0
}

main() {
  parse_args "$@"
  trim_variant
  BINDIR="$PREFIX/bin"
  resolve_version

  if download_and_install; then
    echo "Installed GAT ($VARIANT) binaries to $BINDIR"
    exit 0
  fi

  build_from_source
  echo "Installed GAT ($VARIANT) from source to $BINDIR"
}

main "$@"
