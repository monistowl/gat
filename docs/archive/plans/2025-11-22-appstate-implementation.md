# AppState Implementation Plan (gat-eqb)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create AppState struct holding shared pane data, replace scattered pane states in main.rs with single source of truth.

**Architecture:** AppState lives in a new `src/data/mod.rs` module with fixture data in `src/data/fixtures.rs`. It contains persistent config (TuiConfig) and transient state (datasets, metrics, workflows). All pane states become fields of AppState. Rendering functions are updated to accept `&AppState` instead of individual pane parameters.

**Tech Stack:** Rust, serde for serialization, chrono for timestamps, fixture-based testing

---

## Task 1: Create data module structure

**Files:**
- Create: `crates/gat-tui/src/data/mod.rs`
- Create: `crates/gat-tui/src/data/fixtures.rs`
- Modify: `crates/gat-tui/src/lib.rs` (add module declaration)

**Step 1: Create data/mod.rs with module structure**

Create file `crates/gat-tui/src/data/mod.rs`:

```rust
// Data module for AppState and related structures
pub mod fixtures;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// Re-export fixtures
pub use fixtures::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatasetStatus {
    Ready,
    Idle,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetEntry {
    pub id: String,
    pub name: String,
    pub status: DatasetStatus,
    pub source: String,
    pub row_count: usize,
    pub size_mb: f64,
    pub last_updated: SystemTime,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct DatasetsState {
    pub datasets: Vec<DatasetEntry>,
    pub selected_index: usize,
}

impl DatasetsState {
    pub fn with_fixtures() -> Self {
        Self {
            datasets: create_fixture_datasets(),
            selected_index: 0,
        }
    }
}
```

**Step 2: Create data/fixtures.rs with dataset fixtures**

Create file `crates/gat-tui/src/data/fixtures.rs`:

```rust
use super::{DatasetEntry, DatasetStatus};
use std::time::{SystemTime, Duration};

pub fn create_fixture_datasets() -> Vec<DatasetEntry> {
    let now = SystemTime::now();

    vec![
        DatasetEntry {
            id: "opsd-2024".to_string(),
            name: "OPSD Snapshot".to_string(),
            status: DatasetStatus::Ready,
            source: "OPSD".to_string(),
            row_count: 8_760,
            size_mb: 245.3,
            last_updated: now - Duration::from_secs(3600),
            description: "Open Power System Data hourly generation".to_string(),
        },
        DatasetEntry {
            id: "matpower-ieee118".to_string(),
            name: "Matpower IEEE 118-Bus".to_string(),
            status: DatasetStatus::Idle,
            source: "Matpower".to_string(),
            row_count: 118,
            size_mb: 1.2,
            last_updated: now - Duration::from_secs(86400 * 7),
            description: "IEEE 118-bus test system".to_string(),
        },
        DatasetEntry {
            id: "csv-import-2024".to_string(),
            name: "Custom CSV Import".to_string(),
            status: DatasetStatus::Pending,
            source: "CSV".to_string(),
            row_count: 0,
            size_mb: 0.0,
            last_updated: now - Duration::from_secs(60),
            description: "User-uploaded CSV file (processing)".to_string(),
        },
    ]
}
```

**Step 3: Add data module to lib.rs**

Edit `crates/gat-tui/src/lib.rs`, add after other module declarations:

```rust
pub mod data;
pub use data::{DatasetEntry, DatasetsState, DatasetStatus};
```

**Step 4: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully with no errors

**Step 5: Commit**

```bash
git add crates/gat-tui/src/data/mod.rs crates/gat-tui/src/data/fixtures.rs crates/gat-tui/src/lib.rs
git commit -m "feat: Create data module with Datasets structures (gat-eqb step 1)"
```

---

## Task 2: Create TuiConfig and AppState structures

**Files:**
- Modify: `crates/gat-tui/src/data/mod.rs` (add TuiConfig and AppState)
- Modify: `crates/gat-tui/src/lib.rs` (re-export AppState)

**Step 1: Add TuiConfig to data/mod.rs**

