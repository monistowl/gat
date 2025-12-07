+++
title = "Feature Matrix"
description = "gat-cli Feature Matrix and Distribution Builds"
weight = 33
+++

# gat-cli Feature Matrix

## Distribution Build Variants

For end-user builds, use the `dist*` feature flags:

| Feature | Solvers | TUI | Viz | IPOPT | Binary | Use Case |
|---------|---------|-----|-----|-------|--------|----------|
| `dist` | clarabel, highs | ✓ | ✓ | — | ~61 MB | Desktop users |
| `dist-headless` | clarabel, highs | — | ✓ | — | ~60 MB | Servers, CI |
| `dist-native` | clarabel, highs | ✓ | ✓ | ✓ | ~61 MB | AC-OPF (requires libipopt) |
| `dist-native-headless` | clarabel, highs | — | ✓ | ✓ | ~60 MB | HPC clusters |

**Build commands:**

```bash
# Desktop (TUI + all solvers)
cargo build -p gat-cli --release --no-default-features --features dist

# Server/automation (headless)
cargo build -p gat-cli --release --no-default-features --features dist-headless

# With IPOPT for AC-OPF
cargo build -p gat-cli --release --no-default-features --features dist-native
```

## CI Feature Matrix

The `.github/workflows/rust.yml` CI job runs on every push to test feature combinations:

- **Where it runs:** `ubuntu-latest`
- **What it tests:** `cargo clippy` and `cargo test` with various feature sets

**CI feature sets:**

| Set | Components | Purpose |
|-----|------------|---------|
| `minimal` | solver-clarabel + lean I/O | Fast CI, core functionality |
| `minimal,full-io` | + full I/O stack | Data format coverage |
| `minimal,full-io,viz` | + gat-viz | Visualization helpers |
| `all-solvers` | clarabel + highs | Full solver pool |

## Solver Features

Individual solver features can be combined:

| Feature | Solver | Type | Runtime Deps |
|---------|--------|------|--------------|
| `solver-clarabel` | Clarabel | SOCP/SDP | None (pure Rust) |
| `solver-highs` | HiGHS | LP/MIP | None (pure Rust) |
| `solver-ipopt` | IPOPT | NLP | libipopt.so |

## GPU Acceleration Features

| Feature | Capability | Hardware |
|---------|------------|----------|
| `gpu` | WGSL compute shaders | Vulkan/Metal/DX12 |

GPU acceleration provides parallel computation for:
- ADMM branch flow calculation (2-10x speedup on large networks)
- Monte Carlo reliability simulation
- Batch power flow operations

```bash
# Build with GPU support
cargo build -p gat-cli --release --features gpu
```

Auto-fallback to CPU if GPU unavailable.

## Legacy Features

For minimal or custom builds:

```bash
# Minimal (Clarabel only)
cargo build -p gat-cli --release --no-default-features --features minimal

# Custom combination
cargo build -p gat-cli --release --no-default-features --features "solver-clarabel,solver-highs,tui,viz"
```

Running the feature matrix catches regressions where a feature flag environment might compile but not run under different solver stacks.
