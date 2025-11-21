use crate::ui::{
    ContextButton, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar, SubTabs,
    TableView, Tabs, Tooltip,
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

        let transform_tabs = Tabs::new(["Normalize", "Enrich", "Validate"], 0);
        let transforms = Pane::new("Transforms")
            .body([
                "Radio: (•) Use template 'Grid cleanup'  |  ( ) Start blank",
                "Dropdown: Insert transform ↴ [Resample, Gap-fill, Forecast smoothing]",
                "Quick tip: “Add step” drops a transform under the highlighted row.",
                "Quick tip: “Reorder” keeps dependencies intact and updates previews.",
            ])
            .with_tabs(transform_tabs);

        let outputs = Pane::new("Outputs").body([
            "Dropdown: Delivery target ↴ [Warehouse table, DERMS feed, Notebook]",
            "Radio: (•) Single run report  |  ( ) Continuous subscription",
            "Inline hint: Outputs inherit naming from the selected source and template.",
        ]);

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