Add to `crates/gat-tui/src/data/mod.rs` after the module declaration:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    pub refresh_rate_ms: u64,
    pub enable_animations: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            refresh_rate_ms: 50,
            enable_animations: true,
        }
    }
}
```

**Step 2: Add AppState to data/mod.rs**

Add to `crates/gat-tui/src/data/mod.rs`:

```rust
use crate::{DashboardPaneState, OperationsPaneState, DatasetsPaneState, PipelinePaneState, CommandsPaneState, NavigationLevel};

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: TuiConfig,

    // Pane states
    pub datasets: DatasetsState,
    pub dashboard: DashboardPaneState,
    pub operations: OperationsPaneState,
    pub pipeline: PipelinePaneState,
    pub commands: CommandsPaneState,

    // Navigation
    pub navigation_level: NavigationLevel,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: TuiConfig::default(),
            datasets: DatasetsState::with_fixtures(),
            dashboard: DashboardPaneState { selected_index: 0 },
            operations: OperationsPaneState { selected_index: 0 },
            pipeline: PipelinePaneState { selected_index: 0 },
            commands: CommandsPaneState { selected_index: 0 },
            navigation_level: NavigationLevel::MenuBar,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Update lib.rs exports**

Edit `crates/gat-tui/src/lib.rs`, update the data module export:

```rust
pub use data::{DatasetEntry, DatasetsState, DatasetStatus, TuiConfig, AppState};
```

