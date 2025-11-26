# Phase 1 Service Layer Implementation Plan (gat-xad)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a trait-based QueryBuilder service layer with MockQueryBuilder implementation, integrate with AppState, and demonstrate full async data flow in Datasets pane.

**Architecture:** QueryBuilder trait defines async query interface. MockQueryBuilder implements using fixtures. AppState holds query_builder reference and manages async_tasks with result caching. Datasets pane demonstrates complete flow: spawn task → show spinner → display data/error.

**Tech Stack:** Rust, tokio (async), trait objects, Arc for shared ownership

---

## Task 1: Create QueryBuilder trait and error types

**Files:**
- Create: `crates/gat-tui/src/services/query_builder.rs`
- Modify: `crates/gat-tui/src/services/mod.rs` (add module)
- Modify: `crates/gat-tui/src/lib.rs` (export QueryError)

**Step 1: Create services module structure**

Edit `crates/gat-tui/src/services/mod.rs`, add:

```rust
pub mod query_builder;
pub use query_builder::{QueryBuilder, QueryError};
```

If `services/mod.rs` doesn't exist, create it with the above content.

**Step 2: Create query_builder.rs with trait and error types**

Create `crates/gat-tui/src/services/query_builder.rs`:

```rust
use crate::{DatasetEntry, DatasetStatus};
use std::sync::Arc;

/// Error type for query operations
#[derive(Debug, Clone)]
pub enum QueryError {
    NotFound(String),
    ConnectionFailed(String),
    Timeout,
    ParseError(String),
    Unknown(String),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QueryError::NotFound(msg) => write!(f, "Not found: {}", msg),
            QueryError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            QueryError::Timeout => write!(f, "Query timed out"),
            QueryError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            QueryError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for QueryError {}

/// Trait for querying application data
pub trait QueryBuilder: Send + Sync {
    /// Fetch all available datasets
    async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError>;

    /// Fetch a specific dataset by ID
    async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError>;
}

/// Mock implementation using fixture data
pub struct MockQueryBuilder;

impl QueryBuilder for MockQueryBuilder {
    async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError> {
        Ok(crate::create_fixture_datasets())
    }

    async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError> {
        crate::create_fixture_datasets()
            .into_iter()
            .find(|d| d.id == id)
            .ok_or_else(|| QueryError::NotFound(format!("Dataset {} not found", id)))
    }
}
```

**Step 3: Export QueryError from lib.rs**

Edit `crates/gat-tui/src/lib.rs`, add after existing exports:

```rust
pub use services::{QueryBuilder, QueryError};
```

Also ensure `pub mod services;` is declared (add if missing).

**Step 4: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully with no errors

**Step 5: Commit**

```bash
git add crates/gat-tui/src/services/query_builder.rs crates/gat-tui/src/services/mod.rs crates/gat-tui/src/lib.rs
git commit -m "feat: Create QueryBuilder trait and MockQueryBuilder (phase1 step 1)

- Define QueryBuilder trait with async query methods
- Implement MockQueryBuilder using fixture data
- Define QueryError enum for error handling
- Export from lib.rs for public use

Foundation for all panes to query data via common interface."
```

---

## Task 2: Extend data types and add to AppState

**Files:**
- Modify: `crates/gat-tui/src/data.rs` (add Workflow types)
- Modify: `crates/gat-tui/src/models.rs` (extend AppState)

**Step 1: Add Workflow and related types to data.rs**

Edit `crates/gat-tui/src/data.rs`, add before the tests section (around line 380):

```rust
use serde::{Deserialize, Serialize};

/// A workflow execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub status: WorkflowStatus,
    pub created_by: String,
    pub created_at: SystemTime,
    pub completed_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

/// System metrics for Dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub deliverability_score: f64,  // 0-100
    pub lole_hours_per_year: f64,   // Loss of Load Expectation
    pub eue_mwh_per_year: f64,      // Expected Unserved Energy
}
```

**Step 2: Update AppState to include query_builder and result fields**

Edit `crates/gat-tui/src/models.rs`, update AppState struct (around line 5):

