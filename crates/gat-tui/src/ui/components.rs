/// Reusable UI component builders for common TUI patterns.
///
/// Provides helper functions for:
/// - File browser tables
/// - Progress indicators
/// - Configuration forms
/// - Data preview tables
///
/// These components are used across panes for consistent UI and reduce duplication.
use super::*;

/// Information about a file for display in file browsers
#[derive(Clone, Debug)]
pub struct FileInfo {
    pub path: String,
    pub file_type: String,
    pub size_bytes: u64,
}

impl FileInfo {
    pub fn new(path: impl Into<String>, file_type: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            path: path.into(),
            file_type: file_type.into(),
            size_bytes,
        }
    }

    /// Format file size for display (bytes, KB, MB, GB)
    pub fn format_size(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = self.size_bytes as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        if unit_idx == 0 {
            format!("{} {}", size as u64, UNITS[unit_idx])
        } else {
            format!("{:.1} {}", size, UNITS[unit_idx])
        }
    }
}

/// Create a file browser table showing available files
///
/// # Example
/// ```ignore
/// let files = vec![
///     FileInfo::new("config.yaml", "yaml", 2048),
///     FileInfo::new("data.parquet", "parquet", 1024000),
/// ];
/// let table = file_browser_table(&files);
/// ```
pub fn file_browser_table(files: &[FileInfo]) -> Pane {
    let mut table = TableView::new(["Path", "Type", "Size"]);

    for file in files {
        table = table.add_row([
            file.path.as_str(),
            file.file_type.as_str(),
            &file.format_size(),
        ]);
    }

    Pane::new("Available Files")
        .with_table(table)
        .with_child(Pane::new("").body(["Navigation: ↑↓ select  Enter confirm  Esc cancel"]))
}

/// Create a progress bar with percentage and counts
///
/// # Example
/// ```ignore
/// let bar = progress_bar(40, 100, 80);  // 40/100 done, 80 char width
/// // Output: "[████████░░░░░░░░░░░░] 40%"
/// ```
pub fn progress_bar(current: u32, total: u32, width: u16) -> String {
    let width = width.saturating_sub(15) as usize; // Reserve space for text
    let pct = if total > 0 {
        (current as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let filled = ((pct / 100.0) * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    format!(
        "[{}{}] {:.0}% ({}/{})",
        "█".repeat(filled),
        "░".repeat(empty),
        pct,
        current,
        total
    )
}

/// Job status for display in operation queues
#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Queued => "Queued",
            JobStatus::Running => "Running",
            JobStatus::Completed => "✓ Done",
            JobStatus::Failed => "✗ Failed",
        }
    }
}

/// Job information for display in operation queues
#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub scenario_id: String,
    pub status: JobStatus,
    pub progress_pct: f32,
    pub elapsed_secs: u64,
}

/// Create a job queue table for batch operations
///
/// # Example
/// ```ignore
/// let jobs = vec![
///     Job {
///         id: "job-1".into(),
///         scenario_id: "scenario-1".into(),
///         status: JobStatus::Running,
///         progress_pct: 45.0,
///         elapsed_secs: 12,
///     },
/// ];
/// let table = job_queue_table(&jobs);
/// ```
pub fn job_queue_table(jobs: &[Job]) -> Pane {
    let mut table = TableView::new(["Job ID", "Scenario", "Status", "Progress", "Elapsed"]);

    for job in jobs {
        let progress = format!("{:.0}%", job.progress_pct);
        let elapsed = format_elapsed(job.elapsed_secs);

        table = table.add_row([
            job.id.as_str(),
            job.scenario_id.as_str(),
            job.status.as_str(),
            &progress,
            &elapsed,
        ]);
    }

    Pane::new("Job Queue").with_table(table)
}

