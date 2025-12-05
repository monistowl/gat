+++
title = "TUI Reference"
description = "`gat-tui` architecture, panel registry, and analytics tabs"
weight = 41
+++

# `gat-tui` Architecture

The terminal UI is intentionally small but composable so panes can be rearranged without touching the rendering glue. Everything hangs off a shared `PaneContext` that carries default tooltips and the command modal, and is passed into every pane when the navigation menu is built.

## Seven-Pane Layout

| Pane | Hotkey | Description |
|------|--------|-------------|
| Dashboard | `1` | System health, KPIs, quick actions |
| Commands | `2` | Snippet library, execution history |
| Datasets | `3` | Catalog browser, uploads, scenarios |
| Pipeline | `4` | Workflow DAG visualization |
| Operations | `5` | Batch job monitor, allocation results |
| Analytics | `6` | Seven analysis tabs (see below) |
| Settings | `7` | Display, data, execution preferences |

## Analytics Pane Tabs

The Analytics pane provides comprehensive grid analysis in the terminal:

| Tab | Key | Description |
|-----|-----|-------------|
| Reliability | `Tab` | LOLE, EUE, thermal violations |
| Deliverability | | Delivery capability by zone |
| ELCC | | Effective load carrying capability |
| Power Flow | | Congestion hotspots, voltage violations |
| **Contingency** | `c` | N-1 security screening |
| **PTDF** | `p` | Transfer sensitivity factors |
| **Y-bus** | `y` | Admittance matrix explorer |

### N-1 Contingency Tab

Systematic single-branch outage screening:
- Summary: total contingencies, violations, failed solves
- Status badge: "SECURE" (green) or "N VIOLATIONS" (red)
- Sortable results table: outage branch, max loading %, violation count

### PTDF Tab

Power Transfer Distribution Factor analysis:
- Bus selection: injection (`i`) and withdrawal (`w`) buses
- Transfer sensitivity: PTDF factors for each branch (-1 to +1)
- Flow change preview: MW impact for 100 MW transfer
- Branch ranking by absolute PTDF factor

### Y-bus Matrix Tab

Interactive admittance matrix visualization:
- Three view modes (cycle with `v`):
  - **Heatmap**: ASCII grid with `░▒▓█` by magnitude
  - **List**: Table of (row, col, G, B, magnitude)
  - **Sparsity**: Pattern with `·` for zero, `█` for non-zero

## Application Shell Layout

- `App::new` seeds a `CommandModal` with starter text, wraps it in a `PaneContext`, and hands that context to a `PanelRegistry`.
- `PanelRegistry::register` collects `PaneView` implementations and turns them into `MenuItem` entries (label, hotkey, tooltip, context buttons) with pre-computed `PaneLayout` trees.
- `PanelRegistry::into_shell` produces an `AppShell` that knows how to render the menu bar, active layout, fallback tooltip, and command modal output at a fixed viewport for snapshots.
- The `NavMenu` drives focus changes via hotkeys and exposes the active tooltip so tooltips can come from either the shell default or the current pane.

## Adding a Pane

1. Implement `PaneView` for a new struct in `crates/gat-tui/src/panes/`.
   - Provide a unique `id`, a concise `label`, and a single-character `hotkey`.
   - Build the visual tree in `layout` using `PaneLayout::new`, `Pane`, `Sidebar`, `SubTabs`, or `TableView` as needed.
   - Return optional `Tooltip` text and `ContextButton`s so the menu bar can hint at shortcuts.
2. Register the pane in `App::new` by chaining another `.register(NewPane)` call on the `PanelRegistry`.
3. If the pane needs modal access, read it from the `PaneContext` (which already owns the command modal) rather than creating a duplicate.

## Adding an Analytics Tab

1. Add variant to `AnalyticsTab` enum in `analytics_pane.rs`
2. Add result struct (e.g., `NewAnalysisResultRow`)
3. Add state fields to `AnalyticsPaneState`
4. Update `next_tab()`/`prev_tab()` cycle count
5. Add `is_*_tab()` query method
6. Add selection/detail methods
7. Update `update_metrics_list()` and `format_summary()`

## GridService for Analysis

The `GridService` manages loaded networks and provides analysis methods:

```rust
let service = GridService::new();
let grid_id = service.load_grid_from_arrow("case14.arrow")?;

// Y-bus admittance matrix
let (n_bus, entries) = service.get_ybus(&grid_id)?;

// PTDF for a transfer (bus 1 → bus 5)
let ptdf_results = service.compute_ptdf(&grid_id, 1, 5)?;

// N-1 contingency screening
let contingencies = service.run_n1_contingency(&grid_id)?;
```

## UX Rules of Thumb

- Every pane should render with readable defaults in an 110×32 viewport: prefer short sentences, compact tables, and minimal nesting.
- Hotkeys must be unique across the menu and context buttons; the menu bar should show the active item with `[*]` and a short actions list when available.
- Tooltips should tell operators which focus switches are available (e.g., when a layout will swap visualizers on wide screens).
- Secondary content should collapse gracefully: use `ResponsiveRules` to hide visuals first, and keep empty states explicit with `EmptyState` so gaps are intentional.
- The command modal is the only place to run commands. Keep pane text focused on navigation and discovery; use the modal help and examples to teach invocation patterns.
