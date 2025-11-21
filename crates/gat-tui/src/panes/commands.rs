use crate::ui::{
    ContextButton, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar, TableView,
    Tooltip,
};

pub struct CommandsPane;

impl CommandsPane {
    pub fn layout() -> PaneLayout {
        PaneLayout::new(
            Pane::new("Commands workspace")
                .body([
                    "Author gat-cli commands, stack them as multi-line snippets, and run with a hotkey.",
                    "Dry-runs print the normalized invocation; full runs stream into the modal output.",
                ])
                .with_table(
                    TableView::new(["Snippet", "Purpose"])
                        .add_row([
                            "gat-cli datasets list --limit 5",
                            "Verify dataset catalogue connectivity",
                        ])
                        .add_row([
                            "gat-cli derms envelope --grid-file <case>",
                            "Preview flexibility envelope inputs",
                        ])
                        .add_row([
                            "gat-cli dist import matpower --m <file>",
                            "Convert MATPOWER test cases before ADMS runs",
                        ]),
                )
                .with_child(
                    Pane::new("Hotkeys")
                        .body([
                            "[r] Run custom… opens the modal",
                            "[d] Toggle dry-run vs full execution",
                            "[esc] Close modal after reviewing output",
                        ])
                        .mark_visual(),
                ),
        )
        .with_sidebar(
            Sidebar::new("Recent command results", false).lines([
                "✔ dry-run datasets list (5 rows)",
                "✔ envelope preview (synthetic)",
                "… output scrollable inside modal",
            ]),
        )
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 88,
            tall_threshold: 24,
            expand_visuals_on_wide: false,
        })
    }
}

impl PaneView for CommandsPane {
    fn id(&self) -> &'static str {
        "commands"
    }

    fn label(&self) -> &'static str {
        "Commands"
    }

    fn hotkey(&self) -> char {
        '5'
    }

    fn layout(&self, _context: &PaneContext) -> PaneLayout {
        Self::layout()
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Author and run gat-cli snippets; modal output stays linked to this pane.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![ContextButton::new('r', "[r] Run custom…")]
    }
}
