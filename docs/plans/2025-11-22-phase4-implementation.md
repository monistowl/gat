# Phase 4 Implementation Plan - Interactive Command Execution & Analytics

**Phase**: 4
**Target**: Interactive command execution from Operations pane + command result handling
**Foundation**: Phase 3 (Grid Management & Workflow Tracking - 212 tests)
**Estimated Scope**: 8-10 tasks, 250-350 new tests

## Overview

Phase 4 focuses on bridging the gap between UI state and actual command execution. Users will be able to execute gat-cli commands directly from the Operations pane, see results in real-time, and track execution history. This phase builds heavily on Phase 3's workflow tracking to create a full execution pipeline.

## Success Criteria

- [ ] Users can execute gat-cli commands from Operations pane
- [ ] Command execution shows real-time output/progress
- [ ] Results are captured and persisted in workflow history
- [ ] Command validation prevents invalid invocations
- [ ] Error handling provides helpful feedback
- [ ] All existing tests still pass (212+)
- [ ] Release build successful with zero errors
- [ ] Integration tests verify end-to-end command flow

## Architecture

### Command Execution Flow

```
User Input (UI)
    ↓
Command Parser (validates syntax)
    ↓
Command Validator (checks against gat-cli schema)
    ↓
Send Execute Message
    ↓
Handle Execute (update.rs)
    ↓
Spawn Command (services/command_service.rs)
    ↓
Poll Output (async task)
    ↓
Update AppState with Results
    ↓
Create Workflow Record
    ↓
Display in Operations pane
```

### New Components

1. **Command Parser** - Parse user input into structured commands
2. **Command Validator** - Validate against known gat-cli commands
3. **CommandService** - Execute system commands with timeout
4. **Result Capturing** - Stream output from subprocess
5. **Result Display** - Render command results in modal

## Detailed Tasks (8 Core + 2 Integration)

### Task 1: Add command execution messages
**Goal**: Extend message system for command execution
**Files**: `src/message.rs`
**Changes**:
- Add `OperationsMessage::ExecuteCommand(String)` - parsed command
- Add `OperationsMessage::ExecuteCustom(String)` - raw command line
- Add `OperationsMessage::CancelExecution` - interrupt running command
- Add `OperationsMessage::CommandOutput(String)` - output chunk received
- Add `OperationsMessage::CommandCompleted(Result<CommandResult, String>)` - execution finished
- Add result type for command output, exit code, duration

**Success Criteria**:
- All variants pattern-matched in handlers
- Exhaustive match enforcement by compiler
- 3-5 unit tests for message creation

**Complexity**: Low (similar to grid messages in Phase 3)

---

### Task 2: Implement command execution handlers
**Goal**: Handle all new command messages in update.rs
**Files**: `src/update.rs`
**Changes**:
- Extend `SideEffect` enum:
  - `ExecuteCommand { task_id, command }` - spawn subprocess
  - `StreamOutput { task_id, output }` - output chunk
  - `CancelCommand { task_id }` - send SIGTERM
- Handler for `ExecuteCommand`: validates command, spawns task, shows "Running" state
- Handler for `CommandOutput`: accumulates output buffer
- Handler for `CommandCompleted`: creates workflow record, shows results
- Handler for `CancelExecution`: requests cancellation of running task
- Async task management for long-running commands

**Success Criteria**:
- All handlers implement proper error handling
- Timeout handling (default 300s from settings)
- Output buffering with size limits
- 8-10 unit tests covering all paths

**Complexity**: Medium (async task management + result handling)

---

### Task 3: Create CommandService for execution
**Goal**: System-level command execution with safety guarantees
**Files**: `src/services/command_service.rs` (new)
**Implementation**:
```rust
pub struct CommandService {
    default_timeout: u64,
    max_output_lines: usize,
}

pub struct CommandExecution {
    pub command: String,
    pub working_dir: Option<String>,
    pub timeout_secs: u64,
}

pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
}

impl CommandService {
    pub fn new(default_timeout: u64) -> Self
    pub async fn execute(&self, exec: CommandExecution) -> Result<CommandResult, CommandError>
    pub async fn execute_with_streaming<F>(&self, exec: CommandExecution, on_output: F)
        -> Result<CommandResult, CommandError>
        where F: Fn(String) + Send + 'static
}
```

**Key Features**:
- Configurable timeout (default 300s)
- Output line limiting (prevent OOM)
- Working directory support
- Streaming output callback
- Proper SIGTERM handling on timeout
- Clean separation of stdout/stderr

**Success Criteria**:
- Command execution works locally
- Timeout enforcement works
- Output buffering works
- Error cases handled gracefully
- 10-12 unit tests with mock commands

