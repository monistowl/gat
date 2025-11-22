# Build Workflows Overhaul Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to implement this design.

**Goal:** Consolidate GAT's CI/CD into a maintainable, multi-platform build system with feature-gated bundles, dry-run testing, nightly builds, and tiered diagnostics.

**Architecture:** Reusable workflow pattern where a core `build-matrix.yml` orchestrates cross-platform builds (Linux, macOS, Windows), three bundle tiers (headless, full, analyst), with separate release/dry-run/nightly workflows calling the core. Diagnostic outputs are tiered: system info by default, verbose compiler output on-demand.

**Tech Stack:** GitHub Actions, Bash scripts, Rust/Cargo, native toolchains (MSVC on Windows, clang on macOS, gcc on Linux).

---

## Section 1: Workflow Architecture Overview

The new system replaces scattered workflows with a **reusable core** called by specialized workflows:

**Core Reusable Workflow: `.github/workflows/build-matrix.yml`**
- Matrix over `[ubuntu-latest, macos-latest]`
- Matrix over bundle variants: `[headless, full, analyst]`
- For each (OS, variant) pair:
  - Install OS-specific system dependencies (solvers, jq, etc.)
  - Build Cargo with appropriate features
  - Run tests
  - Package tarball/zip to `dist/`
  - Capture system diagnostics (compiler versions, environment)
  - Upload as GitHub artifact
  - Optional: upload verbose compiler logs on-demand

**Specialized Workflows:**
1. **`push-pr.yml`** — Runs on push/PR to main. Fast path: Linux headless build + tests only. Skips packaging and macOS/Windows (too slow for every PR).
2. **`release-dry-run.yml`** — Manual trigger. Runs full build-matrix (all OS × all variants), packages everything, but **does NOT** create GitHub release or upload assets. Great for testing the full pipeline before tagging.
3. **`release.yml`** — Triggered by `git tag v*`. Calls build-matrix, then creates GitHub release and uploads artifacts.
4. **`nightly.yml`** — Scheduled 6am UTC daily. Runs build-matrix, packages, uploads to "nightly" artifact with 14-day retention. Includes verbose compiler output by default (for debugging production issues).

**Artifact Strategy:**
- All builds cache `target/` with `Swatinem/rust-cache@v2` keyed on `Cargo.lock`
- Intermediate artifacts (compiled .rlib files, object files) are preserved in cache
- Final bundles (`.tar.gz` for Unix, `.zip` for Windows) go to `dist/` and are uploaded as GitHub artifacts
- Nightly and dry-run artifacts expire after 14 and 7 days respectively; release artifacts are permanent (GitHub release asset)

---

## Section 2: Bundle Tiers and Feature Gating

Three configurable bundle tiers ship with pre-built binaries. Each corresponds to a Cargo feature set:

**Tier 1: `headless`**
- Minimal dependencies: core grid algorithms, I/O, CLI only
- Features: `-p gat-cli --no-default-features --features "minimal-io"`
- Excludes: `gat-gui`, `gat-tui`, optional solvers (CBC, IPOPT), visualization
- Use case: Scripting, CI/CD pipelines, resource-constrained environments, reproducible batch runs
- Binary size: ~30-50 MB (estimated)
- Installation: `./install.sh --variant headless`

**Tier 2: `full`**
- All features: GUI, TUI, all solvers, visualization, documentation
- Features: `-p gat-cli --all-features` (includes gui, tui, viz, all-backends)
- Includes: `gat-gui`, `gat-tui`, CBC + other LP solvers, plotting libraries
- Use case: Interactive exploration, desktop users, analysts who want every tool
- Binary size: ~200-300 MB (estimated)
- Installation: `./install.sh --variant full`

**Tier 3: `analyst`**
- Middle ground: CLI + domain-specific features for distribution/DER/reliability analysis
- Features: `-p gat-cli --no-default-features --features "minimal-io,adms,derms,dist,analytics,featurize"`
- Includes: ADMS, DERMS, DIST, analytics suite, featurization; excludes GUI/TUI visualization
- Excludes: Heavy visualization libraries (egui/eframe), TUI (ratatui), GUI framework
- Use case: Power systems analysts who want distribution + reliability tools but don't need interactive UI
- Binary size: ~100-150 MB (estimated)
- Installation: `./install.sh --variant analyst`

