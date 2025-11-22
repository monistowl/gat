# gat-tui Navigation Guide

## Hierarchical Navigation Model

gat-tui uses a consistent three-level hierarchical navigation model that enables intuitive exploration of the application:

```
┌─────────────────────────────────────────┐
│        Level 0: Menu Bar                │
│   (Select which pane to view)           │
│  Dashboard  Operations  Datasets  ...   │
└─────────────────────────────────────────┘
           ↓ (Enter/Down)
┌─────────────────────────────────────────┐
│     Level 1: Pane Sections              │
│    (Navigate within current pane)       │
│  Status | Metrics | Activity | Actions  │
└─────────────────────────────────────────┘
           ↓ (Enter/Down) [Future]
┌─────────────────────────────────────────┐
│   Level 2: Fields/Items [Future]        │
│  (Edit/interact with specific fields)   │
│        Column 1 | Column 2 | ...        │
└─────────────────────────────────────────┘
```

## Keyboard Controls

### Menu Bar Level (Pane Selection)

When the menu bar is focused (shown in bold cyan):

| Key | Action |
|-----|--------|
| `←` (Left arrow) | Switch to previous pane (wraps around) |
| `→` (Right arrow) | Switch to next pane (wraps around) |
| `↓` (Down arrow) | Enter selected pane (move to Level 1) |
| `Enter` | Enter selected pane (move to Level 1) |
| `ESC` | Return to menu bar (if in pane) |
| `Q` | Quit application |

**Example Navigation:**
```
Dashboard → Operations → Datasets → Pipeline → Commands → Dashboard
```

### Pane Content Level (Section Selection)

When inside a pane (sections highlighted with navigation):

| Key | Action |
|-----|--------|
| `↑` (Up arrow) | Select previous section |
| `↓` (Down arrow) | Select next section |
| `ESC` | Return to menu bar (Level 0) |
| `Q` | Quit application |

**Visual Indicator:**
- Selected section displays **bold** text
- Unselected sections show normal text
- Navigation is context-aware (respects pane boundaries)

## Pane Overview

### Dashboard (4 Sections)
Navigate through:
1. Status - Overall system health and workflow status
2. Metrics - Reliability indicators (DS, LOLE, EUE)
3. Recent Activity - Recent workflow runs and their status
4. Quick Actions - Available actions (Enter to run, R to retry, E to edit)

### Operations (4 Sections)
Navigate through:
1. DERMS/ADMS Queue - Envelope queue and stress test status
2. Batch Operations - Batch job queue and parallelism settings
3. Allocation - Rents decomposition and KPI analysis
4. Status Summary - Quick overview of all operations

### Datasets (3 Sections)
Navigate through:
1. Data Catalog - Available datasets and sources
2. Workflows - Data ingestion and transformation workflows
3. Downloads - Queued data fetches and sample data

### Pipeline (3 Sections)
Navigate through:
1. Source Selection - Data source and variant selection
2. Transforms - Available transformations and features
3. Outputs - Output destinations and delivery methods

### Commands (3 Sections)
Navigate through:
1. Workspace - Editor for gat-cli command snippets
2. Command Snippets - Saved command templates
3. Recent Results - History of executed commands

## Navigation Rules

1. **Escape is Context-Aware:**
   - In Menu Bar: ESC quits the application
   - In Pane: ESC returns to Menu Bar

2. **Wrapping:**
   - Menu bar Left/Right wraps around (Dashboard ↔ Commands)
   - Pane Up/Down stops at boundaries

3. **Selection Persistence:**
   - Section selection is maintained when you return to a pane
   - Menu position is persistent across the session

4. **Future Enhancements:**
   - Level 2 (Fields) will be entered with Enter key
   - Left/Right arrows will be used for field navigation in future releases
   - Number keys (1-5) are reserved for command input/hotkeys

## Usage Examples

### Example 1: Browse Metrics
```
Start (Menu bar focused)
  → Press Down to enter Dashboard
  → Press Down twice to reach Metrics section
  → [View metrics]
  → Press ESC to return to menu bar
```

### Example 2: Navigate Between Panes
```
Currently in Dashboard
  → Press ESC to return to menu bar
  → Press Right twice to move: Dashboard → Operations → Datasets
  → Press Down to enter Datasets pane
```

### Example 3: Quit from Anywhere
```
[Any location]
  → Press Q to quit immediately
  → Or: Press ESC to menu bar, then press ESC/Q to quit
```

## Tips for Users

- **Discover Available Options:** Navigate sections within each pane to see what's available
- **Remember Locations:** Your selected section is remembered for each pane
- **Use Keyboard Exclusively:** All features are accessible via keyboard navigation
- **Q for Quick Exit:** You can quit from anywhere with Q
- **Menu Context Help:** The menu bar shows available actions for your current level

## Terminal Compatibility

Tested and working on:
- gnome-terminal
- urxvt (rxvt-unicode)
- Other standard ANSI-compatible terminals

## Future Navigation Enhancements

As gat-tui grows, the navigation model will support:
- **Field-level editing:** Enter sections to edit form fields
- **Table interaction:** Up/Down for rows, Left/Right for columns
- **Command mode:** Number keys for rapid command input
- **Modal dialogs:** Confirmation and input dialogs with Tab navigation
- **Keyboard shortcuts:** Single-key actions within sections (R for retry, E for edit, etc.)

The hierarchical model ensures all new features will integrate naturally while maintaining consistent, discoverable navigation.
