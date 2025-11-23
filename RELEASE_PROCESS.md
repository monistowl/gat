# Release Process

This document describes the simplified manual release workflow for GAT.

## Overview

The release process follows a **manual staging-to-main workflow**:

1. **Develop on staging** - All development and testing happens here
2. **Run diagnostics** - Verify everything works across all features and platforms
3. **Build packages** - Create release artifacts and test them
4. **Merge to main** - When satisfied, merge staging → main
5. **Tag release** - Manually tag the release on main
6. **Distribute** - Packages are available via GitHub artifacts or releases

## Branch Strategy

### `staging` - Development and Pre-Release Testing
- **Purpose**: Active development and release candidate preparation
- **Stability**: Should pass all tests before merging to main
- **Who commits**: Developers during active work
- **Content**: All code, tests, and documentation

### `main` - Production Releases
- **Purpose**: Stable, released versions only
- **Stability**: Critical - only merged from staging after full verification
- **Who merges**: After staging diagnostics pass and packages are tested
- **Content**: Tagged releases only

### `experimental` (optional)
- **Purpose**: Rapid iteration and experimental features
- **Stability**: Unstable
- **Who commits**: Developers trying new ideas
- **Content**: Merge to staging when features are stable

## GitHub Actions Workflows

### 1. Release Verification (Quick Smoke Test)
**File:** `.github/workflows/release-verification.yml`

**Trigger:** Automatically runs on push to `main` or `staging`

**Purpose:** Fast smoke test that packaging works

**What it does:**
- Builds headless variant only (for speed)
- Tests on ubuntu-latest and macos-latest
- Uploads artifacts for quick verification

**When to use:** Runs automatically - no action needed

---

### 2. Staging Diagnostics
**File:** `.github/workflows/staging-diagnostics.yml`

**Trigger:** Manual only (workflow_dispatch)

**Purpose:** Comprehensive testing to answer "what's broken where?"

**What it does:**
- Runs CLI feature matrix tests (minimal, full-io, viz, all-backends)
- Runs subcrate tests (gat-core, gat-io, gat-algo, gat-ts, gat-viz)
- Runs full build matrix (ubuntu/macos × headless/analyst/full)
- Generates comprehensive diagnostic report with actionable next steps
- Uploads build diagnostics JSON files for analysis

**When to use:** Before merging staging → main

**How to run:**
```bash
# From GitHub Actions UI:
# Actions → Staging Diagnostics → Run workflow
# - Enable verbose diagnostics if needed
# - Select which test suites to run
# - Click "Run workflow"
```

---

### 3. Manual Release Build
**File:** `.github/workflows/manual-release.yml`

**Trigger:** Manual only (workflow_dispatch)

**Purpose:** Build release packages for all platforms and variants

**What it does:**
- Builds all requested variants (headless, analyst, full)
- Tests on ubuntu-latest and macos-latest
- Uploads .tar.gz packages with 30-day retention
- Generates summary with next steps

**When to use:** After staging diagnostics pass

**How to run:**
```bash
# From GitHub Actions UI:
# Actions → Manual Release Build → Run workflow
# - Optionally specify variants (default: all three)
# - Click "Run workflow"
```

---

### 4. Build Matrix (Reusable)
**File:** `.github/workflows/build-matrix.yml`

**Trigger:** Called by other workflows (workflow_call)

**Purpose:** Reusable build matrix with diagnostics

**What it does:**
- Builds all os/variant combinations
- Captures system info and build diagnostics
- Runs tests for each configuration
- Uploads diagnostic JSON files

**When to use:** Used internally by staging-diagnostics.yml

---

### 4. Create GitHub Release (Automated)
**File:** `.github/workflows/create-release.yml`

**Trigger:** Automatically on push of tags matching `v*.*.*`

**Purpose:** Automatically create GitHub release when you tag

**What it does:**
- Builds all variants (headless, analyst, full) for Linux and macOS
- Generates release notes from git history
- Uploads all packages to the GitHub release
- Creates release with install instructions and changelog
- 90-day artifact retention for release packages

**When to use:** Triggered automatically when you push a version tag (see Step 6 below)

---

### 5. Deploy Website
**File:** `.github/workflows/deploy-website.yml`

