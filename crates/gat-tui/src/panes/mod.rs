pub mod registry;

// New tuirealm-based pane implementations
pub mod dashboard_pane;
pub mod commands_pane;
pub mod datasets_pane;
pub mod pipeline_pane;
pub mod operations_pane;
pub mod analytics_pane;

// Legacy pane implementations (to be replaced)
pub mod commands;
pub mod dashboard;
pub mod datasets;
pub mod operations;
pub mod pipeline;
pub mod quickstart;

// Re-exports
pub use dashboard_pane::{DashboardPaneState, KPIMetrics, RecentRun, QuickAction};
pub use commands_pane::{CommandsPaneState, CommandSnippet, CommandResult, ExecutionMode, CommandStatus, CommandAction};
pub use datasets_pane::{DatasetsPaneState, Dataset, DatasetStatus, UploadJob, UploadStatus, DatasetMetadata};
pub use pipeline_pane::{PipelinePaneState, PipelineNode, NodeType, TransformTemplate};
pub use operations_pane::{OperationsPaneState, BatchJob, JobStatus, AllocationResult, ReliabilityMetric, MetricStatus, OperationType};
pub use analytics_pane::{AnalyticsPaneState, AnalyticsTab, AnalyticsMetric, ReliabilityResult, DeliverabilityResult, ELCCResult, PowerFlowResult, CongestionStatus, MetricStatus as AnalyticsMetricStatus};
