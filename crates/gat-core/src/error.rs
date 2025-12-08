//! Unified error types for the GAT ecosystem
//!
//! This module provides a common error type [`GatError`] that can represent
//! errors from any part of the system. Domain-specific error types can be
//! converted to `GatError` for uniform error handling at API boundaries.
//!
//! # Example
//!
//! ```ignore
//! use gat_core::{GatError, GatResult};
//!
//! fn process_network(path: &str) -> GatResult<()> {
//!     let network = load_network(path)?;
//!     solve_opf(&network)?;
//!     Ok(())
//! }
//! ```

use thiserror::Error;

/// Unified error type for all GAT operations.
///
/// This enum provides a common error representation for the GAT ecosystem,
/// allowing errors from I/O, parsing, solving, and validation to be handled
/// uniformly.
#[derive(Error, Debug)]
pub enum GatError {
    /// I/O errors (file access, network, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing/deserialization errors
    #[error("Parse error: {0}")]
    Parse(String),

    /// Data validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Solver/algorithm errors
    #[error("Solver error: {0}")]
    Solver(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network structure errors
    #[error("Network error: {0}")]
    Network(String),

    /// Generic errors (for wrapping external errors)
    #[error("{0}")]
    Other(String),
}

/// Convenience type alias for Results using GatError.
pub type GatResult<T> = Result<T, GatError>;

// Conversion from anyhow::Error
impl From<anyhow::Error> for GatError {
    fn from(err: anyhow::Error) -> Self {
        GatError::Other(err.to_string())
    }
}

// Conversion from string-like types for convenience
impl From<String> for GatError {
    fn from(s: String) -> Self {
        GatError::Other(s)
    }
}

impl From<&str> for GatError {
    fn from(s: &str) -> Self {
        GatError::Other(s.to_string())
    }
}

// JSON parsing errors
impl From<serde_json::Error> for GatError {
    fn from(err: serde_json::Error) -> Self {
        GatError::Parse(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = GatError::Solver("convergence failed".into());
        assert!(err.to_string().contains("Solver error"));
        assert!(err.to_string().contains("convergence failed"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let gat_err: GatError = io_err.into();
        assert!(matches!(gat_err, GatError::Io(_)));
    }

    #[test]
    fn test_result_type_alias() {
        fn example_fn() -> GatResult<i32> {
            Ok(42)
        }
        assert_eq!(example_fn().unwrap(), 42);
    }

    #[test]
    fn test_question_mark_operator() {
        fn inner() -> GatResult<()> {
            Err(GatError::Validation("test".into()))
        }

        fn outer() -> GatResult<()> {
            inner()?;
            Ok(())
        }

        assert!(outer().is_err());
    }
}
