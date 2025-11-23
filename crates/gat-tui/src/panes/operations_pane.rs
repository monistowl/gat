/// Operations Pane - Batch processing, allocation, and reliability analysis
///
/// The operations pane provides:
/// - Batch job management
/// - Allocation operations (rents, contributions)
/// - Reliability metrics and analysis
/// - Multi-tab interface for different operation types
use crate::components::*;
use crate::data::Workflow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationType {
    Batch,
    Allocation,
    Reliability,
}

impl OperationType {
    pub fn label(&self) -> &'static str {
        match self {
            OperationType::Batch => "Batch",
            OperationType::Allocation => "Allocation",
            OperationType::Reliability => "Reliability",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            OperationType::Batch => 0,
            OperationType::Allocation => 1,
            OperationType::Reliability => 2,
        }
    }
}

/// Batch job entry
#[derive(Clone, Debug)]
pub struct BatchJob {
    pub id: String,
    pub name: String,
    pub status: JobStatus,
    pub progress: u32,
    pub start_time: String,
    pub est_completion: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            JobStatus::Queued => "⏳",
            JobStatus::Running => "⟳",
            JobStatus::Completed => "✓",
            JobStatus::Failed => "✗",
        }
    }
}

/// Allocation result with comprehensive metrics
#[derive(Clone, Debug)]
pub struct AllocationResult {
    pub node_id: String,
    pub rents: f64,
    pub contribution: f64,
    pub allocation_factor: f64,
    pub revenue_adequacy: f64, // Percentage of revenue needs met
    pub cost_recovery: f64,    // Percentage of costs recovered
    pub surplus_deficit: f64,  // Surplus/deficit in currency units
}

/// Reliability metric
#[derive(Clone, Debug)]
pub struct ReliabilityMetric {
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub status: MetricStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricStatus {
    Excellent,
    Good,
    Warning,
    Critical,
}

impl MetricStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            MetricStatus::Excellent => "✓",
            MetricStatus::Good => "◐",
            MetricStatus::Warning => "⚠",
            MetricStatus::Critical => "✗",
        }
    }
}

/// Operations pane state
#[derive(Clone, Debug)]
pub struct OperationsPaneState {
    // Tab selection
    pub active_tab: OperationType,

    // Batch operations
    pub batch_jobs: Vec<BatchJob>,
    pub selected_batch: usize,

    // Allocation operations
    pub allocation_results: Vec<AllocationResult>,
    pub selected_allocation: usize,

    // Reliability operations
    pub reliability_metrics: Vec<ReliabilityMetric>,
    pub selected_metric: usize,

    // Workflow tracking (Phase 3)
    pub recent_workflows: Vec<Workflow>,
    pub selected_workflow: usize,

    // Command execution (Phase 4)
    pub command_input: String,
    pub command_output: Vec<String>,
    pub command_executing: bool,
    pub command_validated: bool,
    pub last_exit_code: Option<i32>,
    pub last_duration_ms: Option<u64>,

    // Component states
    pub jobs_list: ListWidget,
    pub allocation_list: ListWidget,
    pub metrics_list: ListWidget,
    pub config_input: InputWidget,
    pub status_indicator: StatusWidget,

    // UI state
    pub run_in_progress: bool,
    pub selected_config: String,
}

