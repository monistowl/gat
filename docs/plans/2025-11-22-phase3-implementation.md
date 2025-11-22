# Phase 3: UI Integration and Workflow Tracking

**Status:** 🚀 PLANNING

**Date:** 2025-11-22

**Previous Phase:** Phase 2 Complete - 166 tests passing, real data integration working

**Objective:** Connect the real data layer (Phase 2) to the UI, enabling users to load grids, view network statistics, and track analysis workflows.

## Phase 3 Scope

Phase 3 builds on Phase 2's real data foundation to create an interactive UI experience. The phase focuses on:

1. **Grid Loading UI** - UI commands and modals for loading power networks
2. **Network Visualization** - Display grid stats on Dashboard and Datasets panes
3. **Workflow Tracking** - Track executed analysis operations and results
4. **Interactive Selection** - Let users browse and switch between loaded grids
5. **Error Handling** - Display load errors and status messages to user

## Architecture

```
Phase 3 Integration Points:

User Input (Keyboard/Mouse)
    ↓
Message System (Phase 1c)
    ↓
Update Handler (NEW: grid commands)
    ↓
AppState (has GridService + GatCoreQueryBuilder from Phase 2)
    ↓
UI Rendering (NEW: display grid data)
    ↓
Terminal Display
```

## Tasks (9 Total)

### Task 1: Add Grid Command Messages
**Objective:** New message types for grid operations

**Messages to Add:**
- `LoadGrid(file_path: String)` - Load grid from file
- `UnloadGrid(grid_id: String)` - Unload and free memory
- `SwitchGrid(grid_id: String)` - Switch active grid
- `RefreshGrids` - Reload grid list
- `GridLoaded(grid_id: String)` - Success notification
- `GridLoadFailed(error: String)` - Error handling

**Files:**
- Extend `crate::Message` enum in `src/lib.rs` or `src/message.rs`
- Add variants with documentation

**Tests:** 3 unit tests
- Message creation
- Message serialization/display
- Pattern matching

**Acceptance Criteria:**
- All new variants compile
- Compiler pattern matching exhaustiveness verified
- Update handler ready for new messages

---

### Task 2: Implement Grid Command Handlers
**Objective:** Update handler methods for grid operations

**Handler Methods:**
```rust
fn handle_load_grid(&mut self, file_path: String) -> SideEffect
fn handle_unload_grid(&mut self, grid_id: String) -> SideEffect
fn handle_switch_grid(&mut self, grid_id: String) -> SideEffect
fn handle_refresh_grids(&mut self) -> SideEffect
```

**Behaviors:**
- `load_grid`: Call AppState::load_grid(), update datasets cache, show success/error notification
- `unload_grid`: Call AppState::unload_current_grid(), refresh grid list, invalidate caches
- `switch_grid`: Call AppState::set_current_grid(), invalidate metrics cache, trigger fetch_metrics
- `refresh_grids`: Rebuild grid list display, update Datasets pane

**Files:**
- `crates/gat-tui/src/update.rs` - Add handlers to Update impl

**Tests:** 8 unit tests
- Load grid success/failure paths
- Unload grid validation
- Switch grid cache invalidation
- Error state management

**Acceptance Criteria:**
- All handlers implemented
- Cache invalidation correct
- Error messages clear
- Tests passing

---

### Task 3: Add Grid UI Components
**Objective:** New UI elements for grid management

**Components:**
- `GridBrowserModal` - List loaded grids, select one to switch/unload
- `GridLoadModal` - File path input, async load progress
- `GridInfoDisplay` - Stats for current grid (node count, density, etc)

**Implementation:**
- Add to `src/ui/components/` directory
- Follow tuirealm pattern from Phase 1c
- State stored in AppState pane_states

**Files:**
- `crates/gat-tui/src/ui/components/grid_browser.rs`
- `crates/gat-tui/src/ui/components/grid_load_modal.rs`
- `crates/gat-tui/src/ui/components/grid_info.rs`
- Update `src/ui/components/mod.rs`

**Tests:** 6 unit tests
- Component rendering
- State management
- Focus/selection handling

**Acceptance Criteria:**
- Components render correctly
- State updates work
- Modal interactions smooth

---

### Task 4: Integrate Grid Browser into Datasets Pane
**Objective:** Show loaded grids in Datasets pane

