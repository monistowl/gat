//! Unified diagnostics infrastructure for tracking issues during operations.
//!
//! This module provides a common interface for collecting warnings and errors
//! during imports, validation, transformations, and other operations. It supports:
//!
//! - Severity levels (Warning, Error)
//! - Categories for grouping issues (parse, validation, physical, etc.)
//! - Optional entity references (e.g., "Bus 14", "Branch 1-2")
//! - Optional line numbers for file-based operations
//! - Serialization for JSON output
//!
//! # Example
//!
//! ```
//! use gat_core::diagnostics::{Diagnostics, Severity};
//!
//! let mut diag = Diagnostics::new();
//!
//! // Add a validation warning
//! diag.add_warning("validation", "Network has no loads");
//!
//! // Add an error with entity reference
//! diag.add_error_with_entity("reference", "Bus references non-existent node", "Gen 1");
//!
//! // Check results
//! assert_eq!(diag.warning_count(), 1);
//! assert_eq!(diag.error_count(), 1);
//! ```

use serde::Serialize;

/// Severity level for diagnostic issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Unusual but operation continued (e.g., defaulted value)
    Warning,
    /// Could not complete element/operation (e.g., malformed data)
    Error,
}

/// A single diagnostic issue encountered during an operation
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticIssue {
    /// Severity of the issue
    pub severity: Severity,
    /// Category for grouping (e.g., "parse", "validation", "physical", "reference")
    pub category: String,
    /// Human-readable description of the issue
    pub message: String,
    /// Optional line number (for file-based operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Optional entity reference (e.g., "Bus 14", "Branch 1-2")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
}

impl DiagnosticIssue {
    /// Create a new diagnostic issue
    pub fn new(
        severity: Severity,
        category: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category: category.into(),
            message: message.into(),
            line: None,
            entity: None,
        }
    }

    /// Add line number to the issue
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Add entity reference to the issue
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.entity = Some(entity.into());
        self
    }
}

impl std::fmt::Display for DiagnosticIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };

        write!(f, "[{}:{}] {}", severity, self.category, self.message)?;

        if let Some(entity) = &self.entity {
            write!(f, " ({})", entity)?;
        }
        if let Some(line) = self.line {
            write!(f, " at line {}", line)?;
        }

        Ok(())
    }
}

/// Collection of diagnostic issues for an operation
///
/// This is the primary container for tracking warnings and errors during
/// imports, validation, and other operations. It provides methods for
/// adding issues with various levels of detail.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Diagnostics {
    /// All collected issues
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<DiagnosticIssue>,
}

impl Diagnostics {
    /// Create new empty diagnostics
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a raw issue directly
    pub fn add(&mut self, issue: DiagnosticIssue) {
        self.issues.push(issue);
    }

    // =========================================================================
    // Warning Methods
    // =========================================================================

    /// Add a warning with category and message
    pub fn add_warning(&mut self, category: &str, message: &str) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Warning, category, message));
    }

    /// Add a warning with line number
    pub fn add_warning_at_line(&mut self, category: &str, message: &str, line: usize) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Warning, category, message).with_line(line));
    }

    /// Add a warning with entity reference
    pub fn add_warning_with_entity(&mut self, category: &str, message: &str, entity: &str) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Warning, category, message).with_entity(entity));
    }

    /// Add a validation warning (convenience method with "validation" category)
    pub fn add_validation_warning(&mut self, entity: &str, message: &str) {
        self.issues.push(
            DiagnosticIssue::new(Severity::Warning, "validation", message).with_entity(entity),
        );
    }

    // =========================================================================
    // Error Methods
    // =========================================================================

    /// Add an error with category and message
    pub fn add_error(&mut self, category: &str, message: &str) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Error, category, message));
    }

    /// Add an error with line number
    pub fn add_error_at_line(&mut self, category: &str, message: &str, line: usize) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Error, category, message).with_line(line));
    }

    /// Add an error with entity reference
    pub fn add_error_with_entity(&mut self, category: &str, message: &str, entity: &str) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Error, category, message).with_entity(entity));
    }

    // =========================================================================
    // Query Methods
    // =========================================================================

    /// Count warning issues
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    /// Count error issues
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

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Warning)
    }

    /// Get issues filtered by category
    pub fn issues_by_category<'a>(
        &'a self,
        category: &'a str,
    ) -> impl Iterator<Item = &'a DiagnosticIssue> {
        self.issues.iter().filter(move |i| i.category == category)
    }

    /// Get only error issues
    pub fn errors(&self) -> impl Iterator<Item = &DiagnosticIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error)
    }

    /// Get only warning issues
    pub fn warnings(&self) -> impl Iterator<Item = &DiagnosticIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Merge another diagnostics into this one
    pub fn merge(&mut self, other: Diagnostics) {
        self.issues.extend(other.issues);
    }

    /// Clear all issues
    pub fn clear(&mut self) {
        self.issues.clear();
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        let warnings = self.warning_count();
        let errors = self.error_count();

        match (warnings, errors) {
            (0, 0) => "No issues".to_string(),
            (w, 0) => format!("{} warning{}", w, if w == 1 { "" } else { "s" }),
            (0, e) => format!("{} error{}", e, if e == 1 { "" } else { "s" }),
            (w, e) => format!(
                "{} warning{}, {} error{}",
                w,
                if w == 1 { "" } else { "s" },
                e,
                if e == 1 { "" } else { "s" }
            ),
        }
    }
}

