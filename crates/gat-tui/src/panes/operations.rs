use crate::ui::{
    BarChartView, ColorHint, ContextButton, Pane, PaneContext, PaneLayout, PaneView,
    ProgressBarView, ProgressStatus, Sidebar, SubTabs, Tooltip,
};

pub struct OperationsPane;

impl OperationsPane {
    pub fn layout() -> PaneLayout {
        // DERMS/ADMS section
        let derms_adms = Pane::new("Operations")
            .body([
                "DERMS + ADMS actions",
                "Queue new studies and review topology",
            ])
            .with_tabs(crate::ui::Tabs::new(["DERMS", "ADMS", "State"], 0))
            .with_child(
                Pane::new("DERMS queue").body(["2 queued envelopes", "1 stress-test running"]),
            );

        // Batch operations section with live progress tracking
        let batch_progress = ProgressBarView::new()
            .with_title("Running Jobs")
            .add_progress("Power Flow Analysis", 0.75, ProgressStatus::Active)
            .add_progress("Contingency N-1", 0.42, ProgressStatus::Active)
            .add_progress("State Estimation", 1.0, ProgressStatus::Complete)
            .add_progress("Dataset Upload", 0.0, ProgressStatus::Failed)
            .bar_width(40);

        let batch_ops = Pane::new("Batch Operations")
            .body([
                "Run power flow across scenario manifests",
                "Status: 2 active, 1 complete, 1 failed",
                "",
            ])
            .with_progressbar(batch_progress)
            .with_tabs(crate::ui::Tabs::new(["Power Flow", "Optimal Flow"], 0));

        // Allocation section with bar charts for cost attribution
        let rents_chart = BarChartView::new()
            .with_title("Congestion Rents by Line")
            .add_bar("Line_101", 45000.0, ColorHint::Warning)
            .add_bar("Line_203", 28500.0, ColorHint::Good)
            .add_bar("Line_305", 67200.0, ColorHint::Critical)
            .add_bar("Line_408", 12100.0, ColorHint::Good)
            .value_suffix(" $/hr")
            .bar_width(35)
            .with_legend();

        let contribution_chart = BarChartView::new()
            .with_title("KPI Contribution Analysis")
            .add_bar("Gen_A", 85.0, ColorHint::Good)
            .add_bar("Gen_B", 92.0, ColorHint::Good)
            .add_bar("Gen_C", 78.0, ColorHint::Warning)
            .add_bar("Load_1", 65.0, ColorHint::Warning)
            .max_value(100.0)
            .value_suffix("%")
            .bar_width(35)
            .with_legend();

        let alloc_ops = Pane::new("Allocation Analysis")
            .body([
                "Cost attribution and sensitivity analysis",
                "",
            ])
            .with_barchart(rents_chart)
            .with_child(Pane::new("KPI Contribution").with_barchart(contribution_chart))
            .with_tabs(crate::ui::Tabs::new(["Rents", "Contribution"], 0));

        PaneLayout::new(
            Pane::new("Operations Hub")
                .body(["DERMS + ADMS + Batch + Allocation"])
                .with_child(derms_adms)
                .with_child(batch_ops)
                .with_child(alloc_ops),
        )
        .with_secondary(
            Pane::new("Summary")
                .body([
                    "Queue status: 2 DERMS + 0 ADMS",
                    "Batch status: Ready",
                    "Next action: Dispatch or Schedule",
                ])
                .mark_visual(),
        )
        .with_sidebar(
            Sidebar::new("Operator notes", true).lines(["Review batch progress before dispatch"]),
        )
        .with_subtabs(SubTabs::new(["Control", "Queue", "Results"], 0))
    }
}

impl PaneView for OperationsPane {
    fn id(&self) -> &'static str {
        "operations"
    }

    fn label(&self) -> &'static str {
        "Operations"
    }

    fn hotkey(&self) -> char {
        '2'
    }

    fn layout(&self, _context: &PaneContext) -> PaneLayout {
        Self::layout()
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Review DERMS/ADMS queues, swap focus, and keep operator notes handy.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![
            ContextButton::new('b', "[b] Run batch PF/OPF"),
            ContextButton::new('d', "[d] Dispatch DERMS action"),
            ContextButton::new('a', "[a] Allocation analysis"),
        ]
    }
}