**Complexity**: Medium-High (async subprocess management)

---

### Task 4: Add command validation
**Goal**: Prevent invalid command execution before invoking subprocess
**Files**: `src/services/command_validator.rs` (new)
**Implementation**:
- Known gat-cli subcommands: datasets, derms, opf, pf, etc.
- Known flags per subcommand
- Validate command syntax before execution
- Suggest corrections on typos

```rust
pub struct CommandValidator;

impl CommandValidator {
    pub fn validate(&self, command: &str) -> Result<ValidCommand, ValidationError>
    pub fn suggest_fix(&self, invalid_cmd: &str) -> Option<String>
}

#[derive(Debug)]
pub enum ValidationError {
    UnknownCommand(String),
    InvalidFlags(Vec<String>),
    MissingRequired(Vec<String>),
    SyntaxError(String),
}
```

**Knowledge Base**:
- Load from gat-cli help output or hardcoded schema
- Cover: datasets list/upload/delete, derms envelope, opf analysis, pf solve, etc.

**Success Criteria**:
- Validates known commands
- Rejects unknown subcommands
- Detects invalid flag combinations
- 8-10 unit tests

**Complexity**: Medium (command schema management)

---

### Task 5: Integrate command execution into Operations pane
**Goal**: Add execute button and result display to Operations pane
**Files**: `src/panes/operations_pane.rs`
**Changes**:
- Add fields to `OperationsPaneState`:
  - `command_input: String` - user's command line
  - `command_output: Vec<String>` - execution results
  - `executing: bool` - command running state
  - `last_result: Option<CommandResult>` - last execution result
  - `execution_history: Vec<ExecutedCommand>` - history with results
- Add methods:
  - `set_command_input()` - update command text
  - `execute_command()` -> Result<SideEffect>
  - `add_output_line()` - append output
  - `set_result()` - store final result
  - `format_output_for_display()` - render-ready text
- UI state for:
  - Command input field (multi-line or single-line)
  - Output viewer (scrollable)
  - Status indicator (running/success/error)
  - Execution time display

**Success Criteria**:
- Command input integrates with existing pane state
- Output display formats properly
- Result tracking works with workflow system
- 8-10 tests for all operations

**Complexity**: Medium (pane state integration)

---

### Task 6: Create command result display modal
**Goal**: Beautiful modal for command output viewing
**Files**: `src/ui/command_components.rs` (new)
**Implementation**:
```rust
pub struct CommandResultModal;

impl CommandResultModal {
    pub fn render(result: &CommandResult, command: &str) -> String
}

pub struct CommandOutputViewer {
    pub lines: Vec<String>,
    pub scroll_position: usize,
}

impl CommandOutputViewer {
    pub fn new() -> Self
    pub fn scroll_up(&mut self)
    pub fn scroll_down(&mut self)
    pub fn scroll_to_end(&mut self)
    pub fn render(&self, height: usize) -> Vec<String>
}
```

**Display Features**:
- Command that was executed (boxed)
- Exit code with color (green=0, red=non-zero)
- Duration of execution
- Scrollable output (stdout + stderr mixed or separated)
- Line wrapping for long lines
- Syntax highlighting for common patterns (errors, warnings)
- Copy-to-clipboard hint

**Success Criteria**:
- Renders command info
- Scrolling works
- Line wrapping works
- 6-8 rendering tests

**Complexity**: Low-Medium (rendering only)

---

### Task 7: Add execution history tracking
**Goal**: Track all executed commands with results
**Files**: `src/models.rs` (extend AppState)
**Changes**:
- Add to `AppState`:
  - `command_history: Vec<ExecutedCommand>` - all commands ever run
  - `current_execution: Option<RunningExecution>` - in-progress command
- Structures:
```rust
pub struct ExecutedCommand {
    pub id: String,
    pub command: String,
    pub result: CommandResult,
    pub executed_by: String,
    pub timestamp: SystemTime,
    pub workflow_id: Option<String>,
}

pub struct RunningExecution {
    pub task_id: String,
    pub command: String,
    pub started_at: SystemTime,
    pub output_buffer: String,
}
```
- Methods:
  - `start_command_execution()`
  - `add_command_output()`
  - `complete_command_execution()`
  - `get_command_history()` - with filtering
  - `clear_command_history()`

**Success Criteria**:
- Commands tracked with full metadata
- LRU cleanup (max 500 commands)
- Integration with workflow tracking
- 5-6 tests

**Complexity**: Low (similar to workflow tracking in Phase 3)

---

