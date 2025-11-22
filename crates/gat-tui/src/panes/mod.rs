pub mod registry;

// New tuirealm-based pane implementations
pub mod dashboard_pane;

// Legacy pane implementations (to be replaced)
pub mod commands;
pub mod dashboard;
pub mod datasets;
pub mod operations;
pub mod pipeline;
pub mod quickstart;

// Re-exports
pub use dashboard_pane::{DashboardPaneState, KPIMetrics, RecentRun, QuickAction};
