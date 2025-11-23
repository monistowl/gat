+++
title = "CI/CD Workflows"
description = "CI/CD Workflows"
weight = 32
+++

# CI/CD Workflows

## Overview

GAT uses GitHub Actions for continuous integration, testing, packaging, and release automation. The workflows follow a **simplified manual release process** with comprehensive diagnostics.

## Workflow Structure

### 1. Release Verification (Quick Smoke Test)
**File:** `.github/workflows/release-verification.yml`

- **Triggers:** Automatic on push to `main` or `staging`
- **Scope:** Ubuntu + macOS, headless variant only
- **Duration:** ~5 minutes
- **Purpose:** Fast smoke test that packaging works
- **Artifacts:** Test packages with 5-day retention

### 2. Staging Diagnostics (Comprehensive Testing)
**File:** `.github/workflows/staging-diagnostics.yml`

- **Trigger:** Manual only (workflow_dispatch)
- **Scope:** All platforms, all variants, all feature combinations
- **Purpose:** Comprehensive "what's broken where" testing before release
- **Components:**
  - Feature matrix tests (minimal, full-io, viz, all-backends)
  - Subcrate tests (gat-core, gat-io, gat-algo, gat-ts, gat-viz)
  - Full build matrix (ubuntu/macos × headless/analyst/full)
- **Artifacts:** Build diagnostics JSON files with 14-day retention
- **Output:** Comprehensive diagnostic report with actionable next steps

### 3. Manual Release Build
**File:** `.github/workflows/manual-release.yml`

- **Trigger:** Manual only (workflow_dispatch)
- **Scope:** All platforms, all requested variants
- **Purpose:** Build release packages for distribution
- **Duration:** ~15-20 minutes for all variants
- **Artifacts:** Release tarballs with 30-day retention
- **Output:** Build summary with next steps

### 4. Build Matrix (Reusable)
**File:** `.github/workflows/build-matrix.yml`

- **Type:** Reusable workflow called by staging-diagnostics
- **Platforms:** Linux (ubuntu-latest) + macOS (macos-latest)
- **Variants:** headless, full, analyst
- **Features:** Tiered diagnostics, verbose compiler output on-demand

### 5. Feature Testing
**Files:** `.github/workflows/cli-feature-matrix.yml`, `.github/workflows/feature-subcrate-tests.yml`

- **Triggers:** On PR/push to main (cli-feature-matrix), manual (feature-subcrate-tests)
- **Purpose:** Test different feature combinations and subcrates
- **Included in:** staging-diagnostics workflow

---

## Bundle Variants

### Headless (~30-50 MB)
- CLI only, minimal dependencies
- Features: `--no-default-features --features minimal-io`
- Use case: Scripting, CI/CD, resource-constrained environments

### Analyst (~100-150 MB)
- CLI + ADMS/DERMS/DIST/analytics/featurization
- Features: `--no-default-features --features minimal-io,adms,derms,dist,analytics,featurize`
- Use case: Power systems analysts, domain-focused workflows

### Full (~200-300 MB)
- Everything: GUI, TUI, all solvers, visualization
- Features: `--all-features`
- Use case: Interactive desktop use, exploratory analysis

---

## Release Process

The release process follows a **manual staging-to-main workflow**:

### Step 1: Develop on Staging

```bash
git checkout staging
# Make your changes
git add -A
git commit -m "Add new feature"
git push origin staging
```

### Step 2: Run Staging Diagnostics

1. Go to **Actions → Staging Diagnostics** in GitHub
2. Click **Run workflow** on the `staging` branch
3. Enable all test suites (default)
4. Review the comprehensive diagnostic report

**If diagnostics fail:**
- Fix issues on staging
- Re-run diagnostics
- Repeat until all pass

### Step 3: Build Release Packages

1. Go to **Actions → Manual Release Build** in GitHub
2. Click **Run workflow** on the `staging` branch
3. Keep default variants (`headless analyst full`) or specify subset
4. Download and test the packages locally

### Step 4: Test Packages Locally

```bash
# Download artifact from workflow run
# Extract a package
tar -xzf gat-0.X.Y-linux-x86_64-headless.tar.gz
cd gat-0.X.Y-linux-x86_64-headless

# Test installation
./install.sh --prefix /tmp/gat-test

# Verify it works
/tmp/gat-test/bin/gat-cli --version
```

### Step 5: Merge to Main

```bash
git checkout main
git merge staging --no-ff -m "Release v0.X.Y: [summary]"
git push origin main
```

### Step 6: Tag the Release

```bash
git tag -a v0.X.Y -m "Release v0.X.Y: [summary]"
git push origin v0.X.Y
```

### Step 7: Create GitHub Release (Optional)