```rust
use crate::{QueryBuilder, QueryError, DatasetEntry};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AppState {
    // ... existing fields ...

    // Query service
    pub query_builder: Arc<dyn QueryBuilder>,

    // Async task tracking
    pub datasets_loading: bool,
    pub workflows_loading: bool,
    pub metrics_loading: bool,

    // Results cache
    pub datasets: Option<Result<Vec<DatasetEntry>, QueryError>>,
    pub workflows: Option<Result<Vec<Workflow>, QueryError>>,
    pub metrics: Option<Result<SystemMetrics, QueryError>>,
}
```

Also update `AppState::new()` to initialize these fields:

```rust
impl AppState {
    pub fn new() -> Self {
        let query_builder = Arc::new(MockQueryBuilder);

        let mut pane_states = HashMap::new();
        // ... existing pane state init ...

        AppState {
            // ... existing fields ...
            query_builder,
            datasets_loading: false,
            workflows_loading: false,
            metrics_loading: false,
            datasets: None,
            workflows: None,
            metrics: None,
        }
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add crates/gat-tui/src/data.rs crates/gat-tui/src/models.rs
git commit -m "feat: Add data types and QueryBuilder integration to AppState (phase1 step 2)

- Add Workflow and SystemMetrics types to data.rs
- Extend AppState with query_builder field (Arc<dyn QueryBuilder>)
- Add loading flags and result cache fields
- Initialize with MockQueryBuilder in AppState::new()

Prepares AppState for async data fetching across all panes."
```

---

## Task 3: Add async fetch methods to AppState

**Files:**
- Modify: `crates/gat-tui/src/models.rs` (add async methods)

**Step 1: Add async fetch methods to AppState**

Edit `crates/gat-tui/src/models.rs`, add new methods to the AppState impl block:

```rust
impl AppState {
    // ... existing methods ...

    /// Fetch all datasets asynchronously
    pub async fn fetch_datasets(&mut self) {
        self.datasets_loading = true;
        self.datasets = Some(self.query_builder.get_datasets().await);
        self.datasets_loading = false;
    }

    /// Fetch all workflows asynchronously
    pub async fn fetch_workflows(&mut self) {
        self.workflows_loading = true;
        self.workflows = Some(self.query_builder.get_workflows().await);
        self.workflows_loading = false;
    }

    /// Fetch system metrics asynchronously
    pub async fn fetch_metrics(&mut self) {
        self.metrics_loading = true;
        self.metrics = Some(self.query_builder.get_metrics().await);
        self.metrics_loading = false;
    }
}
```

Wait - this won't compile because QueryBuilder doesn't have get_workflows() yet. Let me fix that:

Edit `crates/gat-tui/src/services/query_builder.rs`, extend QueryBuilder trait:

```rust
pub trait QueryBuilder: Send + Sync {
    // ... existing methods ...

    /// Fetch all workflows
    async fn get_workflows(&self) -> Result<Vec<Workflow>, QueryError>;

    /// Fetch system metrics
    async fn get_metrics(&self) -> Result<SystemMetrics, QueryError>;
}
```

Also extend MockQueryBuilder implementation (in same file):

```rust
impl QueryBuilder for MockQueryBuilder {
    // ... existing implementations ...

    async fn get_workflows(&self) -> Result<Vec<Workflow>, QueryError> {
        Ok(vec![])  // Empty for now, will populate with fixtures later
    }

    async fn get_metrics(&self) -> Result<SystemMetrics, QueryError> {
        Ok(SystemMetrics {
            deliverability_score: 85.5,
            lole_hours_per_year: 9.2,
            eue_mwh_per_year: 15.3,
        })
    }
}
```

Add imports to query_builder.rs (at top):

```rust
use crate::{DatasetEntry, DatasetStatus};
use crate::data::{Workflow, WorkflowStatus, SystemMetrics};
```

**Step 2: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/gat-tui/src/models.rs crates/gat-tui/src/services/query_builder.rs
git commit -m "feat: Add async fetch methods to AppState (phase1 step 3)

- Add fetch_datasets(), fetch_workflows(), fetch_metrics() to AppState
- Each sets loading flag, calls query_builder, caches result
- Extend QueryBuilder trait with all query methods
- Update MockQueryBuilder with stub implementations

