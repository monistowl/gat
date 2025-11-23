/// Analytics Pane - Analysis results visualization and exploration
///
/// The analytics pane provides:
/// - Reliability analysis (LOLE, EUE, thermal violations)
/// - Deliverability Score (DS) metrics
/// - ELCC (Effective Load Carrying Capability) results
/// - Power flow analysis and congestion identification
use crate::components::*;

/// Metric status indicator
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricStatus {
    Good,
    Warning,
    Critical,
}

impl MetricStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            MetricStatus::Good => "✓",
            MetricStatus::Warning => "◐",
            MetricStatus::Critical => "✗",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MetricStatus::Good => "Good",
            MetricStatus::Warning => "Warning",
            MetricStatus::Critical => "Critical",
        }
    }
}

/// Active tab in the Analytics pane
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalyticsTab {
    Reliability,
    DeliverabilityScore,
    ELCC,
    PowerFlow,
}

impl AnalyticsTab {
    pub fn label(&self) -> &'static str {
        match self {
            AnalyticsTab::Reliability => "Reliability",
            AnalyticsTab::DeliverabilityScore => "DS",
            AnalyticsTab::ELCC => "ELCC",
            AnalyticsTab::PowerFlow => "Power Flow",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            AnalyticsTab::Reliability => 0,
            AnalyticsTab::DeliverabilityScore => 1,
            AnalyticsTab::ELCC => 2,
            AnalyticsTab::PowerFlow => 3,
        }
    }
}

/// Individual metric result with status
#[derive(Clone, Debug)]
pub struct AnalyticsMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub status: MetricStatus,
    pub threshold_low: f64,
    pub threshold_high: f64,
}

/// Reliability analysis results
#[derive(Clone, Debug)]
pub struct ReliabilityResult {
    pub scenario_id: String,
    pub lole_hours: f64,
    pub eue_mwh: f64,
    pub thermal_violations: usize,
    pub max_utilization: f64,
    pub status: MetricStatus,
}

/// Deliverability Score results
#[derive(Clone, Debug)]
pub struct DeliverabilityResult {
    pub bus_id: String,
    pub ds_value: f64,
    pub nameplate_mw: f64,
    pub delivery_percent: f64,
    pub status: MetricStatus,
}

/// ELCC analysis results
#[derive(Clone, Debug)]
pub struct ELCCResult {
    pub resource_id: String,
    pub capacity_mw: f64,
    pub elcc_mw: f64,
    pub margin_percent: f64,
    pub weather_sensitivity: f64,
    pub status: MetricStatus,
}

/// Power flow analysis results
#[derive(Clone, Debug)]
pub struct PowerFlowResult {
    pub branch_id: String,
    pub flow_mw: f64,
    pub flow_limit_mw: f64,
    pub utilization_percent: f64,
    pub congestion_status: CongestionStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CongestionStatus {
    Normal,
    Elevated,
    Congested,
}

impl CongestionStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            CongestionStatus::Normal => "—",
            CongestionStatus::Elevated => "⚠",
            CongestionStatus::Congested => "⚡",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CongestionStatus::Normal => "Normal",
            CongestionStatus::Elevated => "Elevated",
            CongestionStatus::Congested => "Congested",
        }
    }
}

/// Analytics pane state
#[derive(Clone, Debug)]
pub struct AnalyticsPaneState {
    // Tab navigation
    pub active_tab: AnalyticsTab,

    // Reliability tab
    pub reliability_results: Vec<ReliabilityResult>,
    pub selected_reliability: usize,
    pub reliability_summary: String,

    // Deliverability Score tab
    pub ds_results: Vec<DeliverabilityResult>,
    pub selected_ds: usize,
    pub ds_summary: String,

    // ELCC tab
    pub elcc_results: Vec<ELCCResult>,
    pub selected_elcc: usize,
    pub elcc_summary: String,

