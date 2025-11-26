# WASM Compatibility Log

Purpose: track blockers and fixes for `wasm32-wasi` (primary) and `wasm32-unknown-unknown` (optional).

Template per entry:
- Date (UTC)
- Target / command
- Error excerpt
- Root cause analysis
- Mitigation / patch link / status

## Current status (2025-11-26)
- First probe: `cargo check -p gat-core -p gat-io --no-default-features --target wasm32-wasip1` (Linux host).
  - **Fail**: `bzip2-sys` and `zstd-sys` C builds error (`bits/libc-header-start.h` not found) when cross-compiling with clang to wasm32-wasip1.
  - **Cause**: C compression backends require a WASI sysroot or should be disabled for wasm. Arrow/parquet default features pull zstd/bzip2.
  - **Next actions**:
    - Add `wasm` feature that disables zstd/bzip2 (use pure-Rust backends only) or wire build scripts to use `WASI_SDK_PATH` when provided.
    - Consider `RUSTFLAGS='-C target-feature=+atomics,+bulk-memory'` only after compression blockers cleared.

## Notes
- Solvers (CBC/HiGHS/Ipopt) expected to be disabled under `wasm` feature.
- Watch for `std::process::Command`, filesystem writes, and thread usage; capture crate + function when seen.
