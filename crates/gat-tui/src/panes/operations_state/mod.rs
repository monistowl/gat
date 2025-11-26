//! Operations pane state management
//!
//! This module provides focused state machines for the Operations pane:
//! - `BatchState` - Batch job queue and operation lifecycle
//! - `AllocationState` - Allocation results and metrics calculations
//! - `CommandState` - Command input/output and execution lifecycle
//!
//! The main `OperationsPaneState` composes these sub-states along with
//! reliability metrics and workflow tracking.

pub mod allocation;
pub mod batch;
pub mod command;
pub mod types;

pub use allocation::AllocationState;
pub use batch::BatchState;
pub use command::CommandState;
pub use types::{
    AllocationResult, BatchJob, JobStatus, MetricStatus, OperationType, ReliabilityMetric,
};

use crate::components::{InputWidget, ListWidget};
use crate::data::Workflow;

/// Operations pane state - composes focused sub-states
#[derive(Clone, Debug)]
pub struct OperationsPaneState {
    // Tab selection
    pub active_tab: OperationType,

    // Composed sub-states
    pub batch: BatchState,
    pub allocation: AllocationState,
    pub command: CommandState,

    // Reliability operations (kept inline as it's simpler)
    reliability_metrics: Vec<ReliabilityMetric>,
    #[allow(dead_code)]
    selected_metric: usize,
    pub metrics_list: ListWidget,

    // Workflow tracking
    recent_workflows: Vec<Workflow>,
    selected_workflow: usize,

    // UI components
    pub config_input: InputWidget,

    // Run describe output
    pub run_details_json: Option<String>,
    pub last_run_path: Option<String>,

    // Configuration
    pub selected_config: String,
}

impl Default for OperationsPaneState {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationsPaneState {
    /// Create a new OperationsPaneState with default sample data
    pub fn new() -> Self {
        let reliability_metrics = vec![
            ReliabilityMetric::new("Deliverability Score", 85.5, "%")
                .with_status(MetricStatus::Good),
            ReliabilityMetric::new("LOLE", 9.2, "h/yr").with_status(MetricStatus::Warning),
            ReliabilityMetric::new("EUE", 15.3, "MWh/yr").with_status(MetricStatus::Good),
        ];

        let mut metrics_list = ListWidget::new("operations_metrics");
        for metric in &reliability_metrics {
            metrics_list.add_item(metric.display_line(), metric.metric_name.clone());
        }

        Self {
            active_tab: OperationType::Batch,
            batch: BatchState::new(),
            allocation: AllocationState::new(),
            command: CommandState::new(),
            reliability_metrics,
            selected_metric: 0,
            metrics_list,
            recent_workflows: Vec::new(),
            selected_workflow: 0,
            config_input: InputWidget::new("operations_config")
                .with_placeholder("Configuration options..."),
            run_details_json: None,
            last_run_path: None,
            selected_config: String::new(),
        }
    }

    // ============================================================================
    // Tab navigation (delegated to OperationType)
    // ============================================================================

    pub fn switch_tab(&mut self, tab: OperationType) {
        self.active_tab = tab;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
    }

    // ============================================================================
    // Batch operations (delegated to BatchState)
    // ============================================================================

    pub fn select_next_job(&mut self) {
        self.batch.select_next();
    }

    pub fn select_prev_job(&mut self) {
        self.batch.select_prev();
    }

    pub fn selected_job(&self) -> Option<&BatchJob> {
        self.batch.selected_job()
    }

    pub fn add_job(&mut self, job: BatchJob) {
        self.batch.add_job(job);
    }

    pub fn start_operation(&mut self) {
        self.batch.start_operation();
    }

    pub fn complete_operation(&mut self, success: bool) {
        self.batch.complete_operation(success);
    }

    pub fn job_count(&self) -> usize {
        self.batch.job_count()
    }

    // ============================================================================
    // Allocation operations (delegated to AllocationState)
    // ============================================================================

    pub fn select_next_allocation(&mut self) {
        self.allocation.select_next();
    }

    pub fn select_prev_allocation(&mut self) {
        self.allocation.select_prev();
    }

    pub fn selected_allocation(&self) -> Option<&AllocationResult> {
        self.allocation.selected()
    }

    pub fn get_allocation_details(&self) -> String {
        self.allocation.get_details()
    }

    pub fn allocation_count(&self) -> usize {
        self.allocation.count()
    }

    pub fn total_rents(&self) -> f64 {
        self.allocation.total_rents()
    }

    pub fn total_contributions(&self) -> f64 {
        self.allocation.total_contributions()
    }

    pub fn avg_allocation_factor(&self) -> f64 {
        self.allocation.avg_allocation_factor()
    }

    pub fn avg_revenue_adequacy(&self) -> f64 {
        self.allocation.avg_revenue_adequacy()
    }

    pub fn avg_cost_recovery(&self) -> f64 {
        self.allocation.avg_cost_recovery()
    }

    pub fn total_surplus_deficit(&self) -> f64 {
        self.allocation.total_surplus_deficit()
    }

    pub fn add_allocation(&mut self, result: AllocationResult) {
        self.allocation.add(result);
    }

    pub fn get_allocation_summary(&self) -> String {
        self.allocation.get_summary()
    }

    // ============================================================================
    // Reliability metrics (inline, simpler state)
    // ============================================================================

    pub fn metric_count(&self) -> usize {
        self.reliability_metrics.len()
    }

    pub fn avg_deliverability(&self) -> f64 {
        self.reliability_metrics
            .iter()
            .find(|m| m.metric_name == "Deliverability Score")
            .map(|m| m.value)
            .unwrap_or(0.0)
    }

