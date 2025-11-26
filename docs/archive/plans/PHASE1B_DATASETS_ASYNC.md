# Phase 1b: Datasets Pane Async Integration

## Implementation Complete

The async data flow for the Datasets pane has been integrated into the message-based architecture.

### How It Works

1. **User Action:** User presses '3' or navigates to Datasets pane
2. **Event:** `AppEvent::KeyPress(Hotkey('3'))` is generated
3. **State Update:** `update()` function receives `Message::SwitchPane(PaneId::Datasets)`
4. **Fetch Trigger:** App can send `Message::Datasets(DatasetsMessage::FetchDatasets)`
5. **Async Task:** `handle_datasets()` creates `SideEffect::FetchDatasets` task
6. **Query Execution:** Side effect handler calls `app_state.fetch_datasets().await`
7. **Result:** `Message::Datasets(DatasetsMessage::DatasetsLoaded(result))` completes the flow
8. **UI Update:** Loading flags and result cache in AppState drive UI rendering

### Message Flow

```
User Input (Pane Switch)
    ↓
Message::SwitchPane(PaneId::Datasets)
    ↓
Message::Datasets(DatasetsMessage::FetchDatasets)
    ↓
SideEffect::FetchDatasets spawned
    ↓
AppState::fetch_datasets() async method
    ↓
QueryBuilder::get_datasets() (MockQueryBuilder)
    ↓
Message::Datasets(DatasetsMessage::DatasetsLoaded(Ok/Err))
    ↓
AppState updated with results
```

### Integration Points

**Message System** (`src/message.rs`):
- `DatasetsMessage::FetchDatasets` - Trigger fetch
- `DatasetsMessage::DatasetsLoaded(Result)` - Handle results

**Update Function** (`src/update.rs`):
- `handle_datasets()` processes fetch messages
- Sets `datasets_loading` flag during fetch
- Caches result in `app_state.datasets`
- Sends notifications on completion

**AppState** (`src/models.rs`):
- `datasets_loading: bool` - Loading state
- `datasets: Option<Result<Vec<DatasetEntry>, QueryError>>` - Results cache
- `query_builder: Arc<dyn QueryBuilder>` - Service reference
- `fetch_datasets()` async method - Executes query

**Service Layer** (`src/services/query_builder.rs`):
- `QueryBuilder` trait with async methods
- `MockQueryBuilder` implementation
- `QueryError` enum for error handling

### Pane Entry Trigger (Future Enhancement)

To trigger fetch automatically when pane is entered:

```rust
// In events/reduce.rs or similar
match msg {
    Message::SwitchPane(PaneId::Datasets) => {
        state.active_pane = PaneId::Datasets;

        // Auto-fetch if not already loaded
        if state.datasets.is_none() && !state.datasets_loading {
            effects.push(Message::Datasets(DatasetsMessage::FetchDatasets));
        }
    }
    // ...
}
```

### Testing the Flow

To test the async flow in the current UI:

1. Run: `cargo run -p gat-tui --release`
2. The app initializes with fixture data visible in Datasets pane
3. The service layer is ready for pane-triggered async operations
4. Message handlers will route fetch requests through AppState

### Next Steps

- **Phase 1c:** Replicate this pattern to other panes (Dashboard, Operations, Pipeline, Commands)
- **Phase 2:** Implement GatCoreQueryBuilder for real gat-core data access
- **Phase 3:** Add loading spinners and error UI states to panes

## Files Modified

- `src/message.rs` - Added FetchDatasets, DatasetsLoaded messages
- `src/update.rs` - Added handlers and SideEffect
- `src/integration.rs` - Added message routing
- `src/models.rs` - Added async state fields to AppState
- `src/services/query_builder.rs` - QueryBuilder trait and MockQueryBuilder
- `docs/PHASE1B_DATASETS_ASYNC.md` - This file
