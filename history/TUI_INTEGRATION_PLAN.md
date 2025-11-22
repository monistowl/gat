# TUI Integration Plan for New CLI Features

**Date:** 2025-11-21
**Epic Issue:** `gat-2u9`
**Status:** Plan filed - Ready for implementation

---

## Executive Summary

This document outlines a comprehensive plan to integrate 11 new CLI features into the **gat-tui** Terminal User Interface. The new features span scenario templating, batch processing, geospatial analysis, cost allocation, and advanced analytics—all recently added to the experimental branch of gat-cli.

**Key Numbers:**
- **New Features to Integrate:** 11 major CLI commands across 6 categories
- **TUI Panes Affected:** All 6 existing panes (Dashboard, Operations, Datasets, Pipeline, Commands, Quickstart)
- **Subtasks Created:** 14 tracked in bd with parent-child relationships
- **Estimated Implementation:** ~2,500-3,000 new lines of code
- **Infrastructure Files:** 3 new modules (components, modals, data types)

---

## Overview: New CLI Features Not Yet in TUI

### 1. **gat scenarios** - Scenario Templating & Materialization
```
gat scenarios {validate, list, expand, materialize}
```
- **Purpose:** Define templated scenario specifications for power system test cases
- **Where it's missing:** Pipeline pane (should show scenario composition step)
- **UI Need:** File browser + manifest preview + materialization progress

### 2. **gat batch** - Batch Power Flow Execution
```
gat batch {pf, opf}
```
- **Purpose:** Fan-out execution across multiple scenarios (CANOS-style parallel execution)
- **Where it's missing:** Operations pane (should have dedicated Batch tab)
- **UI Need:** Job queue visualization + progress indicator + parallelism config

### 3. **gat featurize {gnn, kpi}** - Feature Engineering
```
gat featurize gnn         # Graph Neural Network feature export
gat featurize kpi         # KPI training/evaluation features
```
- **Purpose:** Extract ML-ready features from grid data and simulation results
- **Where it's missing:** Pipeline pane (should be transform steps)
- **UI Need:** Feature configuration wizard + schema preview + sample data

### 4. **gat alloc {rents, kpi}** - Cost Allocation & Sensitivity
```
gat alloc rents           # Congestion rent decomposition
gat alloc kpi             # KPI contribution analysis (gradient-based)
```
- **Purpose:** Attribute grid costs and compute resource sensitivities
- **Where it's missing:** Operations pane (should have Alloc tab)
- **UI Need:** Decomposition table + heatmap + tariff parameter editor

### 5. **gat geo {join, featurize}** - Geospatial Operations
```
gat geo join              # Map buses/feeders to spatial polygons
gat geo featurize         # Polygon-level spatial-temporal features
```
- **Purpose:** Geospatial analysis and spatial-temporal feature extraction
- **Where it's missing:** Datasets pane (should have Geo tab)
- **UI Need:** GIS file browser + spatial join method selector + preview

### 6. **gat analytics {ds, reliability, elcc}** - Advanced Analytics
```
gat analytics ds          # Deliverability Score (capacity value)
gat analytics reliability # LOLE, EUE, thermal violations
gat analytics elcc        # ELCC (Equivalent Load Carrying Capability)
```
- **Purpose:** Compute resource adequacy and reliability metrics
- **Where it's missing:** Dashboard pane (should show KPI cards) + new Analytics pane option
- **UI Need:** Status cards + metrics tables + visualization

---

## TUI Architecture Overview

The **gat-tui** crate uses a **trait-based plugin architecture**:

### Design Principles
1. **Stateless Panes:** No mutable state; panes are zero-sized structs implementing `PaneView` trait
2. **Builder Pattern:** All UI components use fluent builders (no mutation)
3. **Responsive Design:** Terminal resize-aware via `ResponsiveRules`
4. **Decoupled Execution:** Commands run as subprocesses via `CommandRunner` (no CLI imports)
5. **Text-Only Rendering:** Uses iocraft TUI framework (no ncurses/crossterm)

### Current Pane Registry
```
1 → Dashboard      (status cards, recent runs, quick actions)
2 → Operations     (DERMS/ADMS queue management)
3 → Datasets       (data catalog browser)
4 → Pipeline       (pipeline composer: source → transform → output)
5 → Commands       (custom gat-cli snippet workspace)
h → Quickstart     (help, onboarding, checklists)
```

### Available Hotkeys for New Panes
```
Numeric: 6, 7, 8, 9
Letters: a-g, i-z (excluding h)
```

---

## Implementation Plan: 4 Phases

