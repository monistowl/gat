# Reactive TUI Architecture Design

**Date:** 2025-11-24
**Status:** Design Phase
**Author:** Claude (with user validation)

## Executive Summary

Transform GAT's terminal UI from static rendering to a fully reactive, GUI-like experience with mouse interactions, real-time updates, modal dialogs, visual feedback, and interactive grid visualization. Architecture based on unidirectional data flow (Redux/Elm pattern) adapted for TUI constraints.

## Goals

### Primary Objectives
- **Mouse interactions**: Click buttons, select items, navigate with mouse like desktop app
- **Real-time updates**: Live-refreshing data, streaming logs, auto-updating progress
- **Modal dialogs**: Pop-up forms, confirmation dialogs, multi-step wizards
- **Visual feedback**: Hover effects, focus indicators, animations, smooth transitions
- **Grid visualization**: Interactive node-edge graphs for power system topology

### Success Criteria
**Rich visualization** - Engineers should see data presented visually (graphs, charts, diagrams) not just tables and text. The TUI should feel like a desktop app through visual quality.

### Constraints
1. **80x24 compatibility**: Must work at minimal terminal size with graceful degradation
2. **No external dependencies**: Stay within tuirealm/ratatui/crossterm stack
3. **Fast rendering**: Smooth updates even on large datasets (1000+ buses, complex graphs)

## Architecture Overview

### Unidirectional Data Flow

```
User Input → Events → State Updates → Component Re-render → Terminal Output
     ↑                                                              ↓
     └──────────────── Async Effects (timers, jobs) ───────────────┘
```

### Core Components

1. **AppState**: Single source of truth holding all pane states, grid data, running jobs
2. **Event Bus**: Dispatches events (keyboard, mouse, timers, async results)
3. **Reducers**: Pure functions that update state based on events
4. **Components**: Subscribe to state slices, render when their data changes
5. **Effects**: Handle async operations (API calls, timers) and emit events when complete

### Why This Architecture

- **Tuirealm foundation**: Already uses event-driven model via `tui-realm-stdlib`
- **Centralized state**: Prevents desyncs between panes
- **Subscription model**: Enables real-time updates without polling
- **Testability**: Pure reducers, isolated effects

## State Management

### AppState Structure

```rust
pub struct AppState {
    // Navigation
    active_pane: PaneId,
    modal_stack: Vec<Modal>,

    // Pane states (each owns its data)
    dashboard: DashboardState,
    datasets: DatasetsState,
    operations: OperationsState,
    pipeline: PipelineState,
    analytics: AnalyticsState,

    // Shared/global state
    current_grid: Option<GridId>,
    running_jobs: HashMap<JobId, JobProgress>,
    notifications: VecDeque<Notification>,

    // UI state
    terminal_size: (u16, u16),
    mouse_position: Option<(u16, u16)>,
    focus_chain: Vec<ComponentId>,
}
```

### Reducers

```rust
type Reducer = fn(&mut AppState, Event) -> Vec<Effect>;

// Example: Job progress update
fn update_job_progress(state: &mut AppState, event: Event) -> Vec<Effect> {
    if let Event::JobProgress { id, pct } = event {
        state.running_jobs.get_mut(&id).map(|job| job.progress = pct);
        vec![Effect::Render(RenderRegion::Operations)]
    } else {
        vec![]
    }
}
```

### Benefits

- **Single source of truth**: Prevents state desyncs between panes
- **Pure reducers**: Easy to test, predictable behavior
- **Isolated effects**: Async operations don't block UI
- **Selective re-rendering**: Only update changed regions

## Event System

### Event Types

```rust
pub enum Event {
    // Input events
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),

    // Timer events
    Tick,                    // Fast tick (60 FPS for animations)
    SlowTick,                // Slow tick (1 Hz for job polling)

    // Async completion events
    JobProgress { id: JobId, pct: f64 },
    JobComplete { id: JobId, result: JobResult },
    DataRefresh { pane: PaneId, data: Box<dyn Any> },

    // UI events
    FocusChanged { from: ComponentId, to: ComponentId },
    ModalOpen(Modal),
    ModalClose,
}
```

