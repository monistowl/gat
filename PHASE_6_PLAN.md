# Phase 6: Real Backend Integration - Complete Pane Wireup

**Objective**: Wire all 7 panes to actual gat-cli command execution, creating a fully functional TUI that executes real operations.

**Status**: Planning → In Progress → Completion (before version bump)

**Test Target**: 550+ tests (up from 473)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    TUI Application Layer                         │
│  (Dashboard, Commands, Datasets, Pipeline, Operations, etc.)   │
└────────────────────┬────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────────┐
│              AsyncServiceIntegration (PHASE 5)                   │
│          Async event dispatcher with retry/backoff               │
└────────────────────┬────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────────┐
│           TuiServiceLayer (PHASE 5)                              │
│    Unified interface for data operations & command building     │
└────────────────────┬────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────────┐
│    QueryBuilder Adapters (GatCoreCliAdapter, LocalFileAdapter)  │
│        PHASE 6: Enhance to execute actual gat-cli               │
└────────────────────┬────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────────┐
│               gat-cli Command Execution                          │
│        (Real backend, actual grid analysis results)              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase 6 Tasks (9 tasks)

### Task 1: Enhance QueryBuilder for Real Command Execution ⬜
**Status**: Pending
**Scope**: Expand `GatCoreCliAdapter` to actually execute gat-cli commands with proper argument passing

- [ ] Add command execution framework with output parsing
- [ ] Implement timeout handling (30s per command)
- [ ] Add stderr capture and error classification
- [ ] Create output parser for JSON results
- [ ] Add 15+ unit tests

**Acceptance Criteria**:
- Commands execute without panics
- Output parsed into structured data
- Errors gracefully handled and reported
- Tests verify 5+ different command types

---

### Task 2: Wire Dashboard Pane to Real KPI Metrics ⬜
**Status**: Pending
**Scope**: Connect dashboard KPI metrics to actual gat-cli analytics commands

- [ ] Implement metrics refresh from real analytics
- [ ] Add deliverability score calculation
- [ ] Add LOLE (Loss of Load Expectation) fetching
- [ ] Add EUE (Expected Unserved Energy) fetching
- [ ] Create metrics caching layer (5-minute TTL)
- [ ] Add 12+ unit tests

**Acceptance Criteria**:
- Dashboard metrics update from real data
- Cache prevents excessive command calls
- Metrics display with proper formatting
- Tests verify data flow accuracy

---

### Task 3: Wire Datasets Pane to Real Dataset Operations ⬜
**Status**: Pending
**Scope**: Connect datasets pane to actual gat-cli dataset management

- [ ] Implement `dataset list` command integration
- [ ] Implement `dataset upload` with file handling
- [ ] Implement `dataset validate` with error reporting
- [ ] Add progress tracking for uploads
- [ ] Create dataset metadata parser
- [ ] Add 12+ unit tests

**Acceptance Criteria**:
- Can list, upload, and validate datasets
- Progress tracking for long operations
- File paths properly handled
- Tests verify all 3 operations

---

### Task 4: Wire Commands Pane to Execute Real Commands ⬜
**Status**: Pending
**Scope**: Connect commands pane snippets to actual gat-cli execution with output capture

- [ ] Implement snippet command execution
- [ ] Add output capture to history
- [ ] Implement dry-run mode (`--dry-run` flag)
- [ ] Add execution timing
- [ ] Create output display widget
- [ ] Add 15+ unit tests

**Acceptance Criteria**:
- Snippets execute and produce real output
- Dry-run mode works correctly
- History shows actual results
- Tests verify execution modes

---

### Task 5: Wire Pipeline Pane to Scenario Workflows ⬜
**Status**: Pending
**Scope**: Connect pipeline pane to scenario validation and materialization

- [ ] Implement `scenarios validate` integration
- [ ] Implement `scenarios materialize` integration
- [ ] Add progress tracking for materialization
- [ ] Create scenario template parser
- [ ] Add validation error reporting
- [ ] Add 12+ unit tests

**Acceptance Criteria**:
- Scenarios can be validated
- Scenarios can be materialized
- Templates properly parsed
- Tests verify both operations

---

### Task 6: Wire Operations Pane to Batch Job Execution ⬜
**Status**: Pending
**Scope**: Connect operations pane to real batch power flow and OPF jobs

