# AppState Implementation Plan - REVISED (gat-eqb)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate the existing data module into gat-tui, export dataset structures from lib.rs, and populate the Datasets pane with fixture dataset metadata.

**Reality Check:** AppState already exists in `models.rs` with comprehensive structure (pane states, settings, modals, notifications, etc.). A partial `data` module with DatasetEntry and fixtures already exists on disk but isn't wired into the library. gat-eqb's real task is to:
1. Export the data module from lib.rs
2. Create a data domain model that sits alongside the UI state model
3. Populate Datasets pane with fixture data instead of placeholder text

**Architecture:** Data structures live in `src/data/` (already created). AppState in `models.rs` remains the UI state container. Rendering functions in panes/datasets.rs will use both AppState (for UI state like scroll position) and data structures (for content like datasets list).

**Tech Stack:** Rust, serde, existing models.rs AppState

---

## Task 1: Export data module from lib.rs

**Files:**
- Modify: `crates/gat-tui/src/lib.rs`

**Step 1: Add data module declaration**

Edit `crates/gat-tui/src/lib.rs`, add after line 17 (after `pub mod ui;`):

```rust
pub mod data;
```

**Step 2: Add data module exports**

Edit `crates/gat-tui/src/lib.rs`, add to the pub use section (after line 24):

```rust
pub use data::{DatasetEntry, DatasetStatus, DatasetsState, create_fixture_datasets};
```

**Step 3: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully with no errors

**Step 4: Commit**

```bash
git add crates/gat-tui/src/lib.rs
git commit -m "feat: Export data module and dataset structures from lib (gat-eqb step 1)"
```

---

## Task 2: Verify data module completeness

**Files:**
- Read: `crates/gat-tui/src/data/mod.rs`
- Read: `crates/gat-tui/src/data/fixtures.rs`

**Step 1: Verify data/mod.rs has required structures**

Run: `grep -n "pub struct DatasetEntry\|pub enum DatasetStatus\|pub struct DatasetsState" crates/gat-tui/src/data/mod.rs`
Expected: All three structures are defined

**Step 2: Verify fixtures.rs has create_fixture_datasets**

Run: `grep -n "pub fn create_fixture_datasets" crates/gat-tui/src/data/fixtures.rs`
Expected: Function exists and returns Vec<DatasetEntry>

**Step 3: No implementation needed**

The data module is already complete from earlier implementation. This task is verification only.

**Step 4: Commit (empty commit to mark completion)**

```bash
git commit --allow-empty -m "chore: Verified data module structure completeness (gat-eqb step 2)"
```

---

## Task 3: Find and understand Datasets pane rendering

**Files:**
- Locate: Datasets pane renderer

**Step 1: Find Datasets pane file**

Run: `find crates/gat-tui/src -name "*datasets*" -o -name "*pane*" | grep -i dataset`
Expected: Locate the pane rendering file

**Step 2: Read current Datasets pane rendering**

Once located, read the file to understand:
- How it currently renders (what it displays)
- What parameters it takes
- How to integrate dataset fixture data

**Step 3: Note the current rendering approach**

Document where the pane renders its content (which function, which widgets).

**Step 4: Commit (empty)**

```bash
git commit --allow-empty -m "chore: Located and analyzed Datasets pane rendering (gat-eqb step 3)"
```

---

## Task 4: Integrate fixture datasets into Datasets pane

**Files:**
- Modify: Datasets pane renderer file

**Step 1: Import data structures**

At the top of the Datasets pane renderer file, add:

```rust
use crate::data::{DatasetEntry, DatasetStatus, create_fixture_datasets};
```

**Step 2: Get fixture datasets**

In the render function for Datasets pane, add at the beginning:

```rust
let datasets = create_fixture_datasets();
```

**Step 3: Render dataset list**

Update the pane rendering to display datasets. Example pattern:

```rust
// Clear existing placeholder text rendering
// Add new rendering for dataset list:

for (idx, dataset) in datasets.iter().enumerate() {
    let is_selected = app_state.current_pane_state().selected_row == idx;
    let indicator = if is_selected { "▶ " } else { "  " };

    let status_icon = match dataset.status {
        DatasetStatus::Ready => "✓",
        DatasetStatus::Idle => "◆",
        DatasetStatus::Pending => "⟳",
    };

    let line = format!(
        "{}{} {} | {} | {:.1}MB | {}",
        indicator,
        status_icon,
        dataset.name,
        dataset.source,
        dataset.size_mb,
        dataset.description
    );

    // Render line to frame
}
```

**Step 4: Verify compilation**

Run: `cargo check -p gat-tui`
Expected: Compiles successfully

**Step 5: Test rendering**

Run: `cargo run -p gat-tui --release`
- Navigate to Datasets pane (press `3`)
- Verify you see the three fixture datasets displayed with their metadata
- Verify status icons and information display correctly
- Press up/down arrows to navigate datasets

Expected: Datasets pane displays fixture data with proper formatting

**Step 6: Commit**

