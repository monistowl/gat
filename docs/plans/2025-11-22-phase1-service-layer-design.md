# Phase 1 Service Layer Design (gat-xad)

**Date:** 2025-11-22
**Task:** gat-xad - Phase 1: Connect panes to real application state from gat-core
**Goal:** Build a centralized async query service layer that mediates between panes and gat-core, enabling all panes to fetch real data while maintaining responsive UI.

---

## Architecture Overview

The Query Builder Service is a trait-based abstraction that allows panes to request data asynchronously without knowing implementation details. Two implementations support testing and gradual migration:

1. **MockQueryBuilder** - Uses fixture data (current state)
2. **GatCoreQueryBuilder** - Queries real gat-core APIs (future)

```
Panes → AppState → QueryBuilder trait → [MockQueryBuilder | GatCoreQueryBuilder]
                                                ↓
                                        gat-core APIs (eventually)
```

---

## Core Service Interface

### QueryBuilder Trait

```rust
pub trait QueryBuilder: Send + Sync {
    // Dataset operations
    async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError>;
    async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError>;

    // Workflow operations
    async fn get_workflows(&self) -> Result<Vec<Workflow>, QueryError>;
    async fn get_workflow_status(&self, id: &str) -> Result<WorkflowStatus, QueryError>;

    // Metrics operations
    async fn get_metrics(&self) -> Result<SystemMetrics, QueryError>;

    // Add more query methods as panes need them
}

pub enum QueryError {
    NotFound(String),
    ConnectionFailed(String),
    Timeout,
    ParseError(String),
    Unknown(String),
}
```

### Error Handling Strategy

- Errors are returned as `Result<T, QueryError>` to panes
- Panes display errors prominently with:
  - Clear error message describing what failed
  - Retry button for user-initiated retry
  - Option to show cached data if available
- No automatic retries at service layer (panes control retry UX)

---

## Async Data Flow

### Request Lifecycle

```
1. Pane triggers fetch: app_state.fetch_datasets()
2. Service spawns async task: query_builder.get_datasets()
3. Task registered in app_state.async_tasks["datasets"] = Running
4. Pane renders loading spinner while task runs
5. Task completes, result stored in app_state.datasets_result
6. Next render cycle displays data or error
7. User can retry via UI button
```

### AppState Integration

```rust
pub struct AppState {
    // Query service (pluggable)
    pub query_builder: Arc<dyn QueryBuilder>,

    // Task tracking
    pub async_tasks: HashMap<String, AsyncTaskState>,

    // Results cache
    pub datasets_result: Option<Result<Vec<DatasetEntry>, QueryError>>,
    pub workflows_result: Option<Result<Vec<Workflow>, QueryError>>,
    pub metrics_result: Option<Result<SystemMetrics, QueryError>>,
    // ... one for each data type panes need ...
}

pub enum AsyncTaskState {
    Running,
    Pending,
    Completed,
    Failed,
}
```

### Async Task Management

Panes request data like this:

```rust
// Pane spawns fetch task
pub async fn fetch_datasets(&mut self) {
    self.async_tasks.insert("datasets", AsyncTaskState::Running);

    match self.query_builder.get_datasets().await {
        Ok(datasets) => {
            self.datasets_result = Some(Ok(datasets));
            self.async_tasks.insert("datasets", AsyncTaskState::Completed);
        }
        Err(e) => {
            self.datasets_result = Some(Err(e));
            self.async_tasks.insert("datasets", AsyncTaskState::Failed);
        }
    }
}
```

Panes render based on task state:

```rust
match app_state.async_tasks.get("datasets") {
    Some(AsyncTaskState::Running) => render_loading_spinner(),
    Some(AsyncTaskState::Completed | AsyncTaskState::Failed) => {
        match &app_state.datasets_result {
            Some(Ok(datasets)) => render_dataset_table(datasets),
            Some(Err(e)) => render_error_with_retry(e),
            None => render_empty_state(),
        }
    }
    _ => render_empty_state(),
}
```

---

## Implementation Strategy

### Phase 1a: Build Service Layer (Minimal)

Create `src/services/query_builder.rs`:

1. **Define QueryBuilder trait** with methods for each pane's data needs
2. **Implement MockQueryBuilder** using existing fixture data
3. **Integrate with AppState** - add query_builder field, result caches, async_tasks
4. **Add fetch methods to AppState** that spawn async tasks

### Phase 1b: Connect First Pane

Use Datasets pane as foundation:

1. Update Datasets pane to call `app_state.fetch_datasets()`
2. Render loading spinner while fetching
3. Display error with retry button on failure
4. Document pattern for other panes to follow

### Phase 1c: Replicate Pattern

Apply same pattern to remaining panes:
- Dashboard (metrics, workflows)
- Operations (batch jobs, DERMS queue)
- Pipeline (configuration)
- Commands (command history)

### Future: Real gat-core Implementation

Once pattern is solid:

1. Implement GatCoreQueryBuilder using real gat-core APIs
2. Switch AppState to use GatCoreQueryBuilder in production
3. Keep MockQueryBuilder for tests

---

## Benefits of This Architecture

1. **Separation of Concerns:** Panes focus on rendering, service layer handles data access
2. **Pluggable Implementations:** Easy to swap MockQueryBuilder ↔ GatCoreQueryBuilder
3. **Responsive UI:** Async prevents blocking on slow queries
4. **Testability:** Mock implementation enables testing without gat-core
5. **Gradual Migration:** Fixture data → real data, one pane at a time
6. **Error Visibility:** Users see what failed and can retry
7. **Reusability:** All panes use same service interface

---

## Data Types Needed (Expand as Needed)

Based on pane requirements:

- **Datasets:** DatasetEntry (already exists)
- **Workflows:** Workflow, WorkflowStatus (need to define)
- **Metrics:** SystemMetrics (Dashboard KPIs: DS, LOLE, EUE)
- **Batch Jobs:** BatchJob, JobStatus (Operations)
- **DERMS Queue:** EnvelopeJob, StressTest (Operations)
- **Pipeline Config:** PipelineConfig, Transform (Pipeline)
- **Commands:** CommandResult, CommandHistory (Commands)

---

## Success Criteria

- ✓ QueryBuilder trait defined and documented
- ✓ MockQueryBuilder uses existing fixture data
- ✓ AppState integrates query_builder with async task tracking
- ✓ Datasets pane demonstrates full async data flow
- ✓ Error handling with retry UI works end-to-end
- ✓ Pattern clear enough for other panes to replicate
- ✓ All tests pass, no blocking behavior in UI

---

## Next Steps

1. Implement Phase 1a (service layer foundation)
2. Connect Datasets pane (Phase 1b)
3. Document pattern
4. Replicate to other panes (Phase 1c)
5. Eventually: Implement GatCoreQueryBuilder for real data
