/// Dashboard Pane - Overview of system status and quick actions
///
/// The dashboard provides a high-level view of:
/// - Overall system health and status
/// - Reliability KPIs (LOLE, EUE, Deliverability Score)
/// - Recent workflow runs
/// - Quick action shortcuts
/// - Resource usage metrics
use crate::components::*;
use crate::ui::GridInfo;

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

    // Grid management (Phase 3)
    pub current_grid: Option<GridInfo>,
    pub grid_count: usize,

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
    pub der_penetration_percent: f64,
    pub voltage_compliance_percent: f64,
    pub hosting_headroom_mw: f64,
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
            Column {
                header: "Run ID".into(),
                width: 20,
            },
            Column {
                header: "Status".into(),
                width: 12,
            },
            Column {
                header: "Owner".into(),
                width: 12,
            },
            Column {
                header: "Duration".into(),
                width: 10,
            },
        ];

        let mut metrics_list = ListWidget::new("dashboard_metrics");
        metrics_list.add_item("Deliverability Score".into(), "85.5%".into());
        metrics_list.add_item("LOLE (h/year)".into(), "9.2".into());
        metrics_list.add_item("EUE (MWh/year)".into(), "15.3".into());
        metrics_list.add_item("DER Penetration".into(), "32%".into());
        metrics_list.add_item("Voltage Compliance".into(), "98.4%".into());
        metrics_list.add_item("Hosting Headroom".into(), "4.3 MW".into());

        DashboardPaneState {
            overall_status: "Healthy".into(),
            running_count: 1,
            queued_count: 2,
            kpis: KPIMetrics {
                deliverability_score: 85.5,
                lole_hours_per_year: 9.2,
                eue_mwh_per_year: 15.3,
                der_penetration_percent: 32.0,
                voltage_compliance_percent: 98.4,
                hosting_headroom_mw: 4.3,
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
            current_grid: None,
            grid_count: 0,
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
            "Status: {}\nRunning: {}\nQueued: {}\n\nKPIs:\nDeliverability: {:.1}%\nLOLE: {:.1} h/yr\nEUE: {:.1} MWh/yr\nDER penetration: {:.1}%\nVoltage compliance: {:.1}% feeders\nHosting headroom: {:.1} MW",
            self.overall_status,
            self.running_count,
            self.queued_count,
            self.kpis.deliverability_score,
            self.kpis.lole_hours_per_year,
            self.kpis.eue_mwh_per_year,
            self.kpis.der_penetration_percent,
            self.kpis.voltage_compliance_percent,
            self.kpis.hosting_headroom_mw,
        ));
    }

    pub fn health_indicator(&self) -> &'static str {
        if self.running_count > 0 || self.queued_count > 0 {
            "⚙" // Running/queued
        } else if self.kpis.deliverability_score > 90.0 {
            "✓" // Healthy
        } else if self.kpis.deliverability_score > 75.0 {
            "◐" // Warning
        } else {
            "✗" // Error
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

    // Grid management methods (Phase 3)

    /// Set the current active grid for display
    pub fn set_current_grid(&mut self, grid: GridInfo) {
        self.current_grid = Some(grid);
    }

    /// Clear the current grid
    pub fn clear_current_grid(&mut self) {
        self.current_grid = None;
    }

    /// Update grid count
    pub fn update_grid_count(&mut self, count: usize) {
        self.grid_count = count;
    }

    /// Get grid status indicator
    pub fn grid_status_indicator(&self) -> &'static str {
        match &self.current_grid {
            Some(grid) => grid.status.display(),
            None => "○", // Inactive
        }
    }

    /// Get formatted grid info for display
    pub fn grid_info_display(&self) -> String {
        match &self.current_grid {
            Some(grid) => {
                format!(
                    "{} {} ({} nodes, {} branches)",
                    grid.status.display(),
                    grid.id,
                    grid.node_count,
                    grid.branch_count
                )
            }
            None => "No grid loaded".to_string(),
        }
    }

    /// Get grid density as percentage
    pub fn grid_density_percent(&self) -> Option<f64> {
        self.current_grid.as_ref().map(|g| g.density * 100.0)
    }
}

