#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TMP_PREFIX="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_PREFIX"
}
trap cleanup EXIT

echo "Verifying installer fallback path..."
GAT_PREFIX="$TMP_PREFIX" \
GAT_VERSION="0.0.0" \
GAT_RELEASE_BASE="https://example.invalid" \
bash "$SCRIPT_DIR/install.sh" --variant headless

echo "Installer fallback exercised (built from source into $TMP_PREFIX/bin)."
