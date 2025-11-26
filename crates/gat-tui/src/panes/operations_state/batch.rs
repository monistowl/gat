//! Batch operations state management
//!
//! Handles batch job queue, selection, and operation lifecycle.

use crate::components::{ListWidget, StatusWidget};

use super::types::{BatchJob, JobStatus};

/// State for batch job operations
#[derive(Clone, Debug)]
pub struct BatchState {
    /// List of batch jobs
    jobs: Vec<BatchJob>,
    /// Currently selected job index
    selected: usize,
    /// List widget for UI rendering
    pub jobs_list: ListWidget,
    /// Status indicator widget
    pub status_indicator: StatusWidget,
    /// Whether an operation is currently running
    pub run_in_progress: bool,
}

impl Default for BatchState {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchState {
    /// Create a new BatchState with default sample data
    pub fn new() -> Self {
        let jobs = vec![
            BatchJob::new("job_001", "Daily Load Flow")
                .with_status(JobStatus::Completed)
                .with_progress(100)
                .with_times("2024-11-21 08:00", "2024-11-21 08:45"),
            BatchJob::new("job_002", "Scenario Analysis")
                .with_status(JobStatus::Running)
                .with_progress(65)
                .with_times("2024-11-21 14:00", "2024-11-21 16:30"),
        ];

        let mut jobs_list = ListWidget::new("operations_jobs");
        for job in &jobs {
            jobs_list.add_item(job.display_line(), job.id.clone());
        }

        Self {
            jobs,
            selected: 0,
            jobs_list,
            status_indicator: StatusWidget::new("operations_status"),
            run_in_progress: false,
        }
    }

    /// Create an empty BatchState
    pub fn empty() -> Self {
        Self {
            jobs: Vec::new(),
            selected: 0,
            jobs_list: ListWidget::new("operations_jobs"),
            status_indicator: StatusWidget::new("operations_status"),
            run_in_progress: false,
        }
    }

    /// Get all batch jobs
    pub fn jobs(&self) -> &[BatchJob] {
        &self.jobs
    }

    /// Get the number of jobs
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// Select the next job in the list
    pub fn select_next(&mut self) {
        if self.selected < self.jobs.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Select the previous job in the list
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Get the currently selected job
    pub fn selected_job(&self) -> Option<&BatchJob> {
        self.jobs.get(self.selected)
    }

    /// Get the selected index
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Add a new job to the front of the list
    pub fn add_job(&mut self, job: BatchJob) {
        self.jobs_list.add_item(job.display_line(), job.id.clone());
        self.jobs.insert(0, job);
    }

    /// Start an operation
    pub fn start_operation(&mut self) {
        self.run_in_progress = true;
        self.status_indicator =
            StatusWidget::new("operations_status").set_info("Running operation...");
    }

    /// Complete an operation
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

    /// Check if an operation is in progress
    pub fn is_running(&self) -> bool {
        self.run_in_progress
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_state_init() {
        let state = BatchState::new();
        assert_eq!(state.job_count(), 2);
        assert_eq!(state.selected_index(), 0);
        assert!(!state.is_running());
    }

    #[test]
    fn test_batch_state_empty() {
        let state = BatchState::empty();
        assert_eq!(state.job_count(), 0);
        assert!(state.selected_job().is_none());
    }

    #[test]
    fn test_job_selection() {
        let mut state = BatchState::new();
        assert_eq!(state.selected_index(), 0);

        state.select_next();
        assert_eq!(state.selected_index(), 1);

        state.select_next(); // Should not exceed bounds
        assert_eq!(state.selected_index(), 1);

        state.select_prev();
        assert_eq!(state.selected_index(), 0);

        state.select_prev(); // Should not go below 0
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_selected_job() {
        let state = BatchState::new();
        let job = state.selected_job().unwrap();
        assert_eq!(job.id, "job_001");
        assert_eq!(job.name, "Daily Load Flow");
    }

    #[test]
    fn test_add_job() {
        let mut state = BatchState::new();
        let initial_count = state.job_count();

        let new_job = BatchJob::new("job_003", "New Analysis")
            .with_status(JobStatus::Queued)
            .with_progress(0);

        state.add_job(new_job);
        assert_eq!(state.job_count(), initial_count + 1);
        // New job should be at the front
        assert_eq!(state.jobs()[0].id, "job_003");
    }

    #[test]
    fn test_operation_lifecycle() {
        let mut state = BatchState::new();
        assert!(!state.is_running());

        state.start_operation();
        assert!(state.is_running());

        state.complete_operation(true);
        assert!(!state.is_running());
    }

    #[test]
    fn test_operation_failure() {
        let mut state = BatchState::new();
        state.start_operation();
        state.complete_operation(false);
        assert!(!state.is_running());
    }
}