impl Default for OperationsPaneState {
    fn default() -> Self {
        let batch_jobs = vec![
            BatchJob {
                id: "job_001".into(),
                name: "Daily Load Flow".into(),
                status: JobStatus::Completed,
                progress: 100,
                start_time: "2024-11-21 08:00".into(),
                est_completion: "2024-11-21 08:45".into(),
            },
            BatchJob {
                id: "job_002".into(),
                name: "Scenario Analysis".into(),
                status: JobStatus::Running,
                progress: 65,
                start_time: "2024-11-21 14:00".into(),
                est_completion: "2024-11-21 16:30".into(),
            },
        ];

        let allocation_results = vec![
            AllocationResult {
                node_id: "NODE_A".into(),
                rents: 1250.5,
                contribution: 45.2,
                allocation_factor: 0.85,
                revenue_adequacy: 95.3,
                cost_recovery: 98.2,
                surplus_deficit: 152.40,
            },
            AllocationResult {
                node_id: "NODE_B".into(),
                rents: 890.3,
                contribution: 32.1,
                allocation_factor: 0.72,
                revenue_adequacy: 87.6,
                cost_recovery: 91.5,
                surplus_deficit: 65.20,
            },
            AllocationResult {
                node_id: "NODE_C".into(),
                rents: 675.8,
                contribution: 28.5,
                allocation_factor: 0.68,
                revenue_adequacy: 82.1,
                cost_recovery: 85.3,
                surplus_deficit: -45.30,
            },
        ];

        let reliability_metrics = vec![
            ReliabilityMetric {
                metric_name: "Deliverability Score".into(),
                value: 85.5,
                unit: "%".into(),
                status: MetricStatus::Good,
            },
            ReliabilityMetric {
                metric_name: "LOLE".into(),
                value: 9.2,
                unit: "h/yr".into(),
                status: MetricStatus::Warning,
            },
            ReliabilityMetric {
                metric_name: "EUE".into(),
                value: 15.3,
                unit: "MWh/yr".into(),
                status: MetricStatus::Good,
            },
        ];

        let mut jobs_list = ListWidget::new("operations_jobs");
        for job in &batch_jobs {
            jobs_list.add_item(
                format!("{} {} ({}%)", job.status.symbol(), job.name, job.progress),
                job.id.clone(),
            );
        }

        let mut allocation_list = ListWidget::new("operations_allocation");
        for result in &allocation_results {
            allocation_list.add_item(
                format!("{}: ${:.2}", result.node_id, result.rents),
                result.node_id.clone(),
            );
        }

        let mut metrics_list = ListWidget::new("operations_metrics");
        for metric in &reliability_metrics {
            metrics_list.add_item(
                format!("{} {:.1}{}", metric.metric_name, metric.value, metric.unit),
                metric.metric_name.clone(),
            );
        }

        OperationsPaneState {
            active_tab: OperationType::Batch,
            batch_jobs,
            selected_batch: 0,
            allocation_results,
            selected_allocation: 0,
            reliability_metrics,
            selected_metric: 0,
            recent_workflows: Vec::new(),
            selected_workflow: 0,
            command_input: String::new(),
            command_output: Vec::new(),
            command_executing: false,
            command_validated: false,
            last_exit_code: None,
            last_duration_ms: None,
            jobs_list,
            allocation_list,
            metrics_list,
            config_input: InputWidget::new("operations_config")
                .with_placeholder("Configuration options..."),
            status_indicator: StatusWidget::new("operations_status"),
            run_in_progress: false,
            selected_config: String::new(),
        }
    }
}

