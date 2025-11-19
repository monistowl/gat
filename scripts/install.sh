#!/usr/bin/env bash
set -euo pipefail

if [[ $# -gt 1 ]]; then
  echo "Usage: $(basename "$0") [install_prefix]"
  exit 1
fi

PREFIX="${1:-$HOME/.local}"
BINDIR="$PREFIX/bin"

mkdir -p "$BINDIR"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PACKAGE_DIR="$ROOT_DIR/dist"

install_binary() {
  local src_name="$1"
  local dest_name="${2:-$1}"
  local src="$PACKAGE_DIR/bin/$src_name"
  if [[ ! -x "$src" ]]; then
    echo "Missing built binary: $src"
    exit 1
  fi
  cp "$src" "$BINDIR/$dest_name"
  chmod +x "$BINDIR/$dest_name"
}

install_binary "gat-cli" "gat"
install_binary "gat-gui"

echo "Installed gat-cli and gat-gui to $BINDIR"
