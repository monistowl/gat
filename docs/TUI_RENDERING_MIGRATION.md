# gat-tui Rendering Fix: Migration to tuirealm 3.2

## The Problem

The original gat-tui implementation used a **custom string-based rendering system** that generated plain text output and wrote it directly to stdout:

```rust
// OLD APPROACH (broken)
let rendered = shell.render();  // Returns a String
stdout.write_all(rendered.as_bytes())?;
stdout.flush()?;
```

This approach has a fundamental flaw: **plain text written to stdout cannot be positioned at exact terminal coordinates**. Terminal emulators render each character with variable spacing depending on:
- Terminal font rendering
- Anti-aliasing settings
- Unicode character width handling
- Platform-specific glyph rendering

**Result**: Text that appears correctly aligned in the code would render "staggered" on different terminals or emulators.

## Root Cause Analysis

The rendering pipeline had three critical issues:

### 1. **No Coordinate System**
Custom string generation (`AppShell::render()`, `Pane::render_into()`, etc.) calculated indentation using spaces:
```rust
let pad = THEME.indent(indent);  // Just spaces, no coordinates
writeln!(output, "{}item", pad);  // Appended to string
```

When written as raw text, spacing becomes dependent on terminal rendering, not on explicit positioning.

### 2. **No Proper Terminal API Integration**
The app used crossterm for event handling but **bypassed its coordinate system** for rendering:
```rust
execute!(stdout, Clear::All, MoveTo(0, 0))?;  // Position cleared
stdout.write_all(rendered.as_bytes())?;  // Raw string (position ignored!)
```

### 3. **No Framework Structure**
- No Component trait → no reusable UI elements
- No Application management → no proper event lifecycle
- No TerminalBridge → rendering not synchronized with terminal state
- Tests only checked for text content, not actual visual positioning

## The Solution: tuirealm 3.2

tuirealm is a **framework for building TUIs with reusable components**. It properly handles terminal coordination through three key concepts:

### 1. **Component Trait** (Event Handling)
```rust
impl Component<Msg, NoUserEvent> for Label {
    fn on(&mut self, ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        match ev {
            TuiEvent::Keyboard(KeyEvent { code: Key::Esc, .. }) => Some(Msg::AppClose),
            _ => None,
        }
    }
}
```

### 2. **MockComponent Trait** (Rendering)
```rust
impl MockComponent for Label {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // frame has access to exact coordinates via Frame trait
        // area provides precise x, y, width, height positioning
        frame.render_widget(Paragraph::new(text).style(style), area);
    }
}
```

### 3. **TerminalBridge** (Coordinate-Based Rendering)
```rust
let mut terminal = TerminalBridge::init(CrosstermTerminalAdapter::new()?)?;

terminal.draw(|f| {
    // f.area() provides exact terminal dimensions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(f.area());

    // Rendering happens at precise coordinates via ratatui
    f.render_widget(paragraph, chunks[0]);
})?;
```

## Key Architecture Changes

### Before (Custom String Rendering)
```
Application state
  → PaneLayout::render_into() (generates strings)
  → AppShell::render() (concatenates strings)
  → main() writes string to stdout
```

### After (tuirealm + ratatui)
```
Application state
  → Component trait (event handling)
  → MockComponent::view() (renders to Frame via ratatui)
  → TerminalBridge::draw() (coordinates rendering)
  → main() manages terminal lifecycle
```

## Implementation Details

### New main.rs Structure

**1. Component Definition** (Label)
- Stores properties (text, color, style)
- Implements MockComponent for view rendering
- Implements Component for event handling
- Returns messages (Msg enum) for state changes

**2. Application Setup**
```rust
let mut app: Application<Id, Msg, NoUserEvent> = Application::init(event_listener);

// Mount components at specific IDs
app.mount(Id::Dashboard, Box::new(label_component), vec![])?;

// Activate a component
app.active(&Id::Dashboard)?;
```

**3. Event Loop**
```rust
while !should_quit {
    // Render using TerminalBridge (coordinate-based)
    terminal.draw(|f| {
        app.view(&current_pane, f, chunk);
    })?;

    // Handle events (polling)
    match app.tick(PollStrategy::Once) {
        Ok(messages) => {
            for msg in messages {
                match msg {
                    Msg::AppClose => should_quit = true,
                    Msg::SwitchPane(pane) => current_pane = pane,
                }
            }
        }
        _ => {}
    }
}
```

## Testing the Fix

To verify the fix works properly:

```bash
# Build and run the new TUI
cargo run -p gat-tui --release

# You should see:
# 1. Properly aligned text (no staggering)
# 2. Correct colors and styling
# 3. Responsive pane switching (press 1-5)
# 4. Clean exit (press ESC or Q)
# 5. Works identically in different terminals (gnome-terminal, urxvt, etc.)
```

## Migration Path for Full Feature Set

The current implementation is a **functional framework** with placeholder content. To fully migrate the existing panes:

1. **Create tuirealm Components for each pane type**:
   - DashboardComponent (MockComponent + Component)
   - OperationsComponent
   - DatasetsComponent
   - PipelineComponent
   - CommandsComponent

2. **Implement proper rendering** in each MockComponent::view():
   - Use ratatui widgets (Table, List, Paragraph, Block, Gauge, etc.)
   - Apply color and styling through Style + TextModifiers
   - Calculate layouts using ratatui's Layout system

3. **Implement event handling** in each Component::on():
   - Navigation (arrow keys, tab, page up/down)
   - Item selection and scrolling
   - Modal interactions

4. **Replace old pane system**:
   - Remove `ui/layout.rs`, `ui/mod.rs` custom rendering
   - Remove `ui/ansi.rs`, `ui/navigation.rs` (handled by tuirealm/ratatui)
   - Keep `panes/` structure but convert to tuirealm Components

5. **Update tests**:
   - Tests should now verify Component behavior, not string output
   - Use tuirealm's testing patterns

## Comparison: String-Based vs Component-Based

| Aspect | String-Based | tuirealm + ratatui |
|--------|--------------|-------------------|
| **Rendering** | Text concatenation → stdout | Component trait → TerminalBridge → coordinates |
| **Positioning** | Indentation via spaces (fragile) | Layout system with exact coordinates |
| **Colors** | ANSI codes generated manually | ratatui Style system (automatic) |
| **Event Handling** | Custom logic in main() | Component::on() trait (structured) |
| **Reusability** | Custom types per pane | Generic Component trait |
| **Terminal Compatibility** | Variable (depends on rendering) | Guaranteed (uses crossterm/ratatui) |
| **Testing** | Test string output | Test component messages |

## Why This Fixes the "Staggered Text" Issue

1. **Exact Coordinates**: TerminalBridge uses crossterm's cursor positioning, not text spacing
2. **Proper Terminal API**: ratatui handles all ANSI codes and escape sequences
3. **Layout System**: ratatui's Layout calculates exact boundaries before rendering
4. **Terminal Independence**: Works identically across all terminal emulators (gnome-terminal, urxvt, etc.)

The "staggered" appearance was caused by **relying on text spacing for layout**, which varies by terminal. tuirealm/ratatui use **absolute coordinate positioning**, which is consistent everywhere.

## References

- [tuirealm Documentation](https://docs.rs/tuirealm/3.2/)
- [ratatui Documentation](https://docs.rs/ratatui/latest/)
- [crossterm Terminal Documentation](https://docs.rs/crossterm/latest/)
