# Windows and WASM Build Roadmap

Audience: maintainers expanding platform coverage beyond Linux/macOS. Scope includes CI, packaging, runtime ergonomics, and documentation. Updated: 2025-11-26.

## Snapshot: Where We Are (truthy baseline)
- CI: Ubuntu + macOS matrix for `headless/full/analyst`; fast PR workflow is Linux-only, minimal features.
- Artifacts: `.tar.gz` only, no Windows `.zip`, no signed installers.
- Solvers: CBC/HiGHS/Ipopt assumed available via apt/brew; no Windows provisioning story.
- WASM: unsupported; no target or feature flag present.

## Goals (definition of done)
1) Windows MSVC builds in CI with smoke-test execution and published `.zip` artifacts (includes `gat.exe`, LICENSE, README, minimal INSTALL).
2) WASM (wasi-first) `cargo check` green for core IO/analytics with a documented `wasm` feature gate and example harness.
3) Documentation that explains prerequisites, known limits, and how to run sample commands on each platform.

## Windows: Risk + Mitigation Checklist
- Paths and case-insensitivity: replace manual slash joins with `PathBuf`; add round-trip CLI tests for backslash + drive letters; avoid temp files in case-sensitive-only locations.
- Line endings: fixtures and golden outputs must allow `\r\n`; prefer textual comparisons that normalize EOL.
- Shell assumptions: convert bash invocations in workflows/xtask to Rust or PowerShell-safe commands; avoid `$(...)` in PowerShell, use `$()` in bash only.
- Native deps: CBC/OpenSSL via vcpkg; add feature `no-cbc` fallback. Document PATH edits required for DLL discovery.
- Filesystem semantics: guard `std::os::unix` usage; replace symlink writes in tests with copies on Windows; watch for MAX_PATH issues (use `\?\` prefix if needed).
- Packaging: `.zip` with a flat bin layout plus `bin/` friendly PATH instructions; ensure artifact names include `windows-x86_64` and version.

## Windows Workplan (time-ordered)
1. **Inventory blockers (Day 1-2)**
   - `cargo tree --target x86_64-pc-windows-msvc` for each crate tier; log blockers in docs/plans/windows-blockers.md.
   - Run `cargo check -p gat-cli --no-default-features --features minimal-io --target x86_64-pc-windows-msvc` locally or on GH runner to collect first error set.
2. **Patch portability gaps (Week 1)**
   - Replace `std::os::unix` calls with `cfg` gates; add Windows equivalents or noop fallbacks.
   - Normalize file writes in tests (explicit text mode, tolerant comparisons).
   - Gate solver backends behind `cfg(not(target_os = "windows"))` where missing; add `no-cbc` feature for Windows bootstrap.
3. **CI enablement (Week 2)**
   - Update `.github/workflows/build-matrix.yml` to include `windows-latest` and PowerShell steps for deps (vcpkg: openssl, optional CBC). Artifact: `.zip`.
   - Add a minimal Windows smoke job to fast PR workflow: build `gat-cli` (headless features), run `gat --help`.
4. **Packaging + docs (Week 3)**
   - Produce `.zip` with `gat.exe`, `LICENSE`, `README`, `INSTALL-windows.md` (PATH instructions + solver notes).
   - Add verification step to nightly/release workflows: run `gat version` on artifact.
5. **Full feature sweep (Week 4+)**
   - Re-enable optional features (TUI/analytics) once cross-platform crates confirmed; add integration tests for solver discovery on Windows.

## WASM: Risk + Mitigation Checklist
- Target choice: start with `wasm32-wasi` for headless analytics; browser (`wasm32-unknown-unknown`) only if a TUI-to-web path emerges.
- Native solvers: cannot compile; gate off and provide pure-Rust placeholders or error stubs. Ensure feature selection fails fast with clear messages.
- OS bindings: abstract `Command`, env vars, and direct file IO behind traits to allow WASI shims; audit `std::time::SystemTime` and randomness usage.
- Memory and size: enable `opt-level = "s"`, strip debuginfo for wasm; consider `wee_alloc`/`dlmalloc` when beneficial.

## WASM Workplan
1. **Probe build (Day 1)**
   - Add target: `rustup target add wasm32-wasi` in CI helper script.
   - Run `cargo check -p gat-core -p gat-io --no-default-features --features minimal-io --target wasm32-wasi`; capture blocker log at docs/plans/wasm-compat.md.
2. **Feature surfacing (Week 1)**
   - Introduce `wasm` Cargo feature that disables external solvers, process spawning, and file-backed temp usage; expose pure compute + serialization APIs.
   - Add `cfg(target_arch = "wasm32")` shims for time, randomness, and logging sinks.
3. **Bindings + samples (Week 2)**
   - Provide WASI harness example: run a PF on an embedded Arrow asset and print JSON summary (docs/example snippets + `examples/wasm-wasi-run.rs`).
   - If browser is needed, add `wasm-bindgen` feature gate and simple Node test via `wasm-pack test --node` (optional, behind env flag).
4. **CI job (Week 2)**
   - New lightweight job `wasm-check` on Linux runner: install target, run `cargo check --features wasm --target wasm32-wasi`, upload blocker report if fails.

## CI/Release Changes (cross-cutting)
- Build matrix: add Windows; per-OS cache keys; PowerShell-safe env usage (`$env:VAR`).
- Fast workflow: add Windows build-only leg and WASM `cargo check` leg; keep under 10 minutes by skipping solvers.
- Artifacts: naming `gat-{variant}-windows-x86_64.zip`, `gat-{variant}-wasi.wasm`; retain 14/7 days.
- Diagnostics: ensure verbose build logs saved for new targets; add `ci/tools/report-blockers.sh` to summarize failing crates for wasm/windows.

## Open Decisions (capture in PRs)
- CBC on Windows: disable vs. static link via vcpkg binary cache. Default leaning: disable in headless, optional with vcpkg.
- Browser bindings: only add when a concrete consumer appears; defer to keep WASM surface small.
- Signing: whether to sign Windows binaries; out of scope for first pass.

## Acceptance Checks
- Windows: CI green for `cargo check` + smoke run; artifact downloadable and `gat.exe --help` succeeds on runner; docs updated with install steps.
- WASM: `cargo check --features wasm --target wasm32-wasi` green; blocker list empty or documented with `cfg` guards; sample harness runs under `wasmtime`.

## Tracking docs
- `docs/plans/windows-blockers.md` — running list of Windows-specific failures.
- `docs/plans/wasm-compat.md` — wasm target blockers and mitigations.
- `docs/plans/ci-expansion.md` — action items for workflow changes.
