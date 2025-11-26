use crate::ui::{
    ContextButton, EmptyState, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar,
    TableView, Tooltip,
};
use crate::{create_fixture_datasets, DatasetStatus};

pub struct DatasetsPane;

impl DatasetsPane {
    pub fn layout() -> PaneLayout {
        let datasets = create_fixture_datasets();

        // Build dataset table from fixture data
        let mut dataset_table = TableView::new(["Name", "Source", "Size", "Status", "Validation"]);
        for dataset in &datasets {
            let status_icon = match dataset.status {
                DatasetStatus::Ready => "✓",
                DatasetStatus::Idle => "◆",
                DatasetStatus::Pending => "⟳",
            };
            let validation = match dataset.status {
                DatasetStatus::Ready => "✓ Validated",
                DatasetStatus::Idle => "△ Drift check",
                DatasetStatus::Pending => "… Pending",
            };
            dataset_table = dataset_table.add_row([
                dataset.name.as_str(),
                dataset.source.as_str(),
                &format!("{:.1} MB", dataset.size_mb),
                &format!("{} {}", status_icon, dataset.status.as_str()),
                validation,
            ]);
        }

        let schema_table = TableView::new(["Field", "Type", "Notes"])
            .add_row(["bus_id", "Int64", "Primary key, required"])
            .add_row(["name", "Utf8", "Optional label"])
            .add_row(["voltage_kv", "Float64", "Validation: 0.95–1.05 expected"])
            .add_row(["region", "Utf8", "Used for aggregations"]);

        let preview_rows = TableView::new(["bus_id", "name", "voltage_kv", "region"])
            .add_row(["101", "Feeder A", "1.02", "north"])
            .add_row(["205", "Tap-205", "0.98", "central"])
            .add_row(["310", "Substation 3", "1.01", "south"]);

        PaneLayout::new(
            Pane::new("Data catalog")
                .body([
                    "Available datasets (inline validation shows drift and pending checks):",
                    "Select a dataset to view details, validate, or preview rows",
                    "Public data connectors and private uploads share the same validation flow",
                ])
                .with_table(dataset_table)
                .with_child(
                    Pane::new("Schema summary")
                        .body(["Concise schema for the highlighted dataset:"])
                        .with_table(schema_table),
                )
                .with_child(
                    Pane::new("Sample rows")
                        .body(["Preview a few rows before running a full fetch or validation."])
                        .with_table(preview_rows),
                )
                .with_child(Pane::new("Downloads").with_empty_state(EmptyState::new(
                    "No downloads queued",
                    [
                        "Run a fetch to pull sample data",
                        "Queued jobs will appear here",
                    ],
                )))
                .mark_visual(),
        )
        .with_sidebar(Sidebar::new("Metadata", false).lines(["Retained: 30d", "Backups: nightly"]))
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 80,
            tall_threshold: 24,
            expand_visuals_on_wide: true,
            collapse_secondary_first: true,
        })
    }
}

impl PaneView for DatasetsPane {
    fn id(&self) -> &'static str {
        "datasets"
    }

    fn label(&self) -> &'static str {
        "Datasets"
    }

    fn hotkey(&self) -> char {
        '3'
    }

    fn layout(&self, _context: &PaneContext) -> PaneLayout {
        Self::layout()
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Review catalog metadata, preview workflows, and download datasets.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![
            ContextButton::new('f', "[f] Fetch dataset (gat dataset public fetch)"),
            ContextButton::new('d', "[d] Describe dataset (gat dataset public describe)"),
            ContextButton::new('i', "[i] Inspect schema"),
            ContextButton::new('v', "[v] Validate dataset inline"),
            ContextButton::new('p', "[p] Preview rows"),
        ]
    }
}