### Phase 1: Core Workflow Panes (High Priority)

#### Task P1.1 - Pipeline: Add Transforms Subtab
**Issue:** `gat-2hf`
**Goal:** Expand Pipeline pane with new transform operations

**Changes:**
- Add "Transforms" subtab with scenario materialization, GNN, KPI, Geo featurize
- Scenario materialization: YAML file browser + manifest preview
- GNN featurization: grouping options (zone/device/layer)
- KPI featurization: lag/window configuration + feature selection
- Geo featurization: spatial join method + lag window

**Files Modified:** `crates/gat-tui/src/panes/pipeline.rs`
**Est. LOC:** 200
**Complexity:** Medium

---

#### Task P1.2 - Operations: Add Batch Tab
**Issue:** `gat-4wd`
**Goal:** Add job queue management for batch PF/OPF execution

**Changes:**
- New "Batch" tab alongside DERMS/ADMS
- Job queue table (scenario_id, status, progress %)
- Progress bar with elapsed time + estimated remaining
- Max parallelism selector (1-16 jobs)
- Summary stats: completed, failed, total time

**Files Modified:** `crates/gat-tui/src/panes/operations.rs`
**Est. LOC:** 150
**Complexity:** Medium

---

#### Task P1.3 - Operations: Add Alloc Tab
**Issue:** `gat-nwm`
**Goal:** Add allocation and settlement analysis tab

**Changes:**
- New "Alloc" tab with two subtabs: "Rents" and "Contribution"
- Rents subtab: OPF result file browser + decomposition table (surplus, congestion, loss)
- Contribution subtab: KPI sensitivity heatmap + tariff parameter CSV editor
- File browser for selecting OPF result files

**Files Modified:** `crates/gat-tui/src/panes/operations.rs`
**Est. LOC:** 120
**Complexity:** Medium

---

### Phase 2: Analytics & Dashboard (High Priority)

#### Task P2.1 - Dashboard: Add Reliability KPI Cards
**Issue:** `gat-dlz`
**Goal:** Surface key reliability metrics on dashboard

**Changes:**
- New "Reliability Metrics" section
- KPI cards: Deliverability Score (%), LOLE (h/yr), EUE (MWh/yr)
- Color-coded status (green/yellow/red) based on thresholds
- Secondary pane: detailed metrics table with scenario breakdown
- Modal integration: `gat analytics reliability --manifest <path>`

**Files Modified:** `crates/gat-tui/src/panes/dashboard.rs`
**Est. LOC:** 100
**Complexity:** Low

---

#### Task P2.2 - Analytics: Create New Pane (Optional)
**Issue:** `gat-ef3`
**Goal:** Optional consolidated analytics view (if Phase 2.1 makes dashboard too dense)

**Changes:**
- New Analytics pane (hotkey 6) with tabs: Reliability, Deliverability, ELCC, Geo
- Each tab: metric table + mini visualization
- File browser for batch manifest selection
- Interactive filtering/sorting

**Files Created:** `crates/gat-tui/src/panes/analytics.rs` (NEW)
**Est. LOC:** 250
**Complexity:** High
**Priority:** Optional (implement only if dashboard becomes overcrowded)

---

#### Task P2.3 - Dashboard: Add Context Buttons
**Issue:** `gat-jvp`
**Goal:** Quick-launch analytics command modals

**Changes:**
- Context button [d] "Run Deliverability Score"
- Context button [r] "Run Reliability Metrics"
- Context button [e] "Run ELCC Estimation"
- Each button opens pre-configured modal with file browser

**Files Modified:** `crates/gat-tui/src/panes/dashboard.rs`
**Est. LOC:** 80
**Complexity:** Low

---

### Phase 3: Data Management (Medium Priority)

#### Task P3.1 - Datasets: Add Geo Tab
**Issue:** `gat-61j`
**Goal:** Add geospatial data browser and configuration

**Changes:**
- New "Geo" subtab in Datasets pane
- GIS file browser (GeoParquet, Shapefile, GeoJSON)
- Spatial join method selector: point_in_polygon, voronoi, knn
- Bus-to-polygon assignment preview table
- Lag/window configuration for spatial aggregation

**Files Modified:** `crates/gat-tui/src/panes/datasets.rs`
**Est. LOC:** 140
**Complexity:** Medium

---

#### Task P3.2 - Datasets: Add Scenarios Tab
**Issue:** `gat-wic`
**Goal:** Add scenario template browser and manager

**Changes:**
- New "Scenarios" subtab in Datasets
- Scenario YAML/JSON file browser
- Template preview (variables, defaults, expansions)
- Expanded form preview (what materialization produces)
- Post-materialization manifest browser

