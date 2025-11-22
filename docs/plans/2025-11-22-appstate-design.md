# AppState Design for gat-tui (gat-eqb)

**Date:** 2025-11-22
**Task:** gat-eqb - Create AppState struct to hold shared pane data
**Phase:** Phase 1 - Connect panes to real application state from gat-core

## Overview

AppState is the single source of truth for gat-tui state management. It holds:
- Navigation state (current pane, navigation level)
- Per-pane states (selected indices, view modes)
- Shared data (datasets, workflows, metrics)
- Persistent configuration (theme, keybindings, refresh rate)

## Data Model: Datasets (Phase 1 Foundation)

```rust
#[derive(Debug, Clone)]
pub enum DatasetStatus {
    Ready,      // Available and usable
    Idle,       // Exists but not recently accessed
    Pending,    // Loading/processing
}

#[derive(Debug, Clone)]
pub struct DatasetEntry {
    pub id: String,                    // Unique identifier
    pub name: String,                  // Display name
    pub status: DatasetStatus,
    pub source: String,                // "OPSD", "Matpower", "CSV", etc.
    pub row_count: usize,
    pub size_mb: f64,
    pub last_updated: SystemTime,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct DatasetsState {
    pub datasets: Vec<DatasetEntry>,
    pub selected_index: usize,         // For pane navigation
}
```

**Rationale:** Rich dataset metadata allows the Datasets pane to be informative without being overengineered. Status enum keeps state machine explicit.

## Complete AppState Structure

### Configuration Layer (Persistent)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    pub current_pane: PaneId,
    pub theme: ThemeChoice,              // Future: light/dark/custom
    pub refresh_rate_ms: u64,
    pub enable_animations: bool,
    // Pane-specific UI preferences (selected indices, view modes, etc.)
}
```

Configuration is saved to `~/.config/gat-tui/config.toml` and loaded on startup. This prepares for gat-ywa (configuration persistence).

### State Layer (Transient)

```rust
#[derive(Debug, Clone)]
pub struct AppState {
    // Persistent config (can be saved/loaded)
    pub config: TuiConfig,

    // Transient state (resets on restart)
    pub datasets: DatasetsState,
    pub dashboard: DashboardState,
    pub operations: OperationsState,
    pub pipeline: PipelineState,
    pub commands: CommandsState,
    pub workflows: Vec<WorkflowStatus>,
    pub metrics: SystemMetrics,

    // Navigation (transient)
    pub navigation_level: NavigationLevel,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: TuiConfig::default(),
            datasets: DatasetsState::with_fixtures(),
            dashboard: DashboardState::with_fixtures(),
            // ... other pane states
            workflows: create_fixture_workflows(),
            metrics: create_fixture_metrics(),
            navigation_level: NavigationLevel::MenuBar,
        }
    }
}
```

**Key Design Decisions:**

1. **Single AppState** - Replaces scattered pane-level state in main.rs
2. **Pane states remain separate** - Each pane owns its section/focus state
3. **Shared data at top level** - Workflows, metrics accessible across panes
4. **Fixture data** - Use `with_fixtures()` methods for immediate rendering (Phase 1)
5. **Config separation** - TuiConfig is serializable for future save/load

## Integration in main.rs

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ... terminal setup ...

    // Create single source of truth
    let mut app_state = AppState::new();

    // Main event loop
    loop {
        // Render all panes using app_state
        render_all_panes(&mut terminal, &app_state)?;

        // Handle events
        if let Ok(event) = event::read() {
            match event {
                Event::Key(key) => handle_key_event(&mut app_state, key),
                _ => {}
            }
        }
    }
}
```

All rendering functions accept `&AppState` instead of individual pane states.

## Fixture Data Strategy

Fixture data lives in `src/data/fixtures.rs`:

```rust
pub fn create_fixture_datasets() -> Vec<DatasetEntry> {
    vec![
        DatasetEntry {
            id: "opsd-2024".to_string(),
            name: "OPSD Snapshot".to_string(),
            status: DatasetStatus::Ready,
            source: "OPSD".to_string(),
            row_count: 8_760,
            size_mb: 245.3,
            last_updated: SystemTime::now() - Duration::from_secs(3600),
            description: "Open Power System Data hourly generation".to_string(),
        },
        // ... more fixtures
    ]
}
```

Fixtures provide:
- Immediate rendering on startup (no wait for data loading)
- Testing foundation (deterministic data)
- Pattern for later integration with gat-core (Phase 2)

## Implementation Steps

1. Create `src/data/mod.rs` and `src/data/fixtures.rs`
2. Define all data structures (DatasetEntry, DatasetsState, AppState, etc.)
3. Create `AppState::new()` with fixture data
4. Replace current pane states in main.rs with `&app_state` parameter passing
5. Update all render functions to accept `&AppState`
6. Verify all five panes still render correctly with new state structure

## Future Enhancements (Later Phases)

- **Phase 2 (gat-xad):** Replace fixture data with real gat-core queries
- **Phase 3 (gat-ywa):** Implement TuiConfig save/load to file
- **Phase 4:** Add async data fetching with loading indicators
- **Phase 5:** Implement state validation and error recovery

## Testing Strategy

- Unit tests for data structures (serialization, defaults)
- Integration tests for AppState initialization
- Render function tests with fixture data
- Navigation state transitions