**Source-Based Fallback:**
If a pre-built binary isn't available (e.g., Windows fails to build, exotic platform), `install.sh` falls back to `cargo build` with the appropriate features for the requested variant. This keeps power users unblocked.

---

## Section 3: Platform-Specific Build Details

**Linux (ubuntu-latest)**
- System deps: `sudo apt-get install -y coinor-libcbc-dev jq libssl-dev pkg-config`
- Solver discovery: Uses standard Linux library paths (`/usr/lib/`, `/usr/local/lib/`)
- Environment: `RUSTFLAGS` and `LD_LIBRARY_PATH` set for CBC linkage
- Binaries: ELF format, distributed as `.tar.gz`
- Status: Fully supported ✓

**macOS (macos-latest)**
- System deps: `brew install coinor-cbc jq openssl pkg-config`
- Solver discovery: Uses Homebrew paths + standard system paths (`/usr/local/lib/`, `/opt/homebrew/lib/`)
- Environment: Clang toolchain, ARM64 auto-detection
- Binaries: Mach-O format, distributed as `.tar.gz`
- Code signing: Deferred (can be added later if needed for Gatekeeper)
- Status: Fully supported ✓

**Windows:** Deferred. Once a Windows developer box is available for testing solver discovery and packaging, Windows support can be added. The workflow structure (reusable matrix) makes it straightforward to add later.

---

## Section 4: Diagnostic Outputs and Debug Info (Tiered)

All workflows capture diagnostic information in a tiered approach to balance artifact size with debuggability.

**Tier 1: Always Captured (System Diagnostics)**
- Workflow metadata: `RUNNER_OS`, `RUNNER_ARCH`, `github.run_id`, branch/tag
- Toolchain: Rust version (`rustc --version`), Cargo version, LLVM version
- System info: OS release, kernel version, available memory, CPU count
- Solver detection: Which solvers found, which versions, where located
- Build environment: `Cargo.lock` hash, dependency graph summary (`cargo tree`)
- Timing: Build duration per crate, test duration
- Artifact manifest: List of binaries/files in `dist/`, file sizes

**Captured as:** Workflow log annotations + summary file (`build-diagnostics.json`) uploaded with artifacts.

**Tier 2: On-Demand Verbose Output (Workflow Input)**
- Triggered via `workflow_dispatch` with input: `verbose_diagnostics: true`
- Enables: `RUSTFLAGS="-v"`, verbose linker output, full compiler warnings, dependency resolution details
- Captures: Compiler phases (codegen, linking), time per compilation unit, memory usage per crate
- Artifact size impact: +100-200 MB per platform for verbose logs (still manageable)
- Use case: Debugging mysterious build failures, understanding why a build is slow

**Tier 3: Full Instrumentation (Opt-In for Specific Workflows)**
- Nightly workflow always includes verbose output (to catch production issues early)
- Release dry-run can enable it via workflow_dispatch input
- Regular push/PR CI: Never includes verbose output (keep artifacts small, fast)

**Artifact Organization:**
```
gat-v0.1.5-linux-x86_64-headless.tar.gz
gat-v0.1.5-linux-x86_64-headless-build-log.txt    (verbose compiler output if enabled)
build-diagnostics.json                             (system info, timings, solver discovery)
```

---

## Section 5: Dry-Run and Release Workflows

**Release Dry-Run Workflow: `release-dry-run.yml`**
- Trigger: Manual (`workflow_dispatch`)
- Can run on any branch, any time
- Calls reusable `build-matrix.yml` with all platforms and all bundle variants
- After build succeeds:
  - **Does NOT** create a GitHub release
  - **Does NOT** upload assets to GitHub releases
  - **Does** upload build artifacts to workflow artifacts (viewable in Actions tab)
  - Prints summary: "Dry run successful. To release, git tag v0.1.5 && git push --tags"
