use crate::ui::{
    ContextButton, GraphView, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar,
    SubTabs, TableView, Tabs, Tooltip,
};

pub struct PipelinePane;

impl PipelinePane {
    pub fn layout() -> PaneLayout {
        let source_selection = Pane::new("Source selection").body([
            "Guided selectors avoid free-form hotkeys for choosing data.",
            "Radio: (•) Live telemetry stream  |  ( ) Batch archive replay",
            "Dropdown: Dataset variant ↴ [Day-ahead | Real-time | Sandbox]",
            "Quick tip: Swap sources without breaking downstream transforms.",
        ]);

        // Standard transforms
        let standard_transforms = Pane::new("Standard transforms").body([
            "Radio: (•) Use template 'Grid cleanup'  |  ( ) Start blank",
            "Dropdown: Insert transform ↴ [Resample, Gap-fill, Forecast smoothing]",
            "Quick tip: Add step drops a transform under the highlighted row.",
            "Quick tip: Reorder keeps dependencies intact and updates previews.",
        ]);

        // New feature transforms
        let scenario_transforms = Pane::new("Scenario materialization").body([
            "Materialize templated scenarios into full manifest",
            "File: scenarios.yaml → Manifest: scenarios_expanded.json",
            "Status: [Queued] Ready to load template",
        ]);

        let featurize_transforms = Pane::new("Feature engineering").body([
            "Transform grid data into ML-ready features",
            "Available:",
            "  • GNN: Export graph topology for neural networks",
            "  • KPI: Generate training features from batch results",
            "  • Geo: Spatial-temporal features from geospatial data",
        ]);

        let transform_tabs = Tabs::new(["Classic", "Scenarios", "Features"], 0);
        let transforms = Pane::new("Transforms")
            .with_child(standard_transforms)
            .with_child(scenario_transforms)
            .with_child(featurize_transforms)
            .with_tabs(transform_tabs);

        let outputs = Pane::new("Outputs").body([
            "Dropdown: Delivery target ↴ [Warehouse table, DERMS feed, Notebook]",
            "Radio: (•) Single run report  |  ( ) Continuous subscription",
            "Inline hint: Outputs inherit naming from the selected source and template.",
        ]);

        // Visual DAG graph representation (fancy-ui feature)
        let graph = GraphView::new()
            .add_node("n1", "Live telemetry", "◆", 0)
            .add_node("n2", "Resample", "▲", 1)
            .add_node("n3", "Gap fill", "▲", 2)
            .add_node("n4", "Validate", "○", 3)
            .add_node("n5", "Warehouse", "■", 4)
            .add_edge("n1", "n2")
            .add_edge("n2", "n3")
            .add_edge("n3", "n4")
            .add_edge("n4", "n5")
            .with_legend();

        let preview_table = TableView::new(["Step", "From", "To"])
            .add_row(["Source: Live telemetry", "edge-collector", "Normalizer"])
            .add_row(["Transform: Resample", "Normalizer", "Gap fill"])
            .add_row(["Transform: Validate", "Gap fill", "Outputs"])
            .add_row(["Output: Warehouse", "Outputs", "analytics.fact_runs"]);

        let preview = Pane::new("Pipeline graph preview")
            .body([
                "Auto-refreshes as you add or reorder steps; aligns with dropdown choices.",
                "Helpful for confirming fan-in/fan-out before dispatching a run.",
            ])
            .with_graph(graph)
            .with_table(preview_table);

        let controls = Pane::new("Controls")
            .body([
                "Button: [Ctrl+R] Run pipeline — executes the previewed path.",
                "Shows the visible hotkey on the control to reduce guesswork.",
            ])
            .mark_visual();

        PaneLayout::new(
            Pane::new("Pipeline composer")
                .body([
                    "Pick sources, transformations, and outputs with menus instead of ad-hoc keys.",
                    "Inline instructions keep each section self-guided; subtabs appear when crowded.",
                ])
                .with_child(source_selection)
                .with_child(transforms)
                .with_child(outputs),
        )
        .with_secondary(Pane::new("Review & dispatch").with_child(preview).with_child(controls))
        .with_sidebar(
            Sidebar::new("Section tips", false).lines([
                "Use “Add step” to insert under the focused transform.",
                "“Reorder” toggles move mode; preview table updates live.",
                "Keep transforms concise—subtabs split dense step lists.",
            ]),
        )
        .with_subtabs(SubTabs::new(["Compose", "Graph"], 0))
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 92,
            tall_threshold: 26,
            expand_visuals_on_wide: true,
            collapse_secondary_first: true,
        })
    }
}

impl PaneView for PipelinePane {
    fn id(&self) -> &'static str {
        "pipeline"
    }

    fn label(&self) -> &'static str {
        "Pipeline"
    }

    fn hotkey(&self) -> char {
        '4'
    }

    fn layout(&self, _context: &PaneContext) -> PaneLayout {
        Self::layout()
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Compose, reorder, and run Gat pipelines while keeping controls nearby.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![
            ContextButton::new('a', "[a] Add step — inserts under the focused transform"),
            ContextButton::new('o', "[o] Reorder — move step while preserving dependencies"),
            ContextButton::new('r', "[r] Run pipeline — mirrors the [Ctrl+R] control"),
        ]
    }
}