impl std::fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Diagnostics: {}", self.summary())?;
        for issue in &self.issues {
            writeln!(f, "  {}", issue)?;
        }
        Ok(())
    }
}

// ============================================================================
// Import-Specific Extensions
// ============================================================================

/// Statistics about an import operation
///
/// This struct tracks counts of imported elements and issues encountered
/// during file parsing. It is kept separate from `Diagnostics` since it
/// contains import-specific counters.
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
///
/// Combines import statistics with diagnostic issues. This is the
/// primary return type for importer functions.
///
/// This struct provides direct field access to `stats` and `issues`
/// for backwards compatibility with existing code.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportDiagnostics {
    /// Element counts and import statistics
    pub stats: ImportStats,
    /// All collected issues (warnings and errors)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<DiagnosticIssue>,
}

impl ImportDiagnostics {
    /// Create new empty import diagnostics
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a warning
    pub fn add_warning(&mut self, category: &str, message: &str) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Warning, category, message));
    }

    /// Add a warning with line number (increments defaulted_values counter)
    pub fn add_warning_at_line(&mut self, category: &str, message: &str, line: usize) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Warning, category, message).with_line(line));
        self.stats.defaulted_values += 1;
    }

    /// Add a warning with entity reference
    pub fn add_warning_with_entity(&mut self, category: &str, message: &str, entity: &str) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Warning, category, message).with_entity(entity));
    }

    /// Add a validation warning
    pub fn add_validation_warning(&mut self, entity: &str, message: &str) {
        self.issues.push(
            DiagnosticIssue::new(Severity::Warning, "validation", message).with_entity(entity),
        );
    }

    /// Add an error
    pub fn add_error(&mut self, category: &str, message: &str) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Error, category, message));
    }

    /// Add an error with line number (increments skipped_lines counter)
    pub fn add_error_at_line(&mut self, category: &str, message: &str, line: usize) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Error, category, message).with_line(line));
        self.stats.skipped_lines += 1;
    }

    /// Add an error with entity reference
    pub fn add_error_with_entity(&mut self, category: &str, message: &str, entity: &str) {
        self.issues
            .push(DiagnosticIssue::new(Severity::Error, category, message).with_entity(entity));
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

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    /// Merge another import diagnostics into this one
    pub fn merge(&mut self, other: ImportDiagnostics) {
        self.issues.extend(other.issues);
        // Note: stats are not merged - they should be set by the parser
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        let warnings = self.warning_count();
        let errors = self.error_count();
        let issue_summary = match (warnings, errors) {
            (0, 0) => "No issues".to_string(),
            (w, 0) => format!("{} warning{}", w, if w == 1 { "" } else { "s" }),
            (0, e) => format!("{} error{}", e, if e == 1 { "" } else { "s" }),
            (w, e) => format!(
                "{} warning{}, {} error{}",
                w,
                if w == 1 { "" } else { "s" },
                e,
                if e == 1 { "" } else { "s" }
            ),
        };

        format!(
            "{} buses, {} branches, {} gens, {} loads | {}",
            self.stats.buses,
            self.stats.branches,
            self.stats.generators,
            self.stats.loads,
            issue_summary
        )
    }
}