Manually create a GitHub release and upload packages if desired. Otherwise, users can install from artifacts or build from source.

---

## How to Install

### From GitHub Artifacts (Latest Builds)

```bash
# Download artifact from a Manual Release Build workflow run
# Extract and install
tar -xzf gat-0.X.Y-linux-x86_64-headless.tar.gz
cd gat-0.X.Y-linux-x86_64-headless
./install.sh --variant headless
```

### From GitHub Release (If Published)

```bash
# Download specific release
VERSION="v0.2.0"
curl -fsSL "https://github.com/monistowl/gat/releases/download/${VERSION}/gat-${VERSION#v}-linux-x86_64-headless.tar.gz" -o gat.tar.gz

# Install
tar -xzf gat.tar.gz
cd gat-*
./install.sh --variant headless
```

### From Source

```bash
# Clone the repository
git clone https://github.com/monistowl/gat.git
cd gat

# Install with your preferred variant
./scripts/install.sh --variant analyst
```

The installer automatically detects your platform and architecture, downloads binaries if available, or falls back to building from source.

---

## Packaging Individual Bundles

To package a specific bundle variant (e.g., for local testing):

```bash
# Package headless variant
./scripts/package.sh headless

# Package analyst variant
./scripts/package.sh analyst

# Package full variant
./scripts/package.sh full
```

The packaged tarball will be in `dist/`.

---

## Customizing Installations

### Using Environment Variables

```bash
# Install to custom prefix
GAT_PREFIX=/opt/gat ./scripts/install.sh --variant full

# Use specific version
GAT_VERSION=v0.2.0 ./scripts/install.sh --analyst
```

### Installation Methods (Priority)

1. **Binary download** (if available for your platform/version)
2. **Source build** (automatic fallback if binary not available)

---

## Development Workflow

### For Regular Development

```bash
# Work on staging
git checkout staging
# Make changes, commit, push
git push origin staging

# Release Verification runs automatically on push
# Check Actions tab for quick smoke test results
```

### For Pre-Release Testing

```bash
# Run comprehensive diagnostics before release
# Go to Actions → Staging Diagnostics → Run workflow (on staging)

# Review diagnostic report
# Fix any failures
# Re-run until all pass
```

### For Release Preparation

```bash
# Build release packages
# Go to Actions → Manual Release Build → Run workflow (on staging)

# Download and test packages locally
# If satisfied, merge to main and tag
git checkout main
git merge staging
git tag -a v0.X.Y -m "Release v0.X.Y"
git push origin main --tags
```

---

## Diagnostics

### Tier 1 (Always)
- System info, toolchain versions, build flags
- Output: `build-diagnostics-*.json`
- Available in all workflows

### Tier 2 (On-Demand)
- Verbose compiler output
- Enable via `verbose_diagnostics: true` in workflow_dispatch
- Available in staging-diagnostics and build-matrix

### Tier 3 (Comprehensive)
- Feature matrix tests
- Subcrate tests
- Full build matrix
- Available in staging-diagnostics workflow

---

## Troubleshooting

### Diagnostics Failed

1. Review failed job logs
2. Download diagnostic artifacts
3. Reproduce locally:
   ```bash
   cargo test -p gat-cli --no-default-features --features minimal,full-io
   ```
4. Fix issues on staging
5. Re-run diagnostics

### Package Build Failed

1. Check workflow logs for missing dependencies
2. Test locally: `./scripts/package.sh headless`
3. Update workflow if needed
4. Re-run manual release build

### Installation Failed

1. Verify tarball structure:
   ```bash
   tar -tzf gat-*.tar.gz
   ```
2. Expected structure:
   ```
   gat-VERSION-OS-ARCH-VARIANT/
   ├── bin/
   │   ├── gat-cli
   │   └── gat
   ├── README.md
   ├── LICENSE.txt
   └── install.sh
   ```
3. Fix package.sh if structure is wrong
4. Re-run manual release build

---

## Key Differences from Auto-Release

This simplified workflow removes all auto-tagging and auto-release features:

- ❌ No automatic tagging on version bumps
- ❌ No automatic GitHub release creation
- ❌ No automatic package uploads to releases

All release-critical steps are **manual** and **explicit**:

- ✅ Manual staging diagnostics
- ✅ Manual release package builds
- ✅ Manual merge to main
- ✅ Manual git tag creation
- ✅ Manual GitHub release (if desired)

This prevents surprises and gives developers full control over when releases happen.

---

## References

- **Release Process:** `RELEASE_PROCESS.md` (comprehensive guide)
- **Workflow Files:** `.github/workflows/`
- **Packaging Scripts:** `scripts/{package,install}.sh`
