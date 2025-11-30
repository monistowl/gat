#!/bin/bash
set -euo pipefail

# Build CBC (COIN-OR Branch and Cut) from vendor sources
# Prerequisites: build-essential, libblas-dev, liblapack-dev
#
# Usage: ./scripts/build-cbc.sh
#
# IMPORTANT: Run ./scripts/build-clp.sh first! CBC depends on CLP.
#
# This builds CBC and its dependencies in order:
# 1. Cgl - Cut Generator Library (cutting planes for MIP)
# 2. Cbc - Branch and Cut MIP solver

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PREFIX="$PROJECT_ROOT/vendor/local"
VENDOR="$PROJECT_ROOT/vendor"
JOBS=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
export PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}:$PREFIX/lib/pkgconfig"

echo "=== GAT CBC Build ==="
echo "Project root: $PROJECT_ROOT"
echo "Install prefix: $PREFIX"
echo "Build jobs: $JOBS"
echo ""

# Check that CLP is already built (CBC depends on it)
if [ ! -f "$PREFIX/lib/libClp.a" ]; then
    echo "ERROR: CLP not found at $PREFIX/lib/libClp.a"
    echo "Please run ./scripts/build-clp.sh first"
    exit 1
fi

echo "Found CLP at $PREFIX/lib/libClp.a"

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
# Build Cgl (Cut Generator Library)
# =============================================================================
build_cgl() {
    echo ""
    echo "=== Building Cgl ==="

    if [ -f "$PREFIX/lib/libCgl.a" ]; then
        echo "Cgl already built, skipping..."
        return 0
    fi

    extract_if_needed "Cgl-master"

    cd "$VENDOR/Cgl-master"

    # Clean any previous build
    make distclean 2>/dev/null || true

    # Configure with CoinUtils, Osi, and Clp
    ./configure --prefix="$PREFIX" \
        --disable-shared \
        --with-coinutils-lflags="-L$PREFIX/lib -lCoinUtils" \
        --with-coinutils-cflags="-I$PREFIX/include/coin" \
        --with-osi-lflags="-L$PREFIX/lib -lOsi" \
        --with-osi-cflags="-I$PREFIX/include/coin" \
        --with-clp-lflags="-L$PREFIX/lib -lClp -lOsiClp" \
        --with-clp-cflags="-I$PREFIX/include/coin" \
        CXXFLAGS="-O3"

    make -j"$JOBS"
    make install

    echo "Cgl installed to $PREFIX"
}

build_cgl

# =============================================================================
# Build Cbc (Branch and Cut solver)
# =============================================================================
build_cbc() {
    echo ""
    echo "=== Building Cbc ==="

    if [ -f "$PREFIX/lib/libCbc.a" ]; then
        echo "Cbc already built, skipping..."
        return 0
    fi

    extract_if_needed "Cbc-master"

    cd "$VENDOR/Cbc-master"

    # Clean any previous build
    make distclean 2>/dev/null || true

    # Configure with all dependencies
    ./configure --prefix="$PREFIX" \
        --disable-shared \
        --with-coinutils-lflags="-L$PREFIX/lib -lCoinUtils" \
        --with-coinutils-cflags="-I$PREFIX/include/coin" \
        --with-osi-lflags="-L$PREFIX/lib -lOsi" \
        --with-osi-cflags="-I$PREFIX/include/coin" \
        --with-clp-lflags="-L$PREFIX/lib -lClp -lOsiClp" \
        --with-clp-cflags="-I$PREFIX/include/coin" \
        --with-cgl-lflags="-L$PREFIX/lib -lCgl" \
        --with-cgl-cflags="-I$PREFIX/include/coin" \
        --with-lapack="-llapack -lblas" \
        CXXFLAGS="-O3"

    make -j"$JOBS"
    make install

    echo "Cbc installed to $PREFIX"
}

build_cbc

# =============================================================================
# Verify Installation
# =============================================================================
verify_install() {
    echo ""
    echo "=== Verifying Installation ==="

    local ERRORS=0

    # Check libraries exist
    for lib in libCgl.a libCbc.a libOsiCbc.a; do
        if [ -f "$PREFIX/lib/$lib" ]; then
            echo "✓ Found $lib"
        else
            echo "✗ Missing $lib"
            ERRORS=$((ERRORS + 1))
        fi
    done

    # Check pkg-config
    if [ -f "$PREFIX/lib/pkgconfig/cbc.pc" ]; then
        echo "✓ Found cbc.pc"
    else
        echo "✗ Missing cbc.pc"
        ERRORS=$((ERRORS + 1))
    fi

    # Check headers
    if [ -f "$PREFIX/include/coin/CbcModel.hpp" ]; then
        echo "✓ Found CbcModel.hpp"
    else
        echo "✗ Missing CbcModel.hpp"
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
    echo "CBC installed to: $PREFIX"
    echo ""
    echo "Libraries built:"
    ls -la "$PREFIX/lib"/lib*.a | grep -E "(Cgl|Cbc)"
    echo ""
    echo "To use with Cargo:"
    echo "  export PKG_CONFIG_PATH=\"$PREFIX/lib/pkgconfig:\$PKG_CONFIG_PATH\""
    echo ""
    echo "Full COIN-OR stack now available:"
    echo "  CoinUtils -> Osi -> Clp -> Cgl -> Cbc"
}

verify_install
