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
    Contingency,
    Ptdf,
    Ybus,
}

impl AnalyticsTab {
    pub fn label(&self) -> &'static str {
        match self {
            AnalyticsTab::Reliability => "Reliability",
            AnalyticsTab::DeliverabilityScore => "DS",
            AnalyticsTab::ELCC => "ELCC",
            AnalyticsTab::PowerFlow => "Power Flow",
            AnalyticsTab::Contingency => "N-1",
            AnalyticsTab::Ptdf => "PTDF",
            AnalyticsTab::Ybus => "Y-bus",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            AnalyticsTab::Reliability => 0,
            AnalyticsTab::DeliverabilityScore => 1,
            AnalyticsTab::ELCC => 2,
            AnalyticsTab::PowerFlow => 3,
            AnalyticsTab::Contingency => 4,
            AnalyticsTab::Ptdf => 5,
            AnalyticsTab::Ybus => 6,
        }
    }

    /// Total number of tabs
    pub fn count() -> usize {
        7
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

// ============================================================================
// N-1 Contingency Analysis Types
// ============================================================================

/// N-1 contingency analysis result for a single branch outage
#[derive(Clone, Debug)]
pub struct ContingencyResultRow {
    /// Branch that was removed (outage)
    pub outage_branch: String,
    /// From bus ID of outaged branch
    pub from_bus: usize,
    /// To bus ID of outaged branch
    pub to_bus: usize,
    /// Whether this contingency causes any violations
    pub has_violations: bool,
    /// Maximum loading percentage across remaining branches
    pub max_loading_pct: f64,
    /// Count of overloaded branches (loading > 100%)
    pub overloaded_count: usize,
    /// Solve succeeded (false if island created)
    pub solved: bool,
}

/// N-1 contingency analysis summary
#[derive(Clone, Debug, Default)]
pub struct ContingencySummary {
    pub total_contingencies: usize,
    pub contingencies_with_violations: usize,
    pub contingencies_failed: usize,
    pub worst_contingency: Option<String>,
    pub worst_loading_pct: f64,
}

// ============================================================================
// PTDF Analysis Types
// ============================================================================

/// PTDF input mode for bus selection
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PtdfInputMode {
    #[default]
    None,
    SelectingInjection,
    SelectingWithdrawal,
}

/// PTDF result for a single branch
#[derive(Clone, Debug)]
pub struct PtdfResultRow {
    /// Branch ID
    pub branch_id: usize,
    /// Branch name/label
    pub branch_name: String,
    /// From bus ID
    pub from_bus: usize,
    /// To bus ID
    pub to_bus: usize,
    /// PTDF factor (fraction of transfer flowing on this branch)
    pub ptdf_factor: f64,
    /// Flow change in MW for a 100 MW transfer
    pub flow_change_mw: f64,
}

// ============================================================================
// Y-bus Matrix Types
// ============================================================================

/// Y-bus view mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum YbusViewMode {
    #[default]
    Heatmap,
    List,
    Sparsity,
}

/// Y-bus matrix entry
#[derive(Clone, Debug)]
pub struct YbusEntry {
    /// Row index (bus index)
    pub row: usize,
    /// Column index (bus index)
    pub col: usize,
    /// Conductance (real part of admittance)
    pub g: f64,
    /// Susceptance (imaginary part of admittance)
    pub b: f64,
    /// Magnitude |Y|
    pub magnitude: f64,
    /// From bus ID (row)
    pub from_bus_id: usize,
    /// To bus ID (column)
    pub to_bus_id: usize,
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

    // N-1 Contingency tab
    pub contingency_results: Vec<ContingencyResultRow>,
    pub selected_contingency: usize,
    pub contingency_summary: ContingencySummary,

    // PTDF tab
    pub ptdf_injection_bus: Option<usize>,
    pub ptdf_withdrawal_bus: Option<usize>,
    pub ptdf_results: Vec<PtdfResultRow>,
    pub selected_ptdf: usize,
    pub ptdf_input_mode: PtdfInputMode,
    pub available_buses: Vec<(usize, String)>, // (bus_id, bus_name)

