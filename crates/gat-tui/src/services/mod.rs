pub mod gat_service;
pub mod query_builder;
pub mod type_mappers;
pub mod grid_service;
pub mod gat_core_query_builder;
pub mod command_service;
pub mod command_validator;
pub mod command_export;
pub mod tui_service_layer;
pub mod gat_core_service_adapter;
pub mod event_dispatcher;
pub mod async_service_integration;

#[cfg(test)]
mod grid_integration_tests;

pub use query_builder::{QueryBuilder, QueryError, MockQueryBuilder};
pub use gat_service::{GatService, PipelineService, DatasetsService, OperationsService};
pub use type_mappers::{network_to_dataset_entry, graph_stats_to_system_metrics};
pub use grid_service::{GridService, GridError};
pub use gat_core_query_builder::GatCoreQueryBuilder;
pub use command_service::{CommandService, CommandExecution, CommandError};
pub use command_validator::{CommandValidator, ValidCommand, ValidationError};
pub use command_export::{CommandExporter, ExportFormat, CommandStats};
pub use tui_service_layer::{TuiServiceLayer, AnalyticsType};
pub use gat_core_service_adapter::{GatCoreCliAdapter, LocalFileAdapter};
pub use event_dispatcher::{AsyncEvent, EventResult, EventDispatcher, BackgroundEventProcessor, EventDispatcherConfig, EventHandler};
pub use async_service_integration::AsyncServiceIntegration;
