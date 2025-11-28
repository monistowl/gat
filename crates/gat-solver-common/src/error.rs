//! Error types and exit codes for solver communication.

use thiserror::Error;

/// Exit codes for solver subprocess communication.
///
/// These match the protocol defined in the design document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    /// Success (check status in solution for optimality)
    Success = 0,
    /// Invalid input (malformed Arrow, missing fields)
    InvalidInput = 1,
    /// Solver error (license, numerical issues)
    SolverError = 2,
    /// Timeout
    Timeout = 3,
    /// Segfault (SIGSEGV) - native crash
    Segfault = 139,
}

impl ExitCode {
    /// Convert from raw exit code to ExitCode enum.
    pub fn from_raw(code: i32) -> Self {
        match code {
            0 => ExitCode::Success,
            1 => ExitCode::InvalidInput,
            2 => ExitCode::SolverError,
            3 => ExitCode::Timeout,
            139 => ExitCode::Segfault,
            _ => ExitCode::SolverError, // Unknown codes treated as solver error
        }
    }

    /// Check if this exit code indicates success.
    pub fn is_success(&self) -> bool {
        matches!(self, ExitCode::Success)
    }
}

/// Errors that can occur during solver operations.
#[derive(Debug, Error)]
pub enum SolverError {
    /// Unknown solver ID.
    #[error("Unknown solver: {0}")]
    UnknownSolver(String),

    /// Solver is not installed.
    #[error("Solver {solver} is not installed. Install with: gat install {hint}")]
    NotInstalled {
        solver: crate::SolverId,
        hint: String,
    },

    /// No solver available for the given problem type.
    #[error("No solver available for {problem_type}. {hint}")]
    NoSolverAvailable {
        problem_type: crate::ProblemType,
        hint: String,
    },

    /// Native solvers are disabled globally.
    #[error(
        "Native solvers are disabled. Enable with `native_enabled = true` in ~/.gat/config.toml"
    )]
    NativeDisabled,

    /// Solver process failed to start.
    #[error("Failed to start solver process: {0}")]
    ProcessStart(#[source] std::io::Error),

    /// Solver process crashed or returned an error.
    #[error("Solver process failed with exit code {exit_code:?}: {message}")]
    ProcessFailed {
        exit_code: ExitCode,
        message: String,
    },

    /// Timeout while waiting for solver.
    #[error("Solver timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// IPC communication error.
    #[error("IPC error: {0}")]
    Ipc(String),

    /// Arrow serialization/deserialization error.
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),

    /// Generic IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// User declined to accept native solver risk.
    #[error("User declined to accept native solver risk for {0}")]
    RiskNotAccepted(crate::SolverId),

    /// Compute bounds exceeded.
    #[error("Compute bounds exceeded: {message}")]
    ComputeBoundsExceeded { message: String },
}

/// Result type alias for solver operations.
pub type SolverResult<T> = Result<T, SolverError>;