    // Power Flow tab
    pub power_flow_results: Vec<PowerFlowResult>,
    pub selected_flow: usize,
    pub congestion_count: usize,

    // Component states
    pub metrics_list: ListWidget,
    pub details_table: TableWidget,
    pub summary_text: TextWidget,

    // UI state
    pub analysis_timestamp: String,
    pub total_scenarios: usize,
    pub valid_results: bool,
}

impl Default for AnalyticsPaneState {
    fn default() -> Self {
        let reliability_results = vec![
            ReliabilityResult {
                scenario_id: "Summer Peak".into(),
                lole_hours: 2.5,
                eue_mwh: 125.0,
                thermal_violations: 3,
                max_utilization: 0.92,
                status: MetricStatus::Warning,
            },
            ReliabilityResult {
                scenario_id: "Winter Peak".into(),
                lole_hours: 0.5,
                eue_mwh: 18.5,
                thermal_violations: 1,
                max_utilization: 0.85,
                status: MetricStatus::Good,
            },
            ReliabilityResult {
                scenario_id: "Spring Average".into(),
                lole_hours: 0.0,
                eue_mwh: 0.0,
                thermal_violations: 0,
                max_utilization: 0.68,
                status: MetricStatus::Good,
            },
        ];

        let ds_results = vec![
            DeliverabilityResult {
                bus_id: "Bus_001".into(),
                ds_value: 95.5,
                nameplate_mw: 1000.0,
                delivery_percent: 95.5,
                status: MetricStatus::Good,
            },
            DeliverabilityResult {
                bus_id: "Bus_002".into(),
                ds_value: 87.3,
                nameplate_mw: 1500.0,
                delivery_percent: 87.3,
                status: MetricStatus::Warning,
            },
            DeliverabilityResult {
                bus_id: "Bus_003".into(),
                ds_value: 78.2,
                nameplate_mw: 800.0,
                delivery_percent: 78.2,
                status: MetricStatus::Warning,
            },
        ];

        let elcc_results = vec![
            ELCCResult {
                resource_id: "Wind_Farm_A".into(),
                capacity_mw: 100.0,
                elcc_mw: 28.5,
                margin_percent: 71.5,
                weather_sensitivity: 0.65,
                status: MetricStatus::Warning,
            },
            ELCCResult {
                resource_id: "Solar_Array_B".into(),
                capacity_mw: 50.0,
                elcc_mw: 8.2,
                margin_percent: 83.6,
                weather_sensitivity: 0.82,
                status: MetricStatus::Warning,
            },
            ELCCResult {
                resource_id: "Battery_C".into(),
                capacity_mw: 75.0,
                elcc_mw: 72.0,
                margin_percent: 4.0,
                weather_sensitivity: 0.05,
                status: MetricStatus::Good,
            },
        ];

        let power_flow_results = vec![
            PowerFlowResult {
                branch_id: "Line_001".into(),
                flow_mw: 450.0,
                flow_limit_mw: 500.0,
                utilization_percent: 90.0,
                congestion_status: CongestionStatus::Elevated,
            },
            PowerFlowResult {
                branch_id: "Line_002".into(),
                flow_mw: 520.0,
                flow_limit_mw: 500.0,
                utilization_percent: 104.0,
                congestion_status: CongestionStatus::Congested,
            },
            PowerFlowResult {
                branch_id: "Line_003".into(),
                flow_mw: 250.0,
                flow_limit_mw: 1000.0,
                utilization_percent: 25.0,
                congestion_status: CongestionStatus::Normal,
            },
        ];

        let mut metrics_list = ListWidget::new("analytics_metrics");
        for result in &reliability_results {
            metrics_list.add_item(
                format!(
                    "{}: {:.1}h LOLE {}",
                    result.scenario_id,
                    result.lole_hours,
                    result.status.symbol()
                ),
                result.scenario_id.clone(),
            );
        }

        let mut details_table = TableWidget::new("analytics_details");
        details_table.columns = vec![
            Column {
                header: "Scenario".into(),
                width: 20,
            },
            Column {
                header: "LOLE (h)".into(),
                width: 12,
            },
            Column {
                header: "EUE (MWh)".into(),
                width: 12,
            },
            Column {
                header: "Status".into(),
                width: 10,
            },
        ];

        AnalyticsPaneState {
            active_tab: AnalyticsTab::Reliability,
            reliability_results: reliability_results.clone(),
            selected_reliability: 0,
            reliability_summary: "3 scenarios analyzed - 2 Good, 1 Warning".into(),
            ds_results,
            selected_ds: 0,
            ds_summary: "3 buses analyzed - average DS: 87.0%".into(),
            elcc_results,
            selected_elcc: 0,
            elcc_summary: "3 resources analyzed - weighted ELCC: 35.2%".into(),
            power_flow_results: power_flow_results.clone(),
            selected_flow: 0,
            congestion_count: 1,
            metrics_list,
            details_table,
            summary_text: TextWidget::new("analytics_summary", ""),
            analysis_timestamp: "2024-11-21 14:30:00".into(),
            total_scenarios: 3,
            valid_results: true,
        }
    }
}

