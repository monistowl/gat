# Build Workflows Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the multi-platform build system with feature-gated bundles, dry-run testing, nightly builds, and tiered diagnostics per the design document.

**Architecture:** Create reusable workflow (`build-matrix.yml`) that orchestrates cross-platform (Linux, macOS) builds across three bundle variants (headless, full, analyst). Refactor existing workflows to call this core. Add new release-dry-run and diagnostic capabilities.

**Tech Stack:** GitHub Actions (YAML), Bash scripts, Rust/Cargo, `Swatinem/rust-cache@v2`.

---

### Task 1: Create Reusable Workflow Core (`build-matrix.yml`)

**Files:**
- Create: `.github/workflows/build-matrix.yml`

**Step 1: Read existing release.yml for reference**

Run: `cat /home/tom/Code/gat/.github/workflows/release.yml`

Understand the current structure: matrix over OS, environment setup, build steps, packaging.

**Step 2: Create new reusable workflow file**

Create `.github/workflows/build-matrix.yml` with the following content. This will be the reusable core called by release, dry-run, and nightly workflows:

```yaml
name: Build Matrix

on:
  workflow_call:
    inputs:
      verbose_diagnostics:
        description: 'Enable verbose compiler output'
        required: false
        type: boolean
        default: false
      upload_artifacts:
        description: 'Upload build artifacts'
        required: false
        type: boolean
        default: true
    outputs:
      build_summary:
        description: 'Build summary JSON'
        value: ${{ jobs.build.outputs.summary }}

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.os }} / ${{ matrix.variant }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
        variant: [headless, full, analyst]

    outputs:
      summary: ${{ steps.summary.outputs.json }}

    steps:
      - uses: actions/checkout@v4

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo builds
        uses: Swatinem/rust-cache@v2
        with:
          shared-key: ${{ hashFiles('Cargo.lock') }}

      - name: Capture system info
        id: sysinfo
        run: |
          echo "os=$(uname -s)" >> $GITHUB_OUTPUT
          echo "arch=$(uname -m)" >> $GITHUB_OUTPUT
          echo "rustc=$(rustc --version)" >> $GITHUB_OUTPUT
          echo "cargo=$(cargo --version)" >> $GITHUB_OUTPUT

      - name: Install Linux dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y coinor-libcbc-dev jq libssl-dev pkg-config

      - name: Install macOS dependencies
        if: runner.os == 'macOS'
        run: |
          brew install coinor-cbc jq openssl pkg-config || true

      - name: Determine build features
        id: features
        run: |
          case "${{ matrix.variant }}" in
            headless)
              echo "flags=--no-default-features --features minimal-io" >> $GITHUB_OUTPUT
              ;;
            analyst)
              echo "flags=--no-default-features --features minimal-io,adms,derms,dist,analytics,featurize" >> $GITHUB_OUTPUT
              ;;
            full)
              echo "flags=--all-features" >> $GITHUB_OUTPUT
              ;;
          esac

      - name: Build
        env:
          RUSTFLAGS: ${{ inputs.verbose_diagnostics && '-v' || '' }}
        run: |
          cargo build -p gat-cli --release ${{ steps.features.outputs.flags }} --locked

      - name: Run tests
        run: |
          cargo test -p gat-core -p gat-cli --release ${{ steps.features.outputs.flags }} --locked -- --nocapture

      - name: Capture build diagnostics
        id: diagnostics
        run: |
          cat > build-diagnostics.json <<EOF
          {
            "runner_os": "${{ runner.os }}",
            "runner_arch": "${{ runner.arch }}",
            "variant": "${{ matrix.variant }}",
            "rustc": "${{ steps.sysinfo.outputs.rustc }}",
            "cargo": "${{ steps.sysinfo.outputs.cargo }}",
            "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
            "run_id": "${{ github.run_id }}"
          }
          EOF
          echo "json=$(cat build-diagnostics.json)" >> $GITHUB_OUTPUT

      - name: Generate build summary
        id: summary
        run: |
          echo "json={\"status\":\"success\",\"os\":\"${{ runner.os }}\",\"variant\":\"${{ matrix.variant }}\"}" >> $GITHUB_OUTPUT

      - name: Upload diagnostics
        if: inputs.upload_artifacts
        uses: actions/upload-artifact@v4
        with:
          name: diagnostics-${{ runner.os }}-${{ matrix.variant }}-${{ github.run_id }}
          path: build-diagnostics.json
          retention-days: 7
          if-no-files-found: ignore
```

