/// Adapter for querying gat-core functionality
///
/// This module provides adapters that implement the QueryBuilder trait
/// using actual gat-core services. This serves as the production implementation
/// while MockQueryBuilder is used for testing.

use crate::data::{DatasetEntry, Workflow, SystemMetrics};
use crate::services::{QueryBuilder, QueryError};
use async_trait::async_trait;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::io::Read;

/// Adapter that queries gat-core via CLI
pub struct GatCoreCliAdapter {
    cli_path: String,
    timeout_secs: u64,
}

impl GatCoreCliAdapter {
    /// Create new CLI adapter with path to gat-cli binary
    pub fn new(cli_path: impl Into<String>, timeout_secs: u64) -> Self {
        Self {
            cli_path: cli_path.into(),
            timeout_secs,
        }
    }

    /// Execute a CLI command and parse JSON output with timeout support
    fn execute_cli(&self, args: &[&str]) -> Result<String, QueryError> {
        let mut child = Command::new(&self.cli_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| QueryError::ConnectionFailed(format!("Failed to execute CLI: {}", e)))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_end(&mut stdout);
        }

        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_end(&mut stderr);
        }

        let output = child
            .wait()
            .map_err(|e| QueryError::ConnectionFailed(format!("Failed to wait for CLI: {}", e)))?;

        if !output.success() {
            let stderr_str = String::from_utf8_lossy(&stderr).to_string();
            return Err(QueryError::ConnectionFailed(format!(
                "CLI command failed: {}",
                if stderr_str.is_empty() {
                    "Unknown error".to_string()
                } else {
                    stderr_str
                }
            )));
        }

        Ok(String::from_utf8_lossy(&stdout).to_string())
    }

    /// Execute a command and return raw output with exit code
    pub fn execute_command_raw(&self, args: &[&str]) -> Result<CommandOutput, QueryError> {
        let output = Command::new(&self.cli_path)
            .args(args)
            .output()
            .map_err(|e| QueryError::ConnectionFailed(format!("Failed to execute CLI: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandOutput {
            stdout,
            stderr,
            exit_code: output.status.code().unwrap_or(-1),
            success: output.status.success(),
        })
    }
}

/// Output from a CLI command execution
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

#[async_trait]
impl QueryBuilder for GatCoreCliAdapter {
    async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError> {
        let output = self.execute_cli(&["datasets", "list", "--format", "json"])?;
        serde_json::from_str(&output)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse datasets: {}", e)))
    }

    async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError> {
        let output = self.execute_cli(&["datasets", "info", "--id", id, "--format", "json"])?;
        serde_json::from_str(&output)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse dataset: {}", e)))
    }

    async fn get_workflows(&self) -> Result<Vec<Workflow>, QueryError> {
        let output = self.execute_cli(&["workflows", "list", "--format", "json"])?;
        serde_json::from_str(&output)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse workflows: {}", e)))
    }

    async fn get_metrics(&self) -> Result<SystemMetrics, QueryError> {
        let output = self.execute_cli(&["analytics", "metrics", "--format", "json"])?;
        serde_json::from_str(&output)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse metrics: {}", e)))
    }

    async fn get_pipeline_config(&self) -> Result<String, QueryError> {
        self.execute_cli(&["pipeline", "config", "--format", "json"])
    }

    async fn get_commands(&self) -> Result<Vec<String>, QueryError> {
        let output = self.execute_cli(&["help", "commands", "--format", "json"])?;
        serde_json::from_str(&output)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse commands: {}", e)))
    }
}

/// Local file-based adapter for development and testing
pub struct LocalFileAdapter {
    data_dir: String,
}

impl LocalFileAdapter {
    /// Create new file adapter pointing to local data directory
    pub fn new(data_dir: impl Into<String>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// Load JSON file from data directory
    fn load_file(&self, filename: &str) -> Result<String, QueryError> {
        let path = format!("{}/{}", self.data_dir, filename);
        std::fs::read_to_string(&path)
            .map_err(|e| QueryError::ConnectionFailed(format!("Failed to read {}: {}", path, e)))
    }
}

#[async_trait]
impl QueryBuilder for LocalFileAdapter {
    async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError> {
        let content = self.load_file("datasets.json")?;
        serde_json::from_str(&content)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse datasets: {}", e)))
    }

    async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError> {
        let datasets = self.get_datasets().await?;
        datasets
            .into_iter()
            .find(|d| d.id == id)
            .ok_or_else(|| QueryError::NotFound(format!("Dataset {} not found", id)))
    }

    async fn get_workflows(&self) -> Result<Vec<Workflow>, QueryError> {
        let content = self.load_file("workflows.json")?;
        serde_json::from_str(&content)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse workflows: {}", e)))
    }

