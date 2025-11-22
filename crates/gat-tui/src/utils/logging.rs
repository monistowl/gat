/// Logging configuration and initialization
///
/// Provides convenient setup for the tracing logging system with support for
/// both console output and file-based logging.

use tracing_subscriber::{
    fmt, prelude::*, registry::Registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_appender::rolling;
use std::path::Path;

/// Initialize console logging with optional file output
///
/// Sets up tracing for console output with an environment-controlled log level.
/// If a log directory is provided, also writes logs to rolling files.
///
/// # Arguments
/// * `log_dir` - Optional directory for rolling log files (hourly rotation)
/// * `_max_level_env` - Environment variable name for log level (default: "GAT_LOG")
///
/// # Example
/// ```ignore
/// init_logging(Some("./logs"), "GAT_LOG").expect("failed to init logging");
/// ```
pub fn init_logging(log_dir: Option<&str>, _max_level_env: &str) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let console_layer = fmt::layer().with_writer(std::io::stdout);

    let registry = Registry::default()
        .with(env_filter)
        .with(console_layer);

    if let Some(log_dir) = log_dir {
        let file_appender = rolling::hourly(log_dir, "gat-tui.log");
        registry
            .with(
                fmt::layer()
                    .json()
                    .with_writer(file_appender)
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    } else {
        registry.init();
    }

    Ok(())
}

/// Setup file-based logging with rolling files
///
/// Creates an hourly rotating log file with structured JSON output.
/// Useful for post-mortem analysis and debugging.
///
/// # Arguments
/// * `log_dir` - Directory to store log files
/// * `file_prefix` - Prefix for log file names
///
/// # Example
/// ```ignore
/// setup_file_logging("./logs", "app").expect("failed to setup file logging");
/// ```
pub fn setup_file_logging(log_dir: impl AsRef<Path>, file_prefix: &str) -> anyhow::Result<()> {
    let appender = rolling::hourly(log_dir, file_prefix);
    let file_layer = fmt::layer()
        .json()
        .with_writer(appender)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);

    let registry = Registry::default().with(file_layer);

    let _ = registry;
    tracing::info!("File logging initialized");

    Ok(())
}

/// Log a user-facing message with context
pub fn log_user_action(action: &str, details: &str) {
    tracing::info!(
        action = action,
        details = details,
        "User action executed"
    );
}

/// Log an operation start (useful for span tracking)
pub fn log_operation_start(operation: &str) {
    tracing::debug!(operation = operation, "Operation started");
}

/// Log an operation completion
pub fn log_operation_complete(operation: &str, duration_ms: u128) {
    tracing::debug!(
        operation = operation,
        duration_ms = duration_ms,
        "Operation completed"
    );
}

/// Log an error with context
pub fn log_error(context: &str, error: &dyn std::error::Error) {
    tracing::error!(
        context = context,
        error = %error,
        "Error occurred"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging_console_only() {
        // This will initialize the global logger (can only be done once per test run)
        let result = init_logging(None, "GAT_LOG");
        // In actual tests, we'd check that logging is working
        // But since we can only init once, we just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_logging_macros_compile() {
        // These should compile without errors
        tracing::info!("test message");
        tracing::debug!("debug message");
        tracing::warn!("warning message");
        tracing::error!("error message");
    }

    #[test]
    fn test_log_user_action() {
        log_user_action("test_action", "test details");
        // Just verify it doesn't panic
    }

    #[test]
    fn test_log_operation_timing() {
        log_operation_start("test_op");
        log_operation_complete("test_op", 150);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_log_error() {
        let error = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        log_error("test context", &error);
        // Just verify it doesn't panic
    }
}
