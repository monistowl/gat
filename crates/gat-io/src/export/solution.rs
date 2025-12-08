//! Uniform solution export trait for OPF and other solver solutions

use anyhow::Result;
use std::path::Path;

/// Trait for exporting solver solutions to various formats
pub trait SolutionExport {
    /// Export to Parquet format
    #[cfg(feature = "parquet")]
    fn to_parquet(&self, path: &Path) -> Result<()>;

    /// Export to JSON format
    fn to_json(&self, path: &Path) -> Result<()>;

    /// Export to CSV format
    #[cfg(feature = "native-io")]
    fn to_csv(&self, path: &Path) -> Result<()>;

    /// Convert to JSON value (for streaming/stdout)
    fn to_json_value(&self) -> Result<serde_json::Value>;
}

// Implementation note: The actual implementation for gat_algo::opf::OpfSolution
// is in gat-algo crate at src/opf/export.rs to avoid circular dependencies.
// Tests are also located there.
