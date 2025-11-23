pub mod async_service_integration;
pub mod command_export;
pub mod command_service;
pub mod command_validator;
pub mod event_dispatcher;
pub mod gat_core_query_builder;
pub mod gat_core_service_adapter;
pub mod gat_service;
pub mod grid_service;
pub mod query_builder;
pub mod tui_service_layer;
pub mod type_mappers;

#[cfg(test)]
mod grid_integration_tests;

pub use async_service_integration::AsyncServiceIntegration;
pub use command_export::{CommandExporter, CommandStats, ExportFormat};
pub use command_service::{CommandError, CommandExecution, CommandService};
pub use command_validator::{CommandValidator, ValidCommand, ValidationError};
pub use event_dispatcher::{
    AsyncEvent, BackgroundEventProcessor, EventDispatcher, EventDispatcherConfig, EventHandler,
    EventResult,
};
pub use gat_core_query_builder::GatCoreQueryBuilder;
pub use gat_core_service_adapter::{GatCoreCliAdapter, LocalFileAdapter};
pub use gat_service::{DatasetsService, GatService, OperationsService, PipelineService};
pub use grid_service::{GridError, GridService};
pub use query_builder::{MockQueryBuilder, QueryBuilder, QueryError};
pub use tui_service_layer::{AnalyticsType, TuiServiceLayer};
pub use type_mappers::{graph_stats_to_system_metrics, network_to_dataset_entry};
