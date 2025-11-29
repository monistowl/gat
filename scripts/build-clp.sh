#!/bin/bash
set -euo pipefail

# Build CLP (COIN-OR Linear Programming) from vendor sources
# Prerequisites: build-essential, libblas-dev, liblapack-dev
#
# Usage: ./scripts/build-clp.sh
#
# This builds CLP and its dependencies in order:
# 1. CoinUtils - Base utilities (vectors, matrices, I/O)
# 2. Osi - Open Solver Interface (abstract LP interface)
# 3. Clp - COIN-OR LP solver

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PREFIX="$PROJECT_ROOT/vendor/local"
VENDOR="$PROJECT_ROOT/vendor"
JOBS=$(nproc)
BUILD_DIR="/tmp/gat-clp-build-$$"
export PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}:$PREFIX/lib/pkgconfig"

echo "=== GAT CLP Build ==="
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
mkdir -p "$PREFIX/include"

# Create compatibility symlink (COIN-OR sources use #include "coin/..." but install to coin-or/)
if [ ! -L "$PREFIX/include/coin" ]; then
    ln -sf coin-or "$PREFIX/include/coin"
fi

# =============================================================================
# Extract helper
# =============================================================================
extract_if_needed() {
    local NAME="$1"
    local ZIP="$VENDOR/${NAME}.zip"
    local DIR="$VENDOR/${NAME}"

    if [ -d "$DIR" ]; then
        echo "$NAME already extracted"
        return 0
    fi

    if [ ! -f "$ZIP" ]; then
        echo "ERROR: $ZIP not found"
        exit 1
    fi

    echo "Extracting $NAME..."
    cd "$VENDOR"
    unzip -q "$ZIP"
}

# =============================================================================
# Build CoinUtils (base utilities)
# =============================================================================
build_coinutils() {
    echo ""
    echo "=== Building CoinUtils ==="

    if [ -f "$PREFIX/lib/libCoinUtils.a" ]; then
        echo "CoinUtils already built, skipping..."
        return 0
    fi

    extract_if_needed "CoinUtils-master"

    cd "$VENDOR/CoinUtils-master"

    # Clean any previous build
    make distclean 2>/dev/null || true

    # Configure
    ./configure --prefix="$PREFIX" \
        --disable-shared \
        --with-lapack="-llapack -lblas" \
        CXXFLAGS="-O3"

    make -j"$JOBS"
    make install

    echo "CoinUtils installed to $PREFIX"
}

build_coinutils

# =============================================================================
# Build Osi (Open Solver Interface)
# =============================================================================
build_osi() {
    echo ""
    echo "=== Building Osi ==="

    if [ -f "$PREFIX/lib/libOsi.a" ]; then
        echo "Osi already built, skipping..."
        return 0
    fi

    extract_if_needed "Osi-master"

    cd "$VENDOR/Osi-master"

    # Clean any previous build
    make distclean 2>/dev/null || true

    # Configure with CoinUtils
    ./configure --prefix="$PREFIX" \
        --disable-shared \
        --with-coinutils-lflags="-L$PREFIX/lib -lCoinUtils" \
        --with-coinutils-cflags="-I$PREFIX/include/coin" \
        CXXFLAGS="-O3"

    make -j"$JOBS"
    make install

    echo "Osi installed to $PREFIX"
}

build_osi

# =============================================================================
# Build Clp (LP solver)
# =============================================================================
build_clp() {
    echo ""
    echo "=== Building Clp ==="

    if [ -f "$PREFIX/lib/libClp.a" ]; then
        echo "Clp already built, skipping..."
        return 0
    fi

    # Clp is already extracted
    cd "$VENDOR/Clp-master"

    # Clean any previous build
    make distclean 2>/dev/null || true

    # Configure with CoinUtils and Osi (disable MUMPS - we only need simplex)
    ./configure --prefix="$PREFIX" \
        --disable-shared \
        --without-mumps \
        --with-coinutils-lflags="-L$PREFIX/lib -lCoinUtils" \
        --with-coinutils-cflags="-I$PREFIX/include/coin" \
        --with-osi-lflags="-L$PREFIX/lib -lOsi" \
        --with-osi-cflags="-I$PREFIX/include/coin" \
        --with-lapack="-llapack -lblas" \
        CXXFLAGS="-O3"

    make -j"$JOBS"
    make install

    echo "Clp installed to $PREFIX"
}

build_clp

# =============================================================================
# Verify Installation
# =============================================================================
verify_install() {
    echo ""
    echo "=== Verifying Installation ==="

    local ERRORS=0

    # Check libraries exist
    for lib in libCoinUtils.a libOsi.a libClp.a libOsiClp.a; do
        if [ -f "$PREFIX/lib/$lib" ]; then
            echo "✓ Found $lib"
        else
            echo "✗ Missing $lib"
            ERRORS=$((ERRORS + 1))
        fi
    done

    # Check pkg-config
    if [ -f "$PREFIX/lib/pkgconfig/clp.pc" ]; then
        echo "✓ Found clp.pc"
    else
        echo "✗ Missing clp.pc"
        ERRORS=$((ERRORS + 1))
    fi

    # Check headers
    if [ -f "$PREFIX/include/coin/ClpSimplex.hpp" ]; then
        echo "✓ Found ClpSimplex.hpp"
    else
        echo "✗ Missing ClpSimplex.hpp"
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
    echo "CLP installed to: $PREFIX"
    echo ""
    echo "Libraries built:"
    ls -la "$PREFIX/lib"/lib*.a
    echo ""
    echo "To use with Cargo:"
    echo "  export PKG_CONFIG_PATH=\"$PREFIX/lib/pkgconfig:\$PKG_CONFIG_PATH\""
    echo "  export LD_LIBRARY_PATH=\"$PREFIX/lib:\$LD_LIBRARY_PATH\""
    echo ""
    echo "Or use the wrapper script:"
    echo "  ./scripts/with-clp.sh cargo build -p gat-clp"
}

verify_install
