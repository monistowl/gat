---
status: completed
priority: p2
issue_id: "010"
tags: [performance, code-review]
dependencies: []
---

# Missing #[inline] Hints on ID Accessor Methods

## Problem Statement

ID accessor methods like `BusId::value()`, `BranchId::value()` lack `#[inline]` attributes, potentially preventing inlining across crate boundaries in hot loops.

**Why it matters:** These are called thousands of times in power flow calculations.

## Resolution

All ID types and unit types now have `#[inline]` attributes on their accessor methods:

**ID Types in `lib.rs`:**
- `BusId::new()`, `BusId::value()`
- `BranchId::new()`, `BranchId::value()`
- `GenId::new()`, `GenId::value()`
- `LoadId::new()`, `LoadId::value()`
- `TransformerId::new()`, `TransformerId::value()`
- `ShuntId::new()`, `ShuntId::value()`

**Unit Types in `units.rs`:**
- 21 `#[inline]` annotations across unit wrapper methods

## Acceptance Criteria

- [x] All ID type accessors have #[inline]
- [x] All unit wrapper accessors have #[inline]

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | Cross-crate inlining requires hints |
| 2025-12-06 | Verified completion | Already implemented in previous session |
