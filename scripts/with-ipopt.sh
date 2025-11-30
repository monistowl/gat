#!/bin/bash
# Run a command with the optimized IPOPT build in PATH
#
# Usage: ./scripts/with-ipopt.sh cargo test --features solver-ipopt
#        ./scripts/with-ipopt.sh cargo run --release -- opf ac grid.arrow

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
IPOPT_PREFIX="$PROJECT_ROOT/vendor/local"

if [ ! -d "$IPOPT_PREFIX/lib" ]; then
    echo "ERROR: Optimized IPOPT not found at $IPOPT_PREFIX"
    echo "Run ./scripts/build-ipopt.sh first"
    exit 1
fi

export PKG_CONFIG_PATH="$IPOPT_PREFIX/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
export LD_LIBRARY_PATH="$IPOPT_PREFIX/lib:${LD_LIBRARY_PATH:-}"

exec "$@"
