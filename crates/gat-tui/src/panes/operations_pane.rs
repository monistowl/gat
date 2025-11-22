/// Operations Pane - Batch processing, allocation, and reliability analysis
///
/// The operations pane provides:
/// - Batch job management
/// - Allocation operations (rents, contributions)
/// - Reliability metrics and analysis
/// - Multi-tab interface for different operation types

use crate::components::*;

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

/// Allocation result
#[derive(Clone, Debug)]
pub struct AllocationResult {
    pub node_id: String,
    pub rents: f64,
    pub contribution: f64,
    pub allocation_factor: f64,
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
            },
            AllocationResult {
                node_id: "NODE_B".into(),
                rents: 890.3,
                contribution: 32.1,
                allocation_factor: 0.72,
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
        self.status_indicator = StatusWidget::new("operations_status")
            .set_info("Running operation...");
    }

    pub fn complete_operation(&mut self, success: bool) {
        self.run_in_progress = false;
        if success {
            self.status_indicator = StatusWidget::new("operations_status")
                .set_success("Operation completed successfully");
        } else {
            self.status_indicator = StatusWidget::new("operations_status")
                .set_error("Operation failed");
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

    pub fn avg_deliverability(&self) -> f64 {
        self.reliability_metrics
            .iter()
            .find(|m| m.metric_name == "Deliverability Score")
            .map(|m| m.value)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operations_init() {
        let state = OperationsPaneState::new();
        assert_eq!(state.job_count(), 2);
        assert_eq!(state.allocation_count(), 2);
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

    #[test]
    fn test_operation_type_label() {
        assert_eq!(OperationType::Batch.label(), "Batch");
        assert_eq!(OperationType::Allocation.label(), "Allocation");
    }
}