impl std::fmt::Display for ImportDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Import: {}", self.summary())?;
        if self.has_issues() {
            for issue in &self.issues {
                writeln!(f, "  {}", issue)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_counts() {
        let mut diag = Diagnostics::new();
        diag.add_warning("parse", "test warning");
        diag.add_error("parse", "test error");
        diag.add_warning_at_line("parse", "line warning", 42);

        assert_eq!(diag.warning_count(), 2);
        assert_eq!(diag.error_count(), 1);
        assert!(diag.has_issues());
        assert!(diag.has_errors());
        assert!(diag.has_warnings());
    }

    #[test]
    fn test_diagnostics_serialization() {
        let mut diag = Diagnostics::new();
        diag.add_warning_at_line("parse", "Defaulted Qmax", 47);
        diag.add_error_with_entity("reference", "Invalid bus", "Gen 1");

        let json = serde_json::to_string_pretty(&diag).unwrap();
        assert!(json.contains("\"warning\""));
        assert!(json.contains("\"line\": 47"));
        assert!(json.contains("\"entity\": \"Gen 1\""));
    }

    #[test]
    fn test_diagnostic_issue_display() {
        let issue = DiagnosticIssue::new(Severity::Error, "validation", "Invalid value")
            .with_entity("Bus 14")
            .with_line(42);

        let display = format!("{}", issue);
        assert!(display.contains("error"));
        assert!(display.contains("validation"));
        assert!(display.contains("Bus 14"));
        assert!(display.contains("line 42"));
    }

    #[test]
    fn test_diagnostics_summary() {
        let mut diag = Diagnostics::new();
        assert_eq!(diag.summary(), "No issues");

        diag.add_warning("parse", "warning");
        assert_eq!(diag.summary(), "1 warning");

        diag.add_error("parse", "error");
        assert_eq!(diag.summary(), "1 warning, 1 error");

        diag.add_warning("parse", "another warning");
        assert_eq!(diag.summary(), "2 warnings, 1 error");
    }

    #[test]
    fn test_issues_by_category() {
        let mut diag = Diagnostics::new();
        diag.add_warning("parse", "parse warning");
        diag.add_warning("validation", "validation warning");
        diag.add_error("parse", "parse error");

        let parse_issues: Vec<_> = diag.issues_by_category("parse").collect();
        assert_eq!(parse_issues.len(), 2);

        let validation_issues: Vec<_> = diag.issues_by_category("validation").collect();
        assert_eq!(validation_issues.len(), 1);
    }

    #[test]
    fn test_diagnostics_merge() {
        let mut diag1 = Diagnostics::new();
        diag1.add_warning("parse", "warning 1");

        let mut diag2 = Diagnostics::new();
        diag2.add_error("parse", "error 1");

        diag1.merge(diag2);
        assert_eq!(diag1.warning_count(), 1);
        assert_eq!(diag1.error_count(), 1);
    }

    #[test]
    fn test_import_diagnostics() {
        let mut diag = ImportDiagnostics::new();
        diag.stats.buses = 14;
        diag.stats.branches = 20;
        diag.stats.generators = 5;
        diag.stats.loads = 11;

        diag.add_warning_at_line("parse", "Defaulted Qmax", 47);
        diag.add_error_at_line("parse", "Invalid line", 52);

        assert_eq!(diag.warning_count(), 1);
        assert_eq!(diag.error_count(), 1);
        assert_eq!(diag.stats.defaulted_values, 1);
        assert_eq!(diag.stats.skipped_lines, 1);

        let summary = diag.summary();
        assert!(summary.contains("14 buses"));
        assert!(summary.contains("1 warning"));
    }

    #[test]
    fn test_import_diagnostics_serialization() {
        let mut diag = ImportDiagnostics::new();
        diag.stats.buses = 14;
        diag.add_warning("parse", "test warning");

        let json = serde_json::to_string_pretty(&diag).unwrap();
        assert!(json.contains("\"buses\": 14"));
        assert!(json.contains("\"warning\""));
    }
}
