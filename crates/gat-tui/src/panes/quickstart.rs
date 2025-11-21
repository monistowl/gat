use crate::ui::{
    Collapsible, ContextButton, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar,
    TableView, Tooltip,
};

pub struct QuickstartPane;

impl QuickstartPane {
    pub fn layout() -> PaneLayout {
        let overview = Pane::new("Overview").body([
            "Start here for the fastest path to a working demo.",
            "Use the menu hotkeys to swap sections without losing context.",
        ]);

        let prerequisites = Pane::new("Prerequisites").body([
            "Rust toolchain with cargo (stable)",
            "Sample data already lives in test_data/; docs render to docs/",
            "Terminal with UTF-8 and enough width for visual panes",
        ]);

        let workflow_table = TableView::new(["Step", "Command", "Outcome"])
            .add_row([
                "Get data",
                "gat-cli datasets list --limit 3",
                "Confirm connectors work",
            ])
            .add_row([
                "Run pipeline",
                "gat-cli derms envelope --grid-file <case>",
                "Produce envelope artifacts",
            ])
            .add_row([
                "Inspect",
                "gat-viz (or view docs outputs)",
                "Review generated parquet/csv",
            ]);

        let workflow = Pane::new("Standard workflow")
            .body([
                "1) Explore data and docs",
                "2) Prepare pipelines",
                "3) Dispatch runs + review results",
            ])
            .with_table(workflow_table);

        let checklist = Pane::new("First run checklist").with_collapsible(
            Collapsible::new("First run checklist", true).content([
                "cargo run -p gat-tui to view this UI",
                "cargo run -p gat-cli -- datasets list to verify connectors",
                "Open docs/ artifacts to confirm outputs rendered",
            ]),
        );

        let faq = Pane::new("FAQ").with_collapsible(Collapsible::new("FAQ", true).content([
            "Where are samples? test_data/ + docs/ rendered outputs",
            "How to reset? Remove docs/<domain> artifacts and rerun commands",
            "Need help? Press menu hotkeys; sidebar lists docs",
        ]));

        let how_to =
            Pane::new("How-to").with_collapsible(Collapsible::new("How-to", true).content([
                "Switch panes: press the menu hotkey shown in [*?] labels",
                "Open runs: use Commands workspace to paste gat-cli snippets",
                "Stay oriented: watch the tooltip for navigation hints",
            ]));

        PaneLayout::new(
            Pane::new("Quickstart guide")
                .body([
                    "Brief overview, prerequisites, and the core workflow in one place.",
                    "Collapsible FAQ/How-to keep instructions concise for new operators.",
                ])
                .with_child(overview)
                .with_child(prerequisites)
                .with_child(workflow)
                .with_child(checklist)
                .with_child(faq)
                .with_child(how_to),
        )
        .with_sidebar(Sidebar::new("Docs & references", false).lines([
            "README.md | QUICKSTART.md",
            "docs/guide/mcp-onboarding.md",
            "docs/derms/, docs/adms/ sample outputs",
        ]))
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 86,
            tall_threshold: 24,
            expand_visuals_on_wide: true,
            collapse_secondary_first: true,
        })
    }
}

impl PaneView for QuickstartPane {
    fn id(&self) -> &'static str {
        "quickstart"
    }

    fn label(&self) -> &'static str {
        "Help > Quickstart"
    }

    fn hotkey(&self) -> char {
        'h'
    }

    fn layout(&self, _context: &PaneContext) -> PaneLayout {
        Self::layout()
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Browse setup steps, FAQs, and cheatsheets without leaving the terminal.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![ContextButton::new(
            '?',
            "[?] Keep tooltips in view for hints",
        )]
    }
}
