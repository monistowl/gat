# gat-ipopt

IPOPT solver wrapper for GAT's native solver plugin system.

## Overview

This crate provides an IPOPT (Interior Point OPTimizer) wrapper that implements the GAT solver IPC protocol. It runs as a subprocess, receiving optimization problems via stdin and returning solutions via stdout using Arrow IPC format.

## Prerequisites

IPOPT must be installed on your system before building this crate.

### Ubuntu/Debian

```bash
sudo apt install coinor-libipopt-dev
```

### macOS (Homebrew)

```bash
brew install ipopt
```

### From Source

See the official IPOPT installation guide:
https://coin-or.github.io/Ipopt/INSTALL.html

## Building

Build with IPOPT support:

```bash
cargo build -p gat-ipopt --features ipopt-sys --release
```

Or use the xtask helper:

```bash
cargo xtask build-solver ipopt --install
```

## Installation

After building, install to the GAT solvers directory:

```bash
# Manual installation
cp target/release/gat-ipopt ~/.gat/solvers/

# Or with xtask
cargo xtask build-solver ipopt --install
```

Then enable native solvers in `~/.gat/config/gat.toml`:

```toml
[solvers]
native_enabled = true
```

## Protocol

The binary implements GAT's solver IPC protocol:

1. **Input**: Arrow IPC stream on stdin containing:
   - Variable bounds (`var_lower`, `var_upper`)
   - Constraint bounds (`con_lower`, `con_upper`)
   - Objective coefficients (`c`)
   - Initial point (`x0`)
   - Problem dimensions

2. **Output**: Arrow IPC stream on stdout containing:
   - Solution status
   - Optimal objective value
   - Optimal variable values (`x`)
   - Iteration count
   - Solve time

3. **Exit Codes**:
   - `0`: Success
   - `1`: Invalid input
   - `2`: Solver error
   - `3`: Timeout
   - `139`: Segfault (native library crash)

## Usage

The binary is typically invoked by GAT's solver dispatcher, not directly:

```rust
use gat_algo::opf::{SolverDispatcher, DispatchConfig, ProblemClass};

let config = DispatchConfig {
    native_enabled: true,
    ..Default::default()
};

let dispatcher = SolverDispatcher::with_config(config);
let solver = dispatcher.select(ProblemClass::NonlinearProgram)?;
// If IPOPT is installed, this returns SolverBackend::Ipopt
```

## Environment Variables

- `RUST_LOG`: Set logging level (e.g., `RUST_LOG=debug`)

## License

MIT OR Apache-2.0
