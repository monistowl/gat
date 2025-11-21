use crate::ui::{
    ContextButton, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar, TableView,
    Tooltip,
};

pub struct DatasetsPane;

impl DatasetsPane {
    pub fn layout() -> PaneLayout {
        let workflow_table = TableView::new(["Workflow", "Status", "Updated"])
            .add_row(["Ingest", "Ready", "just now"])
            .add_row(["Transform", "Idle", "1m ago"])
            .add_row(["Solve", "Pending", "3m ago"]);

        PaneLayout::new(
            Pane::new("Data catalog")
                .body([
                    "Public data connectors",
                    "OPSD snapshot",
                    "Airtravel tutorial",
                ])
                .with_table(workflow_table)
                .with_child(Pane::new("Downloads").body(["Ready to fetch"].into_iter()))
                .mark_visual(),
        )
        .with_sidebar(Sidebar::new("Metadata", false).lines(["Retained: 30d", "Backups: nightly"]))
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 80,
            tall_threshold: 24,
            expand_visuals_on_wide: true,
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
            ContextButton::new('f', "[f] Fetch dataset"),
            ContextButton::new('i', "[i] Inspect schema"),
        ]
    }
}
