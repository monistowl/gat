---
status: completed
priority: p1
issue_id: "004"
tags: [architecture, code-review, dependencies]
dependencies: []
---

# Inconsistent thiserror Version Across Crates

## Problem Statement

The workspace has mixed versions of `thiserror` (v1.0 and v2.0) across crates, which can cause dependency resolution conflicts and API incompatibilities.

**Why it matters:** Version mismatch can cause build failures and forces pulling both versions into the dependency tree.

## Resolution

The workspace crates now consistently use `thiserror.workspace = true` with the root `Cargo.toml` specifying `thiserror = "2.0"`.

However, thiserror 1.x is still pulled in by **upstream dependencies** (not our code):
- `argmin` 0.10.0
- `clarabel` 0.11.1
- `polars-core` 0.35.4

This is outside our control until these libraries update. The workspace-level standardization is complete.

## Acceptance Criteria

- [x] All workspace crates use same thiserror version (2.0)
- [x] Workspace-level dependency configured
- [x] All tests pass
- [ ] No duplicate thiserror in dependency tree (blocked on upstream)

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | Version mismatch causes dependency bloat |
| 2025-12-06 | Verified workspace config | Already using workspace-level dependency, duplicates from upstream deps |
