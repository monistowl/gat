# Building COIN-OR Solvers from Vendored Sources

This document describes how to build the complete COIN-OR solver stack (CLP, CBC, IPOPT) from vendored sources for fully offline, reproducible builds.

## Why Build from Source?

Building from vendored sources enables:

- **Fully offline builds** - no network access required during build
- **Reproducible CI** - same sources on every build
- **Parallel MUMPS** with OpenMP for multi-threaded factorization (IPOPT)
- **Metis ordering** for better fill-in reduction (IPOPT)
- **Cross-platform support** - Linux and macOS from same sources

## Prerequisites

**Linux (Ubuntu/Debian):**
```bash
sudo apt install build-essential gfortran libblas-dev liblapack-dev libbz2-dev zlib1g-dev pkg-config
```

**macOS:**
```bash
brew install gcc lapack pkg-config
```

## Vendored Sources

All source code is vendored in `vendor/` - no network access required:

| Package | File | Purpose |
|---------|------|---------|
| CoinUtils | `CoinUtils-master.zip` | Base utilities (vectors, matrices, I/O) |
| Osi | `Osi-master.zip` | Open Solver Interface |
| Clp | `Clp-master.zip` | Simplex LP solver |
| Cgl | `Cgl-master.zip` | Cut Generator Library |
| Cbc | `Cbc-master.zip` | Branch and Cut MIP solver |
| Metis 4.0.3 | `metis-4.0.3.tar.gz` | Graph partitioning for fill reduction |
| MUMPS 5.8.1 | `MUMPS_5.8.1.tar.gz` | Parallel sparse direct solver |
| ThirdParty-Metis | `ThirdParty-Metis-stable-2.0.zip` | COIN-OR build wrapper + patches |
| ThirdParty-Mumps | `ThirdParty-Mumps-stable-3.0.zip` | COIN-OR build wrapper + patches |
| IPOPT 3.14 | `Ipopt-stable-3.14/` | Interior point optimizer |

## Build Scripts

### Full Build (all solvers)
```bash
./scripts/build-clp.sh   # CoinUtils → Osi → Clp
./scripts/build-cbc.sh   # Cgl → Cbc (requires CLP)
./scripts/build-ipopt.sh # Metis → MUMPS → IPOPT
```

### Individual Builds

**CLP only (LP solver):**
```bash
./scripts/build-clp.sh
```
Builds: CoinUtils, Osi, Clp, OsiClp

**CBC (requires CLP first):**
```bash
./scripts/build-clp.sh
./scripts/build-cbc.sh
```
Builds: Cgl, Cbc, OsiCbc

**IPOPT (independent):**
```bash
./scripts/build-ipopt.sh
```
Builds: Metis, MUMPS (with OpenMP), IPOPT

Output is installed to `vendor/local/`.

Build times on a modern CPU:
- CLP: ~2 minutes
- CBC: ~1 minute (after CLP)
- IPOPT: ~5 minutes

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

Build scripts:
- `scripts/build-clp.sh` - Builds CoinUtils, Osi, Clp
- `scripts/build-cbc.sh` - Builds Cgl, Cbc (requires CLP)
- `scripts/build-ipopt.sh` - Builds Metis, MUMPS, IPOPT
- `scripts/with-ipopt.sh` - Environment wrapper for Cargo

Output:
- `vendor/local/` - Built libraries (gitignored)
- `vendor/local/lib/` - Static (.a) and shared (.so) libraries
- `vendor/local/include/` - Header files
