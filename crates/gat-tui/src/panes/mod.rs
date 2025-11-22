pub mod registry;

// New tuirealm-based pane implementations
pub mod dashboard_pane;
pub mod commands_pane;

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
