#!/usr/bin/env bash
#
# Shared helpers for release scripts/workflows.

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  echo "This file is intended to be sourced, not executed." >&2
  exit 1
fi

if [[ -n "${GAT_RELEASE_UTILS_LOADED:-}" ]]; then
  return 0
fi
GAT_RELEASE_UTILS_LOADED=1

require_jq() {
  if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required for release helpers" >&2
    return 1
  fi
  return 0
}

release_version() {
  if [[ -n "${GAT_CANONICAL_VERSION:-}" ]]; then
    printf '%s' "$GAT_CANONICAL_VERSION"
    return 0
  fi

  require_jq || return 1
  local version
  version="$(cargo metadata --no-deps --format-version 1 | jq -r '.metadata.release.version')"
  if [[ -z "$version" || "$version" == "null" ]]; then
    echo "workspace metadata release.version is not set" >&2
    return 1
  fi

  GAT_CANONICAL_VERSION="$version"
  printf '%s' "$version"
}

detect_os() {
  if [[ -n "${GAT_RELEASE_OS:-}" ]]; then
    printf '%s' "$GAT_RELEASE_OS"
    return 0
  fi

  local os
  case "$(uname -s)" in
    Linux) os="linux" ;;
    Darwin) os="macos" ;;
    *) os="$(uname -s | tr '[:upper:]' '[:lower:]')" ;;
  esac

  GAT_RELEASE_OS="$os"
  printf '%s' "$os"
}

detect_arch() {
  if [[ -n "${GAT_RELEASE_ARCH:-}" ]]; then
    printf '%s' "$GAT_RELEASE_ARCH"
    return 0
  fi

  local arch
  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="arm64" ;;
    *) arch="$(uname -m)" ;;
  esac

  GAT_RELEASE_ARCH="$arch"
  printf '%s' "$arch"
}

release_asset_base_name() {
  local variant="${1:?}"
  local version="${2:-$(release_version)}"
  local os="${3:-$(detect_os)}"
  local arch="${4:-$(detect_arch)}"
  printf 'gat-%s-%s-%s-%s' "$version" "$os" "$arch" "$variant"
}

release_tarball_name() {
  local base
  base="$(release_asset_base_name "$@")"
  printf '%s.tar.gz' "$base"
}

release_tag_to_version() {
  local tag="${1:-}"
  if [[ -z "$tag" ]]; then
    printf '%s' "$(release_version)"
    return 0
  fi

  if [[ "$tag" == v* ]]; then
    printf '%s' "${tag#v}"
  else
    printf '%s' "$tag"
  fi
}

release_base_url() {
  local base="${1:-${RELEASE_BASE:-https://github.com/monistowl/gat/releases/download}}"
  printf '%s' "${base%/}"
}

release_download_url() {
  local variant="${1:?}"
  local tag="${2:-}"
  local base_url="${3:-}"
  local os="${4:-}"
  local arch="${5:-}"

  if [[ -z "$tag" ]]; then
    tag=$(release_version)
  fi
  if [[ -z "$os" ]]; then
    os=$(detect_os)
  fi
  if [[ -z "$arch" ]]; then
    arch=$(detect_arch)
  fi

  local canonical_version
  canonical_version="$(release_tag_to_version "$tag")"

  local resolved_base
  resolved_base="$(release_base_url "$base_url")"

  local tarball
  tarball="$(release_tarball_name "$variant" "$canonical_version" "$os" "$arch")"

  printf '%s/%s/%s' "$resolved_base" "$tag" "$tarball"
}