    // ============================================================================
    // Workflow tracking
    // ============================================================================

    pub fn add_workflow(&mut self, workflow: Workflow) {
        self.recent_workflows.insert(0, workflow);
        // Keep only last 20 workflows for display
        if self.recent_workflows.len() > 20 {
            self.recent_workflows.pop();
        }
        self.selected_workflow = 0;
    }

    pub fn selected_workflow(&self) -> Option<&Workflow> {
        self.recent_workflows.get(self.selected_workflow)
    }

    pub fn select_next_workflow(&mut self) {
        if self.selected_workflow < self.recent_workflows.len().saturating_sub(1) {
            self.selected_workflow += 1;
        }
    }

    pub fn select_prev_workflow(&mut self) {
        if self.selected_workflow > 0 {
            self.selected_workflow -= 1;
        }
    }

    pub fn workflow_count(&self) -> usize {
        self.recent_workflows.len()
    }

    pub fn clear_workflows(&mut self) {
        self.recent_workflows.clear();
        self.selected_workflow = 0;
    }

    // ============================================================================
    // Command execution (delegated to CommandState)
    // ============================================================================

    pub fn set_command_input(&mut self, input: String) {
        self.command.set_input(input);
    }

    pub fn add_command_char(&mut self, ch: char) {
        self.command.add_char(ch);
    }

    pub fn command_backspace(&mut self) {
        self.command.backspace();
    }

    pub fn clear_command_input(&mut self) {
        self.command.clear_input();
    }

    pub fn set_command_validated(&mut self, validated: bool) {
        self.command.set_validated(validated);
    }

    pub fn get_command_input(&self) -> &str {
        self.command.input()
    }

    pub fn start_command_execution(&mut self) {
        self.command.start_execution();
    }

    pub fn add_command_output(&mut self, line: String) {
        self.command.add_output(line);
    }

    pub fn complete_command_execution(&mut self, exit_code: i32, duration_ms: u64) {
        self.command.complete_execution(exit_code, duration_ms);
    }

    pub fn get_command_output(&self) -> &[String] {
        self.command.output()
    }

    pub fn get_command_output_text(&self) -> String {
        self.command.output_text()
    }

    pub fn is_command_executing(&self) -> bool {
        self.command.is_executing()
    }

    pub fn get_last_exit_code(&self) -> Option<i32> {
        self.command.last_exit_code()
    }

    pub fn get_last_duration_ms(&self) -> Option<u64> {
        self.command.last_duration_ms()
    }

    pub fn clear_command_output(&mut self) {
        self.command.clear_output();
    }

    pub fn command_status(&self) -> &'static str {
        self.command.status()
    }

    // ============================================================================
    // Legacy compatibility accessors
    // ============================================================================

    /// Get batch jobs list (for backward compatibility)
    pub fn batch_jobs(&self) -> &[BatchJob] {
        self.batch.jobs()
    }

    /// Get selected batch index (for backward compatibility)
    pub fn selected_batch(&self) -> usize {
        self.batch.selected_index()
    }

    /// Get allocation results (for backward compatibility)
    pub fn allocation_results(&self) -> &[AllocationResult] {
        self.allocation.results()
    }

    /// Get selected allocation index (for backward compatibility)
    pub fn selected_allocation_index(&self) -> usize {
        self.allocation.selected_index()
    }

    /// Get reliability metrics (for backward compatibility)
    pub fn reliability_metrics(&self) -> &[ReliabilityMetric] {
        &self.reliability_metrics
    }

    /// Check if operation is in progress (for backward compatibility)
    pub fn run_in_progress(&self) -> bool {
        self.batch.is_running()
    }

    /// Check if command is validated (for backward compatibility)
    pub fn command_validated(&self) -> bool {
        self.command.is_validated()
    }

    /// Check if command is executing (for backward compatibility)
    pub fn command_executing(&self) -> bool {
        self.command.is_executing()
    }

    /// Get command input (for backward compatibility)
    pub fn command_input(&self) -> &str {
        self.command.input()
    }

    /// Get command output (for backward compatibility)
    pub fn command_output(&self) -> &[String] {
        self.command.output()
    }

    /// Get last exit code (for backward compatibility)
    pub fn last_exit_code(&self) -> Option<i32> {
        self.command.last_exit_code()
    }

    /// Get last duration (for backward compatibility)
    pub fn last_duration_ms(&self) -> Option<u64> {
        self.command.last_duration_ms()
    }

    /// Get jobs list widget (for backward compatibility)
    pub fn jobs_list(&self) -> &ListWidget {
        &self.batch.jobs_list
    }

    /// Get jobs list widget mutably (for backward compatibility)
    pub fn jobs_list_mut(&mut self) -> &mut ListWidget {
        &mut self.batch.jobs_list
    }

    /// Get allocation list widget (for backward compatibility)
    pub fn allocation_list(&self) -> &ListWidget {
        &self.allocation.allocation_list
    }

    /// Get allocation list widget mutably (for backward compatibility)
    pub fn allocation_list_mut(&mut self) -> &mut ListWidget {
        &mut self.allocation.allocation_list
    }

    /// Get status indicator (for backward compatibility)
    pub fn status_indicator(&self) -> &crate::components::StatusWidget {
        &self.batch.status_indicator
    }

    /// Get status indicator mutably (for backward compatibility)
    pub fn status_indicator_mut(&mut self) -> &mut crate::components::StatusWidget {
        &mut self.batch.status_indicator
    }
}
