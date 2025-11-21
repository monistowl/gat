#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/release-utils.sh"

usage() {
  cat <<'USAGE'
Usage: platform-info.sh [--variant headless|full] [--version VERSION]

Outputs JSON describing the release metadata for the requested variant.
USAGE
}

VARIANT="${1:-headless}"
VERSION="${2:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --variant)
      VARIANT="$2"
      shift 2
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
      shift
      ;;
  esac
done

if [[ "$VARIANT" != "headless" && "$VARIANT" != "full" ]]; then
  echo "Invalid variant: $VARIANT" >&2
  usage
  exit 1
fi

if [[ -z "$VERSION" ]]; then
  VERSION="$(release_version)"
fi

OS="$(detect_os)"
ARCH="$(detect_arch)"
BASE="$(release_asset_base_name "$VARIANT" "$VERSION" "$OS" "$ARCH")"
TARBALL="$(release_tarball_name "$VARIANT" "$VERSION" "$OS" "$ARCH")"
URL="$(release_download_url "$VARIANT" "$VERSION")"

cat <<EOF
{
  "variant": "$VARIANT",
  "version": "$VERSION",
  "os": "$OS",
  "arch": "$ARCH",
  "asset_base": "$BASE",
  "tarball": "$TARBALL",
  "url": "$URL"
}
EOF
