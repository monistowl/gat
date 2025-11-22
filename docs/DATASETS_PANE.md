# Datasets Pane Implementation

## Overview

The Datasets pane displays available datasets with metadata and allows user selection. It's the third pane in gat-tui (hotkey: `3`).

## Data Model

Datasets are defined in `src/data.rs`:

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

Three sample datasets are provided via `create_fixture_datasets()`:

1. **OPSD Snapshot**
   - Status: Ready ✓
   - Source: OPSD
   - Size: 245.3 MB
   - Rows: 8,760
   - Description: "Open Power System Data hourly generation"

2. **Matpower IEEE 118-Bus**
   - Status: Idle ◆
   - Source: Matpower
   - Size: 1.2 MB
   - Rows: 118
   - Description: "IEEE 118-bus test system"

3. **Custom CSV Import**
   - Status: Pending ⟳
   - Source: CSV
   - Size: 0.0 MB (processing)
   - Rows: 0 (processing)
   - Description: "User-uploaded CSV file (processing)"

## Pane Rendering

The Datasets pane is rendered in `src/panes/datasets.rs` using the PaneLayout UI component system.

**Displayed Information:**
- Dataset Name
- Data Source (OPSD, Matpower, CSV, etc.)
- Size in MB (human-readable)
- Status indicator with icon

**Status Icons:**
- `✓` Ready - Dataset is fully processed and available
- `◆` Idle - Dataset exists but hasn't been accessed recently
- `⟳` Pending - Dataset is currently being processed

## Integration with gat-tui

### Imports

```rust
use crate::{create_fixture_datasets, DatasetStatus};
use crate::ui::{...};
```

### Table Generation

```rust
let datasets = create_fixture_datasets();
let mut dataset_table = TableView::new(["Name", "Source", "Size", "Status"]);

for dataset in &datasets {
    let status_icon = match dataset.status {
        DatasetStatus::Ready => "✓",
        DatasetStatus::Idle => "◆",
        DatasetStatus::Pending => "⟳",
    };
    dataset_table = dataset_table.add_row([
        dataset.name.as_str(),
        dataset.source.as_str(),
        &format!("{:.1} MB", dataset.size_mb),
        status_icon,
    ]);
}
```

## User Interaction

**Navigation:**
- `3` or menu navigation: Enter Datasets pane
- Arrow keys (when supported): Navigate between datasets
- `Esc`: Return to menu bar

**Context Actions** (available when Datasets pane is focused):
- `[f] Fetch dataset` - Download or import a dataset
- `[i] Inspect schema` - View dataset structure and metadata

## Testing

Fixture data provides immediate rendering verification without requiring:
- External data sources
- Network connections
- Complex initialization

To test manually:

```bash
cargo run -p gat-tui --release
# Press 3 to go to Datasets pane
# Verify three datasets appear with correct metadata
```

## Future Enhancements

**Phase 2 (gat-xad):**
- Replace fixture data with real gat-core queries
- Add async data loading with spinner
- Support dataset filtering and search
- Enable actual dataset selection and inspection

**Phase 3+:**
- Dataset import/upload UI
- Schema preview and validation
- Sample data display
- Real-time status updates
- Integration with pipeline and batch operations

## Related Files

- `src/data.rs` - Dataset structures and fixture data
- `src/panes/datasets.rs` - Datasets pane rendering
- `src/lib.rs` - Public API exports (DatasetEntry, DatasetStatus, create_fixture_datasets)
- `docs/TUI_WIREUP_COMPLETION.md` - Overall TUI architecture
- `docs/COMPONENT_UTILITIES.md` - Reusable UI component patterns