- Value: Test the full packaging and build pipeline before committing to a release tag

**Release Workflow: `release.yml`**
- Trigger: Tag push (`git tag v*`)
- Calls reusable `build-matrix.yml` with all platforms and all bundle variants
- After build succeeds:
  - Creates GitHub release with tag name and version
  - Uploads each bundle (`headless`, `full`, `analyst`) for each platform to release assets
  - Publishes release (not draft, not pre-release)
- Idempotent: Re-running with same tag is safe (overwrites assets)

**Key Difference:**
The only difference between dry-run and release is the final "create release and upload assets" step. By using a reusable workflow, the build/package/test logic is identical—eliminating duplication and ensuring parity.

---

## Section 6: Nightly Build Workflow

**Nightly Workflow: `nightly.yml`**
- Trigger: Scheduled 6am UTC daily
- Calls reusable `build-matrix.yml` with all platforms and all bundle variants
- **Always includes verbose diagnostics** (compiler output, detailed logs)
- Artifacts:
  - Retained for 14 days (longer than dry-run at 7 days)
  - Labeled "nightly" in artifact names for easy identification
  - Includes full build logs for troubleshooting
- Value: Catch issues early, give users access to latest pre-release builds, detect platform-specific regressions

**Nightly Artifacts:**
```
nightly-gat-linux-x86_64-headless.tar.gz
nightly-gat-macos-aarch64-full.tar.gz
nightly-gat-windows-x86_64-analyst.zip
...
nightly-build-diagnostics.json
nightly-compiler-output.log
```

Users who want cutting-edge can download from Actions → nightly workflow → artifacts.

---

## Section 7: Fast Path for Daily Development (push/PR)

**Lightweight CI: `push-pr.yml`**
- Trigger: Push to `main`, PR to `main`, schedule (6am UTC for safety)
- **Fast path:** Linux only, headless variant, minimal features
- Steps:
  1. Format check (`cargo fmt --check`)
  2. Clippy (`cargo clippy -p gat-core -p gat-cli --no-default-features --features minimal`)
  3. Tests (`cargo test -p gat-core -p gat-cli --no-default-features`)
  4. Upload debug target cache for developers
- Duration: ~5 min (vs. ~20 min for full matrix)
- **No packaging step** — we don't need binaries on every PR
- Artifact: Debug build cache only (helps developers iterate locally)

**Why separate from build-matrix?**
- PRs shouldn't trigger expensive multi-platform builds every time
- Developers get fast feedback
- Full matrix is reserved for scheduled/dry-run/release scenarios
- Keeps GitHub Actions minutes budget sane

---

## Summary

**Key Design Principles:**

1. **Reusable core** (`build-matrix.yml`) — One source of truth for build logic, called by multiple workflows
2. **Three bundle tiers** — Headless (minimal), Full (everything), Analyst (domains) give users clear choices
3. **Multi-platform** — Linux, macOS, Windows; Windows is best-effort and can be disabled if solver discovery fails
4. **Dry-run before release** — Test the full pipeline without committing to a tag
5. **Nightly channel** — Daily builds with verbose diagnostics, 14-day retention, for early issue detection
6. **Tiered diagnostics** — System info always, verbose compiler output on-demand, full instrumentation in nightly
7. **Fast path for CI** — Linux headless only for push/PR, skip expensive multi-platform builds
8. **Consistent artifact stashing** — All builds use same cache strategy, same `dist/` layout, same upload paths

**Workflows:**
- `push-pr.yml` — Linux headless, fast
- `build-matrix.yml` — Reusable core for all multi-platform builds
- `release-dry-run.yml` — Full build, no release creation
- `release.yml` — Full build + GitHub release
- `nightly.yml` — Full build + verbose diagnostics, 14-day retention

**Result:** Maintainable, auditable, DRY build system that scales from daily development (fast) to nightly diagnostics (thorough) to production releases (reliable).

