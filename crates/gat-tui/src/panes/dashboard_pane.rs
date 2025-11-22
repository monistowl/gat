/// Dashboard Pane - Overview of system status and quick actions
///
/// The dashboard provides a high-level view of:
/// - Overall system health and status
/// - Reliability KPIs (LOLE, EUE, Deliverability Score)
/// - Recent workflow runs
/// - Quick action shortcuts
/// - Resource usage metrics

use crate::components::*;

/// Dashboard pane state
#[derive(Clone, Debug)]
pub struct DashboardPaneState {
    // Status cards
    pub overall_status: String,
    pub running_count: usize,
    pub queued_count: usize,

    // KPI metrics
    pub kpis: KPIMetrics,

    // Recent runs table
    pub recent_runs: Vec<RecentRun>,
    pub selected_run: usize,

    // Selected tab
    pub active_tab: usize,

    // Component states
    pub runs_table: TableWidget,
    pub metrics_list: ListWidget,
    pub status_text: TextWidget,
}

#[derive(Clone, Debug)]
pub struct KPIMetrics {
    pub deliverability_score: f64,
    pub lole_hours_per_year: f64,
    pub eue_mwh_per_year: f64,
    pub last_update: String,
}

#[derive(Clone, Debug)]
pub struct RecentRun {
    pub id: String,
    pub status: String,
    pub owner: String,
    pub duration: String,
}

impl Default for DashboardPaneState {
    fn default() -> Self {
        let mut runs_table = TableWidget::new("dashboard_runs");
        runs_table.columns = vec![
            Column { header: "Run ID".into(), width: 20 },
            Column { header: "Status".into(), width: 12 },
            Column { header: "Owner".into(), width: 12 },
            Column { header: "Duration".into(), width: 10 },
        ];

        let mut metrics_list = ListWidget::new("dashboard_metrics");
        metrics_list.add_item(
            "Deliverability Score".into(),
            "85.5%".into(),
        );
        metrics_list.add_item(
            "LOLE (h/year)".into(),
            "9.2".into(),
        );
        metrics_list.add_item(
            "EUE (MWh/year)".into(),
            "15.3".into(),
        );

        DashboardPaneState {
            overall_status: "Healthy".into(),
            running_count: 1,
            queued_count: 2,
            kpis: KPIMetrics {
                deliverability_score: 85.5,
                lole_hours_per_year: 9.2,
                eue_mwh_per_year: 15.3,
                last_update: "2024-11-21 14:30 UTC".into(),
            },
            recent_runs: vec![
                RecentRun {
                    id: "ingest-2304".into(),
                    status: "Succeeded".into(),
                    owner: "alice".into(),
                    duration: "42s".into(),
                },
                RecentRun {
                    id: "transform-7781".into(),
                    status: "Running".into(),
                    owner: "ops".into(),
                    duration: "live".into(),
                },
                RecentRun {
                    id: "solve-9912".into(),
                    status: "Pending".into(),
                    owner: "svc-derms".into(),
                    duration: "queued".into(),
                },
            ],
            selected_run: 0,
            active_tab: 0,
            runs_table,
            metrics_list,
            status_text: TextWidget::new("dashboard_status", ""),
        }
    }
}

impl DashboardPaneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_recent_runs(&mut self) {
        // Populate table with recent runs
        self.runs_table.rows.clear();
        for run in &self.recent_runs {
            self.runs_table.rows.push(TableRow {
                cells: vec![
                    run.id.clone(),
                    run.status.clone(),
                    run.owner.clone(),
                    run.duration.clone(),
                ],
            });
        }
    }

    pub fn select_next_run(&mut self) {
        if self.selected_run < self.recent_runs.len().saturating_sub(1) {
            self.selected_run += 1;
            self.runs_table.set_selected(self.selected_run);
        }
    }

    pub fn select_prev_run(&mut self) {
        if self.selected_run > 0 {
            self.selected_run -= 1;
            self.runs_table.set_selected(self.selected_run);
        }
    }

    pub fn selected_run(&self) -> Option<&RecentRun> {
        self.recent_runs.get(self.selected_run)
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % 2; // 0 = Dashboard, 1 = Visualizer
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = if self.active_tab == 0 { 1 } else { 0 };
    }

    pub fn refresh_metrics(&mut self) {
        // Update status text with current KPIs
        self.status_text.set_content(format!(
            "Status: {}\nRunning: {}\nQueued: {}\n\nKPIs:\nDeliverability: {:.1}%\nLOLE: {:.1} h/yr\nEUE: {:.1} MWh/yr",
            self.overall_status,
            self.running_count,
            self.queued_count,
            self.kpis.deliverability_score,
            self.kpis.lole_hours_per_year,
            self.kpis.eue_mwh_per_year,
        ));
    }

    pub fn health_indicator(&self) -> &'static str {
        if self.running_count > 0 || self.queued_count > 0 {
            "⚙"  // Running/queued
        } else if self.kpis.deliverability_score > 90.0 {
            "✓"  // Healthy
        } else if self.kpis.deliverability_score > 75.0 {
            "◐"  // Warning
        } else {
            "✗"  // Error
        }
    }

    pub fn status_color(&self) -> &'static str {
        match self.overall_status.as_str() {
            "Healthy" => "green",
            "Warning" => "yellow",
            "Error" => "red",
            _ => "gray",
        }
    }
}

/// Quick action shortcuts
pub struct QuickAction {
    pub key: char,
    pub label: String,
    pub description: String,
}

impl QuickAction {
    pub fn all() -> Vec<Self> {
        vec![
            QuickAction {
                key: 'r',
                label: "[r]".into(),
                description: "Run reliability analysis".into(),
            },
            QuickAction {
                key: 'd',
                label: "[d]".into(),
                description: "Run deliverability score".into(),
            },
            QuickAction {
                key: 'e',
                label: "[e]".into(),
                description: "Run ELCC estimation".into(),
            },
            QuickAction {
                key: 'f',
                label: "[f]".into(),
                description: "Filter recent runs".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_init() {
        let state = DashboardPaneState::new();
        assert_eq!(state.overall_status, "Healthy");
        assert_eq!(state.recent_runs.len(), 3);
        assert_eq!(state.selected_run, 0);
    }

    #[test]
    fn test_run_selection() {
        let mut state = DashboardPaneState::new();
        state.select_next_run();
        assert_eq!(state.selected_run, 1);
        state.select_prev_run();
        assert_eq!(state.selected_run, 0);
    }

    #[test]
    fn test_tab_navigation() {
        let mut state = DashboardPaneState::new();
        assert_eq!(state.active_tab, 0);
        state.next_tab();
        assert_eq!(state.active_tab, 1);
        state.next_tab();
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_health_indicator() {
        let mut state = DashboardPaneState::new();
        // Default state has running_count = 1 and queued_count = 2
        assert_eq!(state.health_indicator(), "⚙"); // Running (default state)

        state.running_count = 0;
        state.queued_count = 0;
        // Score 85.5 is between 75 and 90, so it's warning
        assert_eq!(state.health_indicator(), "◐"); // Warning (75 < 85.5 < 90)

        state.kpis.deliverability_score = 95.0;
        assert_eq!(state.health_indicator(), "✓"); // Healthy (95 > 90)

        state.kpis.deliverability_score = 50.0;
        assert_eq!(state.health_indicator(), "✗"); // Error (50 < 75)
    }

    #[test]
    fn test_quick_actions() {
        let actions = QuickAction::all();
        assert_eq!(actions.len(), 4);
        assert_eq!(actions[0].key, 'r');
    }
}
