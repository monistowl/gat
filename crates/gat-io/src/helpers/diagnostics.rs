use gat_core::Network;
use serde::Serialize;

/// Severity level for import issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning, // Unusual but imported (e.g., defaulted value)
    Error,   // Could not import element (e.g., malformed line)
}

/// A single issue encountered during import
#[derive(Debug, Clone, Serialize)]
pub struct ImportIssue {
    pub severity: Severity,
    pub category: String,       // "parse", "validation", "encoding"
    pub message: String,        // "Defaulted missing Qmax to 999.0"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,    // Line number (for -vv detailed mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>, // "Bus 14", "Branch 1-2"
}

/// Statistics about the import
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportStats {
    pub buses: usize,
    pub branches: usize,
    pub generators: usize,
    pub loads: usize,
    pub skipped_lines: usize,
    pub defaulted_values: usize,
}

/// Complete diagnostics for an import operation
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportDiagnostics {
    pub stats: ImportStats,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<ImportIssue>,
}

impl ImportDiagnostics {
    /// Create new empty diagnostics
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a warning issue
    pub fn add_warning(&mut self, category: &str, message: &str) {
        self.issues.push(ImportIssue {
            severity: Severity::Warning,
            category: category.to_string(),
            message: message.to_string(),
            line: None,
            entity: None,
        });
    }

    /// Add a warning with line number (for detailed mode)
    pub fn add_warning_at_line(&mut self, category: &str, message: &str, line: usize) {
        self.issues.push(ImportIssue {
            severity: Severity::Warning,
            category: category.to_string(),
            message: message.to_string(),
            line: Some(line),
            entity: None,
        });
        self.stats.defaulted_values += 1;
    }

    /// Add an error (skipped element)
    pub fn add_error(&mut self, category: &str, message: &str) {
        self.issues.push(ImportIssue {
            severity: Severity::Error,
            category: category.to_string(),
            message: message.to_string(),
            line: None,
            entity: None,
        });
    }

    /// Add an error with line number
    pub fn add_error_at_line(&mut self, category: &str, message: &str, line: usize) {
        self.issues.push(ImportIssue {
            severity: Severity::Error,
            category: category.to_string(),
            message: message.to_string(),
            line: Some(line),
            entity: None,
        });
        self.stats.skipped_lines += 1;
    }

    /// Add a warning with an entity reference (e.g., "Bus 14")
    pub fn add_warning_with_entity(&mut self, category: &str, message: &str, entity: &str) {
        self.issues.push(ImportIssue {
            severity: Severity::Warning,
            category: category.to_string(),
            message: message.to_string(),
            line: None,
            entity: Some(entity.to_string()),
        });
    }

    /// Add a validation warning (post-parse)
    pub fn add_validation_warning(&mut self, entity: &str, message: &str) {
        self.issues.push(ImportIssue {
            severity: Severity::Warning,
            category: "validation".to_string(),
            message: message.to_string(),
            line: None,
            entity: Some(entity.to_string()),
        });
    }

    /// Count warnings
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    /// Count errors
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    /// Check if there are any issues
    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }

    /// Merge another diagnostics into this one (for combining parse + validation)
    pub fn merge(&mut self, other: ImportDiagnostics) {
        self.issues.extend(other.issues);
        // Stats are not merged - they should be set by the parser
    }
}

/// Result of an import operation
pub struct ImportResult {
    pub network: Network,
    pub diagnostics: ImportDiagnostics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_counts() {
        let mut diag = ImportDiagnostics::new();
        diag.add_warning("parse", "test warning");
        diag.add_error("parse", "test error");
        diag.add_warning_at_line("parse", "line warning", 42);

        assert_eq!(diag.warning_count(), 2);
        assert_eq!(diag.error_count(), 1);
        assert!(diag.has_issues());
    }

    #[test]
    fn test_diagnostics_serialization() {
        let mut diag = ImportDiagnostics::new();
        diag.stats.buses = 14;
        diag.stats.branches = 20;
        diag.add_warning_at_line("parse", "Defaulted Qmax", 47);

        let json = serde_json::to_string_pretty(&diag).unwrap();
        assert!(json.contains("\"buses\": 14"));
        assert!(json.contains("\"warning\""));
        assert!(json.contains("\"line\": 47"));
    }
}