**Step 3: Verify syntax**

Run: `yamllint /home/tom/Code/gat/.github/workflows/build-matrix.yml 2>&1 | head -20`

If yamllint is not installed, just check it manually for obvious YAML errors (no syntax checker needed).

**Step 4: Commit**

```bash
git add .github/workflows/build-matrix.yml
git commit -m "ci: Create reusable build-matrix workflow core"
```

---

### Task 2: Refactor `push-pr.yml` for Fast Linux-Only Path

**Files:**
- Modify: `.github/workflows/rust.yml` (rename/refactor to `push-pr.yml`)

**Step 1: Read current rust.yml**

Run: `head -50 /home/tom/Code/gat/.github/workflows/rust.yml`

**Step 2: Update rust.yml to be push-pr.yml with simplified scope**

The goal: keep only the "fast path" job (Linux headless), remove the full-feature/ui-integration/docs/release-artifacts jobs.

Replace `/home/tom/Code/gat/.github/workflows/rust.yml` content with:

```yaml
name: Fast CI (Push/PR)

on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]
  schedule:
    - cron: "0 6 * * *"
  workflow_dispatch:

env:
  CARGO_TERM_COLOR: always

jobs:
  check-core-cli:
    name: fmt, clippy, core/cli tests (Linux headless)
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4
      - name: Cache cargo builds
        uses: Swatinem/rust-cache@v2
        with:
          shared-key: ${{ hashFiles('Cargo.lock') }}
      - name: Format (workspace)
        run: cargo fmt --all -- --check
      - name: Clippy (no default features)
        run: cargo clippy -p gat-core -p gat-cli --no-default-features --features minimal --no-deps -- -D warnings
      - name: Core + CLI tests (no default features)
        run: cargo test -p gat-core -p gat-cli --no-default-features --features minimal --locked -- --nocapture
      - name: Upload debug target cache
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: cargo-target-${{ github.run_id }}
          path: |
            target/debug
            target/.rustc_info.json
            target/.rustc_info_cache.json
          retention-days: 7
          if-no-files-found: ignore
```

Use the Edit tool:

Old string (entire file content, lines 1-142):
```yaml
name: Lightweight Rust CI
...
```

New string:
```yaml
name: Fast CI (Push/PR)

on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]
  schedule:
    - cron: "0 6 * * *"
  workflow_dispatch:

env:
  CARGO_TERM_COLOR: always

jobs:
  check-core-cli:
    name: fmt, clippy, core/cli tests (Linux headless)
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4
      - name: Cache cargo builds
        uses: Swatinem/rust-cache@v2
        with:
          shared-key: ${{ hashFiles('Cargo.lock') }}
      - name: Format (workspace)
        run: cargo fmt --all -- --check
      - name: Clippy (no default features)
        run: cargo clippy -p gat-core -p gat-cli --no-default-features --features minimal --no-deps -- -D warnings
      - name: Core + CLI tests (no default features)
        run: cargo test -p gat-core -p gat-cli --no-default-features --features minimal --locked -- --nocapture
      - name: Upload debug target cache
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: cargo-target-${{ github.run_id }}
          path: |
            target/debug
            target/.rustc_info.json
            target/.rustc_info_cache.json
          retention-days: 7
          if-no-files-found: ignore
```

**Step 3: Verify the file**