Async methods ready for panes to call."
```

---

## Task 4: Add tests for QueryBuilder

**Files:**
- Create: `crates/gat-tui/src/services/query_builder_tests.rs`
- Modify: `crates/gat-tui/src/services/query_builder.rs` (add test module)

**Step 1: Create tests in query_builder.rs**

Edit `crates/gat-tui/src/services/query_builder.rs`, add at end of file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_get_datasets() {
        let qb = MockQueryBuilder;
        let result = qb.get_datasets().await;
        assert!(result.is_ok());
        let datasets = result.unwrap();
        assert_eq!(datasets.len(), 3);  // Three fixture datasets
    }

    #[tokio::test]
    async fn test_mock_get_dataset_found() {
        let qb = MockQueryBuilder;
        let result = qb.get_dataset("opsd-2024").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "OPSD Snapshot");
    }

    #[tokio::test]
    async fn test_mock_get_dataset_not_found() {
        let qb = MockQueryBuilder;
        let result = qb.get_dataset("nonexistent").await;
        assert!(result.is_err());
        match result {
            Err(QueryError::NotFound(_)) => (),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_mock_get_metrics() {
        let qb = MockQueryBuilder;
        let result = qb.get_metrics().await;
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.deliverability_score > 0.0);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p gat-tui --lib services::query_builder`
Expected: All 4 tests pass

**Step 3: Commit**

```bash
git add crates/gat-tui/src/services/query_builder.rs
git commit -m "test: Add QueryBuilder and MockQueryBuilder tests (phase1 step 4)

- Test MockQueryBuilder.get_datasets() returns 3 fixtures
- Test get_dataset() finds existing dataset
- Test get_dataset() returns NotFound for missing dataset
- Test get_metrics() returns valid metrics

All tests use tokio::test for async execution."
```

---

## Task 5: Create service module integration

**Files:**
- Modify: `crates/gat-tui/src/lib.rs` (ensure services exported)

**Step 1: Verify services module is properly exported**

Check `crates/gat-tui/src/lib.rs` has:

```rust
pub mod services;
```

If not present, add it with the other module declarations.

**Step 2: Verify exports include QueryBuilder types**

Check that lib.rs has:

```rust
pub use services::{QueryBuilder, QueryError};
```

If not, add it to the public exports section.

**Step 3: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 4: Commit (may be empty if already done)**

```bash
git commit --allow-empty -m "chore: Verify services module exports (phase1 step 5)"
```

---

## Task 6: Verify full Phase 1a is working

**Files:**
- No new files, just verification

**Step 1: Run all tests**

Run: `cargo test -p gat-tui --lib`
Expected: All tests pass (including new QueryBuilder tests)

**Step 2: Build release**

Run: `cargo build -p gat-tui --release`
Expected: Builds successfully

**Step 3: Verify you can construct AppState with QueryBuilder**

This is checked by the test suite. If tests pass, AppState construction works.

**Step 4: Commit**

```bash
git commit --allow-empty -m "chore: Phase 1a complete - Service layer foundation verified

✓ QueryBuilder trait with MockQueryBuilder
✓ AppState integration with query_builder and result caches
✓ Async fetch methods (fetch_datasets, fetch_workflows, fetch_metrics)
✓ QueryError types for error handling
✓ All tests passing
✓ Release build successful

Ready for Phase 1b: Connect Datasets pane to service layer"
```

---

## Summary

**Phase 1a Implementation:** 6 bite-sized tasks

1. **Task 1:** Create QueryBuilder trait and MockQueryBuilder (10 min)
2. **Task 2:** Add data types, extend AppState (10 min)
3. **Task 3:** Add async fetch methods (5 min)
4. **Task 4:** Add unit tests for QueryBuilder (5 min)
5. **Task 5:** Verify service module exports (5 min)
6. **Task 6:** Full verification and commit (5 min)

**Total scope:** ~100 lines of code, 6 commits, complete test coverage

**Deliverables:**
- Pluggable QueryBuilder trait with async interface
- MockQueryBuilder using fixture data (ready for GatCoreQueryBuilder later)
- AppState with query_builder, loading flags, and result caches
- Async fetch methods ready for panes to call
- Comprehensive unit tests

**Next phase:** Phase 1b - Connect Datasets pane to demonstrate async data flow

