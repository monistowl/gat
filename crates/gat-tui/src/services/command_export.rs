/// Command history export and serialization service
///
/// Provides functionality to export command execution history to various formats
/// and calculate statistics for analysis.

use crate::models::ExecutedCommand;
use std::time::SystemTime;

/// Export format for command history
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    PlainText,
}

/// Command history statistics
#[derive(Debug, Clone)]
pub struct CommandStats {
    pub total_commands: usize,
    pub successful_count: usize,
    pub failed_count: usize,
    pub timed_out_count: usize,
    pub success_rate: f32,
    pub average_duration_ms: f64,
    pub fastest_duration_ms: u64,
    pub slowest_duration_ms: u64,
}

/// Service for exporting and analyzing command history
pub struct CommandExporter;

impl CommandExporter {
    /// Export commands to JSON format
    pub fn export_json(commands: &[ExecutedCommand]) -> Result<String, String> {
        serde_json::to_string_pretty(commands)
            .map_err(|e| format!("JSON serialization failed: {}", e))
    }

    /// Export commands to CSV format
    pub fn export_csv(commands: &[ExecutedCommand]) -> Result<String, String> {
        if commands.is_empty() {
            return Ok("id,command,exit_code,stdout_length,stderr_length,duration_ms,timed_out,executed_at\n".to_string());
        }

        let mut csv = String::from("id,command,exit_code,stdout_length,stderr_length,duration_ms,timed_out,executed_at\n");

        for cmd in commands {
            let timestamp = match cmd.executed_at.duration_since(SystemTime::UNIX_EPOCH) {
                Ok(duration) => duration.as_secs(),
                Err(_) => 0,
            };

            let line = format!(
                "\"{}\",\"{}\",{},{},{},{},{},{}\n",
                Self::escape_csv(&cmd.id),
                Self::escape_csv(&cmd.command),
                cmd.exit_code,
                cmd.stdout.len(),
                cmd.stderr.len(),
                cmd.duration_ms,
                if cmd.timed_out { "true" } else { "false" },
                timestamp
            );
            csv.push_str(&line);
        }

        Ok(csv)
    }

    /// Export commands to plain text format
    pub fn export_text(commands: &[ExecutedCommand]) -> Result<String, String> {
        let mut text = String::from("Command Execution History\n");
        text.push_str("=========================\n\n");

        for cmd in commands {
            text.push_str(&format!("ID: {}\n", cmd.id));
            text.push_str(&format!("Command: {}\n", cmd.command));
            text.push_str(&format!("Exit Code: {}\n", cmd.exit_code));
            text.push_str(&format!("Duration: {}ms\n", cmd.duration_ms));
            text.push_str(&format!("Timed Out: {}\n", cmd.timed_out));
            text.push_str(&format!("Stdout Length: {} bytes\n", cmd.stdout.len()));
            text.push_str(&format!("Stderr Length: {} bytes\n", cmd.stderr.len()));
            text.push_str("---\n\n");
        }

        Ok(text)
    }

    /// Export commands in the specified format
    pub fn export(commands: &[ExecutedCommand], format: ExportFormat) -> Result<String, String> {
        match format {
            ExportFormat::Json => Self::export_json(commands),
            ExportFormat::Csv => Self::export_csv(commands),
            ExportFormat::PlainText => Self::export_text(commands),
        }
    }

