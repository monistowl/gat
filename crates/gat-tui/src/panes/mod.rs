pub mod registry;

// New tuirealm-based pane implementations
pub mod analytics_pane;
pub mod commands_pane;
pub mod dashboard_pane;
pub mod datasets_pane;
pub mod operations_pane;
pub mod pipeline_pane;
pub mod settings_pane;

// Integration tests
#[cfg(test)]
pub mod integration_tests;

// Legacy pane implementations (to be replaced)
pub mod analytics;
pub mod commands;
pub mod dashboard;
pub mod datasets;
pub mod operations;
pub mod pipeline;
pub mod quickstart;

// Re-exports
pub use analytics_pane::{
    AnalyticsMetric, AnalyticsPaneState, AnalyticsTab, CongestionStatus, DeliverabilityResult,
    ELCCResult, MetricStatus as AnalyticsMetricStatus, PowerFlowResult, ReliabilityResult,
};
pub use commands_pane::{
    CommandAction, CommandResult, CommandSnippet, CommandStatus, CommandsPaneState, ExecutionMode,
};
pub use dashboard_pane::{ActionType, DashboardPaneState, KPIMetrics, QuickAction, RecentRun};
pub use datasets_pane::{
    Dataset, DatasetMetadata, DatasetStatus, DatasetsPaneState, UploadJob, UploadStatus,
};
pub use operations_pane::{
    AllocationResult, BatchJob, JobStatus, MetricStatus, OperationType, OperationsPaneState,
    ReliabilityMetric,
};
pub use pipeline_pane::{NodeType, PipelineNode, PipelinePaneState, TransformTemplate};
pub use settings_pane::{
    AdvancedSettings, DataSettings, DisplaySettings, ExecutionSettings, SettingsPaneState,
    SettingsTab,
};