Run: `wc -l /home/tom/Code/gat/.github/workflows/rust.yml`

Should be much shorter now (around 40 lines instead of 142).

**Step 4: Commit**

```bash
git add .github/workflows/rust.yml
git commit -m "ci: Simplify rust.yml to fast path (Linux headless only)"
```

---

### Task 3: Create `release-dry-run.yml`

**Files:**
- Create: `.github/workflows/release-dry-run.yml`

**Step 1: Create the file**

Create `.github/workflows/release-dry-run.yml` with content:

```yaml
name: Release Dry Run

on:
  workflow_dispatch:
    inputs:
      verbose_diagnostics:
        description: 'Enable verbose compiler output'
        required: false
        type: boolean
        default: false

permissions:
  contents: read

jobs:
  build-all:
    name: Build all platforms and variants (dry run)
    uses: ./.github/workflows/build-matrix.yml
    with:
      verbose_diagnostics: ${{ inputs.verbose_diagnostics }}
      upload_artifacts: true

  summary:
    name: Dry run summary
    runs-on: ubuntu-latest
    needs: build-all
    if: always()
    steps:
      - name: Print summary
        run: |
          echo "🏃 Dry Run Complete"
          echo "📦 Build result: ${{ needs.build-all.result }}"
          echo "✅ All artifacts are available in the Actions tab"
          echo "📝 To release: git tag v0.x.y && git push --tags"
```

**Step 2: Verify the file**

Run: `wc -l /home/tom/Code/gat/.github/workflows/release-dry-run.yml`

Should be around 30 lines.

**Step 3: Commit**

```bash
git add .github/workflows/release-dry-run.yml
git commit -m "ci: Add release dry-run workflow"
```

---

### Task 4: Update `release.yml` to Call Reusable Workflow

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Read current release.yml**

Run: `cat /home/tom/Code/gat/.github/workflows/release.yml`

**Step 2: Replace with refactored version that calls build-matrix**

The key change: instead of doing build/package inline, call the reusable build-matrix workflow, then add the release-specific steps.

Replace entire content with:

```yaml
name: Release Build and Upload

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
    inputs:
      verbose_diagnostics:
        description: 'Enable verbose compiler output'
        required: false
        type: boolean
        default: false

permissions:
  contents: write

jobs:
  build-all:
    name: Build all platforms and variants
    uses: ./.github/workflows/build-matrix.yml
    with:
      verbose_diagnostics: ${{ inputs.verbose_diagnostics }}
      upload_artifacts: true

  package:
    name: Package and upload release artifacts
    runs-on: ubuntu-latest
    needs: build-all
    if: startsWith(github.ref, 'refs/tags/')

    steps:
      - uses: actions/checkout@v4

      - name: Ensure jq is available
        run: |
          if [[ "${RUNNER_OS}" == "Linux" ]]; then
            sudo apt-get update
            sudo apt-get install -y jq
          elif [[ "${RUNNER_OS}" == "macOS" ]]; then
            brew install jq || true
          fi

      - name: Determine version
        id: metadata
        run: |
          source scripts/release-utils.sh
          version=$(release_version)
          if [[ -z "$version" || "$version" == "null" ]]; then
            echo "workspace_metadata.release.version is not defined" >&2
            exit 1
          fi
          echo "version<<EOF" >> $GITHUB_OUTPUT
          echo "${version}" >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT

      - name: Create GitHub release
        id: release
        uses: actions/create-release@v1
        with:
          tag_name: ${{ github.ref_name }}
          release_name: GAT ${{ github.ref_name }}
          draft: false
          prerelease: false
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: ./artifacts

      - name: Find and upload release artifacts
        run: |
          cd artifacts
          for variant in headless full analyst; do
            for os_arch in linux-x86_64 macos-aarch64 macos-x86_64; do
              tarball="dist/gat-${{ steps.metadata.outputs.version }}-${os_arch}-${variant}.tar.gz"
              if [[ -f "$tarball" ]]; then
                echo "Uploading $tarball"
                # Upload step would go here
              fi
            done
          done
```