### Main Event Loop

```rust
loop {
    // Poll multiple event sources with timeout
    match event_rx.recv_timeout(Duration::from_millis(16)) {
        Ok(event) => {
            // Dispatch to reducers
            let effects = app_state.reduce(event);

            // Execute effects (spawn async tasks, schedule renders)
            for effect in effects {
                executor.execute(effect);
            }
        }
        Err(Timeout) => {
            // Emit tick event for animations/polling
            let effects = app_state.reduce(Event::Tick);
            executor.execute_all(effects);
        }
    }

    // Render if dirty
    if app_state.needs_render() {
        terminal.draw(|f| app_state.render(f))?;
        app_state.clear_dirty();
    }
}
```

### Real-Time Update Patterns

1. **Streaming job progress**: Background task polls job status, emits `JobProgress` events
2. **Live data refresh**: Timer triggers API calls, results emit `DataRefresh` events
3. **Animations**: Fast tick (16ms) drives spinner frames, progress bar smoothing
4. **Debouncing**: Mouse move events batched to avoid render thrashing

## Interactive Components

### Mouse Event Handling

```rust
impl Component for InteractivePane {
    fn on(&mut self, event: Event) -> Option<Msg> {
        match event {
            Event::Mouse(MouseEvent::Down(MouseButton::Left, x, y)) => {
                // Hit test: which element was clicked?
                if let Some(target) = self.hit_test(x, y) {
                    Some(Msg::Clicked(target))
                }
            }
            Event::Mouse(MouseEvent::Moved(x, y)) => {
                // Update hover state for visual feedback
                self.hovered_element = self.hit_test(x, y);
                Some(Msg::Render)
            }
            _ => None
        }
    }
}
```

### Visual Feedback Patterns

```rust
// Hover effects (color/style changes)
let style = if self.is_hovered() {
    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
} else {
    Style::default()
};

// Focus indicators (border changes)
let border_type = if self.is_focused() {
    BorderType::Double
} else {
    BorderType::Rounded
};

// Button press animations (brief color flash)
if self.just_clicked {
    Style::default().bg(Color::DarkGray)  // Flash for 1 frame
}
```

### Modal System

```rust
pub struct ModalStack {
    modals: Vec<Box<dyn ModalComponent>>,
    backdrop_opacity: u8,
}

// Modal types
enum Modal {
    Confirmation { title: String, message: String, on_confirm: Callback },
    Form { fields: Vec<FormField>, on_submit: Callback },
    CommandPalette { commands: Vec<Command>, filter: String },
    GridViewer { graph: GridGraph, viewport: Viewport },
}

// Rendering: modals draw over main content with backdrop
fn render_with_modals(f: &mut Frame, state: &AppState) {
    // 1. Render main pane
    render_active_pane(f, state);

    // 2. Render backdrop (dimmed overlay)
    if !state.modal_stack.is_empty() {
        render_backdrop(f, state.modal_stack.backdrop_opacity);
    }

    // 3. Render modal stack (top modal gets focus)
    for modal in &state.modal_stack.modals {
        modal.render_centered(f);
    }
}
```

## Grid Visualization

### GridGraphComponent

```rust
pub struct GridGraphComponent {
    nodes: Vec<BusNode>,      // Position, voltage level, status
    edges: Vec<Branch>,        // From/to, flow direction, utilization
    layout: LayoutAlgorithm,   // Force-directed, hierarchical, or geographic
    viewport: Viewport,        // Pan/zoom state for large grids
    interaction: InteractionMode, // Pan, select, inspect
}
```

### Rendering Example

Using box-drawing characters for edges, symbols for nodes:

```
  Gen1──┬──Bus1═══Bus2──Load1
        │       ║
       Bus3═══Bus4──Load2
```