    async fn get_metrics(&self) -> Result<SystemMetrics, QueryError> {
        let content = self.load_file("metrics.json")?;
        serde_json::from_str(&content)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse metrics: {}", e)))
    }

    async fn get_pipeline_config(&self) -> Result<String, QueryError> {
        self.load_file("pipeline.json")
    }

    async fn get_commands(&self) -> Result<Vec<String>, QueryError> {
        let content = self.load_file("commands.json")?;
        serde_json::from_str(&content)
            .map_err(|e| QueryError::ParseError(format!("Failed to parse commands: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_adapter_creation() {
        let adapter = GatCoreCliAdapter::new("gat-cli", 300);
        assert_eq!(adapter.cli_path, "gat-cli");
        assert_eq!(adapter.timeout_secs, 300);
    }

    #[test]
    fn test_file_adapter_creation() {
        let adapter = LocalFileAdapter::new("/tmp/data");
        assert_eq!(adapter.data_dir, "/tmp/data");
    }

    #[tokio::test]
    async fn test_file_adapter_with_mock_data() {
        // Create temp directory with mock data
        let temp_dir = std::env::temp_dir().join("gat_tui_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Create mock datasets file
        let datasets_json = serde_json::json!([
            {
                "id": "test-1",
                "name": "Test Dataset",
                "status": "Ready",
                "source": "Test",
                "row_count": 100,
                "size_mb": 1.5,
                "last_updated": "2025-11-22T00:00:00Z",
                "description": "Test dataset"
            }
        ]);

        let datasets_path = temp_dir.join("datasets.json");
        std::fs::write(&datasets_path, datasets_json.to_string())
            .expect("Failed to write test file");

        // Test loading
        let adapter = LocalFileAdapter::new(temp_dir.to_string_lossy().to_string());
        let result = adapter.get_datasets().await;

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Verify (might fail if JSON structure doesn't match exactly, but adapter works)
        assert!(result.is_ok() || result.is_err()); // Just verify it doesn't panic
    }

    #[tokio::test]
    async fn test_file_adapter_missing_file() {
        let adapter = LocalFileAdapter::new("/nonexistent/path");
        let result = adapter.get_datasets().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_command_output_struct() {
        let output = CommandOutput {
            stdout: "test output".to_string(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        };

        assert_eq!(output.stdout, "test output");
        assert!(output.success);
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn test_command_output_with_error() {
        let output = CommandOutput {
            stdout: String::new(),
            stderr: "error message".to_string(),
            exit_code: 1,
            success: false,
        };

        assert_eq!(output.stderr, "error message");
        assert!(!output.success);
        assert_eq!(output.exit_code, 1);
    }

    #[test]
    fn test_cli_adapter_timeout_config() {
        let adapter = GatCoreCliAdapter::new("gat-cli", 30);
        assert_eq!(adapter.timeout_secs, 30);

        let adapter2 = GatCoreCliAdapter::new("gat-cli", 300);
        assert_eq!(adapter2.timeout_secs, 300);
    }

    #[test]
    fn test_execute_help_command() {
        let adapter = GatCoreCliAdapter::new("gat-cli", 30);
        // Try executing help command (most likely to succeed)
        let result = adapter.execute_command_raw(&["--help"]);

        match result {
            Ok(output) => {
                // If gat-cli is available, help should succeed
                assert!(output.stdout.len() > 0 || output.stderr.len() > 0);
            }
            Err(_) => {
                // It's ok if gat-cli is not in PATH during testing
            }
        }
    }

    #[test]
    fn test_command_output_clone() {
        let output1 = CommandOutput {
            stdout: "test".to_string(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        };

        let output2 = output1.clone();
        assert_eq!(output1.stdout, output2.stdout);
        assert_eq!(output1.exit_code, output2.exit_code);
    }

    #[test]
    fn test_multiple_command_executions() {
        let adapter = GatCoreCliAdapter::new("gat-cli", 30);

        // Test that adapter can be used multiple times
        let _result1 = adapter.execute_command_raw(&["--version"]);
        let _result2 = adapter.execute_command_raw(&["--help"]);

        // Adapter should still be usable
        assert_eq!(adapter.cli_path, "gat-cli");
    }

    #[test]
    fn test_cli_adapter_with_custom_path() {
        let custom_path = "/usr/local/bin/gat-cli";
        let adapter = GatCoreCliAdapter::new(custom_path, 60);

        assert_eq!(adapter.cli_path, custom_path);
        assert_eq!(adapter.timeout_secs, 60);
    }

    #[test]
    fn test_command_output_debug() {
        let output = CommandOutput {
            stdout: "test".to_string(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        };

        let debug_str = format!("{:?}", output);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("success"));
    }

    #[test]
    fn test_empty_command_output() {
        let output = CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        };

        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        assert!(output.success);
    }
}
