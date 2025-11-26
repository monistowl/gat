//! Arrow directory reader with parallel file loading.
//!
//! Reads networks from the normalized multi-file Arrow format with:
//! - Manifest validation and checksum verification
//! - Parallel table loading via rayon
//! - Integrity validation on read
//! - Schema version compatibility checking

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::arrow_manifest::ArrowManifest;
use polars::io::ipc::IpcReader;
use polars::prelude::{DataFrame, SerReader};
use std::collections::HashMap;

/// Arrow directory reader with parallel loading capability
#[derive(Debug)]
pub struct ArrowDirectoryReader {
    /// Base directory path containing Arrow files
    base_path: PathBuf,
    /// Parsed manifest with metadata and checksums
    manifest: ArrowManifest,
}

impl ArrowDirectoryReader {
    /// Open and validate an Arrow network directory
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let base_path = path.as_ref().to_path_buf();

        // Step 1: Verify directory exists
        if !base_path.is_dir() {
            bail!(
                "Arrow network directory not found or is not a directory: {}",
                base_path.display()
            );
        }

        // Step 2: Load and parse manifest
        let manifest =
            Self::load_manifest(&base_path).context("loading and parsing manifest.json")?;

        // Step 3: Validate schema version compatibility
        manifest
            .is_compatible()
            .context("checking schema compatibility")?;

        // Step 4: Verify all required tables are present
        manifest
            .verify_all_tables()
            .context("verifying required tables")?;

        // Step 5: Validate checksums of all table files
        manifest
            .validate_checksums(&base_path)
            .context("validating file checksums")?;

        Ok(Self {
            base_path,
            manifest,
        })
    }

    /// Load and parse manifest from directory
    fn load_manifest(base_path: &Path) -> Result<ArrowManifest> {
        let manifest_path = base_path.join("manifest.json");

        if !manifest_path.exists() {
            bail!(
                "manifest.json not found in {}\n\
                 This directory may be incomplete or corrupted (incomplete write)",
                base_path.display()
            );
        }

        let file = File::open(&manifest_path)
            .with_context(|| format!("opening manifest: {}", manifest_path.display()))?;

        serde_json::from_reader(file).context("parsing manifest.json")
    }

    /// Get reference to the loaded manifest
    pub fn manifest(&self) -> &ArrowManifest {
        &self.manifest
    }

    /// Get the base directory path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get path to a specific table file
    pub fn table_path(&self, table_name: &str) -> PathBuf {
        self.base_path.join(format!("{}.arrow", table_name))
    }

    /// Check if a table file exists and is accessible
    pub fn has_table(&self, table_name: &str) -> bool {
        self.table_path(table_name).exists()
    }

    /// List all available table names
    pub fn available_tables(&self) -> Vec<&str> {
        self.manifest.tables.keys().map(|s| s.as_str()).collect()
    }

    /// Get table metadata
    pub fn table_info(&self, table_name: &str) -> Option<&crate::arrow_manifest::TableInfo> {
        self.manifest.tables.get(table_name)
    }

    /// Load all tables into DataFrames (parallel)
    pub fn load_tables(&self) -> Result<HashMap<String, DataFrame>> {
        let base = self.base_path.clone();
        let tables: Result<Vec<(String, DataFrame)>> = self
            .manifest
            .tables
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|name| {
                let path = base.join(format!("{}.arrow", name));
                let file = File::open(&path)
                    .with_context(|| format!("opening table file {}", path.display()))?;
                let df = IpcReader::new(file)
                    .finish()
                    .with_context(|| format!("reading table {}", name))?;
                Ok((name, df))
            })
            .collect();

        tables.map(|vec| vec.into_iter().collect())
    }
}