**Changes:**
- Modify Datasets pane handler to show grid list instead of mock data
- Add hotkey to open GridBrowserModal (e.g., 'l' for load, 'u' for unload)
- Display: Grid ID, node count, edge count, density, file size
- Selection highlight, navigation keys

**Files:**
- `crates/gat-tui/src/handlers/datasets_handler.rs`
- Update pane display logic

**Tests:** 4 unit tests
- Dataset list display
- Grid count accuracy
- Selection tracking

**Acceptance Criteria:**
- Real grids display in Datasets pane
- Statistics accurate
- Navigation smooth

---

### Task 5: Display Grid Info on Dashboard
**Objective:** Show current grid stats on Dashboard

**Display Elements:**
- Current grid ID (if loaded)
- Network size: "14 buses, 20 branches"
- Metrics: Deliverability, LOLE, EUE
- Load status and file path
- Last update timestamp

**Files:**
- Modify Dashboard pane handler to query GatCoreQueryBuilder metrics
- Add GridInfoDisplay component to Dashboard

**Tests:** 3 unit tests
- Metric display accuracy
- No-grid-loaded case
- Format validation

**Acceptance Criteria:**
- Dashboard shows current grid info
- Metrics refresh on grid switch
- Empty state handled gracefully

---

### Task 6: Implement Workflow Tracking
**Objective:** Track executed commands and results

**Workflow Data:**
```rust
pub struct ExecutedWorkflow {
    pub id: String,
    pub command: String,
    pub grid_id: String,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub status: WorkflowStatus,
    pub results: Option<String>,
}
```

**Tracking:**
- Store in AppState: `executed_workflows: Vec<ExecutedWorkflow>`
- Add to workflow when user executes command (Phase 4)
- Display in Operations pane with timeline

**Implementation:**
- Extend `get_workflows()` in GatCoreQueryBuilder to return executed_workflows
- Add helper to create new workflows
- Implement cleanup (keep last 100)

**Files:**
- Extend `crates/gat-tui/src/data.rs` with ExecutedWorkflow
- Modify `crates/gat-tui/src/models.rs` AppState
- Update `crates/gat-tui/src/services/gat_core_query_builder.rs`

**Tests:** 5 unit tests
- Workflow creation
- Status updates
- Query methods
- Cleanup logic

**Acceptance Criteria:**
- Workflows tracked correctly
- No memory leaks (cleanup works)
- Query returns correct data

---

### Task 7: Add Load Grid Modal with File Dialog
**Objective:** User-friendly file selection for grid loading

**Modal Features:**
- Text input for file path
- Tab-complete for common directories (./test_data)
- Recent files list
- Load button with async progress
- Error display on failure

**Implementation:**
- File path input with validation
- Directory browsing (optional)
- Async load via tokio task
- Progress indicator during load

**Files:**
- `crates/gat-tui/src/ui/components/grid_load_modal.rs` (from Task 3)
- Integrate with message handlers

**Tests:** 4 unit tests
- Path validation
- Async load handling
- Error messages
- UI state transitions

**Acceptance Criteria:**
- Users can load grids via modal
- Paths validate
- Errors display clearly
- Modal closes on completion

---

### Task 8: Add Workflow Display in Operations Pane
**Objective:** Show executed workflows and results

**Display:**
- Timeline of recent workflows
- Command name, time, status (✓ or ✗)
- Quick result preview
- Click to expand full output

**Implementation:**
- Modify Operations pane to query get_workflows()
- Display as timeline list
- Color-coded status (green success, red error)

**Files:**
- `crates/gat-tui/src/handlers/operations_handler.rs`
- Update rendering logic

**Tests:** 4 unit tests
- Workflow list display
- Status formatting
- Timeline accuracy

**Acceptance Criteria:**
- Operations pane shows workflows
- Status indicators work
- Workflows persist correctly

---

### Task 9: Full Integration Testing
**Objective:** End-to-end testing with real workflows

**Test Scenarios:**
1. Load IEEE 14-bus → verify displays in Datasets → check Dashboard metrics
2. Switch to IEEE 33-bus → verify Dashboard updates → check metrics change
3. Unload grid → verify Datasets empty → Dashboard shows "no grid"
4. Execute command on grid → verify workflow recorded → displays in Operations
5. Concurrent grids → load two grids, switch between them, metrics correct

**Test Type:** Integration tests (no mocking)
- Use real grid files from test_data
- Verify UI displays match actual data
- Check cache invalidation works
- Validate workflow tracking

