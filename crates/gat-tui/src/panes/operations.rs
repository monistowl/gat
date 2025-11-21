use crate::ui::{
    ContextButton, Pane, PaneContext, PaneLayout, PaneView, Sidebar, SubTabs, Tooltip,
};

pub struct OperationsPane;

impl OperationsPane {
    pub fn layout() -> PaneLayout {
        PaneLayout::new(
            Pane::new("Operations")
                .body([
                    "DERMS + ADMS actions",
                    "Queue new studies and review topology",
                ])
                .with_tabs(crate::ui::Tabs::new(["DERMS", "ADMS", "State"], 0))
                .with_child(
                    Pane::new("DERMS queue").body(["2 queued envelopes", "1 stress-test running"]),
                ),
        )
        .with_secondary(
            Pane::new("ADMS control")
                .body(["Switching plans", "Voltage watchdogs"])
                .mark_visual(),
        )
        .with_sidebar(Sidebar::new("Operator notes", true).lines(["Next: reload feeders"]))
        .with_subtabs(SubTabs::new(["Switching", "Outage", "Settings"], 2))
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
            ContextButton::new('d', "[d] Dispatch action"),
            ContextButton::new('s', "[s] Schedule study"),
        ]
    }
}
