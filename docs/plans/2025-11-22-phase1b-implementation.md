# Phase 1b: Datasets Pane Async Integration (gat-ic0)

> **For Claude:** Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Connect Datasets pane to the QueryBuilder service layer, demonstrate full async data flow with loading spinner and error handling.

**Architecture:** Datasets pane calls app_state.fetch_datasets() on entry, renders spinner while loading, displays data or error with retry button.

**Tech Stack:** Rust, tokio (async), AppState service integration

---

## Task 1: Update Datasets pane to trigger fetch on entry

**Files:**
- Modify: `crates/gat-tui/src/panes/datasets.rs`
- Modify: `crates/gat-tui/src/app.rs` (if needed for pane lifecycle)

**Step 1: Update DatasetsPane to add fetch trigger**

Edit `crates/gat-tui/src/panes/datasets.rs`, update the `PaneView` impl:

```rust
impl PaneView for DatasetsPane {
    fn id(&self) -> &'static str {
        "datasets"
    }

    fn label(&self) -> &'static str {
        "Datasets"
    }

    fn hotkey(&self) -> char {
        '3'
    }

    fn layout(&self, context: &PaneContext) -> PaneLayout {
        // Check if we need to fetch data
        if context.app_state.datasets.is_none() && !context.app_state.datasets_loading {
            // Trigger fetch - this will be handled by app message loop
            // For now, just use existing fixture data
            Self::layout()
        } else {
            Self::layout_with_data(context)
        }
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Review catalog metadata, preview workflows, and download datasets.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![
            ContextButton::new('f', "[f] Fetch dataset"),
            ContextButton::new('i', "[i] Inspect schema"),
        ]
    }
}
```

**Step 2: Create helper method to render with async state**

Add to DatasetsPane impl block (after layout() method):

```rust
impl DatasetsPane {
    pub fn layout() -> PaneLayout {
        let datasets = create_fixture_datasets();

        // Build dataset table from fixture data
        let mut dataset_table = TableView::new(["Name", "Source", "Size", "Status"]);
        for dataset in &datasets {
            let status_icon = match dataset.status {
                DatasetStatus::Ready => "✓",
                DatasetStatus::Idle => "◆",
                DatasetStatus::Pending => "⟳",
            };
            dataset_table = dataset_table.add_row([
                dataset.name.as_str(),
                dataset.source.as_str(),
                &format!("{:.1} MB", dataset.size_mb),
                status_icon,
            ]);
        }

        PaneLayout::new(
            Pane::new("Data catalog")
                .body([
                    "Available datasets:",
                    "Select a dataset to view details or download",
                ])
                .with_table(dataset_table)
                .with_child(Pane::new("Downloads").with_empty_state(EmptyState::new(
                    "No downloads queued",
                    [
                        "Run a fetch to pull sample data",
                        "Queued jobs will appear here",
                    ],
                )))
                .mark_visual(),
        )
        .with_sidebar(Sidebar::new("Metadata", false).lines(["Retained: 30d", "Backups: nightly"]))
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 80,
            tall_threshold: 24,
            expand_visuals_on_wide: true,
            collapse_secondary_first: true,
        })
    }

    pub fn layout_with_data(context: &PaneContext) -> PaneLayout {
        // Display loading state
        if context.app_state.datasets_loading {
            return PaneLayout::new(
                Pane::new("Data catalog")
                    .body([
                        "Loading datasets...",
                        "",
                        "⟳ Fetching from service...",
                    ])
                    .mark_visual(),
            );
        }

        // Display error state
        if let Some(Err(error)) = &context.app_state.datasets {
            return PaneLayout::new(
                Pane::new("Data catalog")
                    .body([
                        "Error loading datasets",
                        "",
                        &format!("✗ {}", error),
                        "",
                        "[r] Retry  [esc] Dismiss",
                    ])
                    .mark_visual(),
            );
        }

        // Display data
        if let Some(Ok(datasets)) = &context.app_state.datasets {
            let mut dataset_table = TableView::new(["Name", "Source", "Size", "Status"]);
            for dataset in datasets {
                let status_icon = match dataset.status {
                    DatasetStatus::Ready => "✓",
                    DatasetStatus::Idle => "◆",
                    DatasetStatus::Pending => "⟳",
                };
                dataset_table = dataset_table.add_row([
                    dataset.name.as_str(),
                    dataset.source.as_str(),
                    &format!("{:.1} MB", dataset.size_mb),
                    status_icon,
                ]);
            }

            return PaneLayout::new(
                Pane::new("Data catalog")
                    .body([
                        "Available datasets:",
                        "Select a dataset to view details or download",
                    ])
                    .with_table(dataset_table)
                    .with_child(Pane::new("Downloads").with_empty_state(EmptyState::new(
                        "No downloads queued",
                        [
                            "Run a fetch to pull sample data",
                            "Queued jobs will appear here",
                        ],
                    )))
                    .mark_visual(),
            );
        }

        // Fallback to fixture data
        Self::layout()
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add crates/gat-tui/src/panes/datasets.rs
git commit -m "feat: Update Datasets pane to support async data flow (phase1b step 1)

- Add layout_with_data() method to handle loading/error/success states
- Update PaneView impl to check if data needs fetching
- Preserve fixture data fallback for initialization

Loading spinner and error handling ready for service integration."
```

