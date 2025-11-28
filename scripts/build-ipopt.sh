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

# =============================================================================
# Build Metis (graph partitioning for fill reduction)
# =============================================================================
build_metis() {
    echo ""
    echo "=== Building Metis ==="

    if [ -f "$PREFIX/lib/libcoinmetis.a" ]; then
        echo "Metis already built, skipping..."
        return 0
    fi

    local METIS_ZIP="$VENDOR/ThirdParty-Metis-stable-2.0.zip"
    if [ ! -f "$METIS_ZIP" ]; then
        echo "ERROR: Metis source not found at $METIS_ZIP"
        exit 1
    fi

    cd "$BUILD_DIR"
    unzip -q "$METIS_ZIP"
    cd ThirdParty-Metis-stable-2.0

    # Download Metis source
    ./get.Metis

    # Configure and build
    ./configure --prefix="$PREFIX" --disable-shared
    make -j"$JOBS"
    make install

    echo "Metis installed to $PREFIX"
}

build_metis

# =============================================================================
# Build MUMPS (parallel sparse direct solver)
# =============================================================================
build_mumps() {
    echo ""
    echo "=== Building MUMPS (parallel) ==="

    if [ -f "$PREFIX/lib/libcoinmumps.a" ]; then
        echo "MUMPS already built, skipping..."
        return 0
    fi

    local MUMPS_ZIP="$VENDOR/ThirdParty-Mumps-stable-3.0.zip"
    if [ ! -f "$MUMPS_ZIP" ]; then
        echo "ERROR: MUMPS source not found at $MUMPS_ZIP"
        exit 1
    fi

    cd "$BUILD_DIR"
    unzip -q "$MUMPS_ZIP"
    cd ThirdParty-Mumps-stable-3.0

    # Download MUMPS source
    ./get.Mumps

    # Configure with:
    # - Metis for ordering
    # - OpenMP for parallelism
    # - OpenBLAS for linear algebra
    ./configure --prefix="$PREFIX" \
        --with-metis="$PREFIX" \
        --with-lapack="-lopenblas" \
        --disable-shared \
        CFLAGS="-O3 -fopenmp" \
        FCFLAGS="-O3 -fopenmp"

    make -j"$JOBS"
    make install

    echo "MUMPS installed to $PREFIX"
}

build_mumps
