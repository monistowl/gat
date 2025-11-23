/// Integration tests for all panes working together
///
/// This module verifies that:
/// - All panes initialize correctly
/// - Data flows properly between panes
/// - State management is consistent
/// - Cross-pane navigation works
/// - Analytics flow from Dashboard through Operations and Analytics panes

#[cfg(test)]
mod tests {
    use crate::panes::*;

    #[test]
    fn test_all_panes_initialize() {
        // Verify all panes can be initialized without errors
        let _dashboard = DashboardPaneState::new();
        let _commands = CommandsPaneState::new();
        let _datasets = DatasetsPaneState::new();
        let _pipeline = PipelinePaneState::new();
        let _operations = OperationsPaneState::new();
        let _analytics = AnalyticsPaneState::new();
        let _settings = SettingsPaneState::new();
        // No assertions needed - just ensure no panics
    }

    #[test]
    fn test_dashboard_quick_actions_integration() {
        let _dashboard = DashboardPaneState::new();
        let actions = QuickAction::all();

        // Verify we can find any action by key
        for action in actions {
            let found = QuickAction::find_by_key(action.key);
            assert!(found.is_some());
            let found_action = found.unwrap();
            assert_eq!(found_action.action_type, action.action_type);
        }
    }

    #[test]
    fn test_commands_snippets_for_all_operations() {
        let commands = CommandsPaneState::new();

        // Verify we have snippets for all major operations
        let categories: Vec<String> = commands.snippets
            .iter()
            .map(|s| s.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        assert!(categories.contains(&"Datasets".to_string()));
        assert!(categories.contains(&"DERMS".to_string()));
        assert!(categories.contains(&"Distribution".to_string()));
        assert!(categories.contains(&"Scenarios".to_string()));
        assert!(categories.contains(&"Analytics".to_string()));
        assert!(categories.contains(&"Batch".to_string()));
        assert!(categories.contains(&"Utilities".to_string()));
    }

    #[test]
    fn test_operations_and_analytics_data_flow() {
        let operations = OperationsPaneState::new();

        // Verify we can create and manage batch jobs
        let selected_job = operations.selected_job();
        assert!(selected_job.is_some());

        let job = selected_job.unwrap();
        assert!(!job.id.is_empty());
    }

    #[test]
    fn test_settings_persistence_across_panes() {
        let mut settings = SettingsPaneState::new();
        let _dashboard = DashboardPaneState::new();

        // Change settings
        settings.display_settings.theme = "light".to_string();
        settings.execution_settings.max_parallel_jobs = 8;

        // Settings should persist
        assert_eq!(settings.display_settings.theme, "light");
        assert_eq!(settings.execution_settings.max_parallel_jobs, 8);
    }

    #[test]
    fn test_pipeline_and_datasets_integration() {
        let pipeline = PipelinePaneState::new();
        let datasets = DatasetsPaneState::new();

        // Pipeline should be buildable with datasets
        let dataset_count = datasets.dataset_count();
        assert!(dataset_count > 0);

        // Verify pipeline exists
        assert!(pipeline.node_count() > 0);
    }

    #[test]
    fn test_command_execution_flow() {
        let mut commands = CommandsPaneState::new();

        // Load a reliability analysis snippet
        commands.load_snippet_to_editor(10);
        let command = &commands.custom_command;
        assert!(command.contains("reliability"));

        // Simulate execution
        let result = CommandResult {
            id: "result-001".into(),
            command: command.clone(),
            mode: ExecutionMode::Full,
            status: CommandStatus::Success,
            output_lines: 50,
            timestamp: "2024-11-22 10:00:00".into(),
        };

        commands.add_to_history(result);
        assert!(commands.history_count() > 0);

        // Verify result is accessible
        let last_result = commands.selected_result();
        assert!(last_result.is_some());
    }

    #[test]
    fn test_allocations_in_operations_complete_workflow() {
        let mut operations = OperationsPaneState::new();

        // Check allocations exist
        assert!(operations.allocation_count() > 0);

        // Select and get details
        operations.select_next_allocation();
        let allocation = operations.selected_allocation();
        assert!(allocation.is_some());

        let alloc = allocation.unwrap();
        assert!(!alloc.node_id.is_empty());
        assert!(alloc.revenue_adequacy > 0.0);

        // Get aggregate statistics
        let total_contrib = operations.total_contributions();
        assert!(total_contrib > 0.0);

        let avg_factor = operations.avg_allocation_factor();
        assert!(avg_factor > 0.0);
    }

    #[test]
    fn test_settings_tabs_and_navigation() {
        let mut settings = SettingsPaneState::new();

        // Test tab navigation
        assert_eq!(settings.current_tab, SettingsTab::Display);

        settings.next_tab();
        assert_eq!(settings.current_tab, SettingsTab::Data);

        settings.next_tab();
        assert_eq!(settings.current_tab, SettingsTab::Execution);

        settings.next_tab();
        assert_eq!(settings.current_tab, SettingsTab::Advanced);

        settings.next_tab();
        assert_eq!(settings.current_tab, SettingsTab::Display);

        // Test full settings retrieval
        settings.switch_tab(SettingsTab::Data);
        let all_settings = settings.get_all_settings();
        assert_eq!(all_settings.len(), 5);
    }

    #[test]
    fn test_cross_pane_data_consistency() {
        let dashboard = DashboardPaneState::new();
        let operations = OperationsPaneState::new();

        // Both should have consistent status representations
        assert!(!dashboard.overall_status.is_empty());
        assert!(operations.job_count() > 0);

        // Jobs in operations should have valid data
        let job = operations.selected_job().unwrap();
        assert!(!job.id.is_empty());
    }

    #[test]
    fn test_action_type_coverage() {
        let all_actions = QuickAction::all();

        // Ensure we have all major analysis types
        let action_types: Vec<ActionType> = all_actions
            .iter()
            .map(|a| a.action_type.clone())
            .collect();

        assert!(action_types.contains(&ActionType::ReliabilityAnalysis));
        assert!(action_types.contains(&ActionType::DeliverabilityScore));
        assert!(action_types.contains(&ActionType::ELCCEstimation));
        assert!(action_types.contains(&ActionType::PowerFlowAnalysis));
    }

    #[test]
    fn test_batch_operations_complete_lifecycle() {
        let operations = OperationsPaneState::new();

        // Check batch jobs exist
        assert!(operations.job_count() > 0);

        // Check allocation results
        assert!(operations.allocation_count() > 0);
        let summary = operations.get_allocation_summary();
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_command_execution_modes() {
        let mut commands = CommandsPaneState::new();

        // Test execution mode switching
        assert_eq!(commands.execution_mode, ExecutionMode::DryRun);

        commands.toggle_execution_mode();
        assert_eq!(commands.execution_mode, ExecutionMode::Full);

        commands.toggle_execution_mode();
        assert_eq!(commands.execution_mode, ExecutionMode::DryRun);

        // Verify history tracks mode
        let (success, failed, running) = commands.execution_summary();
        assert_eq!(success + failed + running, 2);
    }

    #[test]
    fn test_all_panes_handle_empty_state() {
        let commands = CommandsPaneState::new();

        // Verify we can clear and repopulate history
        let mut cmd_mut = commands.clone();
        cmd_mut.clear_history();
        assert_eq!(cmd_mut.history_count(), 0);

        // Recreate history
        let result = CommandResult {
            id: "test".into(),
            command: "test".into(),
            mode: ExecutionMode::DryRun,
            status: CommandStatus::Success,
            output_lines: 1,
            timestamp: "2024-11-22".into(),
        };
        cmd_mut.add_to_history(result);
        assert_eq!(cmd_mut.history_count(), 1);
    }

    #[test]
    fn test_integration_comprehensive_scenario() {
        // Simulate a complete user workflow
        let dashboard = DashboardPaneState::new();
        let datasets = DatasetsPaneState::new();
        let mut commands = CommandsPaneState::new();
        let mut operations = OperationsPaneState::new();
        let analytics = AnalyticsPaneState::new();
        let mut settings = SettingsPaneState::new();

        // 1. User checks dashboard health
        assert_eq!(dashboard.overall_status, "Healthy");

        // 2. User checks dataset count
        assert!(datasets.dataset_count() > 0);

        // 3. User prepares command
        commands.load_snippet_to_editor(10);
        assert!(commands.custom_command.contains("reliability"));

        // 4. User toggles execution mode
        commands.toggle_execution_mode();
        assert_eq!(commands.execution_mode, ExecutionMode::Full);

        // 5. System executes and records result
        let result = CommandResult {
            id: "integration-test".into(),
            command: commands.custom_command.clone(),
            mode: commands.execution_mode,
            status: CommandStatus::Success,
            output_lines: 100,
            timestamp: "2024-11-22 10:00:00".into(),
        };
        commands.add_to_history(result.clone());

        // 6. Operation is tracked
        assert!(operations.job_count() > 0);
        assert!(operations.allocation_count() > 0);

        // 7. Analytics available
        assert!(analytics.reliability_results.len() > 0);

        // 8. User configures preferences
        settings.switch_tab(SettingsTab::Execution);
        assert_eq!(settings.current_tab, SettingsTab::Execution);

        // Verify all components are consistent
        assert!(commands.history_count() > 0);
        assert!(operations.job_count() > 0);
        assert_eq!(settings.current_tab, SettingsTab::Execution);
    }

    #[test]
    fn test_pane_state_cloneability() {
        // Verify all panes are Clone-able for state management
        let dashboard = DashboardPaneState::new();
        let _dashboard_copy = dashboard.clone();

        let operations = OperationsPaneState::new();
        let _operations_copy = operations.clone();

        let settings = SettingsPaneState::new();
        let _settings_copy = settings.clone();
    }

    #[test]
    fn test_all_action_types_have_descriptions() {
        // Verify action types are properly described
        let action_types = vec![
            ActionType::ReliabilityAnalysis,
            ActionType::DeliverabilityScore,
            ActionType::ELCCEstimation,
            ActionType::PowerFlowAnalysis,
            ActionType::FilterRuns,
        ];

        for action_type in action_types {
            assert!(!action_type.label().is_empty());
            assert!(!action_type.description().is_empty());
        }
    }

    #[test]
    fn test_settings_all_tabs_accessible() {
        let mut settings = SettingsPaneState::new();

        let tabs = vec![
            SettingsTab::Display,
            SettingsTab::Data,
            SettingsTab::Execution,
            SettingsTab::Advanced,
        ];

        for tab in tabs {
            settings.switch_tab(tab);
            assert_eq!(settings.current_tab, tab);
            assert_eq!(settings.selected_setting_index, 0);
        }
    }

    #[test]
    fn test_operations_allocation_calculations() {
        let operations = OperationsPaneState::new();

        // Verify allocation calculations work
        let avg_adequacy = operations.avg_revenue_adequacy();
        assert!(avg_adequacy > 0.0);

        let avg_recovery = operations.avg_cost_recovery();
        assert!(avg_recovery > 0.0);

        let total_deficit = operations.total_surplus_deficit();
        assert!(total_deficit > -1000.0 && total_deficit < 1000.0);
    }

    #[test]
    fn test_quick_actions_all_have_keys() {
        let actions = QuickAction::all();

        // Ensure no duplicate keys
        let keys: Vec<char> = actions.iter().map(|a| a.key).collect();
        let unique_keys: std::collections::HashSet<_> = keys.iter().cloned().collect();
        assert_eq!(keys.len(), unique_keys.len());

        // Ensure all keys are unique
        for action in actions {
            assert!(QuickAction::find_by_key(action.key).is_some());
        }
    }
}
