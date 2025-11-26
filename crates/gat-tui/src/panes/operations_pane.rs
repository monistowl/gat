//! Operations Pane - Batch processing, allocation, and reliability analysis
//!
//! This module re-exports from the refactored `operations_state` module which
//! provides focused state machines for different operation types.
//!
//! The operations pane provides:
//! - Batch job management (via `BatchState`)
//! - Allocation operations with rents and contributions (via `AllocationState`)
//! - Reliability metrics and analysis
//! - Command execution (via `CommandState`)
//! - Multi-tab interface for different operation types

// Re-export everything from operations_state for backward compatibility
pub use super::operations_state::{
    AllocationResult, AllocationState, BatchJob, BatchState, CommandState, JobStatus, MetricStatus,
    OperationType, OperationsPaneState, ReliabilityMetric,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Workflow, WorkflowStatus};

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
        assert_eq!(state.selected_batch(), 1);
        state.select_prev_job();
        assert_eq!(state.selected_batch(), 0);
    }

    #[test]
    fn test_job_status_symbol() {
        assert_eq!(JobStatus::Running.symbol(), "⟳");
        assert_eq!(JobStatus::Completed.symbol(), "✓");
    }

    #[test]
    fn test_operation_execution() {
        let mut state = OperationsPaneState::new();
        assert!(!state.run_in_progress());
        state.start_operation();
        assert!(state.run_in_progress());
        state.complete_operation(true);
        assert!(!state.run_in_progress());
    }

    #[test]
    fn test_metrics_calculation() {
        let state = OperationsPaneState::new();
        let total = state.total_rents();
        assert!(total > 0.0);
        assert_eq!(state.avg_deliverability(), 85.5);
    }

    // Allocation tests

    #[test]
    fn test_allocation_selection() {
        let mut state = OperationsPaneState::new();
        assert_eq!(state.selected_allocation_index(), 0);

        state.select_next_allocation();
        assert_eq!(state.selected_allocation_index(), 1);

        state.select_next_allocation();
        assert_eq!(state.selected_allocation_index(), 2);

        state.select_next_allocation();
        assert_eq!(state.selected_allocation_index(), 2); // Bounds check

        state.select_prev_allocation();
        assert_eq!(state.selected_allocation_index(), 1);
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

    // Workflow tracking tests

    #[test]
    fn test_operations_workflows_init() {
        let state = OperationsPaneState::new();
        assert_eq!(state.workflow_count(), 0);
        assert!(state.selected_workflow().is_none());
    }

    #[test]
    fn test_add_workflow() {
        let mut state = OperationsPaneState::new();
        let workflow = Workflow {
            id: "wf_001".to_string(),
            name: "Test Workflow".to_string(),
            status: WorkflowStatus::Running,
            created_by: "test_user".to_string(),
            created_at: std::time::SystemTime::now(),
            completed_at: None,
        };
        state.add_workflow(workflow);
        assert_eq!(state.workflow_count(), 1);
        assert_eq!(state.selected_workflow().unwrap().id, "wf_001");
    }

    #[test]
    fn test_workflow_navigation() {
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

    // Command execution tests

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
        assert!(!state.command_validated());

        state.set_command_validated(true);
        assert!(state.command_validated());

        state.set_command_input("new command".to_string());
        assert!(!state.command_validated());
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