- [ ] Implement `batch powerflow` command
- [ ] Implement `batch opf` command with solver selection
- [ ] Add job status polling
- [ ] Add progress tracking (0-100%)
- [ ] Create job result parser
- [ ] Add 15+ unit tests

**Acceptance Criteria**:
- Batch jobs can be submitted
- Status properly tracked
- Results parsed and displayed
- Tests verify all job types

---

### Task 7: Wire Analytics Pane to Real Analytics Commands ⬜
**Status**: Pending
**Scope**: Connect analytics pane tabs to actual analytics calculations

- [ ] Wire Reliability tab to `analytics reliability`
- [ ] Wire Deliverability tab to `analytics deliverability`
- [ ] Wire ELCC tab to `analytics elcc`
- [ ] Wire PowerFlow tab to `analytics powerflow`
- [ ] Add result caching per dataset/grid
- [ ] Add 18+ unit tests

**Acceptance Criteria**:
- All 4 analytics types execute
- Results properly formatted
- Caching prevents re-execution
- Tests verify all 4 types

---

### Task 8: Implement Cross-Pane Event Coordination ⬜
**Status**: Pending
**Scope**: Ensure data consistency and event flow between panes when operations complete

- [ ] Add operation completion events
- [ ] Implement dashboard refresh on operations
- [ ] Sync datasets when new ones uploaded
- [ ] Update analytics when new results available
- [ ] Add transactional consistency checks
- [ ] Add 15+ unit tests

**Acceptance Criteria**:
- Operations properly broadcast results
- Panes update when related operations complete
- No stale data issues
- Tests verify event flow

---

### Task 9: End-to-End Real Backend Integration Testing ⬜
**Status**: Pending
**Scope**: Create comprehensive integration tests exercising real gat-cli execution

- [ ] Create workflow: Upload → Validate → Analyze
- [ ] Create workflow: Run batch job → Monitor → Check results
- [ ] Create workflow: Materialize scenario → Execute analytics
- [ ] Add error recovery testing
- [ ] Add timeout handling verification
- [ ] Create 20+ integration tests

**Acceptance Criteria**:
- Multi-step workflows execute successfully
- Errors handled gracefully
- Timeouts respected
- Tests verify complete workflows

---

## Dependencies & Integration Points

```
Task 1 (QueryBuilder)
    ↓
    ├→ Task 2 (Dashboard)
    ├→ Task 3 (Datasets)
    ├→ Task 4 (Commands)
    ├→ Task 5 (Pipeline)
    ├→ Task 6 (Operations)
    └→ Task 7 (Analytics)
        ↓
    Task 8 (Event Coordination)
        ↓
    Task 9 (Integration Testing)
```

---

## Testing Strategy

- **Unit Tests**: Each task adds 12-18 tests
- **Integration Tests**: Task 9 adds 20+ tests
- **Coverage Target**: 550+ tests passing
- **No Regressions**: All Phase 1-5 tests still pass
- **Real Command Verification**: Each task verifies 3+ actual gat-cli calls

---

## Implementation Notes

### Command Execution Pattern
```rust
// Each pane follows this pattern:
async fn execute_operation(&mut self, cmd: &str) -> Result<Output, Error> {
    // 1. Build command with proper arguments
    // 2. Set timeout (30s default)
    // 3. Execute via TuiServiceLayer
    // 4. Parse output to structured data
    // 5. Update pane state with results
    // 6. Broadcast completion event
    // 7. Handle errors gracefully
}
```

### Error Handling
- Network timeouts → Retry with backoff
- Invalid input → Show user-friendly error
- Command not found → Suggest fix
- Output parse errors → Log and show raw output
- Permission denied → Suggest authentication

### Performance Requirements
- Command execution: < 30s per command
- UI remains responsive during execution
- Cache results (5-10 minute TTL)
- Parallel batch operations supported

---

## Completion Criteria

✅ All 9 tasks complete
✅ 550+ tests passing
✅ All panes execute real gat-cli commands
✅ No regressions from Phase 1-5
✅ Error handling comprehensive
✅ Performance acceptable
✅ Ready for version bump to next major version

---

## Success Metrics

- **Functionality**: 100% of command snippets execute successfully
- **Reliability**: < 1% failure rate on repeated operations
- **Performance**: 95% of operations complete in < 5s
- **User Experience**: Clear feedback on all operations
- **Code Quality**: Zero unsafe code, comprehensive error handling

---

**Next**: Proceed with Task 1 when ready
