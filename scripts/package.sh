#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to package releases" >&2
  exit 1
fi

VERSION="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')"

case "$(uname -s)" in
  Linux) OS="linux" ;;
  Darwin) OS="macos" ;;
  *) OS="$(uname -s | tr '[:upper:]' '[:lower:]')" ;;
esac

case "$(uname -m)" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="arm64" ;;
  *) ARCH="$(uname -m)" ;;
esac
DIST_DIR="$ROOT_DIR/dist"

pkg_dir() {
  local variant="$1"
  echo "$DIST_DIR/gat-${VERSION}-${OS}-${ARCH}-${variant}"
}

clean_dist() {
  rm -rf "$DIST_DIR"
  mkdir -p "$DIST_DIR"
}

copy_common_files() {
  local dest="$1"
  cp README.md "$dest"
  cp scripts/install.sh "$dest/"
}

package_headless() {
  echo "Packaging GAT $VERSION for $OS/$ARCH (headless)"
  cargo build --workspace --exclude gat-gui --exclude gat-tui --release

  local dest
  dest="$(pkg_dir headless)"
  mkdir -p "$dest/bin"

  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat-cli"
  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat"
  copy_common_files "$dest"

  tar -czf "$dest.tar.gz" -C "$DIST_DIR" "$(basename "$dest")"
}

package_full() {
  echo "Packaging GAT $VERSION for $OS/$ARCH (full)"
  cargo build --workspace --all-features --release

  local dest
  dest="$(pkg_dir full)"
  mkdir -p "$dest/bin"

  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat-cli"
  cp "$ROOT_DIR/target/release/gat-cli" "$dest/bin/gat"
  cp "$ROOT_DIR/target/release/gat-gui" "$dest/bin/gat-gui"
  if [[ -x "$ROOT_DIR/target/release/gat-tui" ]]; then
    cp "$ROOT_DIR/target/release/gat-tui" "$dest/bin/gat-tui"
  fi

  copy_common_files "$dest"
  cp -r docs "$dest/docs"

  tar -czf "$dest.tar.gz" -C "$DIST_DIR" "$(basename "$dest")"
}

clean_dist
package_headless
package_full

rm -rf "$(pkg_dir headless)" "$(pkg_dir full)"

echo "Artifacts available in $DIST_DIR:"
ls -1 "$DIST_DIR"