```bash
git add crates/gat-tui/src/panes/datasets.rs
git commit -m "feat: Integrate fixture datasets into Datasets pane rendering (gat-eqb step 4)"
```

---

## Task 5: Add basic dataset selection/navigation

**Files:**
- Modify: Datasets pane or panes integration file
- Modify: Event handler for Datasets pane

**Step 1: Ensure up/down navigation works**

Verify in the main event loop that arrow key navigation updates `app_state.current_pane_state_mut().selected_row` when in Datasets pane.

Example (may already exist):

```rust
KeyCode::Up => {
    let state = app_state.current_pane_state_mut();
    state.selected_row = state.selected_row.saturating_sub(1);
},
KeyCode::Down => {
    let state = app_state.current_pane_state_mut();
    let max_rows = datasets.len();
    state.selected_row = (state.selected_row + 1).min(max_rows.saturating_sub(1));
},
```

**Step 2: Verify navigation in Datasets pane**

Run: `cargo run -p gat-tui --release`
- Press `3` to go to Datasets
- Press Down to select next dataset
- Verify selection moves (indicator changes)
- Press Up to move back
- Press Esc to return to menu bar

Expected: Navigation works smoothly

**Step 3: Commit**

```bash
git add crates/gat-tui/src
git commit -m "feat: Add dataset selection and navigation (gat-eqb step 5)"
```

---

## Task 6: Add documentation for Datasets integration

**Files:**
- Create: `docs/DATASETS_PANE.md`

**Step 1: Create documentation**

Create file `docs/DATASETS_PANE.md`:

```markdown
# Datasets Pane Implementation

## Overview

The Datasets pane displays available datasets with metadata and allows user selection.

## Data Model

Datasets are defined in `src/data/mod.rs`:

```rust
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

pub enum DatasetStatus {
    Ready,      // Available and usable
    Idle,       // Not recently accessed
    Pending,    // Loading/processing
}
```

## Fixture Data

Three sample datasets are available via `create_fixture_datasets()`:

1. **OPSD Snapshot** - OPSD source, Ready status, 245.3 MB
2. **Matpower IEEE 118-Bus** - Matpower source, Idle status, 1.2 MB
3. **Custom CSV Import** - CSV source, Pending status (processing)

## Rendering

The Datasets pane in `src/panes/datasets.rs` displays:

- Dataset indicator (✓ Ready, ◆ Idle, ⟳ Pending)
- Name | Source | Size | Description
- Selection highlight with ▶ indicator

## Navigation

- `Up/Down`: Navigate between datasets
- `Esc`: Return to menu bar
- Future: `Enter` to select/open dataset

## Integration with gat-core

Currently uses fixture data. Phase 2 (gat-xad) will:
- Replace fixtures with real gat-core queries
- Add async data loading with spinner
- Support dataset filtering and search

## Testing

Fixture data provides immediate rendering verification:

```rust
let datasets = create_fixture_datasets();
assert_eq!(datasets.len(), 3);
assert_eq!(datasets[0].status, DatasetStatus::Ready);
```

```

**Step 2: Commit**

```bash
git add docs/DATASETS_PANE.md
git commit -m "docs: Add Datasets pane implementation documentation (gat-eqb step 6)"
```

---

## Task 7: Final verification and mark task complete

**Files:**
- Verify: All functionality works
- Update: beads task status

**Step 1: Full build**

Run: `cargo build -p gat-tui --release`
Expected: Builds successfully

**Step 2: Run all tests**

Run: `cargo test -p gat-tui --lib`
Expected: All tests pass

**Step 3: Manual testing**

Run: `cargo run -p gat-tui --release`
- Test complete navigation
- Datasets pane shows 3 fixture datasets
- Selection indicator (▶) works
- Status icons display correctly
- All navigation works (Esc, arrows, pane switching)

Expected: All functionality works as expected

**Step 4: Verify git history**

Run: `git log --oneline | head -10`
Expected: See commits from this task

**Step 5: Mark beads task complete**

Run: `bd update gat-eqb --status closed --reason "Datasets pane integrated with fixture data; navigation working"`

**Step 6: Final commit**

```bash
git commit --allow-empty -m "chore: gat-eqb complete - Datasets pane with fixture data fully integrated"
```

---

## Summary

This revised plan integrates the existing data module into gat-tui:

1. **Task 1:** Export data module from lib.rs
2. **Task 2:** Verify data module completeness
3. **Task 3:** Locate and analyze Datasets pane rendering
4. **Task 4:** Integrate fixture datasets into Datasets pane
5. **Task 5:** Add dataset selection/navigation
6. **Task 6:** Document Datasets integration
7. **Task 7:** Final verification and task closure

**Key difference from original plan:** Works with existing AppState in models.rs rather than creating a new one. Focuses on populating the Datasets pane with real data structures instead of placeholder text.

**Total scope:** ~100 lines of modifications, 7 commits, integration of existing code

**Next task:** gat-xad Phase 1 - Connect Datasets (and other panes) to real gat-core data