---

## Task 2: Add message handler for fetch trigger

**Files:**
- Modify: `crates/gat-tui/src/message.rs`
- Modify: `crates/gat-tui/src/update.rs`

**Step 1: Add FetchDatasets message type**

Edit `crates/gat-tui/src/message.rs`, add to Message enum:

```rust
pub enum Message {
    // ... existing variants ...

    // Data fetching
    FetchDatasets,
    DatasetsLoaded(Result<Vec<DatasetEntry>, QueryError>),
    FetchWorkflows,
    WorkflowsLoaded(Result<Vec<Workflow>, QueryError>),
    FetchMetrics,
    MetricsLoaded(Result<SystemMetrics, QueryError>),
}
```

**Step 2: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/gat-tui/src/message.rs
git commit -m "feat: Add async data fetch messages (phase1b step 2)

- Add FetchDatasets, DatasetsLoaded messages
- Add FetchWorkflows, WorkflowsLoaded messages
- Add FetchMetrics, MetricsLoaded messages

Messages for triggering async data loads and handling results."
```

---

## Task 3: Add async fetch handlers to update function

**Files:**
- Modify: `crates/gat-tui/src/update.rs`

**Step 1: Add FetchDatasets handler**

Edit `crates/gat-tui/src/update.rs`, add to the match statement in update():

```rust
pub fn update(msg: Message, app_state: &mut AppState) -> SideEffect {
    match msg {
        // ... existing handlers ...

        Message::FetchDatasets => {
            // Clone app_state for async block
            let mut state = app_state.clone();
            let query_builder = app_state.query_builder.clone();

            return SideEffect::SpawnTask(Box::new(async move {
                state.fetch_datasets().await;
                // Return message with result
                Message::DatasetsLoaded(state.datasets.take().flatten().ok_or_else(|| {
                    QueryError::Unknown("Fetch failed".to_string())
                }))
            }));
        }

        Message::DatasetsLoaded(result) => {
            app_state.datasets = Some(result);
            app_state.datasets_loading = false;
        }

        // ... other handlers ...
    }

    SideEffect::None
}
```

**Step 2: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/gat-tui/src/update.rs
git commit -m "feat: Add async fetch handlers to update function (phase1b step 3)

- Add FetchDatasets message handler
- Add DatasetsLoaded result handler
- Spawn async task for fetch_datasets()
- Cache result in app_state

Connects message system to QueryBuilder service."
```

---

## Task 4: Trigger fetch when Datasets pane is entered

**Files:**
- Modify: `crates/gat-tui/src/panes/datasets.rs`
- Modify: `crates/gat-tui/src/app.rs` (pane lifecycle)

**Step 1: Add on_enter hook to DatasetsPane**

If PaneView supports on_enter, add:

```rust
impl PaneView for DatasetsPane {
    // ... existing methods ...

    fn on_enter(&mut self, context: &PaneContext) -> Option<Message> {
        // Fetch datasets when pane is entered
        if context.app_state.datasets.is_none() {
            Some(Message::FetchDatasets)
        } else {
            None
        }
    }
}
```

