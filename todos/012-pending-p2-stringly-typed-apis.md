---
status: completed
priority: p2
issue_id: "012"
tags: [architecture, code-review, type-safety]
dependencies: []
---

# Replace Stringly-Typed APIs with Enums

## Problem Statement

Several APIs use String types where enums would provide compile-time type safety.

**Why it matters:** Stringly-typed APIs allow invalid values at runtime that could be caught at compile time.

## Resolution

Updated the TUI event dispatcher to use proper types:

1. **AnalyticsType Enum** - `AsyncEvent::RunAnalytics` now takes `AnalyticsType` enum instead of `String`
2. **PathBuf for Paths** - All file path parameters changed from `String` to `PathBuf`:
   - `FetchDatasetFetch(String, PathBuf)` - output path
   - `RunScenarioValidation(PathBuf)` - spec path
   - `RunScenarioMaterialize(PathBuf, PathBuf)` - template, output
   - `RunBatchPowerFlow(PathBuf, usize)` - manifest path
   - `RunBatchOPF(PathBuf, usize, String)` - manifest path
   - `RunGeoJoin(PathBuf, PathBuf, PathBuf)` - left, right, output
   - `DescribeRun(PathBuf)` - run.json path
   - `ResumeRun(PathBuf)` - run.json path

**Files modified:**
- `crates/gat-tui/src/services/event_dispatcher.rs` - Updated `AsyncEvent` enum
- `crates/gat-tui/src/services/async_service_integration.rs` - Updated method signatures

The existing `AnalyticsType` enum in `tui_service_layer.rs` was already defined with the correct variants (Reliability, DeliverabilityScore, ELCC, PowerFlow).

## Acceptance Criteria

- [x] Analytics types use enum
- [x] Event dispatcher uses PathBuf instead of String for paths
- [x] All 643 TUI tests pass

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | Stringly-typed APIs allow runtime errors |
| 2025-12-06 | Implemented fix | Using PathBuf provides clear distinction between paths and strings |