    /// Calculate statistics from command history
    pub fn calculate_stats(commands: &[ExecutedCommand]) -> CommandStats {
        if commands.is_empty() {
            return CommandStats {
                total_commands: 0,
                successful_count: 0,
                failed_count: 0,
                timed_out_count: 0,
                success_rate: 0.0,
                average_duration_ms: 0.0,
                fastest_duration_ms: 0,
                slowest_duration_ms: 0,
            };
        }

        let total = commands.len();
        let successful = commands.iter().filter(|c| c.exit_code == 0).count();
        let failed = commands.iter().filter(|c| c.exit_code != 0 && !c.timed_out).count();
        let timed_out = commands.iter().filter(|c| c.timed_out).count();

        let total_duration: u64 = commands.iter().map(|c| c.duration_ms).sum();
        let average_duration = if total > 0 {
            total_duration as f64 / total as f64
        } else {
            0.0
        };

        let fastest = commands.iter().map(|c| c.duration_ms).min().unwrap_or(0);
        let slowest = commands.iter().map(|c| c.duration_ms).max().unwrap_or(0);

        let success_rate = if total > 0 {
            (successful as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        CommandStats {
            total_commands: total,
            successful_count: successful,
            failed_count: failed,
            timed_out_count: timed_out,
            success_rate,
            average_duration_ms: average_duration,
            fastest_duration_ms: fastest,
            slowest_duration_ms: slowest,
        }
    }

    /// Escape CSV field values
    fn escape_csv(field: &str) -> String {
        if field.contains(',') || field.contains('"') || field.contains('\n') {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{UNIX_EPOCH, Duration};

    fn create_test_command(id: &str, command: &str, exit_code: i32, duration_ms: u64, timed_out: bool) -> ExecutedCommand {
        ExecutedCommand {
            id: id.to_string(),
            command: command.to_string(),
            exit_code,
            stdout: "test output".to_string(),
            stderr: String::new(),
            duration_ms,
            timed_out,
            executed_at: UNIX_EPOCH + Duration::from_secs(1000),
        }
    }

    #[test]
    fn test_export_json_empty() {
        let result = CommandExporter::export_json(&[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "[]");
    }

    #[test]
    fn test_export_json_single_command() {
        let cmd = create_test_command("cmd_1", "echo test", 0, 150, false);
        let result = CommandExporter::export_json(&[cmd]);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("echo test"));
        assert!(json.contains("\"exit_code\": 0") || json.contains("\"exit_code\":0"));
    }

    #[test]
    fn test_export_csv_empty() {
        let result = CommandExporter::export_csv(&[]);
        assert!(result.is_ok());
        let csv = result.unwrap();
        assert!(csv.contains("id,command,exit_code"));
    }

    #[test]
    fn test_export_csv_single_command() {
        let cmd = create_test_command("cmd_1", "echo test", 0, 150, false);
        let result = CommandExporter::export_csv(&[cmd]);
        assert!(result.is_ok());
        let csv = result.unwrap();
        assert!(csv.contains("cmd_1"));
        assert!(csv.contains("echo test"));
        assert!(csv.contains("150"));
    }

    #[test]
    fn test_export_csv_escaping() {
        let cmd = ExecutedCommand {
            id: "cmd_1".to_string(),
            command: "echo \"test, value\"".to_string(),
            exit_code: 0,
            stdout: "output".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            timed_out: false,
            executed_at: UNIX_EPOCH + Duration::from_secs(1000),
        };
        let result = CommandExporter::export_csv(&[cmd]);
        assert!(result.is_ok());
        let csv = result.unwrap();
        // CSV should have escaped quotes
        assert!(csv.contains("\"echo \"\"test, value\"\"\""));
    }

    #[test]
    fn test_export_text_empty() {
        let result = CommandExporter::export_text(&[]);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Command Execution History"));
    }

    #[test]
    fn test_export_text_single_command() {
        let cmd = create_test_command("cmd_1", "echo test", 0, 150, false);
        let result = CommandExporter::export_text(&[cmd]);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("cmd_1"));
        assert!(text.contains("echo test"));
        assert!(text.contains("Exit Code: 0"));
    }

    #[test]
    fn test_export_format_json() {
        let cmd = create_test_command("cmd_1", "echo test", 0, 150, false);
        let result = CommandExporter::export(&[cmd], ExportFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_format_csv() {
        let cmd = create_test_command("cmd_1", "echo test", 0, 150, false);
        let result = CommandExporter::export(&[cmd], ExportFormat::Csv);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_format_text() {
        let cmd = create_test_command("cmd_1", "echo test", 0, 150, false);
        let result = CommandExporter::export(&[cmd], ExportFormat::PlainText);
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_stats_empty() {
        let stats = CommandExporter::calculate_stats(&[]);
        assert_eq!(stats.total_commands, 0);
        assert_eq!(stats.successful_count, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_calculate_stats_single_success() {
        let cmd = create_test_command("cmd_1", "echo test", 0, 100, false);
        let stats = CommandExporter::calculate_stats(&[cmd]);
        assert_eq!(stats.total_commands, 1);
        assert_eq!(stats.successful_count, 1);
        assert_eq!(stats.failed_count, 0);
        assert_eq!(stats.timed_out_count, 0);
        assert_eq!(stats.success_rate, 100.0);
        assert_eq!(stats.average_duration_ms, 100.0);
        assert_eq!(stats.fastest_duration_ms, 100);
        assert_eq!(stats.slowest_duration_ms, 100);
    }

    #[test]
    fn test_calculate_stats_single_failure() {
        let cmd = create_test_command("cmd_1", "false", 1, 50, false);
        let stats = CommandExporter::calculate_stats(&[cmd]);
        assert_eq!(stats.total_commands, 1);
        assert_eq!(stats.successful_count, 0);
        assert_eq!(stats.failed_count, 1);
        assert_eq!(stats.timed_out_count, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_calculate_stats_timeout() {
        let cmd = create_test_command("cmd_1", "sleep 10", -1, 5000, true);
        let stats = CommandExporter::calculate_stats(&[cmd]);
        assert_eq!(stats.total_commands, 1);
        assert_eq!(stats.successful_count, 0);
        assert_eq!(stats.failed_count, 0);
        assert_eq!(stats.timed_out_count, 1);
    }

    #[test]
    fn test_calculate_stats_mixed() {
        let commands = vec![
            create_test_command("cmd_1", "echo test", 0, 100, false),
            create_test_command("cmd_2", "false", 1, 50, false),
            create_test_command("cmd_3", "sleep 10", -1, 5000, true),
            create_test_command("cmd_4", "echo ok", 0, 200, false),
        ];
        let stats = CommandExporter::calculate_stats(&commands);
        assert_eq!(stats.total_commands, 4);
        assert_eq!(stats.successful_count, 2);
        assert_eq!(stats.failed_count, 1);
        assert_eq!(stats.timed_out_count, 1);
        assert_eq!(stats.success_rate, 50.0);
        assert_eq!(stats.average_duration_ms, 1337.5);
        assert_eq!(stats.fastest_duration_ms, 50);
        assert_eq!(stats.slowest_duration_ms, 5000);
    }

    #[test]
    fn test_calculate_stats_all_successful() {
        let commands = vec![
            create_test_command("cmd_1", "echo a", 0, 100, false),
            create_test_command("cmd_2", "echo b", 0, 200, false),
            create_test_command("cmd_3", "echo c", 0, 150, false),
        ];
        let stats = CommandExporter::calculate_stats(&commands);
        assert_eq!(stats.total_commands, 3);
        assert_eq!(stats.successful_count, 3);
        assert_eq!(stats.failed_count, 0);
        assert_eq!(stats.timed_out_count, 0);
        assert_eq!(stats.success_rate, 100.0);
        assert!((stats.average_duration_ms - 150.0).abs() < 0.1);
    }

    #[test]
    fn test_escape_csv_no_special_chars() {
        let result = CommandExporter::escape_csv("simple");
        assert_eq!(result, "simple");
    }

    #[test]
    fn test_escape_csv_with_comma() {
        let result = CommandExporter::escape_csv("hello,world");
        assert_eq!(result, "\"hello,world\"");
    }

    #[test]
    fn test_escape_csv_with_quotes() {
        let result = CommandExporter::escape_csv("say \"hello\"");
        assert_eq!(result, "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn test_export_json_multiple_commands() {
        let commands = vec![
            create_test_command("cmd_1", "echo test", 0, 100, false),
            create_test_command("cmd_2", "echo hello", 0, 150, false),
        ];
        let result = CommandExporter::export_json(&commands);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("echo test"));
        assert!(json.contains("echo hello"));
    }

    #[test]
    fn test_export_csv_with_newlines() {
        let cmd = ExecutedCommand {
            id: "cmd_1".to_string(),
            command: "echo test".to_string(),
            exit_code: 0,
            stdout: "line1\nline2\nline3".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            timed_out: false,
            executed_at: UNIX_EPOCH + Duration::from_secs(1000),
        };
        let result = CommandExporter::export_csv(&[cmd]);
        assert!(result.is_ok());
        let csv = result.unwrap();
        // "line1\nline2\nline3" has length 17 (including newline chars)
        assert!(csv.contains("17"));
    }
}