**Files Modified:** `crates/gat-tui/src/panes/datasets.rs`
**Est. LOC:** 140
**Complexity:** Medium

---

### Phase 4: Commands & Documentation (Medium Priority)

#### Task P4.1 - Commands: Expand Snippet Sections
**Issue:** `gat-pei`
**Goal:** Add new command snippets to commands reference

**Changes:**
- Expand Commands pane table with new sections:
  - "Scenarios" section: `scenarios {validate, list, expand, materialize}`
  - "Batch" section: `batch {pf, opf}`
  - "Geo" section: `geo {join, featurize}`
  - "Alloc" section: `alloc {rents, kpi}`
  - "Analytics" section: `analytics {ds, reliability, elcc, geo}`
  - "Featurize" section: `featurize {gnn, kpi}`
- Each row: command, description, example flags
- Hotkey [c] to copy command to modal

**Files Modified:** `crates/gat-tui/src/panes/commands.rs`
**Est. LOC:** 80
**Complexity:** Low

---

#### Task P4.2 - Quickstart: Add Feature Guides
**Issue:** `gat-6hn`
**Goal:** Add workflow guides for new features

**Changes:**
- New collapsible sections in Quickstart pane:
  - "Scenario Workflow" (template → validate → expand → materialize → batch)
  - "Feature Engineering" (GNN, KPI, Geo featurization overview)
  - "Reliability Analysis" (batch execution → reliability metrics)
  - "Settlement Analysis" (OPF results → allocation rents/contribution)
- Each section with step-by-step examples and command references
- Links to Commands pane snippets

**Files Modified:** `crates/gat-tui/src/panes/quickstart.rs`
**Est. LOC:** 80
**Complexity:** Low

---

### Infrastructure: Supporting Modules

#### Task INF.1 - UI Component Utilities
**Issue:** `gat-yck`
**Goal:** Reusable UI components for common patterns

**New File:** `crates/gat-tui/src/ui/components.rs`

**Components:**
```rust
// File browser
pub fn file_browser_table(files: &[FileInfo]) -> TableView
pub fn file_browser_with_preview(path: &str) -> PaneLayout

// Progress indicators
pub fn progress_bar(current: u32, total: u32, width: u16) -> String
pub fn job_queue_table(jobs: &[Job]) -> TableView

// Configuration forms
pub fn config_form(title: &str, fields: &[ConfigField]) -> Pane
pub fn method_selector(methods: &[&str]) -> Pane

// Data previews
pub fn feature_preview_table(features: &[FeatureRow]) -> TableView
pub fn manifest_preview(manifest: &ScenarioManifest) -> Pane
pub fn metrics_table(metrics: &[MetricValue]) -> TableView
```

**Est. LOC:** 400
**Complexity:** High
**Blocks:** INF.2

---

#### Task INF.2 - Modal Templates
**Issue:** `gat-ect`
**Goal:** Pre-configured modal dialogs for new commands

**New File:** `crates/gat-tui/src/modals.rs`

**Modals:**
```rust
pub fn scenarios_materialize_modal() -> CommandModal
pub fn batch_pf_modal() -> CommandModal
pub fn batch_opf_modal() -> CommandModal
pub fn geo_join_modal() -> CommandModal
pub fn geo_featurize_modal() -> CommandModal
pub fn alloc_rents_modal() -> CommandModal
pub fn alloc_kpi_modal() -> CommandModal
pub fn featurize_gnn_modal() -> CommandModal
pub fn featurize_kpi_modal() -> CommandModal
pub fn analytics_ds_modal() -> CommandModal
pub fn analytics_reliability_modal() -> CommandModal
pub fn analytics_elcc_modal() -> CommandModal
```

**Est. LOC:** 300
**Complexity:** High
**Blocked by:** INF.1

---

#### Task INF.3 - Data Types Module
**Issue:** `gat-9v0`
**Goal:** Shared data structures for UI state

**New File:** `crates/gat-tui/src/data.rs`

