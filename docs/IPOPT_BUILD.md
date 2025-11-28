# Building Optimized IPOPT from Source

This document describes how to build IPOPT with parallel MUMPS and Metis for improved AC-OPF performance.

## Why Build from Source?

The system IPOPT package (`coinor-libipopt-dev`) uses sequential MUMPS. Building from source enables:

- **Parallel MUMPS** with OpenMP for multi-threaded factorization
- **Metis ordering** for better fill-in reduction (30-50% improvement)
- Static linking of MUMPS into IPOPT for simpler deployment

## Prerequisites

```bash
sudo apt install build-essential gfortran liblapack-dev libblas-dev pkg-config
```

## Build

```bash
./scripts/build-ipopt.sh
```

This script:
1. Builds Metis from `vendor/ThirdParty-Metis-stable-2.0.zip`
2. Builds MUMPS from `vendor/ThirdParty-Mumps-stable-3.0.zip` with OpenMP
3. Builds IPOPT from `vendor/Ipopt-stable-3.14/` linking the above

Output is installed to `vendor/local/`.

Build time: ~5-6 minutes on a modern CPU.

## Usage

### With Wrapper Script

```bash
./scripts/with-ipopt.sh cargo build --features solver-ipopt
./scripts/with-ipopt.sh cargo test --features solver-ipopt
```

### Manual Environment

```bash
export PKG_CONFIG_PATH="$PWD/vendor/local/lib/pkgconfig:$PKG_CONFIG_PATH"
export LD_LIBRARY_PATH="$PWD/vendor/local/lib:$LD_LIBRARY_PATH"
cargo build --features solver-ipopt
```

## Verification

Check linking:
```bash
ldd target/release/gat-cli | grep ipopt
# Should show: vendor/local/lib/libipopt.so.3
```

Run tests:
```bash
./scripts/with-ipopt.sh cargo test --features solver-ipopt -p gat-algo -- ac_nlp
```

## CLI Usage

The IPOPT solver is available via the `opf ac-nlp` command with `--solver ipopt`:

```bash
# Build with IPOPT support
./scripts/with-ipopt.sh cargo build --release --features solver-ipopt

# Run AC-OPF with IPOPT
./scripts/with-ipopt.sh cargo run --release --features solver-ipopt --bin gat-cli -- \
    opf ac-nlp /path/to/network --solver ipopt -o output.json

# Available solvers: lbfgs (default), ipopt
```

## Current Status

| Component | Status |
|-----------|--------|
| IPOPT build | ✅ Working |
| MUMPS parallel | ✅ Static linked with OpenMP |
| Metis ordering | ✅ Integrated |
| Library tests | ✅ 175 tests pass |
| CLI integration | ✅ `--solver ipopt` available |

## Files

- `scripts/build-ipopt.sh` - Build automation
- `scripts/with-ipopt.sh` - Environment wrapper for Cargo
- `vendor/local/` - Built libraries (gitignored)