impl OperationsPaneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn switch_tab(&mut self, tab: OperationType) {
        self.active_tab = tab;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            OperationType::Batch => OperationType::Allocation,
            OperationType::Allocation => OperationType::Reliability,
            OperationType::Reliability => OperationType::Batch,
        };
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            OperationType::Batch => OperationType::Reliability,
            OperationType::Allocation => OperationType::Batch,
            OperationType::Reliability => OperationType::Allocation,
        };
    }

    pub fn select_next_job(&mut self) {
        if self.selected_batch < self.batch_jobs.len().saturating_sub(1) {
            self.selected_batch += 1;
        }
    }

    pub fn select_prev_job(&mut self) {
        if self.selected_batch > 0 {
            self.selected_batch -= 1;
        }
    }

    pub fn selected_job(&self) -> Option<&BatchJob> {
        self.batch_jobs.get(self.selected_batch)
    }

    pub fn add_job(&mut self, job: BatchJob) {
        self.batch_jobs.insert(0, job.clone());
        self.jobs_list.add_item(
            format!("{} {} ({}%)", job.status.symbol(), job.name, job.progress),
            job.id,
        );
    }

    pub fn start_operation(&mut self) {
        self.run_in_progress = true;
        self.status_indicator =
            StatusWidget::new("operations_status").set_info("Running operation...");
    }

    pub fn complete_operation(&mut self, success: bool) {
        self.run_in_progress = false;
        if success {
            self.status_indicator = StatusWidget::new("operations_status")
                .set_success("Operation completed successfully");
        } else {
            self.status_indicator =
                StatusWidget::new("operations_status").set_error("Operation failed");
        }
    }

    pub fn job_count(&self) -> usize {
        self.batch_jobs.len()
    }

    pub fn allocation_count(&self) -> usize {
        self.allocation_results.len()
    }

    pub fn metric_count(&self) -> usize {
        self.reliability_metrics.len()
    }

    pub fn total_rents(&self) -> f64 {
        self.allocation_results.iter().map(|r| r.rents).sum()
    }

    // Enhanced allocation methods (Phase 5)

    pub fn select_next_allocation(&mut self) {
        if self.selected_allocation < self.allocation_results.len().saturating_sub(1) {
            self.selected_allocation += 1;
        }
    }

    pub fn select_prev_allocation(&mut self) {
        if self.selected_allocation > 0 {
            self.selected_allocation -= 1;
        }
    }

    pub fn selected_allocation(&self) -> Option<&AllocationResult> {
        self.allocation_results.get(self.selected_allocation)
    }

    pub fn get_allocation_details(&self) -> String {
        if let Some(result) = self.selected_allocation() {
            format!(
                "Node: {}\nRents: ${:.2}\nContribution: ${:.2}\nAllocation Factor: {:.2}\nRevenue Adequacy: {:.1}%\nCost Recovery: {:.1}%\nSurplus/Deficit: ${:.2}",
                result.node_id,
                result.rents,
                result.contribution,
                result.allocation_factor,
                result.revenue_adequacy,
                result.cost_recovery,
                result.surplus_deficit,
            )
        } else {
            "No allocation results selected".into()
        }
    }

    pub fn total_contributions(&self) -> f64 {
        self.allocation_results.iter().map(|r| r.contribution).sum()
    }

    pub fn avg_allocation_factor(&self) -> f64 {
        if self.allocation_results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .allocation_results
            .iter()
            .map(|r| r.allocation_factor)
            .sum();
        sum / self.allocation_results.len() as f64
    }

    pub fn avg_revenue_adequacy(&self) -> f64 {
        if self.allocation_results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .allocation_results
            .iter()
            .map(|r| r.revenue_adequacy)
            .sum();
        sum / self.allocation_results.len() as f64
    }

    pub fn avg_cost_recovery(&self) -> f64 {
        if self.allocation_results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .allocation_results
            .iter()
            .map(|r| r.cost_recovery)
            .sum();
        sum / self.allocation_results.len() as f64
    }

    pub fn total_surplus_deficit(&self) -> f64 {
        self.allocation_results
            .iter()
            .map(|r| r.surplus_deficit)
            .sum()
    }

    pub fn add_allocation(&mut self, result: AllocationResult) {
        self.allocation_results.insert(0, result.clone());
        self.allocation_list.add_item(
            format!("{}: ${:.2}", result.node_id, result.rents),
            result.node_id,
        );
    }

    pub fn get_allocation_summary(&self) -> String {
        format!(
            "Total Rents: ${:.2}\nTotal Contributions: ${:.2}\nAvg Allocation Factor: {:.2}\nAvg Revenue Adequacy: {:.1}%\nAvg Cost Recovery: {:.1}%\nTotal Surplus/Deficit: ${:.2}",
            self.total_rents(),
            self.total_contributions(),
            self.avg_allocation_factor(),
            self.avg_revenue_adequacy(),
            self.avg_cost_recovery(),
            self.total_surplus_deficit(),
        )
    }

    pub fn avg_deliverability(&self) -> f64 {
        self.reliability_metrics
            .iter()
            .find(|m| m.metric_name == "Deliverability Score")
            .map(|m| m.value)
            .unwrap_or(0.0)
    }

    // Workflow tracking methods (Phase 3)

    /// Add a workflow to recent workflows
    pub fn add_workflow(&mut self, workflow: Workflow) {
        self.recent_workflows.insert(0, workflow);
        // Keep only last 20 workflows for display
        if self.recent_workflows.len() > 20 {
            self.recent_workflows.pop();
        }
        self.selected_workflow = 0;
    }

    /// Get currently selected workflow
    pub fn selected_workflow(&self) -> Option<&Workflow> {
        self.recent_workflows.get(self.selected_workflow)
    }

    /// Navigate to next workflow
    pub fn select_next_workflow(&mut self) {
        if self.selected_workflow < self.recent_workflows.len().saturating_sub(1) {
            self.selected_workflow += 1;
        }
    }

    /// Navigate to previous workflow
    pub fn select_prev_workflow(&mut self) {
        if self.selected_workflow > 0 {
            self.selected_workflow -= 1;
        }
    }

    /// Get workflow count
    pub fn workflow_count(&self) -> usize {
        self.recent_workflows.len()
    }

    /// Clear all workflows
    pub fn clear_workflows(&mut self) {
        self.recent_workflows.clear();
        self.selected_workflow = 0;
    }

    // Command execution methods (Phase 4)

    /// Set the command input text
    pub fn set_command_input(&mut self, input: String) {
        self.command_input = input;
        self.command_validated = false;
    }

    /// Add a character to command input
    pub fn add_command_char(&mut self, ch: char) {
        self.command_input.push(ch);
        self.command_validated = false;
    }

    /// Remove last character from command input
    pub fn command_backspace(&mut self) {
        self.command_input.pop();
        self.command_validated = false;
    }

    /// Clear command input
    pub fn clear_command_input(&mut self) {
        self.command_input.clear();
        self.command_validated = false;
    }

    /// Mark command as being validated
    pub fn set_command_validated(&mut self, validated: bool) {
        self.command_validated = validated;
    }

    /// Get the current command input
    pub fn get_command_input(&self) -> &str {
        &self.command_input
    }

    /// Start command execution
    pub fn start_command_execution(&mut self) {
        self.command_executing = true;
        self.command_output.clear();
        self.last_exit_code = None;
        self.last_duration_ms = None;
    }

    /// Add output line from running command
    pub fn add_command_output(&mut self, line: String) {
        self.command_output.push(line);
        // Limit output to 1000 lines to prevent memory issues
        if self.command_output.len() > 1000 {
            self.command_output.remove(0);
        }
    }

    /// Complete command execution with result
    pub fn complete_command_execution(&mut self, exit_code: i32, duration_ms: u64) {
        self.command_executing = false;
        self.last_exit_code = Some(exit_code);
        self.last_duration_ms = Some(duration_ms);
    }

    /// Get all command output lines
    pub fn get_command_output(&self) -> &[String] {
        &self.command_output
    }

    /// Get command output as single string
    pub fn get_command_output_text(&self) -> String {
        self.command_output.join("\n")
    }

    /// Check if command is currently executing
    pub fn is_command_executing(&self) -> bool {
        self.command_executing
    }

    /// Get last exit code
    pub fn get_last_exit_code(&self) -> Option<i32> {
        self.last_exit_code
    }

    /// Get last command duration in milliseconds
    pub fn get_last_duration_ms(&self) -> Option<u64> {
        self.last_duration_ms
    }

    /// Clear command output
    pub fn clear_command_output(&mut self) {
        self.command_output.clear();
    }

    /// Get command execution status string
    pub fn command_status(&self) -> &'static str {
        if self.command_executing {
            "Running..."
        } else if let Some(code) = self.last_exit_code {
            if code == 0 {
                "Success"
            } else {
                "Failed"
            }
        } else {
            "Ready"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operations_init() {
        let state = OperationsPaneState::new();
        assert_eq!(state.job_count(), 2);
        assert_eq!(state.allocation_count(), 3);
        assert_eq!(state.metric_count(), 3);
        assert_eq!(state.active_tab, OperationType::Batch);
    }

    #[test]
    fn test_tab_navigation() {
        let mut state = OperationsPaneState::new();
        state.next_tab();
        assert_eq!(state.active_tab, OperationType::Allocation);
        state.next_tab();
        assert_eq!(state.active_tab, OperationType::Reliability);
        state.next_tab();
        assert_eq!(state.active_tab, OperationType::Batch);
    }

    #[test]
    fn test_job_selection() {
        let mut state = OperationsPaneState::new();
        state.select_next_job();
        assert_eq!(state.selected_batch, 1);
        state.select_prev_job();
        assert_eq!(state.selected_batch, 0);
    }

    #[test]
    fn test_job_status_symbol() {
        assert_eq!(JobStatus::Running.symbol(), "⟳");
        assert_eq!(JobStatus::Completed.symbol(), "✓");
    }

    #[test]
    fn test_operation_execution() {
        let mut state = OperationsPaneState::new();
        assert!(!state.run_in_progress);
        state.start_operation();
        assert!(state.run_in_progress);
        state.complete_operation(true);
        assert!(!state.run_in_progress);
    }

    #[test]
    fn test_metrics_calculation() {
        let state = OperationsPaneState::new();
        let total = state.total_rents();
        assert!(total > 0.0);
        assert_eq!(state.avg_deliverability(), 85.5);
    }

    // Enhanced allocation tests (Phase 5)

    #[test]
    fn test_allocation_selection() {
        let mut state = OperationsPaneState::new();
        assert_eq!(state.selected_allocation, 0);

        state.select_next_allocation();
        assert_eq!(state.selected_allocation, 1);

        state.select_next_allocation();
        assert_eq!(state.selected_allocation, 2);

        state.select_next_allocation();
        assert_eq!(state.selected_allocation, 2); // Bounds check

        state.select_prev_allocation();
        assert_eq!(state.selected_allocation, 1);
    }

    #[test]
    fn test_selected_allocation() {
        let state = OperationsPaneState::new();
        let result = state.selected_allocation().unwrap();
        assert_eq!(result.node_id, "NODE_A");
        assert_eq!(result.rents, 1250.5);
    }

    #[test]
    fn test_allocation_details_formatting() {
        let state = OperationsPaneState::new();
        let details = state.get_allocation_details();
        assert!(details.contains("NODE_A"));
        assert!(details.contains("1250.50"));
        assert!(details.contains("95.3"));
    }

    #[test]
    fn test_allocation_count() {
        let state = OperationsPaneState::new();
        assert_eq!(state.allocation_count(), 3);
    }

    #[test]
    fn test_total_rents() {
        let state = OperationsPaneState::new();
        let total = state.total_rents();
        assert!(total > 2800.0);
        assert!(total < 2820.0);
    }

    #[test]
    fn test_total_contributions() {
        let state = OperationsPaneState::new();
        let total = state.total_contributions();
        assert!(total > 105.0);
        assert!(total < 110.0);
    }

    #[test]
    fn test_avg_allocation_factor() {
        let state = OperationsPaneState::new();
        let avg = state.avg_allocation_factor();
        assert!(avg > 0.70);
        assert!(avg < 0.80);
    }

    #[test]
    fn test_avg_revenue_adequacy() {
        let state = OperationsPaneState::new();
        let avg = state.avg_revenue_adequacy();
        assert!(avg > 80.0);
        assert!(avg < 100.0);
    }

    #[test]
    fn test_avg_cost_recovery() {
        let state = OperationsPaneState::new();
        let avg = state.avg_cost_recovery();
        assert!(avg > 85.0);
        assert!(avg < 100.0);
    }

    #[test]
    fn test_total_surplus_deficit() {
        let state = OperationsPaneState::new();
        let total = state.total_surplus_deficit();
        assert!(total > 170.0);
        assert!(total < 180.0);
    }

    #[test]
    fn test_add_allocation() {
        let mut state = OperationsPaneState::new();
        let initial_count = state.allocation_count();

        let new_result = AllocationResult {
            node_id: "NODE_D".into(),
            rents: 500.0,
            contribution: 20.0,
            allocation_factor: 0.65,
            revenue_adequacy: 80.0,
            cost_recovery: 85.0,
            surplus_deficit: 25.0,
        };

        state.add_allocation(new_result);
        assert_eq!(state.allocation_count(), initial_count + 1);
        assert_eq!(state.selected_allocation().unwrap().node_id, "NODE_D");
    }

    #[test]
    fn test_allocation_summary() {
        let state = OperationsPaneState::new();
        let summary = state.get_allocation_summary();
        assert!(summary.contains("Total Rents:"));
        assert!(summary.contains("Total Contributions:"));
        assert!(summary.contains("Avg Allocation Factor:"));
        assert!(summary.contains("Avg Revenue Adequacy:"));
        assert!(summary.contains("Avg Cost Recovery:"));
        assert!(summary.contains("Total Surplus/Deficit:"));
    }

    #[test]
    fn test_operation_type_label() {
        assert_eq!(OperationType::Batch.label(), "Batch");
        assert_eq!(OperationType::Allocation.label(), "Allocation");
    }

    // Workflow tracking tests (Phase 3)

    #[test]
    fn test_operations_workflows_init() {
        let state = OperationsPaneState::new();
        assert_eq!(state.workflow_count(), 0);
        assert!(state.selected_workflow().is_none());
    }

    #[test]
    fn test_add_workflow() {
        use crate::data::WorkflowStatus;

        let mut state = OperationsPaneState::new();
        let workflow = Workflow {
            id: "wf_001".to_string(),
            name: "Test Workflow".to_string(),
            status: WorkflowStatus::Running,
            created_by: "test_user".to_string(),
            created_at: std::time::SystemTime::now(),
            completed_at: None,
        };
        state.add_workflow(workflow.clone());
        assert_eq!(state.workflow_count(), 1);
        assert_eq!(state.selected_workflow().unwrap().id, "wf_001");
    }

    #[test]
    fn test_workflow_navigation() {
        use crate::data::WorkflowStatus;

        let mut state = OperationsPaneState::new();
        let wf1 = Workflow {
            id: "wf_001".to_string(),
            name: "Workflow 1".to_string(),
            status: WorkflowStatus::Running,
            created_by: "user".to_string(),
            created_at: std::time::SystemTime::now(),
            completed_at: None,
        };
        let wf2 = Workflow {
            id: "wf_002".to_string(),
            name: "Workflow 2".to_string(),
            status: WorkflowStatus::Succeeded,
            created_by: "user".to_string(),
            created_at: std::time::SystemTime::now(),
            completed_at: Some(std::time::SystemTime::now()),
        };

        state.add_workflow(wf1);
        state.add_workflow(wf2);

        assert_eq!(state.selected_workflow().unwrap().id, "wf_002");
        state.select_next_workflow();
        assert_eq!(state.selected_workflow().unwrap().id, "wf_001");
        state.select_prev_workflow();
        assert_eq!(state.selected_workflow().unwrap().id, "wf_002");
    }

    #[test]
    fn test_workflow_max_20() {
        use crate::data::WorkflowStatus;

        let mut state = OperationsPaneState::new();
        // Add 25 workflows
        for i in 0..25 {
            let workflow = Workflow {
                id: format!("wf_{:03}", i),
                name: format!("Workflow {}", i),
                status: WorkflowStatus::Succeeded,
                created_by: "user".to_string(),
                created_at: std::time::SystemTime::now(),
                completed_at: Some(std::time::SystemTime::now()),
            };
            state.add_workflow(workflow);
        }
        // Should keep only last 20
        assert_eq!(state.workflow_count(), 20);
        // Most recent should be wf_024
        assert_eq!(state.selected_workflow().unwrap().id, "wf_024");
    }

    #[test]
    fn test_clear_workflows() {
        use crate::data::WorkflowStatus;

        let mut state = OperationsPaneState::new();
        let workflow = Workflow {
            id: "wf_001".to_string(),
            name: "Test".to_string(),
            status: WorkflowStatus::Running,
            created_by: "user".to_string(),
            created_at: std::time::SystemTime::now(),
            completed_at: None,
        };
        state.add_workflow(workflow);
        assert_eq!(state.workflow_count(), 1);
        state.clear_workflows();
        assert_eq!(state.workflow_count(), 0);
    }

    // Command execution tests (Phase 4)

    #[test]
    fn test_command_input_management() {
        let mut state = OperationsPaneState::new();
        assert_eq!(state.get_command_input(), "");

        state.set_command_input("gat-cli datasets list".to_string());
        assert_eq!(state.get_command_input(), "gat-cli datasets list");

        state.clear_command_input();
        assert_eq!(state.get_command_input(), "");
    }

    #[test]
    fn test_command_char_input() {
        let mut state = OperationsPaneState::new();

        state.add_command_char('e');
        state.add_command_char('c');
        state.add_command_char('h');
        state.add_command_char('o');

        assert_eq!(state.get_command_input(), "echo");
    }

    #[test]
    fn test_command_backspace() {
        let mut state = OperationsPaneState::new();
        state.set_command_input("hello".to_string());

        state.command_backspace();
        assert_eq!(state.get_command_input(), "hell");

        state.command_backspace();
        state.command_backspace();
        assert_eq!(state.get_command_input(), "he");
    }

    #[test]
    fn test_command_validation_flag() {
        let mut state = OperationsPaneState::new();
        assert!(!state.command_validated);

        state.set_command_validated(true);
        assert!(state.command_validated);

        state.set_command_input("new command".to_string());
        assert!(!state.command_validated);
    }

    #[test]
    fn test_command_execution_lifecycle() {
        let mut state = OperationsPaneState::new();

        // Initially not executing
        assert!(!state.is_command_executing());
        assert!(state.get_last_exit_code().is_none());

        // Start execution
        state.start_command_execution();
        assert!(state.is_command_executing());
        assert_eq!(state.command_status(), "Running...");

        // Add output
        state.add_command_output("Line 1".to_string());
        state.add_command_output("Line 2".to_string());
        assert_eq!(state.get_command_output().len(), 2);

        // Complete execution
        state.complete_command_execution(0, 150);
        assert!(!state.is_command_executing());
        assert_eq!(state.get_last_exit_code(), Some(0));
        assert_eq!(state.get_last_duration_ms(), Some(150));
        assert_eq!(state.command_status(), "Success");
    }

    #[test]
    fn test_command_output_accumulation() {
        let mut state = OperationsPaneState::new();
        state.start_command_execution();

        for i in 0..10 {
            state.add_command_output(format!("Output line {}", i));
        }

        assert_eq!(state.get_command_output().len(), 10);
        assert_eq!(
            state.get_command_output_text(),
            "Output line 0\nOutput line 1\nOutput line 2\nOutput line 3\nOutput line 4\n\
             Output line 5\nOutput line 6\nOutput line 7\nOutput line 8\nOutput line 9"
        );
    }

    #[test]
    fn test_command_output_limit() {
        let mut state = OperationsPaneState::new();
        state.start_command_execution();

        // Add more than 1000 lines
        for i in 0..1100 {
            state.add_command_output(format!("Line {}", i));
        }

        // Should only keep last 1000
        assert_eq!(state.get_command_output().len(), 1000);
        assert!(state.get_command_output()[0].contains("100")); // First kept line
    }

    #[test]
    fn test_command_clear_output() {
        let mut state = OperationsPaneState::new();
        state.start_command_execution();
        state.add_command_output("Some output".to_string());

        assert!(!state.get_command_output().is_empty());
        state.clear_command_output();
        assert!(state.get_command_output().is_empty());
    }

    #[test]
    fn test_command_status_strings() {
        let mut state = OperationsPaneState::new();

        assert_eq!(state.command_status(), "Ready");

        state.start_command_execution();
        assert_eq!(state.command_status(), "Running...");

        state.complete_command_execution(0, 100);
        assert_eq!(state.command_status(), "Success");

        state.start_command_execution();
        state.complete_command_execution(1, 50);
        assert_eq!(state.command_status(), "Failed");
    }

    #[test]
    fn test_command_failed_execution() {
        let mut state = OperationsPaneState::new();

        state.start_command_execution();
        state.add_command_output("Error message".to_string());
        state.complete_command_execution(127, 200);

        assert!(!state.is_command_executing());
        assert_eq!(state.get_last_exit_code(), Some(127));
        assert_eq!(state.command_status(), "Failed");
    }
}
