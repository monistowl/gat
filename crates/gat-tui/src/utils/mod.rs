/// Utilities and helpers for gat-tui
///
/// This module provides wrappers and facades around external crates to abstract
/// away implementation details and provide convenient application-specific APIs.

pub mod id_generator;
pub mod logging;
pub mod time_utils;
pub mod config_loader;

pub use id_generator::{IdGenerator, TaskId};
pub use logging::{init_logging, setup_file_logging};
pub use time_utils::Timestamp;
pub use config_loader::ConfigManager;