**Trigger:** Automatically on push to `main` (website changes only)

**Purpose:** Deploy website to GitHub Pages

**What it does:**
- Builds Zola static site with integrated documentation
- Deploys to gh-pages branch
- Updates live website at https://monistowl.github.io/gat/

**When to use:** Runs automatically on main; can manually trigger for testing

---

### 6. CLI Feature Matrix & Subcrate Tests
**Files:**
- `.github/workflows/cli-feature-matrix.yml`
- `.github/workflows/feature-subcrate-tests.yml`

**Trigger:** On PR/push to main (cli-feature-matrix), manual (feature-subcrate-tests)

**Purpose:** Test different feature combinations

**What they do:**
- Test various feature flags in isolation
- Test individual subcrates with different feature sets

**When to use:** Automatically on PRs; included in staging-diagnostics

---

## Step-by-Step Release Process

### Step 1: Prepare Staging Branch

Make sure all changes are committed and tests pass locally:

```bash
# Make sure you're on staging
git checkout staging
git status

# Run local tests
cargo test --workspace
cargo clippy --workspace
```

### Step 2: Run Staging Diagnostics

Run the comprehensive diagnostic workflow:

1. Go to **Actions → Staging Diagnostics** in GitHub
2. Click **Run workflow**
3. Enable all test suites (default)
4. Optionally enable verbose diagnostics if investigating issues
5. Click **Run workflow**

**Review the results:**
- Check the workflow summary for pass/fail status
- If any tests fail:
  - Download diagnostic artifacts
  - Fix issues on staging
  - Re-run diagnostics
- If all pass: proceed to next step

### Step 3: Build Release Packages

Once diagnostics pass, build the release packages:

1. Go to **Actions → Manual Release Build** in GitHub
2. Click **Run workflow**
3. Keep default variants (`headless analyst full`) or specify subset
4. Click **Run workflow**

**Review and test packages:**
- Download the artifacts from the workflow run
- Test installation locally:

```bash
# Extract a package
tar -xzf gat-0.X.Y-linux-x86_64-headless.tar.gz
cd gat-0.X.Y-linux-x86_64-headless

# Test installation
./install.sh --prefix /tmp/gat-test

# Verify it works
/tmp/gat-test/bin/gat-cli --version
```

### Step 4: Update Version and Changelog

Update version numbers if not already done:

```bash
# Edit Cargo.toml files to bump version
vim Cargo.toml

# Update changelog or release notes
vim CHANGELOG.md

# Commit version bump
git add -A
git commit -m "chore: Bump version to 0.X.Y for release"
git push origin staging
```

### Step 5: Merge to Main

When satisfied with testing and packages:

```bash
# Merge staging to main
git checkout main
git merge staging --no-ff -m "Release v0.X.Y: [summary]"

# Push to main
git push origin main
```

### Step 6: Tag the Release

Tag the release on main. When you push the tag, it will automatically trigger the GitHub release workflow:

```bash
# Create annotated tag
git tag -a v0.X.Y -m "$(cat <<'EOF'
Release v0.X.Y: [Summary]

## Major Features
- Feature 1
- Feature 2

## Enhancements
- Enhancement 1
- Enhancement 2

## Bug Fixes
- Fix 1
- Fix 2

## Breaking Changes
[If any]

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"

# Push the tag - this triggers the automated release workflow
git push origin v0.X.Y
```

### Step 7: Verify GitHub Release (Automated)

The `create-release.yml` workflow automatically runs when you push a tag:

**What happens automatically:**
1. Workflow builds all variants (headless/analyst/full) for Linux and macOS
2. Release notes are generated from git commits since last tag
3. GitHub release is created with all packages attached
4. Release includes install instructions and package list

**Verify the release:**
1. Go to **Actions → Create GitHub Release** and watch the workflow
2. Once complete, check **Releases** in GitHub
3. Verify all 6 packages are attached (3 variants × 2 platforms)
4. Test installation from release:

```bash
# Example: Install headless variant from release
curl -fsSL https://github.com/monistowl/gat/releases/download/v0.X.Y/gat-0.X.Y-linux-x86_64-headless.tar.gz | tar xz
cd gat-0.X.Y-linux-x86_64-headless
./install.sh
gat-cli --version
```

