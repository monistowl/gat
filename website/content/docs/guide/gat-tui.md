+++
title = "TUI Reference"
description = "`gat-tui` architecture and panel registry"
weight = 41
+++

# `gat-tui` architecture and panel registry

The terminal UI is intentionally small but composable so panes can be rearranged without touching the rendering glue. Everything hangs off a shared `PaneContext` that carries default tooltips and the command modal, and is passed into every pane when the navigation menu is built.

## Application shell layout

- `App::new` seeds a `CommandModal` with starter text, wraps it in a `PaneContext`, and hands that context to a `PanelRegistry`.
- `PanelRegistry::register` collects `PaneView` implementations and turns them into `MenuItem` entries (label, hotkey, tooltip, context buttons) with pre-computed `PaneLayout` trees.
- `PanelRegistry::into_shell` produces an `AppShell` that knows how to render the menu bar, active layout, fallback tooltip, and command modal output at a fixed viewport for snapshots.
- The `NavMenu` drives focus changes via hotkeys and exposes the active tooltip so tooltips can come from either the shell default or the current pane.

## Adding a pane

1. Implement `PaneView` for a new struct in `crates/gat-tui/src/panes/`.
   - Provide a unique `id`, a concise `label`, and a single-character `hotkey`.
   - Build the visual tree in `layout` using `PaneLayout::new`, `Pane`, `Sidebar`, `SubTabs`, or `TableView` as needed.
   - Return optional `Tooltip` text and `ContextButton`s so the menu bar can hint at shortcuts.
2. Register the pane in `App::new` by chaining another `.register(NewPane)` call on the `PanelRegistry`.
3. If the pane needs modal access, read it from the `PaneContext` (which already owns the command modal) rather than creating a duplicate.

## UX rules of thumb

- Every pane should render with readable defaults in an 110×32 viewport: prefer short sentences, compact tables, and minimal nesting.
- Hotkeys must be unique across the menu and context buttons; the menu bar should show the active item with `[*]` and a short actions list when available.
- Tooltips should tell operators which focus switches are available (e.g., when a layout will swap visualizers on wide screens).
- Secondary content should collapse gracefully: use `ResponsiveRules` to hide visuals first, and keep empty states explicit with `EmptyState` so gaps are intentional.
- The command modal is the only place to run commands. Keep pane text focused on navigation and discovery; use the modal help and examples to teach invocation patterns.
