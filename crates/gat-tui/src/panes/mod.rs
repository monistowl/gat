pub mod registry;

// Pane implementations with state and PaneView
pub mod analytics_pane;
pub mod commands_pane;
pub mod dashboard_pane;
pub mod datasets_pane;
pub mod operations_pane; // Re-exports from operations_state for backward compatibility
pub mod operations_state; // Refactored operations state (focused sub-modules)
pub mod pipeline_pane;
pub mod quickstart_pane;
pub mod settings_pane;

// Integration tests
#[cfg(test)]
pub mod integration_tests;

// Re-exports: State types
pub use analytics_pane::{
    AnalyticsMetric, AnalyticsPane, AnalyticsPaneState, AnalyticsTab, CongestionStatus,
    DeliverabilityResult, ELCCResult, MetricStatus as AnalyticsMetricStatus, PowerFlowResult,
    ReliabilityResult,
};
pub use commands_pane::{
    CommandAction, CommandResult, CommandSnippet, CommandStatus, CommandsPane, CommandsPaneState,
    ExecutionMode,
};
pub use dashboard_pane::{
    ActionType, DashboardPane, DashboardPaneState, KPIMetrics, QuickAction, RecentRun,
};
pub use datasets_pane::{
    Dataset, DatasetMetadata, DatasetStatus, DatasetTab, DatasetsPane, DatasetsPaneState, GeoLayer,
    GeoLayerStatus, GeoLayerType, LagConfig, ScenarioStatus, ScenarioTemplate, SpatialJoinConfig,
    SpatialJoinType, UploadJob, UploadStatus, WeightMatrixType,
};
pub use operations_pane::{
    AllocationResult, BatchJob, JobStatus, MetricStatus, OperationsPane, OperationType,
    OperationsPaneState, ReliabilityMetric,
};
pub use pipeline_pane::{
    NodeType, PipelineNode, PipelinePane, PipelinePaneState, TransformTemplate,
};
pub use quickstart_pane::QuickstartPane;
pub use settings_pane::{
    AdvancedSettings, DataSettings, DisplaySettings, ExecutionSettings, SettingsPaneState,
    SettingsTab,
};