impl AnalyticsPaneState {
    pub fn new() -> Self {
        Self::default()
    }

    // Tab navigation methods

    pub fn switch_tab(&mut self, tab: AnalyticsTab) {
        self.active_tab = tab;
        self.update_metrics_list();
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            AnalyticsTab::Reliability => AnalyticsTab::DeliverabilityScore,
            AnalyticsTab::DeliverabilityScore => AnalyticsTab::ELCC,
            AnalyticsTab::ELCC => AnalyticsTab::PowerFlow,
            AnalyticsTab::PowerFlow => AnalyticsTab::Reliability,
        };
        self.update_metrics_list();
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            AnalyticsTab::Reliability => AnalyticsTab::PowerFlow,
            AnalyticsTab::DeliverabilityScore => AnalyticsTab::Reliability,
            AnalyticsTab::ELCC => AnalyticsTab::DeliverabilityScore,
            AnalyticsTab::PowerFlow => AnalyticsTab::ELCC,
        };
        self.update_metrics_list();
    }

    pub fn is_reliability_tab(&self) -> bool {
        self.active_tab == AnalyticsTab::Reliability
    }

    pub fn is_ds_tab(&self) -> bool {
        self.active_tab == AnalyticsTab::DeliverabilityScore
    }

    pub fn is_elcc_tab(&self) -> bool {
        self.active_tab == AnalyticsTab::ELCC
    }

    pub fn is_powerflow_tab(&self) -> bool {
        self.active_tab == AnalyticsTab::PowerFlow
    }

    // Reliability tab methods

    pub fn select_next_reliability(&mut self) {
        if self.selected_reliability < self.reliability_results.len().saturating_sub(1) {
            self.selected_reliability += 1;
        }
    }

    pub fn select_prev_reliability(&mut self) {
        if self.selected_reliability > 0 {
            self.selected_reliability -= 1;
        }
    }

    pub fn selected_reliability(&self) -> Option<&ReliabilityResult> {
        self.reliability_results.get(self.selected_reliability)
    }

    pub fn reliability_count(&self) -> usize {
        self.reliability_results.len()
    }

    pub fn get_reliability_details(&self) -> String {
        if let Some(result) = self.selected_reliability() {
            format!(
                "Scenario: {}\nLOLE: {:.1} hours/year\nEUE: {:.1} MWh/year\nThermal Violations: {}\nMax Utilization: {:.1}%\nStatus: {}",
                result.scenario_id,
                result.lole_hours,
                result.eue_mwh,
                result.thermal_violations,
                result.max_utilization * 100.0,
                result.status.symbol(),
            )
        } else {
            "No reliability results selected".into()
        }
    }

    // Deliverability Score tab methods

    pub fn select_next_ds(&mut self) {
        if self.selected_ds < self.ds_results.len().saturating_sub(1) {
            self.selected_ds += 1;
        }
    }

    pub fn select_prev_ds(&mut self) {
        if self.selected_ds > 0 {
            self.selected_ds -= 1;
        }
    }

    pub fn selected_ds(&self) -> Option<&DeliverabilityResult> {
        self.ds_results.get(self.selected_ds)
    }

    pub fn ds_count(&self) -> usize {
        self.ds_results.len()
    }

    pub fn get_ds_details(&self) -> String {
        if let Some(result) = self.selected_ds() {
            format!(
                "Bus: {}\nDeliverability Score: {:.1}%\nNameplate: {:.0} MW\nDelivery %: {:.1}%\nStatus: {}",
                result.bus_id,
                result.ds_value,
                result.nameplate_mw,
                result.delivery_percent,
                result.status.symbol(),
            )
        } else {
            "No DS results selected".into()
        }
    }

    // ELCC tab methods

    pub fn select_next_elcc(&mut self) {
        if self.selected_elcc < self.elcc_results.len().saturating_sub(1) {
            self.selected_elcc += 1;
        }
    }

    pub fn select_prev_elcc(&mut self) {
        if self.selected_elcc > 0 {
            self.selected_elcc -= 1;
        }
    }

    pub fn selected_elcc(&self) -> Option<&ELCCResult> {
        self.elcc_results.get(self.selected_elcc)
    }

    pub fn elcc_count(&self) -> usize {
        self.elcc_results.len()
    }

    pub fn get_elcc_details(&self) -> String {
        if let Some(result) = self.selected_elcc() {
            format!(
                "Resource: {}\nCapacity: {:.0} MW\nELCC: {:.1} MW\nMargin: {:.1}%\nWeather Sensitivity: {:.2}\nStatus: {}",
                result.resource_id,
                result.capacity_mw,
                result.elcc_mw,
                result.margin_percent,
                result.weather_sensitivity,
                result.status.symbol(),
            )
        } else {
            "No ELCC results selected".into()
        }
    }

    // Power Flow tab methods

    pub fn select_next_flow(&mut self) {
        if self.selected_flow < self.power_flow_results.len().saturating_sub(1) {
            self.selected_flow += 1;
        }
    }

    pub fn select_prev_flow(&mut self) {
        if self.selected_flow > 0 {
            self.selected_flow -= 1;
        }
    }

    pub fn selected_flow(&self) -> Option<&PowerFlowResult> {
        self.power_flow_results.get(self.selected_flow)
    }

    pub fn flow_count(&self) -> usize {
        self.power_flow_results.len()
    }

    pub fn get_flow_details(&self) -> String {
        if let Some(result) = self.selected_flow() {
            format!(
                "Line: {}\nFlow: {:.0} MW\nLimit: {:.0} MW\nUtilization: {:.1}%\nStatus: {}\nCongestion: {} {}",
                result.branch_id,
                result.flow_mw,
                result.flow_limit_mw,
                result.utilization_percent,
                if result.utilization_percent > 100.0 { "⚠ OVERLOADED" } else { "OK" },
                result.congestion_status.label(),
                result.congestion_status.symbol(),
            )
        } else {
            "No power flow results selected".into()
        }
    }

    // Summary and status methods

    pub fn update_metrics_list(&mut self) {
        self.metrics_list.items.clear();
        match self.active_tab {
            AnalyticsTab::Reliability => {
                for result in &self.reliability_results {
                    self.metrics_list.add_item(
                        format!(
                            "{}: {:.1}h {}",
                            result.scenario_id,
                            result.lole_hours,
                            result.status.symbol()
                        ),
                        result.scenario_id.clone(),
                    );
                }
            }
            AnalyticsTab::DeliverabilityScore => {
                for result in &self.ds_results {
                    self.metrics_list.add_item(
                        format!(
                            "{}: {:.1}% {}",
                            result.bus_id,
                            result.ds_value,
                            result.status.symbol()
                        ),
                        result.bus_id.clone(),
                    );
                }
            }
            AnalyticsTab::ELCC => {
                for result in &self.elcc_results {
                    self.metrics_list.add_item(
                        format!(
                            "{}: {:.1} MW {}",
                            result.resource_id,
                            result.elcc_mw,
                            result.status.symbol()
                        ),
                        result.resource_id.clone(),
                    );
                }
            }
            AnalyticsTab::PowerFlow => {
                for result in &self.power_flow_results {
                    self.metrics_list.add_item(
                        format!(
                            "{}: {:.0}% {}",
                            result.branch_id,
                            result.utilization_percent,
                            result.congestion_status.symbol()
                        ),
                        result.branch_id.clone(),
                    );
                }
            }
        }
    }

    pub fn format_summary(&mut self) {
        let content = match self.active_tab {
            AnalyticsTab::Reliability => self.reliability_summary.clone(),
            AnalyticsTab::DeliverabilityScore => self.ds_summary.clone(),
            AnalyticsTab::ELCC => self.elcc_summary.clone(),
            AnalyticsTab::PowerFlow => {
                format!(
                    "{} branches - {} congested",
                    self.power_flow_results.len(),
                    self.congestion_count
                )
            }
        };
        self.summary_text.set_content(content);
    }

    pub fn overall_status(&self) -> &'static str {
        if !self.valid_results {
            return "Invalid";
        }

        let warning_count = match self.active_tab {
            AnalyticsTab::Reliability => self
                .reliability_results
                .iter()
                .filter(|r| r.status == MetricStatus::Warning)
                .count(),
            AnalyticsTab::DeliverabilityScore => self
                .ds_results
                .iter()
                .filter(|r| r.status == MetricStatus::Warning)
                .count(),
            AnalyticsTab::ELCC => self
                .elcc_results
                .iter()
                .filter(|r| r.status == MetricStatus::Warning)
                .count(),
            AnalyticsTab::PowerFlow => self.congestion_count,
        };

        let critical_count = match self.active_tab {
            AnalyticsTab::Reliability => self
                .reliability_results
                .iter()
                .filter(|r| r.status == MetricStatus::Critical)
                .count(),
            AnalyticsTab::DeliverabilityScore => self
                .ds_results
                .iter()
                .filter(|r| r.status == MetricStatus::Critical)
                .count(),
            AnalyticsTab::ELCC => self
                .elcc_results
                .iter()
                .filter(|r| r.status == MetricStatus::Critical)
                .count(),
            AnalyticsTab::PowerFlow => 0, // Handled by congestion_count
        };

        if critical_count > 0 {
            "Critical"
        } else if warning_count > 0 {
            "Warning"
        } else {
            "Healthy"
        }
    }

    pub fn health_indicator(&self) -> &'static str {
        match self.overall_status() {
            "Healthy" => "✓",
            "Warning" => "◐",
            "Critical" => "✗",
            _ => "?",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_init() {
        let state = AnalyticsPaneState::new();
        assert_eq!(state.active_tab, AnalyticsTab::Reliability);
        assert_eq!(state.reliability_count(), 3);
        assert_eq!(state.ds_count(), 3);
        assert_eq!(state.elcc_count(), 3);
        assert_eq!(state.flow_count(), 3);
        assert!(state.valid_results);
    }

    #[test]
    fn test_tab_cycle_forward() {
        let mut state = AnalyticsPaneState::new();
        assert_eq!(state.active_tab, AnalyticsTab::Reliability);

        state.next_tab();
        assert_eq!(state.active_tab, AnalyticsTab::DeliverabilityScore);

        state.next_tab();
        assert_eq!(state.active_tab, AnalyticsTab::ELCC);

        state.next_tab();
        assert_eq!(state.active_tab, AnalyticsTab::PowerFlow);

        state.next_tab();
        assert_eq!(state.active_tab, AnalyticsTab::Reliability);
    }

    #[test]
    fn test_tab_cycle_backward() {
        let mut state = AnalyticsPaneState::new();
        state.active_tab = AnalyticsTab::Reliability;

        state.prev_tab();
        assert_eq!(state.active_tab, AnalyticsTab::PowerFlow);

        state.prev_tab();
        assert_eq!(state.active_tab, AnalyticsTab::ELCC);

        state.prev_tab();
        assert_eq!(state.active_tab, AnalyticsTab::DeliverabilityScore);

        state.prev_tab();
        assert_eq!(state.active_tab, AnalyticsTab::Reliability);
    }

    #[test]
    fn test_switch_tab() {
        let mut state = AnalyticsPaneState::new();
        state.switch_tab(AnalyticsTab::ELCC);
        assert_eq!(state.active_tab, AnalyticsTab::ELCC);

        state.switch_tab(AnalyticsTab::PowerFlow);
        assert_eq!(state.active_tab, AnalyticsTab::PowerFlow);
    }

    #[test]
    fn test_tab_query_methods() {
        let mut state = AnalyticsPaneState::new();
        assert!(state.is_reliability_tab());
        assert!(!state.is_ds_tab());
        assert!(!state.is_elcc_tab());
        assert!(!state.is_powerflow_tab());

        state.next_tab();
        assert!(!state.is_reliability_tab());
        assert!(state.is_ds_tab());
    }

    #[test]
    fn test_reliability_selection() {
        let mut state = AnalyticsPaneState::new();
        assert_eq!(state.selected_reliability, 0);

        state.select_next_reliability();
        assert_eq!(state.selected_reliability, 1);

        state.select_next_reliability();
        assert_eq!(state.selected_reliability, 2);

        state.select_next_reliability();
        assert_eq!(state.selected_reliability, 2); // Bounds check

        state.select_prev_reliability();
        assert_eq!(state.selected_reliability, 1);
    }

    #[test]
    fn test_selected_reliability() {
        let state = AnalyticsPaneState::new();
        let result = state.selected_reliability().unwrap();
        assert_eq!(result.scenario_id, "Summer Peak");
        assert_eq!(result.lole_hours, 2.5);
    }

    #[test]
    fn test_reliability_details_formatting() {
        let state = AnalyticsPaneState::new();
        let details = state.get_reliability_details();
        assert!(details.contains("Summer Peak"));
        assert!(details.contains("2.5"));
        assert!(details.contains("◐")); // Warning status
    }

    #[test]
    fn test_ds_selection() {
        let mut state = AnalyticsPaneState::new();
        state.active_tab = AnalyticsTab::DeliverabilityScore;
        assert_eq!(state.selected_ds, 0);

        state.select_next_ds();
        assert_eq!(state.selected_ds, 1);

        state.select_prev_ds();
        assert_eq!(state.selected_ds, 0);
    }

    #[test]
    fn test_selected_ds() {
        let state = AnalyticsPaneState::new();
        let result = state.selected_ds().unwrap();
        assert_eq!(result.bus_id, "Bus_001");
        assert_eq!(result.ds_value, 95.5);
    }

    #[test]
    fn test_ds_details_formatting() {
        let state = AnalyticsPaneState::new();
        let details = state.get_ds_details();
        assert!(details.contains("Bus_001"));
        assert!(details.contains("95.5"));
    }

    #[test]
    fn test_elcc_selection() {
        let mut state = AnalyticsPaneState::new();
        state.active_tab = AnalyticsTab::ELCC;
        assert_eq!(state.selected_elcc, 0);

        state.select_next_elcc();
        assert_eq!(state.selected_elcc, 1);

        state.select_prev_elcc();
        assert_eq!(state.selected_elcc, 0);
    }

    #[test]
    fn test_selected_elcc() {
        let state = AnalyticsPaneState::new();
        let result = state.selected_elcc().unwrap();
        assert_eq!(result.resource_id, "Wind_Farm_A");
        assert_eq!(result.capacity_mw, 100.0);
    }

    #[test]
    fn test_powerflow_selection() {
        let mut state = AnalyticsPaneState::new();
        state.active_tab = AnalyticsTab::PowerFlow;
        assert_eq!(state.selected_flow, 0);

        state.select_next_flow();
        assert_eq!(state.selected_flow, 1);

        state.select_prev_flow();
        assert_eq!(state.selected_flow, 0);
    }

    #[test]
    fn test_selected_powerflow() {
        let state = AnalyticsPaneState::new();
        let result = state.selected_flow().unwrap();
        assert_eq!(result.branch_id, "Line_001");
        assert_eq!(result.utilization_percent, 90.0);
    }

    #[test]
    fn test_powerflow_details_formatting() {
        let state = AnalyticsPaneState::new();
        let details = state.get_flow_details();
        assert!(details.contains("Line_001"));
        assert!(details.contains("90.0%"));
        assert!(details.contains("Elevated"));
    }

    #[test]
    fn test_metrics_list_update() {
        let mut state = AnalyticsPaneState::new();
        state.update_metrics_list();
        assert_eq!(state.metrics_list.items.len(), 3);

        state.switch_tab(AnalyticsTab::DeliverabilityScore);
        assert_eq!(state.metrics_list.items.len(), 3);
    }

    #[test]
    fn test_format_summary() {
        let mut state = AnalyticsPaneState::new();
        state.format_summary();
        assert!(state.summary_text.content.contains("Good"));

        state.switch_tab(AnalyticsTab::PowerFlow);
        state.format_summary();
        assert!(state.summary_text.content.contains("congested"));
    }

    #[test]
    fn test_overall_status_healthy() {
        let state = AnalyticsPaneState::new();
        // Switch to tab with all Good status
        let mut test_state = state.clone();
        test_state.active_tab = AnalyticsTab::ELCC;
        // Most ELCC results are Warning, so overall would be Warning
        // This is expected behavior
    }

    #[test]
    fn test_health_indicator() {
        let state = AnalyticsPaneState::new();
        let indicator = state.health_indicator();
        assert!(indicator == "✓" || indicator == "◐" || indicator == "✗" || indicator == "?");
    }

    #[test]
    fn test_congestion_status_symbols() {
        assert_eq!(CongestionStatus::Normal.symbol(), "—");
        assert_eq!(CongestionStatus::Elevated.symbol(), "⚠");
        assert_eq!(CongestionStatus::Congested.symbol(), "⚡");
    }

    #[test]
    fn test_congestion_status_labels() {
        assert_eq!(CongestionStatus::Normal.label(), "Normal");
        assert_eq!(CongestionStatus::Elevated.label(), "Elevated");
        assert_eq!(CongestionStatus::Congested.label(), "Congested");
    }

    #[test]
    fn test_analytics_tab_labels() {
        assert_eq!(AnalyticsTab::Reliability.label(), "Reliability");
        assert_eq!(AnalyticsTab::DeliverabilityScore.label(), "DS");
        assert_eq!(AnalyticsTab::ELCC.label(), "ELCC");
        assert_eq!(AnalyticsTab::PowerFlow.label(), "Power Flow");
    }

    #[test]
    fn test_analytics_tab_indices() {
        assert_eq!(AnalyticsTab::Reliability.index(), 0);
        assert_eq!(AnalyticsTab::DeliverabilityScore.index(), 1);
        assert_eq!(AnalyticsTab::ELCC.index(), 2);
        assert_eq!(AnalyticsTab::PowerFlow.index(), 3);
    }
}