**Types:**
```rust
pub enum JobStatus { Queued, Running, Completed, Failed }

pub struct Job {
    pub id: String,
    pub scenario_id: String,
    pub status: JobStatus,
    pub progress_pct: f32,
    pub elapsed_secs: u64,
}

pub struct FileInfo {
    pub path: String,
    pub file_type: String,      // "yaml", "parquet", "shapefile", etc.
    pub size_bytes: u64,
    pub modified: SystemTime,
}

pub enum MetricStatus { Good, Warning, Critical }

pub struct MetricValue {
    pub name: String,           // "LOLE", "EUE", "DS", etc.
    pub value: f64,
    pub unit: String,           // "h/yr", "MWh/yr", "%", etc.
    pub threshold: Option<f64>,
    pub status: MetricStatus,
}

pub struct ConfigField {
    pub name: String,
    pub label: String,
    pub field_type: ConfigFieldType,
    pub value: String,
}

pub enum ConfigFieldType {
    Text,
    Number,
    Dropdown(Vec<String>),
    FileBrowser(String), // filter pattern
}
```

**Est. LOC:** 150
**Complexity:** Low

---

### Testing & Validation

#### Task TEST.1 - Integration Tests
**Issue:** `gat-okz`
**Goal:** Comprehensive test coverage for new panes

**New Tests in:** `crates/gat-tui/tests/tui.rs`

**Test Coverage:**
- Each new pane renders without panicking
- Each new subtab is accessible
- Context buttons trigger expected modals
- File browser filtering works
- Progress bar rendering
- Modal command generation
- Terminal resize handling (small/medium/large)
- Modal submission with new commands

**Est. LOC:** 200
**Complexity:** Medium

---

## Implementation Sequencing

### Week 1: Infrastructure (Prerequisite for all phases)
1. **INF.1** - UI component utilities (gat-yck)
2. **INF.3** - Data types module (gat-9v0)
3. **INF.2** - Modal templates (gat-ect) [depends on INF.1]

### Week 2: High-Priority Features (P1 & P2)
1. **P1.1** - Pipeline transforms (gat-2hf)
2. **P1.2** - Operations batch tab (gat-4wd)
3. **P2.1** - Dashboard KPI cards (gat-dlz)
4. **P2.3** - Dashboard context buttons (gat-jvp)

### Week 3: Additional Coverage (P1.3 & P3)
1. **P1.3** - Operations alloc tab (gat-nwm)
2. **P3.1** - Datasets geo tab (gat-61j)
3. **P3.2** - Datasets scenarios tab (gat-wic)

### Week 4: Documentation & Polish (P4 & Testing)
1. **P4.1** - Commands snippets (gat-pei)
2. **P4.2** - Quickstart guides (gat-6hn)
3. **TEST.1** - Integration tests (gat-okz)
4. **P2.2** - Analytics pane [optional] (gat-ef3)

---

## File Change Summary

### Modified Files
| File | Purpose | Est. Lines |
|------|---------|-----------|
| `crates/gat-tui/src/panes/pipeline.rs` | Add transforms subtab | +200 |
| `crates/gat-tui/src/panes/operations.rs` | Add batch & alloc tabs | +270 |
| `crates/gat-tui/src/panes/dashboard.rs` | Add KPI cards & buttons | +180 |
| `crates/gat-tui/src/panes/datasets.rs` | Add geo & scenarios tabs | +280 |
| `crates/gat-tui/src/panes/commands.rs` | Expand snippets | +80 |
| `crates/gat-tui/src/panes/quickstart.rs` | Add guides | +80 |
| `crates/gat-tui/tests/tui.rs` | New integration tests | +200 |

### New Files
| File | Purpose | Est. Lines |
|------|---------|-----------|
| `crates/gat-tui/src/ui/components.rs` | Reusable UI utilities | ~400 |
| `crates/gat-tui/src/modals.rs` | Modal templates | ~300 |
| `crates/gat-tui/src/data.rs` | Data types | ~150 |

**Total New Code:** ~2,500-3,000 lines

---

## UI/UX Patterns & Conventions

### File Browsers
- Table format: [Path] [Type] [Size] [Modified]
- Sortable by column
- Filter by file extension
- Preview pane shows file content (first 20 lines for text)
- Keyboard navigation (↑/↓ to select, Enter to confirm)

### Progress Indicators
- Linear progress bar: `[████████░░░░░░░░░░░░] 40% (40/100)`
- Shows elapsed time in seconds
- Estimated time remaining (if available)
- Color: green = running, yellow = stalled, red = error

### Configuration Forms
- Dropdown for fixed options: `[▼ point_in_polygon▼]`
- Text input for free form: `[________________]`
- Number spinner: `[ ▲ 8 ▼ ]` (1-16)
- Checkboxes for flags: `[✓] Include seasonal features`

### Data Preview Tables
- Compact ASCII table format
- Max 5 columns (horizontal scroll if needed)
- Max 10 rows (paginated if more)
- Column headers right-aligned for numbers
- Empty state: `(no data)`