**If the workflow fails:**
- Check the workflow logs for errors
- Fix the issue on main if needed
- Delete the tag locally and remotely: `git tag -d v0.X.Y && git push origin :refs/tags/v0.X.Y`
- Fix the issue and re-tag

---

## Quick Reference

### Common Commands

```bash
# Local testing before staging diagnostics
cargo test --workspace
cargo clippy --workspace
cargo fmt --check

# Check what version will be packaged
cargo metadata --no-deps --format-version 1 | jq -r '.metadata.release.version'

# Build and test a variant locally
scripts/package.sh headless
ls -lh dist/

# Test installation from local package
cd dist
tar -xzf gat-*.tar.gz
cd gat-*
./install.sh --prefix /tmp/test-install
```

### Workflow Decision Tree

```
Is code ready for release?
├─ No → Keep developing on staging
└─ Yes → Run Staging Diagnostics
    ├─ Failed → Fix issues, re-run diagnostics
    └─ Passed → Run Manual Release Build
        ├─ Packages don't work → Fix issues, re-run build
        └─ Packages work → Merge staging → main → Tag release
            └─ Automated: GitHub release created with all packages
```

### Key Rules

✅ **Always:**
- Run full diagnostics before merging to main
- Test packages locally before tagging
- Use annotated tags (`git tag -a`)
- Keep main stable and tagged-only

❌ **Never:**
- Push directly to main without merging from staging
- Tag a release without running diagnostics and testing packages
- Skip testing packages locally
- Use auto-tagging or auto-release features

---

## Troubleshooting

### Diagnostics Failed

**Problem:** Staging diagnostics workflow shows failures

**Solution:**
1. Click into the failed job to see logs
2. Download diagnostic artifacts if available
3. Reproduce the failure locally:
   ```bash
   # For feature matrix failures
   cargo test -p gat-cli --no-default-features --features minimal,full-io

   # For build matrix failures
   scripts/package.sh headless
   ```
4. Fix the issue on staging
5. Re-run diagnostics

### Package Build Failed

**Problem:** Manual release build fails on certain platforms

**Solution:**
1. Check if dependencies are missing in the workflow
2. Review package.sh for variant-specific issues
3. Test locally on a similar platform
4. Update .github/workflows/manual-release.yml if needed

### Installation Test Failed

**Problem:** Downloaded package doesn't install correctly

**Solution:**
1. Check tarball structure: `tar -tzf gat-*.tar.gz`
2. Verify expected structure:
   ```
   gat-VERSION-OS-ARCH-VARIANT/
   ├── bin/
   │   ├── gat-cli
   │   └── gat
   ├── README.md
   ├── LICENSE.txt
   └── install.sh
   ```
3. Update package.sh if structure is wrong
4. Re-run manual release build

### Version Mismatch

**Problem:** Package version doesn't match expected version

**Solution:**
1. Check `Cargo.toml` workspace metadata:
   ```bash
   cargo metadata --no-deps --format-version 1 | jq -r '.metadata.release.version'
   ```
2. Update version in Cargo.toml:
   ```toml
   [workspace.metadata.release]
   version = "0.X.Y"
   ```
3. Rebuild packages

---

## Historical Workflows (Removed)

The following workflows have been **removed** as part of the simplification:

- ~~`release.yml`~~ - Had auto-tagging on push to tags (removed)
- ~~`release-dry-run.yml`~~ - Redundant with staging-diagnostics (removed)

These were replaced by the simpler manual workflows above.

---

## Summary

The simplified release process is:

1. **Develop on staging**
2. **Run Staging Diagnostics** (manual, comprehensive)
3. **Run Manual Release Build** (manual, creates packages for testing)
4. **Test packages locally** (manual)
5. **Merge staging → main** (manual)
6. **Tag release on main** (manual) - Triggers automated release creation
7. **Verify GitHub release** (automatic creation, manual verification)

All critical decision points are **manual** and **explicit** (diagnostics, merge, tag), while tedious tasks are automated (package building for release, release creation, website deployment). The diagnostics workflow provides comprehensive "what's broken where" information to catch issues before release.