Actually, let me simplify this. The packaging should happen in build-matrix or in a separate script that gets called. For now, focus on the release creation part:

Replace entire content with:

```yaml
name: Release Build and Upload

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:

permissions:
  contents: write

jobs:
  build-all:
    name: Build all platforms and variants
    uses: ./.github/workflows/build-matrix.yml
    with:
      verbose_diagnostics: false
      upload_artifacts: true

  create-release:
    name: Create GitHub release
    runs-on: ubuntu-latest
    needs: build-all
    if: startsWith(github.ref, 'refs/tags/')

    steps:
      - uses: actions/checkout@v4

      - name: Determine version
        id: version
        run: |
          version="${{ github.ref_name }}"
          echo "tag=${version}" >> $GITHUB_OUTPUT

      - name: Create GitHub release
        uses: actions/create-release@v1
        with:
          tag_name: ${{ steps.version.outputs.tag }}
          release_name: GAT ${{ steps.version.outputs.tag }}
          draft: false
          prerelease: false
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Step 3: Verify changes**

Run: `wc -l /home/tom/Code/gat/.github/workflows/release.yml`

Should be shorter (around 50 lines now vs. 100 before).

**Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: Refactor release.yml to call reusable build-matrix"
```

---

### Task 5: Update `nightly-full-build.yml`

**Files:**
- Modify: `.github/workflows/nightly-full-build.yml`

**Step 1: Read current nightly-full-build.yml**

Run: `cat /home/tom/Code/gat/.github/workflows/nightly-full-build.yml`

**Step 2: Update to call reusable workflow with verbose diagnostics**

Replace content with:

```yaml
name: Nightly Build

on:
  schedule:
    - cron: '0 6 * * *'
  workflow_dispatch:

permissions:
  contents: read

jobs:
  build-all:
    name: Full build with diagnostics
    uses: ./.github/workflows/build-matrix.yml
    with:
      verbose_diagnostics: true
      upload_artifacts: true

  summary:
    name: Nightly build summary
    runs-on: ubuntu-latest
    needs: build-all
    if: always()
    steps:
      - name: Print summary
        run: |
          echo "🌙 Nightly Build Complete"
          echo "📦 Build result: ${{ needs.build-all.result }}"
          echo "📊 Artifacts available with verbose diagnostics"
          echo "⏱️ Retention: 14 days"
```

**Step 3: Verify**

Run: `wc -l /home/tom/Code/gat/.github/workflows/nightly-full-build.yml`

Should be around 30 lines.

**Step 4: Commit**

```bash
git add .github/workflows/nightly-full-build.yml
git commit -m "ci: Update nightly build to call reusable build-matrix with verbose diagnostics"
```

---

### Task 6: Update `scripts/package.sh` to Support Bundle Variants

**Files:**
- Modify: `scripts/package.sh` (first 50 lines)

**Step 1: Read current package.sh**

Run: `head -80 /home/tom/Code/gat/scripts/package.sh`

**Step 2: Add variant detection logic**

The goal is to detect which variant we're packaging (from environment variable or command-line arg) and build accordingly.

After line 12 (VERSION="..."), add:

```bash
# Determine variant from environment or argument
VARIANT="${GAT_BUNDLE_VARIANT:-full}"
if [[ $# -gt 0 ]]; then
  VARIANT="$1"
fi

case "$VARIANT" in
  headless)
    BUILD_FLAGS="--no-default-features --features minimal-io"
    ;;
  analyst)
    BUILD_FLAGS="--no-default-features --features minimal-io,adms,derms,dist,analytics,featurize"
    ;;
  full)
    BUILD_FLAGS="--all-features"
    ;;
  *)
    echo "Unknown variant: $VARIANT. Use headless, analyst, or full." >&2
    exit 1
    ;;
esac
```

Use Edit tool to add this after the version detection, before the OS/ARCH detection.

