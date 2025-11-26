//! Re-exports from gat-core diagnostics plus gat-io specific types.
//!
//! The core diagnostic types (`Severity`, `DiagnosticIssue`, `Diagnostics`,
//! `ImportDiagnostics`, `ImportStats`) are defined in `gat_core::diagnostics`
//! and re-exported here for convenience.
//!
//! This module adds `ImportResult`, which bundles a `Network` with its
//! diagnostics - something specific to gat-io's import operations.

use gat_core::Network;

// Re-export core diagnostics types for backwards compatibility
pub use gat_core::{DiagnosticIssue as ImportIssue, ImportDiagnostics, ImportStats, Severity};

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
