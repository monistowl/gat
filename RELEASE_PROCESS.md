# Release Process & Branch Strategy

This document describes how to manage the flow from development to stable releases.

## Branch Strategy

### `experimental` - Active Development
- **Purpose**: Rapid iteration and feature development
- **Stability**: Unstable, features under development
- **Who commits**: Developers and AI agents during active work
- **Content**:
  - Implementation commits
  - Test additions and fixes
  - Dev-facing documentation (PHASE_*.md, *_PLAN.md, etc.)
  - Git history shows the full development narrative

**Keep on experimental only:**
- `PHASE_*.md` - Phase planning and progress
- `*_PLAN.md`, `*_ROADMAP.md` - Development plans
- `.claude/`, `.agent/` - Agent-specific configurations
- Development TODOs and mid-progress design docs

### `staging` - Release Candidate
- **Purpose**: Stabilized code ready for release
- **Stability**: High - full test suite passing
- **Who merges**: After explicit decision to create release candidate
- **Content**:
  - Clean, production-ready code
  - Comprehensive user-facing documentation
  - Release notes and API docs
  - Dev docs are **excluded** (they stay on experimental)

**Replace on staging:**
- `AGENTS.md` - User-facing agent onboarding (not dev-facing)
- Release-specific documentation

### `master` - Production Releases
- **Purpose**: Stable, released versions
- **Stability**: Critical - only tagged releases
- **Who merges**: After release QA passes
- **Content**:
  - Same as staging, but tagged with version
  - Historical reference for all released versions

## Release Workflow

### Step 1: Stabilize on `experimental`
```bash
# When you're ready to create a release candidate:
# Make sure all tests pass
cargo test -p gat-tui --lib
# Ensure code is clean and committed
git status
```

### Step 2: Merge to `staging` (Replace Dev Docs)
```bash
git checkout staging
git merge experimental

# Remove dev-facing docs that were committed on experimental
git rm PHASE_*.md *_PLAN.md *_ROADMAP.md 2>/dev/null || true

# Replace AGENTS.md with user-facing version (if different)
# The AGENTS.md on experimental may have dev-facing notes
# Keep only the clean, user-facing parts:
# - bd (beads) usage
# - MCP server setup
# - Quick start for agents
# - Issue tracking workflow

# Update version in Cargo.toml
vim crates/gat-tui/Cargo.toml  # Bump version

# Create comprehensive release notes
cat > RELEASE_NOTES.md << 'EOF'
# Release Notes v0.X.Y

## Summary
[What was accomplished]

## New Features
- Feature 1
- Feature 2

## Improvements
- Improvement 1

## Bug Fixes
- Fix 1

## Breaking Changes
[If any]

## Migration Guide
[If needed]

## Test Coverage
- X tests passing
- Y% code coverage

## Dependencies Updated
[If any]
EOF

# Commit everything
git add -A
git commit -m "Release v0.X.Y: [Summary]"
```

### Step 3: Tag & Push to `master`
```bash
git checkout master
git merge staging

# Tag the release
git tag -a v0.X.Y -m "Release v0.X.Y: [Summary]"

# Push everything
git push origin master staging experimental --tags
```

## Key Rules

### ✅ Always
- Keep `master` and `staging` clean and human-legible
- Run full test suite before merging to staging
- Write clear commit messages
- Include test counts in release commits
- Tag all releases with semantic versioning

### ✅ On `experimental`
- Generate PHASE_*.md, *_PLAN.md freely during development
- Keep development TODOs and scratch notes
- Frequent commits showing iteration
- Agent-specific files are OK here

### ❌ Never
- Commit dev docs (PHASE_*.md, *_PLAN.md) to staging/master
- Push to staging/master without running full test suite
- Skip release notes when moving to staging
- Forget to update version numbers

## Automatic Filtering via .gitignore

The `.gitignore` automatically excludes dev-facing docs when they exist on staging/master:

```
# Development/agent-facing docs (kept on experimental only)
PHASE_*.md
*_PLAN.md
*_ROADMAP.md
.claude/
.agent/
```

If a dev doc is accidentally committed to staging/master, remove it:
```bash
git checkout staging
git rm --cached PHASE_*.md
git commit -m "Remove dev docs from staging"
```

## Example Release Flow

**Current state on experimental:**
- Phase 6 complete with 536 tests passing
- PHASE_6_PLAN.md (dev-facing, shows iteration)
- Multiple commits showing development process

**Decision to release (0.3.0):**
1. Run full test suite ✅
2. Merge experimental → staging
3. Remove PHASE_6_PLAN.md from staging
4. Update Cargo.toml: 0.3.0
5. Write RELEASE_NOTES.md
6. Commit: "Release v0.3.0: Complete Phase 6 real backend integration"
7. Tag: v0.3.0
8. Merge staging → master
9. Push with tags

**Result:**
- `master`: Clean release with version tags, release notes
- `staging`: Staging branch ready for next release
- `experimental`: Dev work continues with phase planning, iteration history

## Future Releases

When starting the next phase:
- Stay on `experimental`
- Generate new PHASE_*.md files as needed
- Commit freely with iteration history
- When ready to release: merge to staging, clean up dev docs, update version

This keeps `master` and `staging` as pristine release artifacts while `experimental` captures the full development narrative.
