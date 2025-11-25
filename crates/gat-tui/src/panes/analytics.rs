use crate::ui::{
    BarChartView, ColorHint, ContextButton, EmptyState, Pane, PaneContext, PaneLayout, PaneView,
    ResponsiveRules, Sidebar, SubTabs, TableView, Tabs, Tooltip,
};

pub struct AnalyticsPane;

impl AnalyticsPane {
    pub fn layout() -> PaneLayout {
        // Reliability tab - LOLE and EUE by scenario
        let reliability_chart = BarChartView::new()
            .with_title("LOLE (Loss of Load Expectation)")
            .add_bar("Summer Peak", 2.5, ColorHint::Warning)
            .add_bar("Winter Peak", 0.5, ColorHint::Good)
            .add_bar("Spring Average", 0.0, ColorHint::Good)
            .value_suffix(" h/year")
            .bar_width(35)
            .with_legend();

        let reliability_table = TableView::new([
            "Scenario",
            "LOLE (h/yr)",
            "EUE (MWh)",
            "Violations",
            "Status",
        ])
        .add_row(["Summer Peak", "2.5", "125.0", "3", "◐ Warning"])
        .add_row(["Winter Peak", "0.5", "18.5", "1", "✓ Good"])
        .add_row(["Spring Average", "0.0", "0.0", "0", "✓ Good"]);

        let reliability_pane = Pane::new("Reliability Analysis")
            .body([
                "Evaluate system reliability under different operating scenarios.",
                "Lower LOLE and EUE values indicate better system reliability.",
            ])
            .with_barchart(reliability_chart)
            .with_table(reliability_table);

        // Deliverability Score tab - DS by bus
        let ds_chart = BarChartView::new()
            .with_title("Deliverability Score by Bus")
            .add_bar("Bus_001", 95.5, ColorHint::Good)
            .add_bar("Bus_002", 87.3, ColorHint::Warning)
            .add_bar("Bus_003", 78.2, ColorHint::Warning)
            .max_value(100.0)
            .value_suffix("%")
            .bar_width(35)
            .with_legend();

        let ds_table = TableView::new(["Bus ID", "DS Value", "Nameplate (MW)", "Status"])
            .add_row(["Bus_001", "95.5%", "1000.0", "✓ Good"])
            .add_row(["Bus_002", "87.3%", "1500.0", "◐ Warning"])
            .add_row(["Bus_003", "78.2%", "800.0", "◐ Warning"]);

        let ds_pane = Pane::new("Deliverability Score")
            .body([
                "Assess how effectively power can be delivered to different buses.",
                "Scores above 90% indicate good deliverability.",
            ])
            .with_barchart(ds_chart)
            .with_table(ds_table);

        // ELCC tab - Capacity vs ELCC by resource
        let elcc_chart = BarChartView::new()
            .with_title("Effective Load Carrying Capability")
            .add_bar("Wind Farm A", 28.5, ColorHint::Warning)
            .add_bar("Solar Array B", 8.2, ColorHint::Warning)
            .add_bar("Battery C", 72.0, ColorHint::Good)
            .value_suffix(" MW")
            .bar_width(35)
            .with_legend();

        let elcc_table = TableView::new([
            "Resource",
            "Capacity (MW)",
            "ELCC (MW)",
            "Margin %",
            "Status",
        ])
        .add_row(["Wind Farm A", "100.0", "28.5", "71.5%", "◐ Warning"])
        .add_row(["Solar Array B", "50.0", "8.2", "83.6%", "◐ Warning"])
        .add_row(["Battery C", "75.0", "72.0", "4.0%", "✓ Good"]);

        let elcc_pane = Pane::new("ELCC Analysis")
            .body([
                "Compare nameplate capacity to effective load carrying capability.",
                "Battery storage typically has higher ELCC than weather-dependent renewables.",
            ])
            .with_barchart(elcc_chart)
            .with_table(elcc_table);

        // Power Flow tab - Line utilization
        let pf_chart = BarChartView::new()
            .with_title("Line Utilization")
            .add_bar("Line_001", 90.0, ColorHint::Warning)
            .add_bar("Line_002", 104.0, ColorHint::Critical)
            .add_bar("Line_003", 25.0, ColorHint::Good)
            .max_value(120.0)
            .value_suffix("%")
            .bar_width(35)
            .with_legend();

        let pf_table =
            TableView::new(["Branch", "Flow (MW)", "Limit (MW)", "Utilization", "Status"])
                .add_row(["Line_001", "450.0", "500.0", "90.0%", "⚠ Elevated"])
                .add_row(["Line_002", "520.0", "500.0", "104.0%", "⚡ Congested"])
                .add_row(["Line_003", "250.0", "1000.0", "25.0%", "— Normal"]);

        let pf_pane = Pane::new("Power Flow Results")
            .body([
                "Identify congestion hotspots and thermal constraint violations.",
                "Values above 100% indicate overloaded lines requiring corrective action.",
            ])
            .with_barchart(pf_chart)
            .with_table(pf_table);

        // Distribution automation sub-tab
        let switching_steps = TableView::new(["Step", "Device", "Target", "Status"])
            .add_row(["1", "Switch SW-14", "Open to isolate feeder", "Queued"])
            .add_row(["2", "Recloser RC-7", "Close to backfeed", "Pending"])
            .add_row(["3", "SCADA Tag", "Validate voltage profile", "Awaiting"])
            .with_empty_state(EmptyState::new(
                "No switching plan loaded",
                ["Import a FLISR plan to preview step-by-step execution."],
            ));

        let voltage_profiles =
            TableView::new(["Feeder", "Min (pu)", "Max (pu)", "Cap banks", "Tap changes"])
                .add_row(["F-12", "0.96", "1.03", "2 active", "1 planned"])
                .add_row(["F-21", "0.94", "1.05", "1 active", "2 planned"])
                .add_row(["F-7", "0.98", "1.02", "0 active", "0 planned"])
                .with_empty_state(EmptyState::new(
                    "No voltage profiles captured",
                    ["Run VVO to refresh regulator and cap bank telemetry."],
                ));

        let automation_pane = Pane::new("Distribution Automation")
            .body([
                "Preview FLISR/VVO steps with compact tables that stay readable in 110x32 viewports.",
                "Use status flags to stage edits before dispatching switching plans.",
            ])
            .with_table(switching_steps)
            .with_table(voltage_profiles)
            .with_tabs(Tabs::new(["Steps", "Profiles"], 0));

        // Hosting capacity sub-tab
        let hosting_capacity = TableView::new([
            "Bus/Zone",
            "Capacity (MW)",
            "Thermal flag",
            "Voltage flag",
            "Limit band",
        ])
        .add_row(["Bus 101", "5.5", "OK", "⚠ Near limit", "0.95–1.05 pu"])
        .add_row(["Zone A", "12.0", "OK", "OK", "Thermal headroom"])
        .add_row(["Bus 214", "3.2", "⚠ Limited", "OK", "0.94–1.05 pu"])
        .with_empty_state(EmptyState::new(
            "No hosting-capacity study loaded",
            [
                "Run hosting-capacity analytics to populate bus and zone headroom.",
                "Use zone files to aggregate capacity with limit flags.",
            ],
        ));

        let hosting_summary = Pane::new("Hosting Capacity Outputs")
            .body([
                "Capacity summaries keep voltage and thermal flags visible for siting decisions.",
                "Tables stay compact; empty states explain how to refresh results.",
            ])
            .with_table(hosting_capacity)
            .with_tabs(Tabs::new(["Bus", "Zone"], 0))
            .mark_visual();

        // Combine tabs
        let analytics_tabs = Tabs::new(["Reliability", "DS", "ELCC", "Power Flow"], 0);
        let main_content = Pane::new("Analytics Results")
            .body([
                "Comprehensive analysis results across multiple domains.",
                "Switch tabs to view different analytics categories.",
            ])
            .with_child(reliability_pane)
            .with_child(ds_pane)
            .with_child(elcc_pane)
            .with_child(pf_pane)
            .with_tabs(analytics_tabs)
            .mark_visual();

        let automation_hosting = Pane::new("Automation & Hosting")
            .with_child(automation_pane)
            .with_child(hosting_summary)
            .with_tabs(Tabs::new(["Automation", "Hosting"], 0));

        PaneLayout::new(main_content)
            .with_secondary(automation_hosting)
            .with_sidebar(Sidebar::new("Analysis Tips", false).lines([
                "Use arrow keys to navigate between tabs",
                "Bar charts show relative magnitudes at a glance",
                "Table data provides precise numerical values",
                "Automation and hosting subtabs stay compact on smaller viewports",
            ]))
            .with_subtabs(
                SubTabs::new(["Metrics", "Automation", "Hosting"], 0).with_compact_active(2),
            )
            .with_responsive_rules(ResponsiveRules {
                wide_threshold: 100,
                tall_threshold: 28,
                expand_visuals_on_wide: true,
                collapse_secondary_first: true,
            })
    }
}

impl PaneView for AnalyticsPane {
    fn id(&self) -> &'static str {
        "analytics"
    }

    fn label(&self) -> &'static str {
        "Analytics"
    }

    fn hotkey(&self) -> char {
        '6'
    }

    fn layout(&self, _context: &PaneContext) -> PaneLayout {
        Self::layout()
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "View and explore analysis results with visual charts and detailed tables.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![
            ContextButton::new('e', "[e] Export results to file"),
            ContextButton::new('r', "[r] Refresh analysis data"),
            ContextButton::new('f', "[f] Filter by scenario/resource"),
        ]
    }
}