### Mouse Interactions

- **Click node**: Inspect bus details (voltage, load, generation)
- **Drag**: Pan viewport for large grids
- **Scroll**: Zoom in/out
- **Hover**: Show tooltip with bus/branch info

## Responsive Design

### Terminal Size Adaptation

```rust
pub struct ResponsiveLayout {
    // Breakpoints
    compact: (u16, u16),    // 80x24  - minimal layout
    standard: (u16, u16),   // 120x40 - balanced layout
    wide: (u16, u16),       // 160x50+ - full features
}

impl AppState {
    fn on_resize(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);

        // Adapt layout based on size
        match (width, height) {
            (w, h) if w < 100 || h < 30 => {
                // Compact mode: hide sidebars, collapse secondary panes
                self.layout_mode = LayoutMode::Compact;
                self.sidebar_visible = false;
            }
            (w, h) if w < 140 || h < 40 => {
                // Standard mode: balanced layout
                self.layout_mode = LayoutMode::Standard;
                self.sidebar_visible = true;
            }
            _ => {
                // Wide mode: expand visuals, show all panels
                self.layout_mode = LayoutMode::Wide;
                self.sidebar_visible = true;
                self.expand_graphs = true;
            }
        }
    }
}
```

### Layout Modes

| Mode | Terminal Size | Features |
|------|--------------|----------|
| **Compact** | 80x24 - 100x30 | Single pane, collapsed sidebars, text-heavy |
| **Standard** | 100x30 - 140x40 | Balanced layout, sidebars visible, moderate visuals |
| **Wide** | 140x40+ | Full features, expanded graphs, all panels |

## Performance Optimizations

### 1. Dirty Tracking

Only re-render changed regions:

```rust
pub struct DirtyRegions {
    regions: HashSet<RenderRegion>,
}
```

### 2. Virtual Scrolling

For large datasets:

```rust
pub struct VirtualList<T> {
    items: Vec<T>,
    viewport_start: usize,
    viewport_size: usize,
    // Only render visible items + small buffer
}
```

### 3. Incremental Graph Layout

```rust
pub struct IncrementalGraphLayout {
    positions: HashMap<NodeId, (f32, f32)>,
    dirty_nodes: HashSet<NodeId>,
    // Only recalculate positions for changed nodes
}
```

### 4. Debounced Events

```rust
pub struct DebouncedEventSource {
    last_emit: Instant,
    min_interval: Duration,
    // Batch rapid events (e.g., mouse moves)
}
```

### Memory Budget for Large Grids

- **Streaming rendering**: Process grid in chunks, don't load entire network into memory
- **Level-of-detail**: Simplify visualization for distant/small elements
- **Paging**: Load bus/branch details on-demand when inspecting nodes

## Component Library

### New Components

```rust
// Collapsible sections for space management
pub struct Collapsible {
    title: String,
    expanded: bool,
    content: Box<dyn Component>,
    expand_icon: &'static str,  // "▶" collapsed, "▼" expanded

    // Keyboard: Space/Enter to toggle
    // Mouse: Click header to toggle
}

// Accordion (only one section expanded at a time)
pub struct Accordion {
    sections: Vec<Collapsible>,
    active: usize,
}

// Command palette (Ctrl+K style quick access)
pub struct CommandPalette {
    commands: Vec<Command>,
    filter: String,
    fuzzy_matcher: FuzzyMatcher,
    // Type to filter, Enter to execute
}

// Inline sparklines for compact trend visualization
pub struct Sparkline {
    data: Vec<f64>,
    width: usize,
    style: SparklineStyle,  // Line, Bar, or Area
}
```

## Testing Strategy

### State Reducer Tests (Pure Functions)

```rust
#[test]
fn test_job_progress_reducer() {
    let mut state = AppState::default();
    let event = Event::JobProgress { id: 123, pct: 0.75 };

    let effects = reduce_job_progress(&mut state, event);

    assert_eq!(state.running_jobs[&123].progress, 0.75);
    assert!(effects.contains(&Effect::Render(RenderRegion::Operations)));
}
```

