use crate::ui::{
    Collapsible, ContextButton, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar,
    SubTabs, TableView, Tooltip,
};

pub struct DashboardPane;

impl DashboardPane {
    pub fn layout() -> PaneLayout {
        let status_card = Pane::new("Status").body([
            "Overall: healthy",
            "Running: 1 workflow",
            "Queued: 2 actions awaiting approvals",
        ]);

        // Reliability KPI cards
        let reliability_cards = Pane::new("Reliability Metrics").body([
            "✓ Deliverability Score: 85.5%  |  ⚠ LOLE: 9.2 h/yr  |  ⚠ EUE: 15.3 MWh/yr",
            "✓ DER penetration: 32% of feeder peak | ⚠ Hosting headroom: 4.3 MW min",
            "✓ Voltage compliance: 98.4% feeders in band | ⚠ Watchlist: F-21 taps drifting",
            "",
            "Last update: 2024-11-21 14:30 UTC",
            "Source: analytics reliability (batch_2024-11-21)",
        ]);

        let recent_runs_table = TableView::new(["Run", "Status", "Owner", "Duration"])
            .add_row(["ingest-2304", "Succeeded", "alice", "42s"])
            .add_row(["transform-7781", "Running", "ops", "live"])
            .add_row(["solve-9912", "Pending", "svc-derms", "queued"]);

        let recent_runs = Pane::new("Recent runs")
            .body(["Latest activity pulled from gat-core fixtures."])
            .with_table(recent_runs_table);

        let quick_actions = Pane::new("Quick actions").body([
            "[Enter] Run highlighted workflow",
            "[R] Retry last failed step",
            "[E] Edit config before dispatch",
            "[H] Refresh hosting-capacity study",
            "[V] Check feeder voltage compliance",
            "[P] Snapshot DER penetration KPI",
        ]);

        let details =
            Pane::new("Details").with_collapsible(Collapsible::new("Details", true).content([
                "Active workflow: solve-9912",
                "Dataset: opsd-sample",
                "Schedule: 5m cadence",
            ]));

        let logs = Pane::new("Logs").with_collapsible(Collapsible::new("Logs", true).content([
            "[L] Tail live logs (contextual)",
            "[O] Open previous run output",
            "[.] Pause stream when reviewing",
        ]));

        let resources =
            Pane::new("Resources").with_collapsible(Collapsible::new("Resources", true).content([
                "[H] Open guide/shortcuts",
                "[S] Share layout snapshot",
                "[G] Generate run summary",
            ]));

        let layout_visualizer = Pane::new("Layout visualizer")
            .body([
                "Pane composition map for the dashboard.",
                "Auto-expands in wide terminals to expose grid hints.",
                "Shown in its own subtab when space is constrained.",
            ])
            .mark_visual();

        let subtabs = SubTabs::new(["Dashboard", "Visualizer"], 0).with_compact_active(1);

        let sidebar = Sidebar::new("Contextual hotkeys", false).lines([
            "[D] Focus dashboard cards",
            "[V] Jump to visualizer",
            "[Q] Toggle quick actions",
        ]);

        PaneLayout::new(
            Pane::new("Dashboard")
                .body([
                    "Status cards, recent runs, and quick actions for operators.",
                    "Collapsible Details, Logs, and Resources start expanded on wide viewports.",
                ])
                .with_child(status_card)
                .with_child(reliability_cards)
                .with_child(recent_runs)
                .with_child(quick_actions)
                .with_child(details)
                .with_child(logs)
                .with_child(resources),
        )
        .with_secondary(layout_visualizer)
        .with_sidebar(sidebar)
        .with_subtabs(subtabs)
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 96,
            tall_threshold: 28,
            expand_visuals_on_wide: true,
            collapse_secondary_first: true,
        })
    }
}

impl PaneView for DashboardPane {
    fn id(&self) -> &'static str {
        "dashboard"
    }

    fn label(&self) -> &'static str {
        "Dashboard"
    }

    fn hotkey(&self) -> char {
        '1'
    }

    fn layout(&self, _context: &PaneContext) -> PaneLayout {
        Self::layout()
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Track operator health, quick actions, and run history from a single view.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![
            ContextButton::new('r', "[r] Run reliability analysis"),
            ContextButton::new('d', "[d] Run deliverability score"),
            ContextButton::new('e', "[e] Run ELCC estimation"),
        ]
    }
}