/// Quick action shortcuts
#[derive(Clone, Debug)]
pub struct QuickAction {
    pub key: char,
    pub label: String,
    pub description: String,
    pub action_type: ActionType,
}

/// Type of action triggered by quick action button
#[derive(Clone, Debug, PartialEq)]
pub enum ActionType {
    ReliabilityAnalysis,
    DeliverabilityScore,
    ELCCEstimation,
    PowerFlowAnalysis,
    FilterRuns,
}

impl ActionType {
    pub fn label(&self) -> &'static str {
        match self {
            ActionType::ReliabilityAnalysis => "Reliability",
            ActionType::DeliverabilityScore => "Deliverability",
            ActionType::ELCCEstimation => "ELCC",
            ActionType::PowerFlowAnalysis => "Power Flow",
            ActionType::FilterRuns => "Filter",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ActionType::ReliabilityAnalysis => "Run reliability analysis (LOLE, EUE)",
            ActionType::DeliverabilityScore => "Run deliverability score calculation",
            ActionType::ELCCEstimation => "Run ELCC resource adequacy",
            ActionType::PowerFlowAnalysis => "Run power flow analysis",
            ActionType::FilterRuns => "Filter and search recent runs",
        }
    }
}

impl QuickAction {
    pub fn new(key: char, label: String, description: String, action_type: ActionType) -> Self {
        Self {
            key,
            label,
            description,
            action_type,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            QuickAction {
                key: 'r',
                label: "[r]".into(),
                description: "Run reliability analysis".into(),
                action_type: ActionType::ReliabilityAnalysis,
            },
            QuickAction {
                key: 'd',
                label: "[d]".into(),
                description: "Run deliverability score".into(),
                action_type: ActionType::DeliverabilityScore,
            },
            QuickAction {
                key: 'e',
                label: "[e]".into(),
                description: "Run ELCC estimation".into(),
                action_type: ActionType::ELCCEstimation,
            },
            QuickAction {
                key: 'p',
                label: "[p]".into(),
                description: "Run power flow analysis".into(),
                action_type: ActionType::PowerFlowAnalysis,
            },
            QuickAction {
                key: 'f',
                label: "[f]".into(),
                description: "Filter recent runs".into(),
                action_type: ActionType::FilterRuns,
            },
        ]
    }

    pub fn analytics_actions() -> Vec<Self> {
        vec![
            QuickAction {
                key: 'r',
                label: "[r]".into(),
                description: "Run reliability analysis".into(),
                action_type: ActionType::ReliabilityAnalysis,
            },
            QuickAction {
                key: 'd',
                label: "[d]".into(),
                description: "Run deliverability score".into(),
                action_type: ActionType::DeliverabilityScore,
            },
            QuickAction {
                key: 'e',
                label: "[e]".into(),
                description: "Run ELCC estimation".into(),
                action_type: ActionType::ELCCEstimation,
            },
            QuickAction {
                key: 'p',
                label: "[p]".into(),
                description: "Run power flow analysis".into(),
                action_type: ActionType::PowerFlowAnalysis,
            },
        ]
    }

