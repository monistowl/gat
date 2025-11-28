#!/bin/bash
set -euo pipefail

# Build optimized IPOPT with parallel MUMPS
# Prerequisites: build-essential, gfortran, libopenblas-dev
#
# Usage: ./scripts/build-ipopt.sh
#
# This builds IPOPT from vendor sources with:
# - Metis 4.0.3 for graph partitioning (fill reduction)
# - MUMPS 5.6.2 with OpenMP parallelism
# - Links against system OpenBLAS

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PREFIX="$PROJECT_ROOT/vendor/local"
VENDOR="$PROJECT_ROOT/vendor"
JOBS=$(nproc)
BUILD_DIR="/tmp/gat-ipopt-build-$$"

echo "=== GAT IPOPT Build ==="
echo "Project root: $PROJECT_ROOT"
echo "Install prefix: $PREFIX"
echo "Build jobs: $JOBS"
echo ""

# Cleanup function
cleanup() {
    echo "Cleaning up build directory..."
    rm -rf "$BUILD_DIR"
}
trap cleanup EXIT

mkdir -p "$PREFIX" "$BUILD_DIR"