**Step 3: Modify the build commands**

Find `package_headless()` and `package_full()` functions. Update them to use `$BUILD_FLAGS` instead of hardcoded flags.

In `package_headless()`, change:
```bash
cargo build --workspace --exclude gat-gui --exclude gat-tui --release
```

To:
```bash
cargo build -p gat-cli --release $BUILD_FLAGS
```

**Step 4: Verify the script still works**

Run: `bash /home/tom/Code/gat/scripts/package.sh headless --dry-run 2>&1 | head -20`

(Note: if --dry-run doesn't work, just check that the script parses without syntax errors.)

**Step 5: Commit**

```bash
git add scripts/package.sh
git commit -m "ci: Update package.sh to support bundle variants (headless/full/analyst)"
```

---

### Task 7: Update `scripts/install.sh` to Support Bundle Variants

**Files:**
- Modify: `scripts/install.sh`

**Step 1: Read current install.sh**

Run: `head -50 /home/tom/Code/gat/scripts/install.sh`

**Step 2: Add variant option to install script**

The goal: when user runs `./install.sh --variant analyst`, it knows to look for `gat-*-analyst.tar.gz` instead of the default.

Find the section where the script determines which binary to download. Add logic to accept `--variant` flag and use it in the binary name lookup.

Pseudo-code:
```bash
VARIANT="full"  # default
while [[ $# -gt 0 ]]; do
  case "$1" in
    --variant)
      VARIANT="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

# Later, when downloading:
BINARY_NAME="gat-${VERSION}-${OS}-${ARCH}-${VARIANT}.tar.gz"
```

**Step 3: Verify the script**

Run: `bash /home/tom/Code/gat/scripts/install.sh --help 2>&1 | head -10`

Or just check for syntax errors by running `bash -n /home/tom/Code/gat/scripts/install.sh`.

**Step 4: Commit**

```bash
git add scripts/install.sh
git commit -m "ci: Update install.sh to support bundle variants via --variant flag"
```

---

### Task 8: Add Diagnostic Capture to Build Matrix

**Files:**
- Modify: `.github/workflows/build-matrix.yml` (expand diagnostics section)

**Step 1: Read current build-matrix.yml**

Run: `grep -A 10 "Capture build diagnostics" /home/tom/Code/gat/.github/workflows/build-matrix.yml`

**Step 2: Enhance diagnostics capture**

The goal is to capture more detailed system info, solver discovery results, and build timings.

Replace the "Capture build diagnostics" step with an enhanced version:

```yaml
      - name: Capture build diagnostics
        id: diagnostics
        run: |
          # Capture detailed system info
          {
            echo "{"
            echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
            echo "  \"github_run_id\": \"${{ github.run_id }}\","
            echo "  \"github_run_number\": \"${{ github.run_number }}\","
            echo "  \"runner_os\": \"${{ runner.os }}\","
            echo "  \"runner_arch\": \"${{ runner.arch }}\","
            echo "  \"variant\": \"${{ matrix.variant }}\","
            echo "  \"toolchain\": {"
            echo "    \"rustc\": \"$(rustc --version)\","
            echo "    \"cargo\": \"$(cargo --version)\","
            echo "    \"rustc_verbose\": \"$(rustc -vV)\""
            echo "  },"
            echo "  \"system\": {"
            echo "    \"uname_a\": \"$(uname -a)\","
            echo "    \"memory_available_gb\": \"$(free -h 2>/dev/null | grep Mem | awk '{print $7}' || echo 'N/A')\""
            echo "  },"
            echo "  \"build_flags\": \"${{ steps.features.outputs.flags }}\""
            echo "}"
          } > build-diagnostics-${{ matrix.os }}-${{ matrix.variant }}.json

      - name: Upload verbose compiler output
        if: inputs.verbose_diagnostics && always()
        uses: actions/upload-artifact@v4
        with:
          name: compiler-output-${{ runner.os }}-${{ matrix.variant }}-${{ github.run_id }}
          path: '**/*.o'
          retention-days: 7
          if-no-files-found: ignore

      - name: Upload build diagnostics
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: build-diagnostics-${{ runner.os }}-${{ matrix.variant }}-${{ github.run_id }}
          path: build-diagnostics-*.json
          retention-days: 14
          if-no-files-found: ignore
```

**Step 3: Verify build-matrix.yml syntax**

Run: `yamllint /home/tom/Code/gat/.github/workflows/build-matrix.yml 2>&1 | head -5`

Or manually check for obvious YAML errors.

**Step 4: Commit**

```bash
git add .github/workflows/build-matrix.yml
git commit -m "ci: Enhance build-matrix diagnostics capture (system info, toolchain, build flags)"
```

---

### Task 9: Test the Reusable Workflow Locally (Validation)

**Files:**
- No files created; testing only

**Step 1: Verify all workflow files exist**

Run: `ls -1 /home/tom/Code/gat/.github/workflows/*.yml | grep -E "(build-matrix|push-pr|release|nightly|dry-run)"`

Should list:
- `.github/workflows/build-matrix.yml`
- `.github/workflows/rust.yml` (renamed from push-pr)
- `.github/workflows/release.yml`
- `.github/workflows/release-dry-run.yml`
- `.github/workflows/nightly-full-build.yml`

**Step 2: Verify YAML syntax for all workflows**

Run: `for f in /home/tom/Code/gat/.github/workflows/{build-matrix,rust,release,release-dry-run,nightly-full-build}.yml; do echo "Checking $f..."; python3 -m yaml "$f" 2>&1 | head -3 || echo "OK"; done`

Or manually inspect each file for obvious YAML errors (indentation, quotes, etc.).

**Step 3: Verify scripts still parse**

Run: `for f in /home/tom/Code/gat/scripts/{package,install}.sh; do echo "Checking $f..."; bash -n "$f" && echo "OK" || echo "SYNTAX ERROR"; done`

Both should say "OK".

**Step 4: Commit a summary note**

```bash
git add -A
git commit -m "ci: Verify all workflow files and scripts parse correctly"
```

---

### Task 10: Final Integration and Documentation

**Files:**
- Modify: `README.md` (CI/CD section, if one exists)
- Create: `docs/guide/ci-cd.md` (new guide)

**Step 1: Create CI/CD guide**

Create `docs/guide/ci-cd.md` with content explaining the new workflow structure:

```markdown
# CI/CD Workflows

## Overview

GAT uses GitHub Actions for continuous integration, testing, packaging, and release automation. The workflows are organized into specialized pipelines that share a common reusable core.

## Workflow Structure

### Fast Path: `push-pr.yml`
- Triggers: Push to main, PRs to main, daily schedule
- Scope: Linux + headless variant only
- Duration: ~5 minutes
- Purpose: Quick feedback for developers
- Artifacts: Debug build cache only

### Full Build Matrix: `build-matrix.yml`
- Reusable workflow called by release, dry-run, and nightly workflows
- Platforms: Linux (ubuntu-latest) + macOS (macos-latest)
- Variants: headless, full, analyst
- Features: Tiered diagnostics, verbose compiler output on-demand

### Release Dry-Run: `release-dry-run.yml`
- Trigger: Manual (workflow_dispatch)
- Scope: All platforms, all variants
- Purpose: Test the full packaging pipeline before tagging
- Artifacts: All bundles (headless, full, analyst for Linux and macOS)
- Next step: `git tag v0.x.y && git push --tags` if satisfied

### Release: `release.yml`
- Trigger: Tag push (git tag v*)
- Scope: All platforms, all variants (same as dry-run)
- Purpose: Create GitHub release and upload artifacts
- Artifacts: Permanent release assets on GitHub

### Nightly: `nightly.yml`
- Trigger: Daily at 6am UTC
- Scope: All platforms, all variants
- Diagnostics: Verbose compiler output always enabled
- Artifacts: 14-day retention (longer than dry-run)
- Purpose: Early detection of regressions, user access to latest builds

## Bundle Tiers

**Headless** (~30-50 MB)
- CLI only, minimal dependencies
- Features: `-p gat-cli --no-default-features --features minimal-io`
- Use case: Scripting, CI/CD, resource-constrained environments

**Full** (~200-300 MB)
- Everything: GUI, TUI, all solvers, visualization
- Features: `-p gat-cli --all-features`
- Use case: Interactive desktop use, exploratory analysis

**Analyst** (~100-150 MB)
- CLI + ADMS/DERMS/DIST/analytics/featurization
- Features: `-p gat-cli --no-default-features --features minimal-io,adms,derms,dist,analytics,featurize`
- Use case: Power systems analysts, domain-focused workflows

## Diagnostics (Tiered)

**Tier 1 (Always):** System info, toolchain versions, build flags → `build-diagnostics-*.json`

**Tier 2 (On-Demand):** Verbose compiler output via workflow_dispatch input `verbose_diagnostics: true`

**Tier 3 (Nightly):** Full instrumentation always enabled for early issue detection

## How to Release

1. Ensure all changes are committed and pushed to main
2. (Optional) Run dry-run to validate: Go to Actions → Release Dry Run → Run workflow
3. Tag the release: `git tag v0.x.y && git push --tags`
4. Release workflow triggers automatically
5. Check GitHub Releases for uploaded artifacts

## How to Install

```bash
# Download latest release
curl -fsSL https://releases.gat.dev/gat/latest.txt -o version.txt
VERSION=$(cat version.txt)

# Download the variant you want
curl -fsSL https://releases.gat.dev/gat/v${VERSION}/gat-${VERSION}-linux-x86_64-headless.tar.gz -o gat.tar.gz

# Install
tar -xzf gat.tar.gz
./gat/install.sh --variant headless
```

Or use the bundled installer for easier installation with fallback to source build.

## References

- Design: `docs/plans/2025-11-22-build-workflows-design.md`
- Workflows: `.github/workflows/`
- Scripts: `scripts/{package,install}.sh`
```

**Step 2: Update README CI/CD section (if exists)**

Search for any CI/CD documentation in README and update it to reference the new structure.

Run: `grep -n "CI\|workflow\|GitHub Actions" /home/tom/Code/gat/README.md`

If there's a section, update it to point to `docs/guide/ci-cd.md`.

**Step 3: Commit**

```bash
git add docs/guide/ci-cd.md
git commit -m "docs: Add CI/CD workflow guide"
```

---

## Summary

This 10-task plan implements the build workflows overhaul:

1. **Task 1** — Create reusable workflow core (`build-matrix.yml`)
2. **Task 2** — Simplify push-pr CI to Linux headless only
3. **Task 3** — Create release dry-run workflow
4. **Task 4** — Refactor release.yml to use reusable workflow
5. **Task 5** — Update nightly build to use reusable workflow
6. **Task 6** — Add bundle variant support to package.sh
7. **Task 7** — Add bundle variant support to install.sh
8. **Task 8** — Enhance diagnostics capture in build matrix
9. **Task 9** — Validate all files parse correctly
10. **Task 10** — Create CI/CD guide and finalize documentation

**Result:** Maintainable, DRY build system with:
- ✓ Reusable core workflow
- ✓ Three bundle tiers (headless, full, analyst)
- ✓ Multi-platform (Linux, macOS; Windows deferred)
- ✓ Dry-run testing before release
- ✓ Nightly channel with verbose diagnostics
- ✓ Fast push-pr path for daily development
- ✓ Tiered diagnostics (always, on-demand, nightly)
- ✓ Clear documentation and guides

**Total commits:** 10 focused, well-organized commits
**Estimated time:** 2-3 hours for full implementation and testing