    pub fn find_by_key(key: char) -> Option<Self> {
        Self::all().into_iter().find(|a| a.key == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::GridStatus;

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
        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0].key, 'r');
        assert_eq!(actions[0].action_type, ActionType::ReliabilityAnalysis);
    }

    #[test]
    fn test_action_type_labels() {
        assert_eq!(ActionType::ReliabilityAnalysis.label(), "Reliability");
        assert_eq!(ActionType::DeliverabilityScore.label(), "Deliverability");
        assert_eq!(ActionType::ELCCEstimation.label(), "ELCC");
        assert_eq!(ActionType::PowerFlowAnalysis.label(), "Power Flow");
        assert_eq!(ActionType::FilterRuns.label(), "Filter");
    }

    #[test]
    fn test_action_type_descriptions() {
        assert!(ActionType::ReliabilityAnalysis
            .description()
            .contains("reliability"));
        assert!(ActionType::DeliverabilityScore
            .description()
            .contains("deliverability"));
        assert!(ActionType::ELCCEstimation.description().contains("ELCC"));
        assert!(ActionType::PowerFlowAnalysis
            .description()
            .contains("power"));
        assert!(ActionType::FilterRuns.description().contains("Filter"));
    }

    #[test]
    fn test_analytics_actions() {
        let actions = QuickAction::analytics_actions();
        assert_eq!(actions.len(), 4);
        assert!(actions
            .iter()
            .all(|a| a.action_type != ActionType::FilterRuns));
    }

    #[test]
    fn test_quick_action_constructor() {
        let action = QuickAction::new(
            'x',
            "[x]".into(),
            "Test action".into(),
            ActionType::ReliabilityAnalysis,
        );
        assert_eq!(action.key, 'x');
        assert_eq!(action.label, "[x]");
        assert_eq!(action.description, "Test action");
        assert_eq!(action.action_type, ActionType::ReliabilityAnalysis);
    }

    #[test]
    fn test_find_action_by_key() {
        let action = QuickAction::find_by_key('r');
        assert!(action.is_some());
        let action = action.unwrap();
        assert_eq!(action.action_type, ActionType::ReliabilityAnalysis);

        let action = QuickAction::find_by_key('p');
        assert!(action.is_some());
        let action = action.unwrap();
        assert_eq!(action.action_type, ActionType::PowerFlowAnalysis);

        let action = QuickAction::find_by_key('x');
        assert!(action.is_none());
    }

    // Grid management tests (Phase 3)

    #[test]
    fn test_grid_initial_state() {
        let state = DashboardPaneState::new();
        assert!(state.current_grid.is_none());
        assert_eq!(state.grid_count, 0);
    }

    #[test]
    fn test_set_current_grid() {
        let mut state = DashboardPaneState::new();
        let grid = GridInfo {
            id: "ieee14".to_string(),
            node_count: 14,
            branch_count: 20,
            density: 0.14,
            status: GridStatus::Active,
        };
        state.set_current_grid(grid.clone());
        assert!(state.current_grid.is_some());
        assert_eq!(state.current_grid.unwrap().id, "ieee14");
    }

    #[test]
    fn test_clear_current_grid() {
        let mut state = DashboardPaneState::new();
        let grid = GridInfo {
            id: "ieee14".to_string(),
            node_count: 14,
            branch_count: 20,
            density: 0.14,
            status: GridStatus::Active,
        };
        state.set_current_grid(grid);
        assert!(state.current_grid.is_some());
        state.clear_current_grid();
        assert!(state.current_grid.is_none());
    }

    #[test]
    fn test_update_grid_count() {
        let mut state = DashboardPaneState::new();
        assert_eq!(state.grid_count, 0);
        state.update_grid_count(3);
        assert_eq!(state.grid_count, 3);
    }

    #[test]
    fn test_grid_status_indicator() {
        let mut state = DashboardPaneState::new();
        assert_eq!(state.grid_status_indicator(), "○");

        let grid = GridInfo {
            id: "ieee14".to_string(),
            node_count: 14,
            branch_count: 20,
            density: 0.14,
            status: GridStatus::Active,
        };
        state.set_current_grid(grid);
        assert_eq!(state.grid_status_indicator(), "●");
    }

    #[test]
    fn test_grid_info_display() {
        let mut state = DashboardPaneState::new();
        assert_eq!(state.grid_info_display(), "No grid loaded");

        let grid = GridInfo {
            id: "ieee14".to_string(),
            node_count: 14,
            branch_count: 20,
            density: 0.14,
            status: GridStatus::Active,
        };
        state.set_current_grid(grid);
        let display = state.grid_info_display();
        assert!(display.contains("ieee14"));
        assert!(display.contains("14 nodes"));
        assert!(display.contains("20 branches"));
    }

    #[test]
    fn test_grid_density_percent() {
        let mut state = DashboardPaneState::new();
        assert!(state.grid_density_percent().is_none());

        let grid = GridInfo {
            id: "ieee14".to_string(),
            node_count: 14,
            branch_count: 20,
            density: 0.14,
            status: GridStatus::Active,
        };
        state.set_current_grid(grid);
        let density = state.grid_density_percent().unwrap();
        assert!((density - 14.0).abs() < 0.01); // 0.14 * 100 = 14.0
    }
}
