//! Manifest schema for Arrow network datasets with version tracking and integrity validation.
//!
//! Each Arrow network directory contains a `manifest.json` file that:
//! - Tracks schema version for migration support
//! - Stores SHA256 checksums of all table files
//! - Records source file provenance and GAT version
//! - Enables format compatibility checking

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Current schema version (semver)
pub const CURRENT_SCHEMA_VERSION: &str = "2.0.0";

/// Complete manifest for an Arrow network dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowManifest {
    /// Schema version for migration support (e.g., "2.0.0")
    pub schema_version: String,

    /// Timestamp when dataset was created
    pub created_at: DateTime<Utc>,

    /// GAT version that created this dataset
    pub gat_version: String,

    /// Optional source file information (for provenance)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceInfo>,

    /// Metadata for each table file
    pub tables: HashMap<String, TableInfo>,
}

/// Information about the source file that was imported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Source file name (e.g., "case14.m")
    pub file: String,

    /// Source format (e.g., "matpower")
    pub format: String,

    /// SHA256 hash of original file
    pub file_hash: String,
}

/// Metadata for a single Arrow table file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// SHA256 checksum of the .arrow file
    pub sha256: String,

    /// Number of rows in the table
    pub row_count: u64,

    /// File size in bytes
    pub file_size_bytes: u64,
}

impl ArrowManifest {
    /// Create a new manifest for the current schema version
    pub fn new(gat_version: String, source: Option<SourceInfo>) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            created_at: Utc::now(),
            gat_version,
            source,
            tables: HashMap::new(),
        }
    }

    /// Add table metadata to the manifest
    pub fn add_table(&mut self, name: impl Into<String>, info: TableInfo) {
        self.tables.insert(name.into(), info);
    }

    /// Check if this manifest's schema version is compatible with current code
    pub fn is_compatible(&self) -> Result<()> {
        use semver::Version;

        let manifest_version = Version::parse(&self.schema_version)
            .map_err(|e| anyhow::anyhow!("Invalid schema version in manifest: {}", e))?;

        let current_version = Version::parse(CURRENT_SCHEMA_VERSION)?;

        // Allow reading same major version (e.g., 2.0.0 can read 2.1.0)
        // But reject if manifest is from newer major version
        if manifest_version.major > current_version.major {
            bail!(
                "Schema v{} is too new (this version supports up to v{})",
                self.schema_version,
                CURRENT_SCHEMA_VERSION
            );
        }

        Ok(())
    }

    /// Validate checksums of all table files
    pub fn validate_checksums(&self, base_path: &Path) -> Result<()> {
        for (table_name, info) in &self.tables {
            let file_path = base_path.join(format!("{}.arrow", table_name));

            if !file_path.exists() {
                bail!("Table file not found: {}", file_path.display());
            }

            let actual_hash = compute_sha256(&file_path)?;

            if actual_hash != info.sha256 {
                bail!(
                    "Checksum mismatch for table '{}': expected {}, got {}",
                    table_name,
                    info.sha256,
                    actual_hash
                );
            }
        }

        Ok(())
    }

    /// Get all required tables
    pub fn required_tables() -> &'static [&'static str] {
        &["system", "buses", "generators", "loads", "branches"]
    }

    /// Verify all required tables are present
    pub fn verify_all_tables(&self) -> Result<()> {
        for required in Self::required_tables() {
            if !self.tables.contains_key(*required) {
                bail!("Missing required table: {}", required);
            }
        }
        Ok(())
    }
}

/// Compute SHA256 hash of a file
pub fn compute_sha256(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path).map_err(|e| {
        anyhow::anyhow!("Failed to open file for hashing {}: {}", path.display(), e)
    })?;

    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let n = file.read(&mut buffer).map_err(|e| {
            anyhow::anyhow!("Failed to read file for hashing {}: {}", path.display(), e)
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_creation() {
        let manifest = ArrowManifest::new(env!("CARGO_PKG_VERSION").to_string(), None);

        assert_eq!(manifest.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(manifest.gat_version, env!("CARGO_PKG_VERSION"));
        assert!(manifest.tables.is_empty());
    }

    #[test]
    fn test_manifest_add_table() {
        let mut manifest = ArrowManifest::new("0.4.0".to_string(), None);

        manifest.add_table(
            "buses",
            TableInfo {
                sha256: "abc123".to_string(),
                row_count: 14,
                file_size_bytes: 1024,
            },
        );

        assert_eq!(manifest.tables.len(), 1);
        assert!(manifest.tables.contains_key("buses"));
    }

    #[test]
    fn test_version_compatibility_same() {
        let manifest = ArrowManifest::new("0.4.0".to_string(), None);
        assert!(manifest.is_compatible().is_ok());
    }

    #[test]
    fn test_version_compatibility_minor_older() {
        let mut manifest = ArrowManifest::new("0.4.0".to_string(), None);
        manifest.schema_version = "2.0.0".to_string();
        assert!(manifest.is_compatible().is_ok());
    }

    #[test]
    fn test_version_compatibility_major_newer() {
        let mut manifest = ArrowManifest::new("0.4.0".to_string(), None);
        manifest.schema_version = "3.0.0".to_string();
        assert!(manifest.is_compatible().is_err());
    }

    #[test]
    fn test_required_tables_all_present() {
        let mut manifest = ArrowManifest::new("0.4.0".to_string(), None);

        for table_name in ArrowManifest::required_tables() {
            manifest.add_table(
                *table_name,
                TableInfo {
                    sha256: "hash".to_string(),
                    row_count: 1,
                    file_size_bytes: 100,
                },
            );
        }

        assert!(manifest.verify_all_tables().is_ok());
    }

    #[test]
    fn test_required_tables_missing() {
        let manifest = ArrowManifest::new("0.4.0".to_string(), None);
        assert!(manifest.verify_all_tables().is_err());
    }

    #[test]
    fn test_source_info_serialization() {
        let source = SourceInfo {
            file: "case14.m".to_string(),
            format: "matpower".to_string(),
            file_hash: "def456".to_string(),
        };

        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("case14.m"));
        assert!(json.contains("matpower"));
    }

    #[test]
    fn test_manifest_serialization() {
        let mut manifest = ArrowManifest::new("0.4.0".to_string(), None);

        manifest.add_table(
            "buses",
            TableInfo {
                sha256: "abc123".to_string(),
                row_count: 14,
                file_size_bytes: 1024,
            },
        );

        let json = serde_json::to_string_pretty(&manifest).unwrap();

        assert!(json.contains("2.0.0"));
        assert!(json.contains("buses"));
        assert!(json.contains("abc123"));

        // Should deserialize back
        let restored: ArrowManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.schema_version, "2.0.0");
        assert_eq!(restored.tables.len(), 1);
    }
}