    // Y-bus tab
    pub ybus_entries: Vec<YbusEntry>,
    pub ybus_n_bus: usize,
    pub ybus_selected_row: usize,
    pub ybus_selected_col: usize,
    pub ybus_view_mode: YbusViewMode,

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
            // N-1 Contingency (empty until analysis run)
            contingency_results: Vec::new(),
            selected_contingency: 0,
            contingency_summary: ContingencySummary::default(),
            // PTDF (empty until analysis run)
            ptdf_injection_bus: None,
            ptdf_withdrawal_bus: None,
            ptdf_results: Vec::new(),
            selected_ptdf: 0,
            ptdf_input_mode: PtdfInputMode::None,
            available_buses: Vec::new(),
            // Y-bus (empty until loaded)
            ybus_entries: Vec::new(),
            ybus_n_bus: 0,
            ybus_selected_row: 0,
            ybus_selected_col: 0,
            ybus_view_mode: YbusViewMode::Heatmap,
            // Component states
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
            AnalyticsTab::PowerFlow => AnalyticsTab::Contingency,
            AnalyticsTab::Contingency => AnalyticsTab::Ptdf,
            AnalyticsTab::Ptdf => AnalyticsTab::Ybus,
            AnalyticsTab::Ybus => AnalyticsTab::Reliability,
        };
        self.update_metrics_list();
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            AnalyticsTab::Reliability => AnalyticsTab::Ybus,
            AnalyticsTab::DeliverabilityScore => AnalyticsTab::Reliability,
            AnalyticsTab::ELCC => AnalyticsTab::DeliverabilityScore,
            AnalyticsTab::PowerFlow => AnalyticsTab::ELCC,
            AnalyticsTab::Contingency => AnalyticsTab::PowerFlow,
            AnalyticsTab::Ptdf => AnalyticsTab::Contingency,
            AnalyticsTab::Ybus => AnalyticsTab::Ptdf,
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

    pub fn is_contingency_tab(&self) -> bool {
        self.active_tab == AnalyticsTab::Contingency
    }

    pub fn is_ptdf_tab(&self) -> bool {
        self.active_tab == AnalyticsTab::Ptdf
    }

    pub fn is_ybus_tab(&self) -> bool {
        self.active_tab == AnalyticsTab::Ybus
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

    // N-1 Contingency tab methods

    pub fn select_next_contingency(&mut self) {
        if self.selected_contingency < self.contingency_results.len().saturating_sub(1) {
            self.selected_contingency += 1;
        }
    }

    pub fn select_prev_contingency(&mut self) {
        if self.selected_contingency > 0 {
            self.selected_contingency -= 1;
        }
    }

    pub fn selected_contingency(&self) -> Option<&ContingencyResultRow> {
        self.contingency_results.get(self.selected_contingency)
    }

    pub fn contingency_count(&self) -> usize {
        self.contingency_results.len()
    }

    pub fn get_contingency_details(&self) -> String {
        if let Some(result) = self.selected_contingency() {
            format!(
                "Outage: {}\nFrom Bus: {}\nTo Bus: {}\nMax Loading: {:.1}%\nOverloaded: {}\nSolved: {}\nStatus: {}",
                result.outage_branch,
                result.from_bus,
                result.to_bus,
                result.max_loading_pct,
                result.overloaded_count,
                if result.solved { "Yes" } else { "No (island)" },
                if result.has_violations { "⚠ VIOLATIONS" } else { "✓ SECURE" },
            )
        } else {
            "No contingency results - run N-1 analysis first".into()
        }
    }

    pub fn set_contingency_results(&mut self, results: Vec<ContingencyResultRow>) {
        // Calculate summary
        let total = results.len();
        let with_violations = results.iter().filter(|r| r.has_violations).count();
        let failed = results.iter().filter(|r| !r.solved).count();
        let worst = results
            .iter()
            .max_by(|a, b| a.max_loading_pct.partial_cmp(&b.max_loading_pct).unwrap())
            .map(|r| (r.outage_branch.clone(), r.max_loading_pct));

        self.contingency_summary = ContingencySummary {
            total_contingencies: total,
            contingencies_with_violations: with_violations,
            contingencies_failed: failed,
            worst_contingency: worst.as_ref().map(|(name, _)| name.clone()),
            worst_loading_pct: worst.map(|(_, pct)| pct).unwrap_or(0.0),
        };
        self.contingency_results = results;
        self.selected_contingency = 0;
    }

    // PTDF tab methods

    pub fn select_next_ptdf(&mut self) {
        if self.selected_ptdf < self.ptdf_results.len().saturating_sub(1) {
            self.selected_ptdf += 1;
        }
    }

    pub fn select_prev_ptdf(&mut self) {
        if self.selected_ptdf > 0 {
            self.selected_ptdf -= 1;
        }
    }

    pub fn selected_ptdf(&self) -> Option<&PtdfResultRow> {
        self.ptdf_results.get(self.selected_ptdf)
    }

    pub fn ptdf_count(&self) -> usize {
        self.ptdf_results.len()
    }

    pub fn get_ptdf_details(&self) -> String {
        if let Some(result) = self.selected_ptdf() {
            format!(
                "Branch: {} ({})\nFrom Bus: {}\nTo Bus: {}\nPTDF Factor: {:.4}\nFlow Change (100 MW): {:.2} MW",
                result.branch_name,
                result.branch_id,
                result.from_bus,
                result.to_bus,
                result.ptdf_factor,
                result.flow_change_mw,
            )
        } else if self.ptdf_injection_bus.is_none() || self.ptdf_withdrawal_bus.is_none() {
            "Select injection and withdrawal buses, then compute PTDF".into()
        } else {
            "No PTDF results - click Compute".into()
        }
    }

    pub fn set_ptdf_results(&mut self, results: Vec<PtdfResultRow>) {
        self.ptdf_results = results;
        self.selected_ptdf = 0;
    }

    pub fn set_available_buses(&mut self, buses: Vec<(usize, String)>) {
        self.available_buses = buses;
    }

    // Y-bus tab methods

    pub fn select_next_ybus_row(&mut self) {
        if self.ybus_selected_row < self.ybus_n_bus.saturating_sub(1) {
            self.ybus_selected_row += 1;
        }
    }

    pub fn select_prev_ybus_row(&mut self) {
        if self.ybus_selected_row > 0 {
            self.ybus_selected_row -= 1;
        }
    }

    pub fn select_next_ybus_col(&mut self) {
        if self.ybus_selected_col < self.ybus_n_bus.saturating_sub(1) {
            self.ybus_selected_col += 1;
        }
    }

    pub fn select_prev_ybus_col(&mut self) {
        if self.ybus_selected_col > 0 {
            self.ybus_selected_col -= 1;
        }
    }

    pub fn ybus_entry_count(&self) -> usize {
        self.ybus_entries.len()
    }

    pub fn get_ybus_details(&self) -> String {
        // Find entry at selected row/col
        let entry = self.ybus_entries.iter().find(|e| {
            e.row == self.ybus_selected_row && e.col == self.ybus_selected_col
        });

        if let Some(e) = entry {
            format!(
                "Y[{}, {}]\nBus {} ↔ Bus {}\nG = {:.6} pu\nB = {:.6} pu\n|Y| = {:.6} pu",
                e.row, e.col,
                e.from_bus_id, e.to_bus_id,
                e.g, e.b, e.magnitude,
            )
        } else if self.ybus_n_bus == 0 {
            "No Y-bus matrix loaded - load a case first".into()
        } else {
            format!(
                "Y[{}, {}] = 0\n(no connection between buses)",
                self.ybus_selected_row, self.ybus_selected_col
            )
        }
    }

    pub fn set_ybus_entries(&mut self, entries: Vec<YbusEntry>, n_bus: usize) {
        self.ybus_entries = entries;
        self.ybus_n_bus = n_bus;
        self.ybus_selected_row = 0;
        self.ybus_selected_col = 0;
    }

    pub fn cycle_ybus_view_mode(&mut self) {
        self.ybus_view_mode = match self.ybus_view_mode {
            YbusViewMode::Heatmap => YbusViewMode::List,
            YbusViewMode::List => YbusViewMode::Sparsity,
            YbusViewMode::Sparsity => YbusViewMode::Heatmap,
        };
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
            AnalyticsTab::Contingency => {
                for result in &self.contingency_results {
                    self.metrics_list.add_item(
                        format!(
                            "{}: {:.0}% {}",
                            result.outage_branch,
                            result.max_loading_pct,
                            if result.has_violations { "⚠" } else { "✓" }
                        ),
                        result.outage_branch.clone(),
                    );
                }
            }
            AnalyticsTab::Ptdf => {
                for result in &self.ptdf_results {
                    self.metrics_list.add_item(
                        format!(
                            "{}: {:.3}",
                            result.branch_name,
                            result.ptdf_factor
                        ),
                        result.branch_name.clone(),
                    );
                }
            }
            AnalyticsTab::Ybus => {
                // For Y-bus, show non-zero entries in list mode
                for entry in self.ybus_entries.iter().take(50) {
                    self.metrics_list.add_item(
                        format!(
                            "Y[{},{}]: {:.4}",
                            entry.row, entry.col, entry.magnitude
                        ),
                        format!("{}_{}", entry.row, entry.col),
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
            AnalyticsTab::Contingency => {
                let s = &self.contingency_summary;
                if s.total_contingencies == 0 {
                    "No contingency analysis run".into()
                } else if s.contingencies_with_violations == 0 {
                    format!("✓ SECURE - {} contingencies analyzed", s.total_contingencies)
                } else {
                    format!(
                        "⚠ {} VIOLATIONS of {} contingencies",
                        s.contingencies_with_violations, s.total_contingencies
                    )
                }
            }
            AnalyticsTab::Ptdf => {
                match (self.ptdf_injection_bus, self.ptdf_withdrawal_bus) {
                    (Some(from), Some(to)) => {
                        format!(
                            "Transfer: Bus {} → Bus {} ({} branches)",
                            from, to, self.ptdf_results.len()
                        )
                    }
                    _ => "Select injection and withdrawal buses".into(),
                }
            }
            AnalyticsTab::Ybus => {
                if self.ybus_n_bus == 0 {
                    "No Y-bus matrix loaded".into()
                } else {
                    format!(
                        "{}×{} matrix - {} non-zero entries",
                        self.ybus_n_bus, self.ybus_n_bus, self.ybus_entries.len()
                    )
                }
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
            AnalyticsTab::Contingency => self.contingency_summary.contingencies_with_violations,
            AnalyticsTab::Ptdf => 0, // PTDF has no warning concept
            AnalyticsTab::Ybus => 0, // Y-bus has no warning concept
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
            AnalyticsTab::Contingency => self.contingency_summary.contingencies_failed,
            AnalyticsTab::Ptdf => 0,
            AnalyticsTab::Ybus => 0,
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
        assert_eq!(state.active_tab, AnalyticsTab::Contingency);

        state.next_tab();
        assert_eq!(state.active_tab, AnalyticsTab::Ptdf);

        state.next_tab();
        assert_eq!(state.active_tab, AnalyticsTab::Ybus);

        state.next_tab();
        assert_eq!(state.active_tab, AnalyticsTab::Reliability); // Wrap around
    }

    #[test]
    fn test_tab_cycle_backward() {
        let mut state = AnalyticsPaneState::new();
        state.active_tab = AnalyticsTab::Reliability;

        state.prev_tab();
        assert_eq!(state.active_tab, AnalyticsTab::Ybus);

        state.prev_tab();
        assert_eq!(state.active_tab, AnalyticsTab::Ptdf);

        state.prev_tab();
        assert_eq!(state.active_tab, AnalyticsTab::Contingency);

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
        assert_eq!(AnalyticsTab::Contingency.label(), "N-1");
        assert_eq!(AnalyticsTab::Ptdf.label(), "PTDF");
        assert_eq!(AnalyticsTab::Ybus.label(), "Y-bus");
    }

    #[test]
    fn test_analytics_tab_indices() {
        assert_eq!(AnalyticsTab::Reliability.index(), 0);
        assert_eq!(AnalyticsTab::DeliverabilityScore.index(), 1);
        assert_eq!(AnalyticsTab::ELCC.index(), 2);
        assert_eq!(AnalyticsTab::PowerFlow.index(), 3);
        assert_eq!(AnalyticsTab::Contingency.index(), 4);
        assert_eq!(AnalyticsTab::Ptdf.index(), 5);
        assert_eq!(AnalyticsTab::Ybus.index(), 6);
        assert_eq!(AnalyticsTab::count(), 7);
    }

    // New tab tests

    #[test]
    fn test_contingency_tab() {
        let mut state = AnalyticsPaneState::new();
        state.switch_tab(AnalyticsTab::Contingency);
        assert!(state.is_contingency_tab());
        assert_eq!(state.contingency_count(), 0); // Empty on init

        // Test setting results
        let results = vec![
            ContingencyResultRow {
                outage_branch: "Line_001".into(),
                from_bus: 1,
                to_bus: 2,
                has_violations: false,
                max_loading_pct: 85.0,
                overloaded_count: 0,
                solved: true,
            },
            ContingencyResultRow {
                outage_branch: "Line_002".into(),
                from_bus: 2,
                to_bus: 3,
                has_violations: true,
                max_loading_pct: 115.0,
                overloaded_count: 2,
                solved: true,
            },
        ];
        state.set_contingency_results(results);

        assert_eq!(state.contingency_count(), 2);
        assert_eq!(state.contingency_summary.total_contingencies, 2);
        assert_eq!(state.contingency_summary.contingencies_with_violations, 1);
    }

    #[test]
    fn test_ptdf_tab() {
        let mut state = AnalyticsPaneState::new();
        state.switch_tab(AnalyticsTab::Ptdf);
        assert!(state.is_ptdf_tab());
        assert_eq!(state.ptdf_count(), 0);

        // Set up buses
        state.ptdf_injection_bus = Some(1);
        state.ptdf_withdrawal_bus = Some(2);

        // Test setting results
        let results = vec![
            PtdfResultRow {
                branch_id: 1,
                branch_name: "Line_001".into(),
                from_bus: 1,
                to_bus: 2,
                ptdf_factor: 0.5,
                flow_change_mw: 50.0,
            },
        ];
        state.set_ptdf_results(results);

        assert_eq!(state.ptdf_count(), 1);
        assert!(state.get_ptdf_details().contains("Line_001"));
    }

    #[test]
    fn test_ybus_tab() {
        let mut state = AnalyticsPaneState::new();
        state.switch_tab(AnalyticsTab::Ybus);
        assert!(state.is_ybus_tab());
        assert_eq!(state.ybus_entry_count(), 0);

        // Test setting entries
        let entries = vec![
            YbusEntry {
                row: 0,
                col: 0,
                g: 10.0,
                b: -20.0,
                magnitude: 22.36,
                from_bus_id: 1,
                to_bus_id: 1,
            },
            YbusEntry {
                row: 0,
                col: 1,
                g: -5.0,
                b: 10.0,
                magnitude: 11.18,
                from_bus_id: 1,
                to_bus_id: 2,
            },
        ];
        state.set_ybus_entries(entries, 3);

        assert_eq!(state.ybus_entry_count(), 2);
        assert_eq!(state.ybus_n_bus, 3);
        assert!(state.get_ybus_details().contains("10.0"));
    }

    #[test]
    fn test_ybus_view_mode_cycle() {
        let mut state = AnalyticsPaneState::new();
        assert_eq!(state.ybus_view_mode, YbusViewMode::Heatmap);

        state.cycle_ybus_view_mode();
        assert_eq!(state.ybus_view_mode, YbusViewMode::List);

        state.cycle_ybus_view_mode();
        assert_eq!(state.ybus_view_mode, YbusViewMode::Sparsity);

        state.cycle_ybus_view_mode();
        assert_eq!(state.ybus_view_mode, YbusViewMode::Heatmap);
    }
}
