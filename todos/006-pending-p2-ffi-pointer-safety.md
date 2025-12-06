---
status: completed
priority: p2
issue_id: "006"
tags: [security, code-review, ffi, solvers]
dependencies: []
---

# Unchecked Raw Pointer Dereferences in IPOPT FFI

## Problem Statement

The IPOPT FFI callbacks at `crates/gat-ipopt-sys/src/wrapper.rs` dereference raw pointers from C without null checks or bounds validation.

**Why it matters:** Memory corruption if IPOPT passes invalid pointers or incorrect sizes.

## Resolution

Added comprehensive safety checks to `eval_h_callback` in wrapper.rs:

```rust
// Null pointer checks
if user_data.is_null() { return 0; }

// Size validation
let n_usize = n as usize;
let m_usize = m as usize;
let nnz = nele_hess as usize;
if n < 0 || m < 0 || nele_hess < 0 { return 0; }
if n_usize > MAX_PROBLEM_SIZE || m_usize > MAX_PROBLEM_SIZE || nnz > MAX_PROBLEM_SIZE {
    return 0;
}

// Null checks for output arrays
if new_x && (iRow.is_null() || jCol.is_null()) { return 0; }
```

Uses existing `MAX_PROBLEM_SIZE` constant (100_000) for bounds validation.

## Acceptance Criteria

- [x] Null pointer checks before all dereferences
- [x] Size validation before slice creation
- [x] Compilation verified

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | FFI boundaries need defensive programming |
| 2025-12-06 | Added safety checks | Use existing MAX_PROBLEM_SIZE constant |