**Files:**
- `crates/gat-tui/src/ui_integration_tests.rs` (NEW)

**Tests:** 5 comprehensive integration tests

**Acceptance Criteria:**
- All workflows execute without panics
- UI displays match backend data
- Performance acceptable
- No resource leaks

---

## Implementation Order

**Priority 1 (Core):** Tasks 1, 2, 6
- Establish message handling, command processing, workflow tracking
- Foundation for UI integration

**Priority 2 (UI Display):** Tasks 4, 5, 8
- Show data to users
- Dashboard and Datasets visualization

**Priority 3 (User Interaction):** Tasks 3, 7
- Load modals, grid browser
- Smooth UX

**Priority 4 (Validation):** Task 9
- Integration testing
- End-to-end verification

## Testing Strategy

```
Unit Tests: 32 tests
  - Task 1: 3 tests (messages)
  - Task 2: 8 tests (handlers)
  - Task 3: 6 tests (components)
  - Task 4: 4 tests (datasets pane)
  - Task 5: 3 tests (dashboard)
  - Task 6: 5 tests (workflows)
  - Task 7: 4 tests (load modal)

Integration Tests: 5 tests
  - Task 9: End-to-end scenarios

Expected Total: 166 (Phase 2) + 37 (Phase 3) = 203 tests
```

## Success Criteria

- ✅ All 37 new tests passing
- ✅ 203 total tests passing
- ✅ Release build succeeds
- ✅ Users can load grids via UI
- ✅ Grid info displays on Dashboard
- ✅ Workflow history tracks operations
- ✅ No new compiler warnings
- ✅ Backward compatible with Phase 1-2

## Key Design Decisions

### 1. Keep MockQueryBuilder as Default in Tests
- Existing tests continue to work unchanged
- Integration tests explicitly load real grids
- Gradual migration to real data for all tests possible later

### 2. Message-Driven Grid Operations
- Consistent with Phase 1c architecture
- LoadGrid message triggers AppState::load_grid()
- No direct UI→AppState calls (maintain separation)

### 3. Workflow Tracking in AppState
- Centralized history storage
- Easy to query and filter
- Survives grid switches

### 4. Optional Async Grid Loading
- File I/O can block UI (acceptable for MVP)
- Future: Move to background task with progress
- Current: Simple blocking load with error display

### 5. Grid Browser Modal
- Optional UI enhancement
- Shows all loaded grids
- Allows quick switching without file dialogs

## Future Enhancements (Phase 4+)

- Async grid loading with progress bar
- Grid comparison/diff view
- Scenario creation from loaded grids
- Command execution on specific grids
- Results visualization and export
- Persistent grid cache (save/load state)
- Network topology visualization

## Open Questions

1. **File Dialog:** Should we implement a file browser modal, or just path input?
   - Recommendation: Start with path input (simpler), add browser in Phase 4

2. **Workflow Persistence:** Should executed workflows be saved to disk?
   - Recommendation: Keep in memory (Phase 3), add persistence in Phase 4

3. **Grid Display Format:** How much detail in grid list (size, density, etc)?
   - Recommendation: Show node count, branch count, density percentage

4. **Error Recovery:** What happens if grid becomes corrupted during load?
   - Recommendation: Show error modal, user can retry or cancel

## Related Documentation

- `docs/PHASE2_COMPLETION.md` - Phase 2 architecture (reference for GridService, GatCoreQueryBuilder)
- `docs/PHASE2_FINAL_SUMMARY.md` - Design decisions and performance notes
- Phase 1c plan - Message system and pane handlers

## Timeline Estimate

Assuming 2-3 hours per task:
- Tasks 1-2: 4-6 hours (foundation)
- Tasks 3-5: 6-9 hours (UI components and panes)
- Tasks 6-7: 4-6 hours (workflows and modals)
- Task 8: 2-3 hours (operations pane)
- Task 9: 3-4 hours (integration testing)

**Total: 19-28 hours** for complete Phase 3

## Summary

Phase 3 brings Phase 2's real data layer to users through an integrated UI. Users can load power system networks, see network statistics on the dashboard, browse loaded grids, and track analysis operations. The implementation maintains the message-driven architecture from Phase 1c while leveraging the GridService and GatCoreQueryBuilder from Phase 2.

Ready to begin Task 1 when approved.