Otherwise, trigger in app.rs pane switch logic:

```rust
// In app.rs when switching to Datasets pane
if current_pane_id != PaneId::Datasets && next_pane_id == PaneId::Datasets {
    if app_state.datasets.is_none() {
        return Message::FetchDatasets;
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/gat-tui/src/panes/datasets.rs crates/gat-tui/src/app.rs
git commit -m "feat: Trigger fetch when Datasets pane is entered (phase1b step 4)

- Add on_enter hook to DatasetsPane (or integrate into app.rs logic)
- Fetch datasets only if not already loaded
- Sends FetchDatasets message on pane entry

Demonstrates lazy-load pattern for pane data."
```

---

## Task 5: Add unit tests for async flow

**Files:**
- Create: `crates/gat-tui/tests/integration_test_datasets_async.rs`
- Modify: `crates/gat-tui/src/update.rs` (add tests)

**Step 1: Add update handler tests**

Edit `crates/gat-tui/src/update.rs`, add test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_datasets_message() {
        let mut app_state = AppState::new();
        assert!(app_state.datasets.is_none());

        // Simulate fetch
        let _ = update(Message::FetchDatasets, &mut app_state);
        // In real scenario, this would spawn a task
        // For now, verify message is accepted
    }

    #[tokio::test]
    async fn test_datasets_loaded_message() {
        let mut app_state = AppState::new();
        let datasets = create_fixture_datasets();

        let _ = update(
            Message::DatasetsLoaded(Ok(datasets)),
            &mut app_state,
        );

        assert!(app_state.datasets.is_some());
        assert!(!app_state.datasets_loading);
    }

    #[tokio::test]
    async fn test_datasets_error_handling() {
        let mut app_state = AppState::new();

        let _ = update(
            Message::DatasetsLoaded(Err(QueryError::ConnectionFailed("test".to_string()))),
            &mut app_state,
        );

        assert!(app_state.datasets.is_some());
        assert!(matches!(
            app_state.datasets.as_ref().unwrap(),
            Err(QueryError::ConnectionFailed(_))
        ));
    }
}
```

**Step 2: Verify compilation and tests pass**

Run: `cargo test -p gat-tui --lib update`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/gat-tui/src/update.rs
git commit -m "test: Add async flow tests for Datasets pane (phase1b step 5)

- Test FetchDatasets message handler
- Test DatasetsLoaded with success result
- Test DatasetsLoaded with error result

Validates message-based async flow."
```

---

## Task 6: Full integration test and verification

**Files:**
- No new files, just verification

**Step 1: Run all tests**

Run: `cargo test -p gat-tui --lib`
Expected: All tests pass

**Step 2: Build release**

Run: `cargo build -p gat-tui --release`
Expected: Builds successfully

**Step 3: Verify Datasets pane renders**

Run: `cargo run -p gat-tui --release`
- Navigate to Datasets pane (press '3')
- Verify datasets table renders
- Verify loading state works if you add tracing to see fetch trigger

**Step 4: Commit**

```bash
git commit --allow-empty -m "chore: Phase 1b complete - Datasets pane async integration verified

✓ Datasets pane triggers fetch on entry
✓ Loading state renders while fetching
✓ Error state with retry handling
✓ Success state displays dataset table
✓ All tests passing
✓ Release build successful

Ready for Phase 1c: Replicate pattern to other panes

Implements foundation for async data flow in all panes."
```

---

## Summary

**Phase 1b Implementation:** 6 bite-sized tasks

1. **Task 1:** Update Datasets pane for async states (10 min)
2. **Task 2:** Add fetch messages (5 min)
3. **Task 3:** Implement message handlers (10 min)
4. **Task 4:** Trigger fetch on pane entry (5 min)
5. **Task 5:** Add tests (10 min)
6. **Task 6:** Full verification (5 min)

**Total scope:** ~150 lines of code, 6 commits

**Deliverables:**
- Datasets pane with async data flow
- Loading spinner UI state
- Error handling with retry
- Message-based architecture for async operations
- Full test coverage

**Next phase:** Phase 1c - Replicate pattern to Dashboard, Operations, Pipeline, Commands panes

