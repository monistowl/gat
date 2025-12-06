//! # GAT UI Common
//!
//! Shared services for GAT user interfaces (TUI and GUI).
//!
//! This crate provides a unified backend that both `gat-tui` and `gat-gui` consume,
//! ensuring feature parity and reducing code duplication.
//!
//! ## Architecture
//!
//! ```text
//! gat-tui ──┐
//!           ├──► UiService ──► gat-algo, gat-io, gat-core
//! gat-gui ──┘
//! ```
//!
//! ## Core Components
//!
//! - [`UiService`]: Main entry point combining all services
//! - [`Workspace`]: Network state and analysis cache
//! - [`AnalysisService`]: Solver execution with async job management
//! - [`JobTracker`]: Background job tracking with progress channels
//! - [`GatConfig`]: Unified configuration for all UIs
//!
//! ## Usage
//!
//! ```ignore
//! use gat_ui_common::{UiService, AnalysisRequest};
//!
//! let service = UiService::new()?;
//!
//! // Load a network
//! service.workspace().load("case14.m").await?;
//!
//! // Run analysis
//! let handle = service.analysis()
//!     .run(AnalysisRequest::AcPowerFlow(Default::default()))
//!     .await?;
//!
//! // Wait for result
//! let result = handle.result.await?;
//! ```

pub mod analysis;
pub mod config;
pub mod error;
pub mod events;
pub mod jobs;
pub mod service;
pub mod workspace;

// Re-exports for convenience
pub use analysis::{AnalysisRequest, AnalysisService};
pub use config::{CoreConfig, GatConfig, GuiConfig, GuiTheme, TuiConfig};
pub use error::{Error, Result};
pub use events::{AnalysisKind, JobEvent, WorkspaceEvent};
pub use jobs::{JobHandle, JobId, JobTracker, Progress};
pub use service::UiService;
pub use workspace::{AnalysisCache, Workspace};
