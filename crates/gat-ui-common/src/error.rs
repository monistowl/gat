//! Error types for UI services.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for UI service operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors from UI service operations.
#[derive(Debug, Error)]
pub enum Error {
    /// No network is currently loaded.
    #[error("no network loaded")]
    NoNetworkLoaded,

    /// Failed to load network file.
    #[error("failed to load network from {path}: {source}")]
    LoadFailed {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },

    /// Analysis failed.
    #[error("analysis failed: {0}")]
    AnalysisFailed(String),

    /// Job not found.
    #[error("job not found: {0}")]
    JobNotFound(crate::JobId),

    /// Job was cancelled.
    #[error("job cancelled")]
    JobCancelled,

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Channel closed unexpectedly.
    #[error("channel closed")]
    ChannelClosed,
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(e: toml::ser::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Error::ChannelClosed
    }
}
