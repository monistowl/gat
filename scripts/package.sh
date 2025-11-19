#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERSION="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
DIST_DIR="$ROOT_DIR/dist"
PKG_NAME="gat-${VERSION}-${OS}-${ARCH}"
PKG_DIR="$DIST_DIR/$PKG_NAME"

echo "Packaging GAT $VERSION for $OS/$ARCH"

cargo build --workspace --release

rm -rf "$DIST_DIR"
mkdir -p "$PKG_DIR/bin"

cp "$ROOT_DIR/target/release/gat-cli" "$PKG_DIR/bin/gat-cli"
cp "$ROOT_DIR/target/release/gat-cli" "$PKG_DIR/bin/gat"
cp "$ROOT_DIR/target/release/gat-gui" "$PKG_DIR/bin/"

cp README.md "$PKG_DIR"
cp -r docs "$PKG_DIR/docs"
cp scripts/install.sh "$PKG_DIR/"

tar -czf "$DIST_DIR/$PKG_NAME.tar.gz" -C "$DIST_DIR" "$PKG_NAME"
rm -rf "$PKG_DIR"

echo "Packaged artifact: $DIST_DIR/$PKG_NAME.tar.gz"
