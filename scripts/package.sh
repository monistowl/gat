#!/usr/bin/env bash
#
# Package GAT for distribution
#
# Usage: scripts/package.sh [headless|analyst|full]
#
# This script is used by:
#   - .github/workflows/release-verification.yml (smoke test)
#   - .github/workflows/manual-release.yml (full release builds)
#
# The resulting tarballs are named: gat-{version}-{os}-{arch}-{variant}.tar.gz
# and can be installed via scripts/install.sh
#
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

source "$ROOT_DIR/scripts/release-utils.sh"
source "$ROOT_DIR/scripts/solver-discovery.sh"

ensure_linux_library_paths

VERSION="$(release_version)"

OS="$(detect_os)"
ARCH="$(detect_arch)"
DIST_DIR="$ROOT_DIR/dist"

# Determine variant from environment or argument
VARIANT="${GAT_BUNDLE_VARIANT:-full}"
if [[ $# -gt 0 ]]; then
  VARIANT="$1"
fi

case "$VARIANT" in
  headless)
    BUILD_FLAGS="--no-default-features --features minimal-io"
    ;;
  analyst)
    BUILD_FLAGS="--no-default-features --features minimal-io,viz,all-backends"
    ;;
  full)
    BUILD_FLAGS="--all-features"
    ;;
  *)
    echo "Unknown variant: $VARIANT. Use headless, analyst, or full." >&2
    exit 1
    ;;
esac

pkg_dir() {
  local variant="$1"
  echo "$DIST_DIR/gat-${VERSION}-${OS}-${ARCH}-${variant}"
}

install_solver_deps
ensure_solvers_available

clean_dist() {
  rm -rf "$DIST_DIR"
  mkdir -p "$DIST_DIR"
}

copy_common_files() {
  local dest="$1"
  cp README.md "$dest"
  cp scripts/install.sh "$dest/"
  cp LICENSE.txt "$dest/"
}

package_headless() {
  echo "Packaging GAT $VERSION for $OS/$ARCH (headless)"
  cargo build -p gat-cli --release $BUILD_FLAGS

  local dest
  dest="$(pkg_dir headless)"
  mkdir -p "$dest/bin"

  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat-cli"
  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat"
  copy_common_files "$dest"

  tar -czf "$dest.tar.gz" -C "$DIST_DIR" "$(basename "$dest")"
}

package_analyst() {
  echo "Packaging GAT $VERSION for $OS/$ARCH (analyst)"
  cargo build -p gat-cli --release $BUILD_FLAGS

  local dest
  dest="$(pkg_dir analyst)"
  mkdir -p "$dest/bin"

  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat-cli"
  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat"
  copy_common_files "$dest"

  tar -czf "$dest.tar.gz" -C "$DIST_DIR" "$(basename "$dest")"
}

package_full() {
  echo "Packaging GAT $VERSION for $OS/$ARCH (full)"
  cargo build -p gat-cli --release $BUILD_FLAGS

  local dest
  dest="$(pkg_dir full)"
  mkdir -p "$dest/bin"

  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat-cli"
  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat"
  if [[ -x "$ROOT_DIR/target/release/gat-gui" ]]; then
    cp "$ROOT_DIR/target/release/gat-gui" "$dest/bin/gat-gui"
  fi
  if [[ -x "$ROOT_DIR/target/release/gat-tui" ]]; then
    cp "$ROOT_DIR/target/release/gat-tui" "$dest/bin/gat-tui"
  fi

  copy_common_files "$dest"
  cp -r docs "$dest/docs"

  tar -czf "$dest.tar.gz" -C "$DIST_DIR" "$(basename "$dest")"
}

install_solver_deps
ensure_solvers_available
clean_dist

case "$VARIANT" in
  headless)
    package_headless
    rm -rf "$(pkg_dir headless)"
    ;;
  analyst)
    package_analyst
    rm -rf "$(pkg_dir analyst)"
    ;;
  full)
    package_full
    rm -rf "$(pkg_dir full)"
    ;;
esac

echo "Artifacts available in $DIST_DIR:"
ls -1 "$DIST_DIR"
