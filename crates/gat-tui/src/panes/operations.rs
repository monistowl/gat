use crate::ui::{
    ContextButton, Pane, PaneContext, PaneLayout, PaneView, Sidebar, SubTabs, Tooltip,
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

        // Batch operations section
        let batch_ops = Pane::new("Batch Operations")
            .body([
                "Run power flow across scenario manifests",
                "Status: Ready",
                "",
                "Active jobs: 0/4 (max parallelism: 4)",
                "Last run: scenarios_2024-11-21.json - 0s elapsed",
            ])
            .with_tabs(crate::ui::Tabs::new(["Power Flow", "Optimal Flow"], 0));

        // Allocation section
        let alloc_ops = Pane::new("Allocation Analysis")
            .body([
                "Cost attribution and sensitivity analysis",
                "",
                "Available results:",
                "  • Congestion rents decomposition",
                "  • KPI contribution sensitivity",
                "",
                "Ready to load OPF results for analysis",
            ])
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