**Step 4: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add crates/gat-tui/src/data/mod.rs crates/gat-tui/src/lib.rs
git commit -m "feat: Add TuiConfig and AppState structures (gat-eqb step 2)"
```

---

## Task 3: Update main.rs to use AppState

**Files:**
- Modify: `crates/gat-tui/src/main.rs` (replace scattered pane states with AppState)

**Step 1: Update main function signature**

In `crates/gat-tui/src/main.rs`, find the `main()` function and after setting up the terminal, add:

```rust
// Create single source of truth
let mut app_state = AppState::new();
```

Remove or replace any existing pane state initialization (the old `DashboardPaneState`, `OperationsPaneState`, etc. instantiations).

**Step 2: Update event handling to use app_state**

Find the event loop in main.rs. Update the key event handler to reference `app_state` instead of individual pane states. Example (look for current navigation handling):

Change from:
```rust
let mut dashboard_state = DashboardPaneState { selected_index: 0 };
// ... later
dashboard_state.selected_index = ...
```

To:
```rust
// In event handler
match event {
    Event::Key(key) => {
        match app_state.navigation_level {
            NavigationLevel::MenuBar => {
                match key.code {
                    KeyCode::Left => { /* navigate panes */ },
                    KeyCode::Right => { /* navigate panes */ },
                    KeyCode::Down | KeyCode::Enter => {
                        app_state.navigation_level = NavigationLevel::PaneContent;
                    },
                    _ => {}
                }
            },
            NavigationLevel::PaneContent => {
                // Use app_state.get_current_pane_state_mut() to update state
                match key.code {
                    KeyCode::Up => {
                        match app_state.current_pane {
                            PaneId::Datasets => app_state.datasets.selected_index = app_state.datasets.selected_index.saturating_sub(1),
                            PaneId::Dashboard => app_state.dashboard.selected_index = app_state.dashboard.selected_index.saturating_sub(1),
                            // ... other panes
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        }
    },
    _ => {}
}
```

**Step 3: Update all render function calls**

Find all `render_*` function calls in main.rs. Update signatures to pass `&app_state`:

Change from:
```rust
render_dashboard_pane(&mut frame, dashboard_state, area)
```

To:
```rust
render_dashboard_pane(&mut frame, &app_state, area)
```

**Step 4: Update render function signatures**

Update each `render_*` function to accept `&AppState`:

Change from:
```rust
fn render_dashboard_pane(
    frame: &mut Frame,
    state: DashboardPaneState,
    area: Rect,
) {
    // ... uses state.selected_index
}
```

To:
```rust
fn render_dashboard_pane(
    frame: &mut Frame,
    app_state: &AppState,
    area: Rect,
) {
    let state = &app_state.dashboard;
    // ... uses state.selected_index
}
```

Apply this pattern to all five pane render functions.

**Step 5: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 6: Test the application**

Run: `cargo run -p gat-tui --release`
Expected: Application starts, renders all panes with fixture data, navigation still works

**Step 7: Commit**

```bash
git add crates/gat-tui/src/main.rs
git commit -m "refactor: Replace scattered pane states with AppState (gat-eqb step 3)"
```

---

## Task 4: Verify all panes render with AppState

**Files:**
- Test: Manual testing of all five panes

**Step 1: Test Dashboard pane**

Run: `cargo run -p gat-tui --release`
- Press `1` to go to Dashboard
- Press `Down` to navigate sections
- Press `Esc` to return to menu bar
- Verify: All four dashboard sections render correctly

**Step 2: Test Datasets pane**

- Press `3` to go to Datasets
- Press `Down` to navigate sections
- Should see fixture datasets with their metadata (name, status, source, size)
- Press `Esc` to return to menu bar

**Step 3: Test Operations, Pipeline, Commands panes**

- Press `2`, `4`, `5` for each pane
- Verify all render without errors
- Navigation (up/down, esc) works in each

**Step 4: Test menu bar wrapping**

- From any pane, press `Esc` to return to menu bar
- Press `Left`/`Right` to navigate panes
- Verify menu bar updates and pane selection wraps around

**Step 5: Test quit functionality**

- From anywhere, press `Q` to quit
- Verify application exits cleanly

**Step 6: Manual testing complete**

If all manual tests pass, proceed to step 7.

**Step 7: Commit test results**

```bash
git commit --allow-empty -m "test: Manual verification of AppState rendering (gat-eqb step 4)"
```

---

## Task 5: Add unit tests for AppState

**Files:**
- Create: `crates/gat-tui/src/data/tests.rs`
- Modify: `crates/gat-tui/src/data/mod.rs` (add tests module)

**Step 1: Create data/tests.rs**

Create file `crates/gat-tui/src/data/tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::data::*;

    #[test]
    fn test_appstate_creation() {
        let state = AppState::new();
        assert_eq!(state.navigation_level, NavigationLevel::MenuBar);
        assert_eq!(state.datasets.selected_index, 0);
        assert!(!state.datasets.datasets.is_empty());
    }

    #[test]
    fn test_fixture_datasets_count() {
        let datasets = create_fixture_datasets();
        assert_eq!(datasets.len(), 3);
    }

    #[test]
    fn test_fixture_dataset_names() {
        let datasets = create_fixture_datasets();
        let names: Vec<_> = datasets.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"OPSD Snapshot"));
        assert!(names.contains(&"Matpower IEEE 118-Bus"));
        assert!(names.contains(&"Custom CSV Import"));
    }

    #[test]
    fn test_dataset_status_ready() {
        let datasets = create_fixture_datasets();
        let opsd = datasets.iter().find(|d| d.id == "opsd-2024").unwrap();
        assert_eq!(opsd.status, DatasetStatus::Ready);
    }

    #[test]
    fn test_tuiconfig_defaults() {
        let config = TuiConfig::default();
        assert_eq!(config.refresh_rate_ms, 50);
        assert!(config.enable_animations);
    }

    #[test]
    fn test_appstate_default() {
        let state1 = AppState::new();
        let state2 = AppState::default();
        assert_eq!(state1.config.refresh_rate_ms, state2.config.refresh_rate_ms);
    }
}
```

**Step 2: Add tests module to data/mod.rs**

Add to end of `crates/gat-tui/src/data/mod.rs`:

```rust
#[cfg(test)]
mod tests;
```

**Step 3: Run tests**

Run: `cargo test -p gat-tui --lib data`
Expected: All 7 tests pass

**Step 4: Commit**

```bash
git add crates/gat-tui/src/data/tests.rs crates/gat-tui/src/data/mod.rs
git commit -m "test: Add unit tests for AppState and fixtures (gat-eqb step 5)"
```

---

## Task 6: Document AppState API

**Files:**
- Create: `docs/APPSTATE.md`

**Step 1: Create documentation**

Create file `docs/APPSTATE.md`:

```markdown
# AppState API Documentation

AppState is the single source of truth for gat-tui state management.

## Quick Start

```rust
use gat_tui::AppState;

let mut app_state = AppState::new();

// Access pane states
app_state.datasets.selected_index = 1;

// Access config
println!("Refresh rate: {} ms", app_state.config.refresh_rate_ms);

// Check navigation level
match app_state.navigation_level {
    NavigationLevel::MenuBar => println!("In menu bar"),
    NavigationLevel::PaneContent => println!("In pane content"),
}
```

## Structure

### Config (Persistent)

```rust
pub struct TuiConfig {
    pub refresh_rate_ms: u64,
    pub enable_animations: bool,
}
```

Serializable to `~/.config/gat-tui/config.toml` (future).

### Pane States (Transient)

- `datasets: DatasetsState` - List of available datasets with selection tracking
- `dashboard: DashboardPaneState` - Dashboard content state
- `operations: OperationsPaneState` - Operations content state
- `pipeline: PipelinePaneState` - Pipeline content state
- `commands: CommandsPaneState` - Commands content state

### Navigation State

- `navigation_level: NavigationLevel` - Currently in MenuBar or PaneContent

## Fixture Data

All pane states are initialized with fixture data via `AppState::new()`:

- **Datasets:** 3 sample datasets (OPSD, Matpower, CSV import)
- **Dashboard:** Placeholder health/metrics data
- **Other panes:** Default empty states

Fixture data will be replaced with real gat-core data in Phase 2 (gat-xad).

## Usage in Rendering

All render functions accept `&AppState`:

```rust
fn render_datasets_pane(
    frame: &mut Frame,
    app_state: &AppState,
    area: Rect,
) {
    let state = &app_state.datasets;
    // ... render using state
}
```

## Usage in Event Handling

Update state in response to keyboard events:

```rust
match key.code {
    KeyCode::Down => {
        app_state.datasets.selected_index =
            (app_state.datasets.selected_index + 1)
            .min(app_state.datasets.datasets.len() - 1);
    },
    _ => {}
}
```

## Testing

Create test AppState with fixtures:

```rust
#[test]
fn test_something() {
    let state = AppState::new();
    assert_eq!(state.datasets.datasets.len(), 3);
}
```

## Future Enhancements

- Real data integration from gat-core (Phase 2)
- Async data fetching with loading states
- Configuration persistence to file
- State validation and recovery
```

**Step 2: Verify documentation**

Verify `docs/APPSTATE.md` is readable and comprehensive.

**Step 3: Commit**

```bash
git add docs/APPSTATE.md
git commit -m "docs: AppState API documentation (gat-eqb step 6)"
```

---

## Task 7: Final verification and cleanup

**Files:**
- Verify: All files compile and tests pass
- Review: Code follows project patterns

**Step 1: Full build**

Run: `cargo build -p gat-tui --release`
Expected: Builds successfully

**Step 2: Run all tests**

Run: `cargo test -p gat-tui --lib`
Expected: All tests pass

**Step 3: Run application**

Run: `cargo run -p gat-tui --release`
- Test all navigation
- Verify all panes render
- Test menu bar wrapping
- Test quit

Expected: Application works as before, but now using AppState

**Step 4: Verify git history**

Run: `git log --oneline -7`
Expected: Shows the 6 commits we made for this task

**Step 5: Mark task complete**

Run: `bd update gat-eqb --status closed --reason "AppState struct implemented with fixture data and comprehensive testing"`

**Step 6: Final commit**

```bash
git commit --allow-empty -m "chore: gat-eqb complete - AppState implementation verified"
```

---

## Summary

This plan implements AppState in 7 focused tasks:

1. **Task 1:** Create data module structure (DatasetsState, DatasetEntry)
2. **Task 2:** Create TuiConfig and AppState
3. **Task 3:** Update main.rs to use AppState instead of scattered states
4. **Task 4:** Manual verification of all panes
5. **Task 5:** Unit tests for AppState and fixtures
6. **Task 6:** API documentation
7. **Task 7:** Final verification and task closure

**Total scope:** ~300 lines of code, 7 commits, complete unit test coverage

**Next task:** gat-xad Phase 1 - Connect panes to real gat-core data

