/// Shared data structures for TUI state and display.
///
/// Provides types for:
/// - Job/task tracking
/// - File metadata
/// - Metric values
/// - Configuration state
///
/// These are used across panes and the test harness for consistent data modeling.

use std::time::SystemTime;

/// Status of a background job in a queue
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
            JobStatus::Completed => "Completed",
            JobStatus::Failed => "Failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Completed | JobStatus::Failed)
    }
}

/// A background job (batch operation, scenario generation, etc.)
#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub status: JobStatus,
    pub progress_pct: f32,
    pub elapsed_secs: u64,
    pub error_msg: Option<String>,
}

impl Job {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: JobStatus::Queued,
            progress_pct: 0.0,
            elapsed_secs: 0,
            error_msg: None,
        }
    }

    pub fn with_status(mut self, status: JobStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_progress(mut self, pct: f32) -> Self {
        self.progress_pct = pct.clamp(0.0, 100.0);
        self
    }

    pub fn with_elapsed(mut self, secs: u64) -> Self {
        self.elapsed_secs = secs;
        self
    }

    pub fn with_error(mut self, msg: impl Into<String>) -> Self {
        self.error_msg = Some(msg.into());
        self
    }
}

/// File metadata for file browser displays
#[derive(Clone, Debug)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub file_type: String,
    pub size_bytes: u64,
    pub modified: SystemTime,
}

impl FileInfo {
    pub fn new(
        path: impl Into<String>,
        name: impl Into<String>,
        file_type: impl Into<String>,
        size_bytes: u64,
    ) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            file_type: file_type.into(),
            size_bytes,
            modified: SystemTime::now(),
        }
    }

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

/// Status indicator for metric values
#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn as_word(&self) -> &'static str {
        match self {
            MetricStatus::Good => "Good",
            MetricStatus::Warning => "Warning",
            MetricStatus::Critical => "Critical",
        }
    }
}

/// A single metric value with optional threshold and status
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

    /// Set a threshold and compute status. If value < threshold, status = Critical.
    pub fn with_min_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self.status = if self.value < threshold {
            MetricStatus::Critical
        } else if self.value < threshold * 1.1 {
            MetricStatus::Warning
        } else {
            MetricStatus::Good
        };
        self
    }

    /// Set a threshold and compute status. If value > threshold, status = Critical.
    pub fn with_max_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self.status = if self.value > threshold {
            MetricStatus::Critical
        } else if self.value > threshold * 0.9 {
            MetricStatus::Warning
        } else {
            MetricStatus::Good
        };
        self
    }

    /// Manually set status
    pub fn with_status(mut self, status: MetricStatus) -> Self {
        self.status = status;
        self
    }
}

/// Configuration field type for form rendering
#[derive(Clone, Debug)]
pub enum ConfigFieldType {
    Text,
    Number { min: i32, max: i32 },
    Dropdown { options: Vec<String> },
    Checkbox,
}

/// A single configuration field
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

    pub fn number(
        name: impl Into<String>,
        label: impl Into<String>,
        min: i32,
        max: i32,
    ) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            field_type: ConfigFieldType::Number { min, max },
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
            field_type: ConfigFieldType::Dropdown {
                options: options.clone(),
            },
            value: options.first().unwrap_or(&String::new()).clone(),
        }
    }

    pub fn checkbox(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            field_type: ConfigFieldType::Checkbox,
            value: "false".into(),
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn is_valid(&self) -> bool {
        match &self.field_type {
            ConfigFieldType::Text => !self.value.is_empty(),
            ConfigFieldType::Number { min, max } => {
                if let Ok(n) = self.value.parse::<i32>() {
                    n >= *min && n <= *max
                } else {
                    false
                }
            }
            ConfigFieldType::Dropdown { options } => options.contains(&self.value),
            ConfigFieldType::Checkbox => {
                matches!(self.value.as_str(), "true" | "false")
            }
        }
    }
}

/// Scenario template information
#[derive(Clone, Debug)]
pub struct ScenarioTemplate {
    pub name: String,
    pub path: String,
    pub variables: Vec<(String, String)>,
    pub description: Option<String>,
}

/// Power flow execution result
#[derive(Clone, Debug)]
pub struct PFResult {
    pub scenario_id: String,
    pub converged: bool,
    pub base_mva: f64,
    pub total_load_mw: f64,
    pub total_gen_mw: f64,
    pub loss_mw: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status() {
        assert!(!JobStatus::Running.is_terminal());
        assert!(JobStatus::Completed.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
    }

    #[test]
    fn test_metric_status() {
        assert_eq!(MetricStatus::Good.as_word(), "Good");
        assert_eq!(MetricStatus::Critical.as_word(), "Critical");
    }

    #[test]
    fn test_metric_min_threshold() {
        let m = MetricValue::new("LOLE", 12.0, "h/yr")
            .with_min_threshold(10.0);
        assert_eq!(m.status, MetricStatus::Good);

        let m = MetricValue::new("LOLE", 5.0, "h/yr")
            .with_min_threshold(10.0);
        assert_eq!(m.status, MetricStatus::Critical);
    }

    #[test]
    fn test_config_field_validation() {
        let field = ConfigField::text("name", "Name");
        assert!(!field.is_valid()); // empty

        let field = field.with_value("test");
        assert!(field.is_valid());

        let field = ConfigField::number("count", "Count", 1, 10).with_value("5");
        assert!(field.is_valid());

        let field = ConfigField::number("count", "Count", 1, 10).with_value("50");
        assert!(!field.is_valid());
    }
}