### Component Interaction Tests

```rust
#[test]
fn test_collapsible_toggle() {
    let mut collapsible = Collapsible::new("Section", true);

    collapsible.on(Event::Key(KeyCode::Enter));
    assert_eq!(collapsible.expanded, false);

    collapsible.on(Event::Mouse(MouseEvent::Click(0, 0)));
    assert_eq!(collapsible.expanded, true);
}
```

### Integration Tests with Virtual Terminal

```rust
#[test]
fn test_modal_keyboard_navigation() {
    let mut app = TestApp::new();
    app.open_modal(Modal::confirmation("Save changes?"));

    app.send_key(KeyCode::Tab);  // Focus "Cancel"
    app.send_key(KeyCode::Tab);  // Focus "Confirm"
    app.send_key(KeyCode::Enter);

    assert!(app.state.modal_stack.is_empty());
    assert_eq!(app.last_action, Action::Confirmed);
}
```

### Snapshot Testing for Layouts

```rust
#[test]
fn test_compact_layout_80x24() {
    let app = AppState::new();
    let output = render_to_string(&app, 80, 24);

    insta::assert_snapshot!(output);
}
```

## Implementation Phases

### Phase 1: Foundation (Weeks 1-2)
- Event system and event loop
- AppState structure and reducer pattern
- Basic mouse event handling
- Dirty region tracking

### Phase 2: Core Components (Weeks 3-4)
- Modal system with backdrop
- Collapsible sections
- Visual feedback (hover, focus, press)
- Command palette

### Phase 3: Grid Visualization (Weeks 5-6)
- GridGraphComponent with layout algorithms
- Viewport management (pan/zoom)
- Interactive node/edge inspection
- Performance optimization for large grids

### Phase 4: Real-Time Updates (Week 7)
- Timer system (fast/slow ticks)
- Job progress streaming
- Live data refresh
- Animation system

### Phase 5: Responsive Design (Week 8)
- Breakpoint system
- Layout mode adaptation
- Virtual scrolling for large lists
- Performance tuning

### Phase 6: Polish & Testing (Week 9)
- Comprehensive test coverage
- Snapshot tests for all layouts
- Performance benchmarks
- Documentation

## Future Adaptations

This architecture supports future enhancements:

- **Live monitoring**: Real-time telemetry streaming from SCADA/EMS systems
- **Multi-user collaboration**: Shared state synchronization across terminals
- **Replay mode**: Time-travel debugging through event log replay
- **Plugin system**: Third-party components subscribing to event bus
- **Remote access**: Thin client rendering via SSH with low-bandwidth optimization

## Technical Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tuirealm API limitations | High | Review tuirealm-stdlib and ratatui source for extension points |
| Performance degradation on large grids | Medium | Implement virtual scrolling and level-of-detail early |
| Complex state management | Medium | Start with simple reducers, add complexity incrementally |
| Testing difficulty with TUI | Low | Use virtual terminal for integration tests |

## Success Metrics

1. **User feedback**: Engineers report TUI feels like desktop app
2. **Performance**: Smooth 60 FPS rendering on standard grids (100-500 buses)
3. **Test coverage**: >80% coverage for reducers and components
4. **Responsive**: All features work at 80x24, enhanced features at larger sizes
5. **Adoption**: Increased TUI usage over CLI for interactive workflows

## Conclusion

This reactive architecture transforms GAT's TUI from static rendering to a rich, interactive experience that rivals desktop applications. By leveraging unidirectional data flow, we achieve predictable state management, real-time updates, and smooth visual feedback while maintaining compatibility with minimal terminal sizes.

The design prioritizes **rich visualization** as the primary success criterion, ensuring power system engineers can explore complex grid data through interactive graphs, real-time charts, and responsive UI components.