/// Open an Arrow network directory and validate it
pub fn open_arrow_directory(path: impl AsRef<Path>) -> Result<ArrowDirectoryReader> {
    ArrowDirectoryReader::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_manifest::{ArrowManifest, TableInfo};
    use tempfile::TempDir;

    fn create_test_directory_with_manifest() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_network");
        std::fs::create_dir(&dir_path).unwrap();

        // Create a valid manifest
        let mut manifest = ArrowManifest::new("0.4.0".to_string(), None);

        // Add all required tables with correct hashes for empty files
        let empty_file_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        for table_name in ArrowManifest::required_tables() {
            manifest.add_table(
                *table_name,
                TableInfo {
                    sha256: empty_file_hash.to_string(),
                    row_count: 0,
                    file_size_bytes: 0,
                },
            );
        }

        // Write manifest
        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        std::fs::write(dir_path.join("manifest.json"), manifest_json).unwrap();

        // Create dummy table files (empty)
        for table_name in ArrowManifest::required_tables() {
            std::fs::write(dir_path.join(format!("{}.arrow", table_name)), &[]).unwrap();
        }

        (temp_dir, dir_path)
    }

    #[test]
    fn test_open_valid_directory() {
        let (_temp_dir, dir_path) = create_test_directory_with_manifest();
        let reader = ArrowDirectoryReader::open(&dir_path);
        assert!(reader.is_ok());
    }

    #[test]
    fn test_missing_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("no_manifest");
        std::fs::create_dir(&dir_path).unwrap();

        let result = ArrowDirectoryReader::open(&dir_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("manifest.json"));
    }

    #[test]
    fn test_directory_not_found() {
        let result = ArrowDirectoryReader::open("/nonexistent/path/network");
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_access() {
        let (_temp_dir, dir_path) = create_test_directory_with_manifest();
        let reader = ArrowDirectoryReader::open(&dir_path).unwrap();

        assert_eq!(reader.manifest().gat_version, "0.4.0");
        assert_eq!(reader.manifest().schema_version, "2.0.0");
    }

    #[test]
    fn test_base_path() {
        let (_temp_dir, dir_path) = create_test_directory_with_manifest();
        let reader = ArrowDirectoryReader::open(&dir_path).unwrap();

        assert_eq!(reader.base_path(), dir_path);
    }

    #[test]
    fn test_table_path() {
        let (_temp_dir, dir_path) = create_test_directory_with_manifest();
        let reader = ArrowDirectoryReader::open(&dir_path).unwrap();

        let buses_path = reader.table_path("buses");
        assert!(buses_path.ends_with("buses.arrow"));
    }

    #[test]
    fn test_has_table() {
        let (_temp_dir, dir_path) = create_test_directory_with_manifest();
        let reader = ArrowDirectoryReader::open(&dir_path).unwrap();

        assert!(reader.has_table("buses"));
        assert!(reader.has_table("generators"));
        assert!(!reader.has_table("nonexistent"));
    }

    #[test]
    fn test_available_tables() {
        let (_temp_dir, dir_path) = create_test_directory_with_manifest();
        let reader = ArrowDirectoryReader::open(&dir_path).unwrap();

        let tables = reader.available_tables();
        assert_eq!(tables.len(), 5);
        assert!(tables.contains(&"buses"));
        assert!(tables.contains(&"generators"));
    }

    #[test]
    fn test_table_info() {
        let (_temp_dir, dir_path) = create_test_directory_with_manifest();
        let reader = ArrowDirectoryReader::open(&dir_path).unwrap();

        let info = reader.table_info("buses");
        assert!(info.is_some());
        assert_eq!(info.unwrap().row_count, 0);
    }

    #[test]
    fn test_missing_required_table() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("incomplete_network");
        std::fs::create_dir(&dir_path).unwrap();

        // Create manifest but with missing table
        let manifest = ArrowManifest::new("0.4.0".to_string(), None);
        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        std::fs::write(dir_path.join("manifest.json"), manifest_json).unwrap();

        let result = ArrowDirectoryReader::open(&dir_path);
        assert!(result.is_err());
    }
}
