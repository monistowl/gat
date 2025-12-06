---
status: completed
priority: p3
issue_id: "014"
tags: [quality, code-review, ergonomics]
dependencies: []
---

# Add Serde Derives to Core ID Types

## Problem Statement

Core ID types (`BusId`, `BranchId`, etc.) lack Serialize/Deserialize derives, preventing direct serialization of Network to JSON/TOML for debugging.

**Why it matters:** Forces awkward conversions when serializing networks.

## Resolution

All ID types now derive Serialize and Deserialize with `#[serde(transparent)]`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BusId(usize);
```

**Types updated:**
- `BusId`
- `BranchId`
- `GenId`
- `LoadId`
- `TransformerId`
- `ShuntId`

## Acceptance Criteria

- [x] All ID types derive Serialize, Deserialize
- [x] Use #[serde(transparent)] for clean JSON
- [x] Network can be serialized to JSON

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | Serde derives enable debugging |
| 2025-12-06 | Verified completion | Already implemented in previous session |
