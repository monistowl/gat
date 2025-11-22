pub mod app;
pub mod models;
pub mod events;
pub mod theme;
pub mod components;
pub mod pane_components;
pub mod message;
pub mod update;
pub mod navigation;
pub mod modal_renderer;
pub mod modal_examples;
pub mod services;
pub mod integration;
pub mod utils;

pub mod panes;
pub mod ui;
mod data;

pub use app::Application;
pub use models::{AppState, PaneId};
pub use events::AppEvent;
pub use message::Message;
pub use update::{update, SideEffect};
pub use modal_renderer::ModalRenderer;

// Data structures (from data.rs module)
pub use data::{DatasetEntry, DatasetStatus, DatasetsState, create_fixture_datasets};

// Query builder service
pub use services::{QueryBuilder, QueryError};