/// Format elapsed seconds as human-readable duration
fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {:.0}s", secs / 60, secs % 60)
    } else {
        format!("{}h {:.0}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Configuration field types for form builders
#[derive(Clone, Debug)]
pub enum ConfigFieldType {
    Text,
    Number,
    Dropdown(Vec<String>),
}

/// A single field in a configuration form
#[derive(Clone, Debug)]
pub struct ConfigField {
    pub name: String,
    pub label: String,
    pub field_type: ConfigFieldType,
    pub value: String,
}

impl ConfigField {
    pub fn text(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            field_type: ConfigFieldType::Text,
            value: String::new(),
        }
    }

    pub fn number(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            field_type: ConfigFieldType::Number,
            value: "1".into(),
        }
    }

    pub fn dropdown(
        name: impl Into<String>,
        label: impl Into<String>,
        options: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            field_type: ConfigFieldType::Dropdown(options),
            value: String::new(),
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
}

/// Create a configuration form from fields
///
/// # Example
/// ```ignore
/// let fields = vec![
///     ConfigField::text("name", "Name"),
///     ConfigField::dropdown("method", "Join Method", vec![
///         "point_in_polygon".into(),
///         "voronoi".into(),
///     ]).with_value("point_in_polygon"),
/// ];
/// let form = config_form("Settings", &fields);
/// ```
pub fn config_form(title: &str, fields: &[ConfigField]) -> Pane {
    let mut body = vec![];

    for field in fields {
        match &field.field_type {
            ConfigFieldType::Text => {
                body.push(format!("▍ {}: [{}]", field.label, field.value));
            }
            ConfigFieldType::Number => {
                body.push(format!("▍ {}: [ ▲ {} ▼ ]", field.label, field.value));
            }
            ConfigFieldType::Dropdown(options) => {
                body.push(format!("▍ {} [▼ {}]", field.label, field.value));
                for opt in options {
                    if opt == &field.value {
                        body.push(format!("    [✓] {}", opt));
                    } else {
                        body.push(format!("    [ ] {}", opt));
                    }
                }
            }
        }
    }

    Pane::new(title).body(body.as_slice())
}

/// Metric value with status for KPI cards
#[derive(Clone, Debug)]
pub enum MetricStatus {
    Good,
    Warning,
    Critical,
}

impl MetricStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricStatus::Good => "✓",
            MetricStatus::Warning => "⚠",
            MetricStatus::Critical => "✗",
        }
    }
}

/// A single metric for display
#[derive(Clone, Debug)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub threshold: Option<f64>,
    pub status: MetricStatus,
}

impl MetricValue {
    pub fn new(name: impl Into<String>, value: f64, unit: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value,
            unit: unit.into(),
            threshold: None,
            status: MetricStatus::Good,
        }
    }

    pub fn with_threshold(mut self, threshold: f64, critical_below: bool) -> Self {
        self.threshold = Some(threshold);
        self.status = if critical_below {
            if self.value < threshold {
                MetricStatus::Critical
            } else {
                MetricStatus::Good
            }
        } else {
            if self.value > threshold {
                MetricStatus::Critical
            } else {
                MetricStatus::Good
            }
        };
        self
    }
}

/// Create a metrics table from metric values
///
/// # Example
/// ```ignore
/// let metrics = vec![
///     MetricValue::new("LOLE", 8.5, "h/yr").with_threshold(10.0, false),
///     MetricValue::new("DS", 85.5, "%").with_threshold(80.0, true),
/// ];
/// let table = metrics_table(&metrics);
/// ```
pub fn metrics_table(metrics: &[MetricValue]) -> Pane {
    let mut table = TableView::new(["Metric", "Value", "Unit", "Status"]);

    for metric in metrics {
        let status_str = metric.status.as_str();
        table = table.add_row([
            metric.name.as_str(),
            &format!("{:.1}", metric.value),
            metric.unit.as_str(),
            status_str,
        ]);
    }

    Pane::new("Metrics").with_table(table)
}

/// Create a preview pane for manifest or result data
pub fn manifest_preview(title: &str, lines: &[impl AsRef<str>]) -> Pane {
    let string_lines: Vec<String> = lines.iter().map(|l| l.as_ref().to_string()).collect();
    Pane::new(title)
        .body(string_lines.as_slice())
        .with_child(Pane::new("").body(vec!["▍ Scroll: ↑↓ Page up/down"]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_info_format_size() {
        assert_eq!(FileInfo::new("a", "txt", 512).format_size(), "512 B");
        assert_eq!(FileInfo::new("a", "txt", 2048).format_size(), "2.0 KB");
        assert_eq!(FileInfo::new("a", "txt", 1048576).format_size(), "1.0 MB");
    }

    #[test]
    fn test_progress_bar() {
        let bar = progress_bar(50, 100, 50);
        assert!(bar.contains("50%"));
        assert!(bar.contains("50/100"));
    }

    #[test]
    fn test_job_status() {
        assert_eq!(JobStatus::Queued.as_str(), "Queued");
        assert_eq!(JobStatus::Running.as_str(), "Running");
        assert_eq!(JobStatus::Completed.as_str(), "✓ Done");
        assert_eq!(JobStatus::Failed.as_str(), "✗ Failed");
    }

    #[test]
    fn test_metric_status() {
        assert_eq!(MetricStatus::Good.as_str(), "✓");
        assert_eq!(MetricStatus::Warning.as_str(), "⚠");
        assert_eq!(MetricStatus::Critical.as_str(), "✗");
    }
}
