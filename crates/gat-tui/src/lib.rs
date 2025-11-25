pub mod app;
pub mod command_runner;
pub mod components;
pub mod events;
pub mod integration;
pub mod message;
pub mod modal_examples;
pub mod modal_renderer;
pub mod models;
pub mod navigation;
pub mod pane_components;
pub mod services;
pub mod theme;
pub mod update;
pub mod utils;

mod data;
pub mod panes;
pub mod ui;

pub use app::Application;
pub use command_runner::CommandHandle;
pub use events::AppEvent;
pub use message::Message;
pub use modal_renderer::ModalRenderer;
pub use models::{AppState, PaneId};
pub use ui::App;
pub use update::{update, SideEffect};

// Data structures (from data.rs module)
pub use data::{create_fixture_datasets, DatasetEntry, DatasetStatus, DatasetsState};

// Query builder service
pub use services::{QueryBuilder, QueryError};
