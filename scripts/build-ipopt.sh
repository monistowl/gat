#!/bin/bash
set -euo pipefail

# Build optimized IPOPT with parallel MUMPS
# Prerequisites: build-essential, gfortran, libblas-dev, liblapack-dev
#
# Usage: ./scripts/build-ipopt.sh
#
# This builds IPOPT from vendor sources with:
# - Metis 4.0.3 for graph partitioning (fill reduction)
# - MUMPS 5.6.2 with OpenMP parallelism
# - Links against system BLAS/LAPACK

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PREFIX="$PROJECT_ROOT/vendor/local"
VENDOR="$PROJECT_ROOT/vendor"
JOBS=$(nproc)
BUILD_DIR="/tmp/gat-ipopt-build-$$"
export PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}:$PREFIX/lib/pkgconfig"

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
    # - System BLAS/LAPACK for linear algebra
    ./configure --prefix="$PREFIX" \
        --with-metis-lflags="-L$PREFIX/lib -lcoinmetis -lm" \
        --with-metis-cflags="-I$PREFIX/include/coin-or/metis" \
        --with-lapack="-llapack -lblas" \
        --disable-shared \
        CFLAGS="-O3 -fopenmp" \
        FCFLAGS="-O3 -fopenmp"

    make -j"$JOBS"
    make install

    echo "MUMPS installed to $PREFIX"
}

build_mumps

# =============================================================================
# Build IPOPT (interior point optimizer)
# =============================================================================
build_ipopt() {
    echo ""
    echo "=== Building IPOPT ==="

    if [ -f "$PREFIX/lib/libipopt.so" ]; then
        echo "IPOPT already built, skipping..."
        return 0
    fi

    local IPOPT_DIR="$VENDOR/Ipopt-stable-3.14"
    if [ ! -d "$IPOPT_DIR" ]; then
        echo "ERROR: IPOPT source not found at $IPOPT_DIR"
        exit 1
    fi

    cd "$IPOPT_DIR"

    # Clean any previous build
    make distclean 2>/dev/null || true

    # Configure with our custom MUMPS and Metis
    ./configure --prefix="$PREFIX" \
        --with-mumps-lflags="-L$PREFIX/lib -lcoinmumps" \
        --with-mumps-cflags="-I$PREFIX/include/coin-or/mumps" \
        --with-metis-lflags="-L$PREFIX/lib -lcoinmetis -lm" \
        --with-metis-cflags="-I$PREFIX/include/coin-or/metis" \
        --with-lapack="-llapack -lblas" \
        --enable-shared \
        CXXFLAGS="-O3 -fopenmp" \
        LDFLAGS="-fopenmp"

    make -j"$JOBS"
    make install

    echo "IPOPT installed to $PREFIX"
}

build_ipopt

# =============================================================================
# Verify Installation
# =============================================================================
verify_install() {
    echo ""
    echo "=== Verifying Installation ==="

    local ERRORS=0

    # Check libraries exist
    for lib in libipopt.so libcoinmumps.a libcoinmetis.a; do
        if [ -f "$PREFIX/lib/$lib" ]; then
            echo "✓ Found $lib"
        else
            echo "✗ Missing $lib"
            ERRORS=$((ERRORS + 1))
        fi
    done

    # Check pkg-config
    if [ -f "$PREFIX/lib/pkgconfig/ipopt.pc" ]; then
        echo "✓ Found ipopt.pc"
    else
        echo "✗ Missing ipopt.pc"
        ERRORS=$((ERRORS + 1))
    fi

    if [ $ERRORS -gt 0 ]; then
        echo ""
        echo "ERROR: Installation incomplete ($ERRORS errors)"
        exit 1
    fi

    echo ""
    echo "=== Build Complete ==="
    echo ""
    echo "IPOPT installed to: $PREFIX"
    echo ""
    echo "To use with Cargo:"
    echo "  export PKG_CONFIG_PATH=\"$PREFIX/lib/pkgconfig:\$PKG_CONFIG_PATH\""
    echo "  export LD_LIBRARY_PATH=\"$PREFIX/lib:\$LD_LIBRARY_PATH\""
    echo "  cargo build --features solver-ipopt"
    echo ""
    echo "Or use the wrapper script:"
    echo "  ./scripts/with-ipopt.sh cargo test --features solver-ipopt"
}

verify_install
