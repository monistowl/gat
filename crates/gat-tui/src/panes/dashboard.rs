use crate::ui::{Collapsible, Pane, PaneLayout, ResponsiveRules, Sidebar, SubTabs, TableView};

pub struct DashboardPane;

impl DashboardPane {
    pub fn layout() -> PaneLayout {
        let status_card = Pane::new("Status").body([
            "Overall: healthy",
            "Running: 1 workflow",
            "Queued: 2 actions awaiting approvals",
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
            wide_threshold: 100,
            tall_threshold: 28,
            expand_visuals_on_wide: true,
        })
    }
}