### Modals
- Title: descriptive command purpose
- Help text: short usage instructions
- Command text: editable (supports multi-line)
- Execution mode: Dry-run vs Full
- Submit hotkey: 'r' (mnemonic: "run")
- Cancel hotkey: Esc or 'q'

### Tooltips
- Per-pane tooltip (shown in footer when pane selected)
- Brief description of what the pane does
- Example usage (1-2 lines)

### Context Buttons
- Syntax: `[key] Description`
- Shown in menu bar (max 3 per pane)
- Common buttons: [r] Run, [d] Delete, [c] Copy, [e] Edit

---

## Quality Checkpoints

### Before Merging Each Phase

1. **Code Quality**
   - No clippy warnings
   - Proper error handling (no unwraps)
   - Comments on non-obvious logic

2. **Testing**
   - All new tests pass
   - Existing tests still pass
   - Manual terminal size testing (80x24, 120x40)

3. **UI/UX**
   - No text truncation at typical terminal sizes (80x24)
   - Responsive rules work correctly
   - Modal submission integrates with command runner
   - File browser handles 100+ files without lag

4. **Documentation**
   - Pane tooltips are clear and helpful
   - Quickstart guides are complete
   - Commands pane snippets are correct syntax

---

## Known Constraints & Decisions

### Terminal Size Support
- **Minimum:** 80x24 (VGA)
- **Typical:** 120x30 (modern terminal)
- **Large:** 200x50 (wide monitors)
- Responsive rules collapse secondary panes on small screens

### No External Dependencies
- Text-only rendering (no graphics)
- No async/await (thread-based command execution)
- No database (all UI state in memory)
- No network calls from TUI (subprocess execution only)

### State Management
- All panes are stateless (no mutable state)
- File browser state: current directory + scroll position (managed by modal)
- Job queue state: polling `CommandRunner` for updates (future enhancement)

### Command Execution
- All commands run as subprocesses (no library imports from gat-cli)
- Commands can be dry-run (prefixed with `echo`) or full execution
- Output captured via `CommandRunner` thread + channel

---

## Future Enhancements (Out of Scope)

1. **Live Progress Polling:** Batch jobs could auto-update progress without manual polling
2. **Result Caching:** Cache analytics outputs to avoid re-computation
3. **Configuration Persistence:** Save user-selected options (parallelism, method selectors)
4. **Keyboard Shortcuts:** Additional vim-like keybindings (j/k for navigation)
5. **Export/Report:** Generate PDF/HTML reports of analytics results
6. **Theming:** User-customizable color schemes
7. **History:** Keep log of executed commands and results

---

## References

### bd Issues Created
- **Epic:** `gat-2u9` - Integrate new CLI features into gat-tui
- **Phase 1 Tasks:** gat-2hf, gat-4wd, gat-nwm
- **Phase 2 Tasks:** gat-dlz, gat-ef3, gat-jvp
- **Phase 3 Tasks:** gat-61j, gat-wic
- **Phase 4 Tasks:** gat-pei, gat-6hn
- **Infrastructure:** gat-yck, gat-ect, gat-9v0
- **Testing:** gat-okz

### Source Files
- TUI Architecture: `crates/gat-tui/src/lib.rs`, `src/ui/registry.rs`
- Pane Implementations: `crates/gat-tui/src/panes/*.rs`
- UI Components: `crates/gat-tui/src/ui/mod.rs`
- Tests: `crates/gat-tui/tests/tui.rs`

### CLI Features to Integrate
- `crates/gat-cli/src/commands/scenarios/` - Scenario commands
- `crates/gat-cli/src/commands/batch/` - Batch execution
- `crates/gat-cli/src/commands/featurize/` - Feature extraction
- `crates/gat-cli/src/commands/analytics/` - Analytics commands
- `crates/gat-cli/src/commands/geo/` - Geospatial commands
- `crates/gat-cli/src/commands/alloc/` - Allocation commands

---

## Sign-Off

**Plan Created:** November 21, 2025
**Created By:** Claude Code (claude-haiku-4-5-20251001)
**Status:** Ready for Implementation
**Epic Tracking:** `gat-2u9` in bd issue tracker

All tasks have been filed in bd with:
- Clear descriptions and requirements
- Estimated lines of code
- Priority levels (P1 high, P2 medium)
- Complexity assessments
- File-level change mappings
- Parent-child relationships to epic

**Next Steps:**
1. Review this plan and epic structure in bd
2. Start with INF.1 (UI components) in Week 1
3. Update issue status to `in_progress` as work begins
4. Commit `.beads/issues.jsonl` with each phase completion

