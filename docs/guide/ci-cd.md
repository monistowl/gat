# CI/CD Workflows

## Overview

GAT uses GitHub Actions for continuous integration, testing, packaging, and release automation. The workflows are organized into specialized pipelines that share a common reusable core.

## Workflow Structure

### Fast Path: `push-pr.yml` (rust.yml)
- **Triggers:** Push to main, PRs to main, daily schedule
- **Scope:** Linux + headless variant only
- **Duration:** ~5 minutes
- **Purpose:** Quick feedback for developers
- **Artifacts:** Debug build cache only

### Full Build Matrix: `build-matrix.yml`
- **Type:** Reusable workflow called by release, dry-run, and nightly workflows
- **Platforms:** Linux (ubuntu-latest) + macOS (macos-latest)
- **Variants:** headless, full, analyst
- **Features:** Tiered diagnostics, verbose compiler output on-demand

### Release Dry-Run: `release-dry-run.yml`
- **Trigger:** Manual (workflow_dispatch)
- **Scope:** All platforms, all variants
- **Purpose:** Test the full packaging pipeline before tagging
- **Artifacts:** All bundles (headless, full, analyst for Linux and macOS)
- **Next Step:** `git tag v0.x.y && git push --tags` if satisfied

### Release: `release.yml`
- **Trigger:** Tag push (git tag v*)
- **Scope:** All platforms, all variants (same as dry-run)
- **Purpose:** Create GitHub release and upload artifacts
- **Artifacts:** Permanent release assets on GitHub

### Nightly: `nightly-full-build.yml`
- **Trigger:** Daily at 6am UTC
- **Scope:** All platforms, all variants
- **Diagnostics:** Verbose compiler output always enabled
- **Artifacts:** 14-day retention (longer than dry-run)
- **Purpose:** Early detection of regressions, user access to latest builds

## Bundle Tiers

### Headless (~30-50 MB)
- CLI only, minimal dependencies
- Features: `-p gat-cli --no-default-features --features minimal-io`
- Use case: Scripting, CI/CD, resource-constrained environments

### Analyst (~100-150 MB)
- CLI + ADMS/DERMS/DIST/analytics/featurization
- Features: `-p gat-cli --no-default-features --features minimal-io,adms,derms,dist,analytics,featurize`
- Use case: Power systems analysts, domain-focused workflows

### Full (~200-300 MB)
- Everything: GUI, TUI, all solvers, visualization
- Features: `-p gat-cli --all-features`
- Use case: Interactive desktop use, exploratory analysis

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

### From Release

```bash
# Download latest release
curl -fsSL https://github.com/monistowl/gat/releases/latest -H "Accept: application/vnd.github.v3+json" \
  | grep tag_name | cut -d'"' -f4 > /tmp/version.txt
VERSION=$(cat /tmp/version.txt)

# Download the variant you want (headless example)
curl -fsSL "https://github.com/monistowl/gat/releases/download/${VERSION}/gat-${VERSION#v}-linux-x86_64-headless.tar.gz" -o gat.tar.gz

# Install
tar -xzf gat.tar.gz
./gat-*/install.sh --variant headless
```

### Using install.sh from Source

```bash
# Clone the repository
git clone https://github.com/monistowl/gat.git
cd gat

# Install with your preferred variant
./scripts/install.sh --variant analyst
```

## Packaging Individual Bundles

To package a specific bundle variant (e.g., for local testing):

```bash
# Package headless variant
GAT_BUNDLE_VARIANT=headless ./scripts/package.sh

# Or via argument
./scripts/package.sh analyst
```

The packaged tarball will be in `dist/`.

## Customizing Installations

### Using Environment Variables

```bash
# Install to custom prefix
GAT_PREFIX=/opt/gat ./scripts/install.sh --variant full

# Use specific version (if built)
GAT_VERSION=v0.1.0 ./scripts/install.sh --analyst
```

### Installation Methods (Priority)

1. **Binary download** (if available for your platform/version)
2. **Source build** (automatic fallback if binary not available)

The installer automatically detects your platform and architecture.

## Development Workflow

### For Regular Development
- Push to main → Triggers `rust.yml` → Fast Linux headless check
- 5-minute turnaround for basic CI

### For Full Platform Testing
- Label PR with `full-ci` → Triggers optional jobs in old workflows
- Or run Release Dry-Run manually from Actions tab

### For Release Preparation
1. Create release tag locally: `git tag v0.x.y`
2. (Optional) Run Release Dry-Run first to validate
3. Push tag: `git push --tags`
4. Release workflow runs automatically

## References

- **Design Document:** `docs/plans/2025-11-22-build-workflows-design.md`
- **Implementation Plan:** `docs/plans/2025-11-22-build-workflows-implementation.md`
- **Workflow Files:** `.github/workflows/`
- **Packaging Scripts:** `scripts/{package,install}.sh`
