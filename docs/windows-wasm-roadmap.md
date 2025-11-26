# Windows and WASM Build Roadmap

## Goals and Current CI Baseline
- Extend deliverables beyond Linux/macOS to **Windows (MSVC)** and **WASM** while keeping release packaging, diagnostics, and feature-gated bundles consistent.
- Present CI focuses on Linux/macOS only: the reusable build matrix runs Ubuntu/macOS across the `headless`, `full`, and `analyst` variants, and the fast PR workflow only validates Linux with minimal features. Adding Windows/WASM requires matrix, dependency, and shell adjustments to avoid regressions.

## Windows Readiness: Risks to Tackle Early
- **Path handling:** Audit all filesystem joins/prints to avoid hard-coded forward slashes; prefer `PathBuf`/`display` and normalize inputs (e.g., handle `C:\` roots, UNC paths, case-insensitive collisions). Add tests that round-trip backslash-heavy paths through CLI argument parsing.
- **Line endings and encoding:** Ensure tests that compare golden files tolerate `\r\n`, and explicitly set text mode for generated files when byte-for-byte matches matter.
- **Shell assumptions:** Replace bash-only snippets (`scripts/`, `xtask`, workflow steps) with Rust helpers or cross-platform tooling. For PowerShell, double-quote environment variable expansions and avoid `$(...)` command substitution.
- **Native dependencies:** CBC/openssl/pkg-config currently install via `apt`/`brew`; Windows needs a vcpkg or prebuilt strategy (static CBC, openssl via `vcpkg install openssl:x64-windows`). Provide a feature flag to build without CBC while retaining core functionality.
- **Process and filesystem semantics:** Review uses of `std::os::unix`, symlinks, and file locks; add `cfg` guards and fallbacks (e.g., copy instead of symlink on NTFS). Verify temp directory handling with long paths and spaces.
- **Packaging:** Ship `.zip` artifacts with DLL discovery notes. Validate runtime search paths for dynamic solvers (PATH updates or `System32` placement) and ensure installer scripts do not assume tarballs.

## Windows Implementation Plan (Phased)
1. **Dependency + feature audit**
   - Run `cargo tree --target x86_64-pc-windows-msvc` to flag crates using `nix`, `mmap` modes, or `libloading`/`dlfcn` assumptions.
   - Add `cfg(target_os = "windows")` shims for `std::os::unix` calls; gate optional crates that cannot build on Windows behind existing feature tiers (`headless` as bootstrap target).
   - Decide CBC story: (a) disable by default on Windows, (b) static link via vcpkg, or (c) ship bundled DLL.
2. **Bootstrap build + tests (headless)**
   - Ensure `gat-cli` + core crates compile with `--no-default-features --features minimal-io` on Windows.
   - Create Windows-specific path normalization tests and CLI round-trips; add CRLF fixtures where comparisons are strict.
3. **Enable full feature set**
   - Incrementally enable GUI/TUI/analytics features; replace unix-only terminal or epoll usage with cross-platform crates (e.g., crossterm).
   - Validate solver discovery on Windows (PATH checks, environment variables) and document expected install locations.
4. **Packaging + release validation**
   - Produce `.zip` bundles with `gat.exe`, required DLLs, diagnostics JSON, and a short `INSTALL.md` describing PATH updates.
   - Smoke-test PowerShell installs on `windows-latest` runners: build, run sample CLI command, and archive artifacts.

## WASM Readiness and Risk Areas
- **Target choice:** Prefer `wasm32-wasi` for headless analytics; `wasm32-unknown-unknown` only if browser UI is required. Confirm crates that depend on threads, sockets, or file I/O have fallbacks or are feature-gated.
- **FFI and solvers:** Native solvers (CBC/HiGHS/Ipopt) will not compile to WASM; gate them off and expose pure-Rust or mock backends for deterministic tests.
- **Env/process usage:** Replace `std::process::Command` and `std::env::var` calls in shared logic with trait-based abstractions so WASM builds can stub them. Audit uses of `std::fs` for temporary files or memory-mapped I/O.
- **Binary size + performance:** Turn off debug info, prefer `wee_alloc`/`dlmalloc` when appropriate, and provide streaming I/O APIs to avoid large linear-memory allocations.

## WASM Implementation Plan (Phased)
1. **Core compilation probe**
   - Add `wasm32-wasi` target, run `cargo check -p gat-core -p gat-io --no-default-features --features minimal-io --target wasm32-wasi` to inventory blockers.
   - Track blockers in a compatibility list (per-crate/per-feature) and add `cfg(target_arch = "wasm32")` shims for randomness, time, and file operations.
2. **Feature gating + API surface**
   - Define a `wasm` Cargo feature that disables solvers, CLI process spawning, and OS-specific transport layers while exposing pure computation and serialization routines.
   - Introduce `no_std`-friendly or WASI-compatible replacements for logging, timers, and path normalization.
3. **Bindings and packaging**
   - If browser delivery is needed, wrap the `wasm` feature set with `wasm-bindgen`/`wasm-pack` and publish an npm package; for WASI, publish `.wasm` + `wit` metadata.
   - Provide example harnesses (Node + WASI runtime) to validate import/export conventions and I/O (stdin/stdout vs. file descriptors).

## GitHub Actions Changes Required
- **Build matrix expansion:** Add `windows-latest` to `.github/workflows/build-matrix.yml` and split dependency steps by OS: use Chocolatey/vcpkg for CBC/openssl and ensure artifacts upload as `.zip`. Keep verbose diagnostics flag compatible with PowerShell. Current matrix only targets Ubuntu/macOS, so Windows needs a new leg.
- **Nightly/release propagation:** Because nightly and release workflows call the reusable matrix, Windows support will propagate automatically once the matrix includes it; ensure artifact naming includes `windows-x86_64` and retention policies remain 14/7 days.
- **WASM job:** Add a lightweight `wasm-check` job (Linux runner) that installs `wasm32-wasi` (and `wasm-pack` if browser bindings are planned), runs `cargo check` with the `wasm` feature set, and uploads a compatibility report.
- **Fast CI guardrails:** The fast PR workflow currently runs Linux-only formatting, clippy, and tests with minimal features; consider adding a Windows smoke test (build-only) and a WASM `cargo check` to catch regressions without running full packaging on every PR.
- **Cache keys and shells:** Use OS-specific cache keys (Linux/macOS/Windows) to avoid cross-platform target reuse, and ensure Windows steps use PowerShell-friendly syntax (`$env:VAR`) instead of bash-only constructs.