### Task 8: Command result persistence
**Goal**: Save command results for later analysis
**Files**: `src/data.rs` (extend)
**Changes**:
- Add serializable `ExecutedCommand` type
- Add `CommandResult` serialization
- Methods for:
  - Exporting command history to JSON/CSV
  - Searching history by command pattern
  - Filtering by date/status
  - Statistics (success rate, avg duration)

**Success Criteria**:
- Results serializable to JSON
- Search/filter works
- Statistics accurate
- 4-6 tests

**Complexity**: Low (data serialization)

---

### Task 9: Full integration - end-to-end testing
**Goal**: Verify entire command execution pipeline works
**Files**: `src/lib.rs` integration tests
**Test Coverage**:
- Execute valid command (mock or simple command like `echo`)
- Capture output correctly
- Handle timeout gracefully
- Show results in Operations pane
- Create workflow record
- Track in command history
- Reject invalid commands
- Cancel running command
- Handle command errors

**Success Criteria**:
- All 212 Phase 3 tests still pass
- 15-20 new integration tests pass
- Release build successful
- Zero compiler errors

**Complexity**: Medium (orchestration of all components)

---

### Task 10 (Optional): Command template system
**Goal**: Pre-built command templates for common operations
**Files**: `src/commands/templates.rs` (new)
**Features**:
- Predefined templates for common workflows
- Parameter substitution
- Suggested commands based on loaded grid
- One-click execution for templates

**Success Criteria**:
- Templates load correctly
- Substitution works
- 4-5 tests

**Complexity**: Low (but nice-to-have)

---

## Testing Strategy

### Unit Tests
- Message creation/validation (5 tests)
- Handler functions (10 tests)
- CommandService execution (12 tests)
- CommandValidator rules (10 tests)
- Result display/formatting (8 tests)
- History tracking (6 tests)
- **Subtotal**: ~50 unit tests

### Integration Tests
- Full command execution flow (3 tests)
- Output capture with streaming (2 tests)
- Error handling and recovery (3 tests)
- Workflow integration (2 tests)
- Pane state updates (2 tests)
- History persistence (2 tests)
- **Subtotal**: ~14 integration tests

### Total New Tests: ~60-70 tests
**Expected Final Count**: 212 + 65 = **277 tests**

## Implementation Strategy

1. **Week 1**: Tasks 1-3 (Messages, Handlers, CommandService)
2. **Week 2**: Tasks 4-6 (Validation, Pane Integration, Result Display)
3. **Week 3**: Tasks 7-8 (History Tracking, Persistence)
4. **Week 4**: Task 9 (Integration Testing) + Task 10 (Optional)

## Dependencies & Risks

### Dependencies
- Phase 3 must be complete (✅ Done)
- gat-cli must be on PATH for real execution
- Tokio for async subprocess management

### Risks
- Command execution security (injection attacks)
  - *Mitigation*: CommandValidator + argument parsing
- Long-running commands blocking UI
  - *Mitigation*: Async task + timeout enforcement
- Output buffering OOM on huge outputs
  - *Mitigation*: Line count limit + streaming

### Mitigations
- All commands validated before execution
- Output line limiting (configurable)
- Timeout enforcement (default 300s)
- SIGTERM on timeout
- Clear error messages for user

## Success Metrics

- [ ] All 212 Phase 3 tests pass
- [ ] 60+ new tests added (Phase 4)
- [ ] Release build successful
- [ ] Command execution works with real gat-cli
- [ ] Output captured and displayed
- [ ] Results persisted in history
- [ ] No memory leaks on long commands
- [ ] Timeout enforcement verified

## Phase 4 Final Goals

Users should be able to:
1. Type a gat-cli command in Operations pane
2. See validation feedback (✓/✗)
3. Execute with confirmation
4. See real-time output
5. View final result with exit code
6. Replay from history
7. Export results
8. Track execution in workflows

---

## Implementation Notes

### Code Organization
```
src/
  services/
    command_service.rs     (execution)
    command_validator.rs   (validation)
  ui/
    command_components.rs  (result display)
  commands/
    templates.rs          (optional)
  panes/
    operations_pane.rs    (integration)
  message.rs              (extend)
  update.rs               (handlers)
  models.rs               (state)
  data.rs                 (persistence)
```

### Key Design Decisions
1. **Async subprocess**: Use tokio::process::Command
2. **Output buffering**: Vec<String> with line limit
3. **Timeout**: Configurable per command + global default
4. **Validation**: Before execution, not after
5. **History**: In-memory + optional persistence

### Testing Approach
1. Mock commands for unit tests
2. Real commands (echo, date) for integration
3. Staged deployment (validate → stream → complete)

---

**Ready to implement Phase 4** ✅
