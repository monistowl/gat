//! Path validation utilities for secure file operations
//!
//! This module provides defense-in-depth against path traversal attacks,
//! symlink-based attacks, and resource exhaustion via large files.
//!
//! # Security Model
//!
//! - **Path traversal prevention**: Validates that resolved paths stay within allowed directories
//! - **Extension allowlisting**: Only permits known-safe file extensions
//! - **Size limits**: Prevents resource exhaustion from extremely large files
//! - **Symlink handling**: Resolves symlinks and validates the final target
//!
//! # Usage
//!
//! ```ignore
//! use gat_io::helpers::path_security::{SecurePath, PathValidator};
//!
//! let validator = PathValidator::new()
//!     .allow_extensions(&["m", "json", "xml", "rdf", "zip", "arrow"])
//!     .max_file_size(100 * 1024 * 1024);  // 100 MB
//!
//! let secure_path = validator.validate(user_path, &allowed_base_dir)?;
//! let content = secure_path.read_to_string()?;
//! ```

use anyhow::{anyhow, Context, Result};
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

/// Default maximum file size (100 MB)
pub const DEFAULT_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Allowed file extensions for grid data imports
pub const GRID_EXTENSIONS: &[&str] = &[
    "m",       // MATPOWER
    "json",    // pandapower, JSON configs
    "xml",     // CIM RDF/XML
    "rdf",     // CIM RDF
    "zip",     // Archives
    "arrow",   // Arrow/Parquet
    "parquet", // Parquet files
    "csv",     // CSV data
    "raw",     // PSS/E RAW format
    "dyr",     // PSS/E dynamics
];

/// Error types for path validation
#[derive(Debug, Clone)]
pub enum PathSecurityError {
    /// Path contains traversal sequences (../)
    PathTraversal(String),
    /// Path resolves outside allowed directory
    EscapedAllowedDir { path: PathBuf, allowed: PathBuf },
    /// File extension not in allowlist
    DisallowedExtension {
        extension: String,
        allowed: Vec<String>,
    },
    /// File exceeds size limit
    FileTooLarge { size: u64, max: u64 },
    /// Path does not exist
    NotFound(PathBuf),
    /// Path is not a file (e.g., directory when file expected)
    NotAFile(PathBuf),
    /// Path is not a directory (when directory expected)
    NotADirectory(PathBuf),
    /// Symlink resolution failed
    SymlinkError(String),
    /// IO error during validation
    IoError(String),
}

impl std::fmt::Display for PathSecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathSecurityError::PathTraversal(path) => {
                write!(f, "Path traversal detected in: {}", path)
            }
            PathSecurityError::EscapedAllowedDir { path, allowed } => {
                write!(
                    f,
                    "Path '{}' resolves outside allowed directory '{}'",
                    path.display(),
                    allowed.display()
                )
            }
            PathSecurityError::DisallowedExtension { extension, allowed } => {
                write!(
                    f,
                    "Extension '{}' not allowed. Permitted: {:?}",
                    extension, allowed
                )
            }
            PathSecurityError::FileTooLarge { size, max } => {
                write!(f, "File size {} bytes exceeds maximum {} bytes", size, max)
            }
            PathSecurityError::NotFound(path) => {
                write!(f, "Path not found: {}", path.display())
            }
            PathSecurityError::NotAFile(path) => {
                write!(f, "Path is not a file: {}", path.display())
            }
            PathSecurityError::NotADirectory(path) => {
                write!(f, "Path is not a directory: {}", path.display())
            }
            PathSecurityError::SymlinkError(msg) => {
                write!(f, "Symlink resolution error: {}", msg)
            }
            PathSecurityError::IoError(msg) => {
                write!(f, "IO error during path validation: {}", msg)
            }
        }
    }
}

impl std::error::Error for PathSecurityError {}

/// A validated, secure path that is guaranteed to be within allowed bounds
#[derive(Debug, Clone)]
pub struct SecurePath {
    /// The canonicalized (resolved) path
    canonical: PathBuf,
    /// Original path provided by user (for error messages)
    original: PathBuf,
    /// Metadata cached during validation
    metadata: Option<SecurePathMetadata>,
}

#[derive(Debug, Clone)]
struct SecurePathMetadata {
    size: u64,
    is_file: bool,
    is_dir: bool,
}

impl SecurePath {
    /// Get the canonical (resolved) path
    pub fn path(&self) -> &Path {
        &self.canonical
    }

    /// Get the original path provided by user
    pub fn original(&self) -> &Path {
        &self.original
    }

    /// Read the file contents as a string
    pub fn read_to_string(&self) -> Result<String> {
        fs::read_to_string(&self.canonical)
            .with_context(|| format!("reading file '{}'", self.canonical.display()))
    }

    /// Open the file for reading
    pub fn open(&self) -> Result<File> {
        File::open(&self.canonical)
            .with_context(|| format!("opening file '{}'", self.canonical.display()))
    }

    /// Read directory entries (if this is a directory)
    pub fn read_dir(&self) -> Result<fs::ReadDir> {
        fs::read_dir(&self.canonical)
            .with_context(|| format!("reading directory '{}'", self.canonical.display()))
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.metadata.as_ref().map(|m| m.is_file).unwrap_or(false)
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.metadata.as_ref().map(|m| m.is_dir).unwrap_or(false)
    }

    /// Get file size in bytes
    pub fn size(&self) -> u64 {
        self.metadata.as_ref().map(|m| m.size).unwrap_or(0)
    }
}

/// Path validator with configurable security policies
#[derive(Debug, Clone)]
pub struct PathValidator {
    /// Allowed file extensions (lowercase, without dot)
    allowed_extensions: HashSet<String>,
    /// Maximum file size in bytes (None = no limit)
    max_file_size: Option<u64>,
    /// Whether to allow directories
    allow_directories: bool,
    /// Whether to follow symlinks (if false, symlinks are rejected)
    follow_symlinks: bool,
}

impl Default for PathValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PathValidator {
    /// Create a new PathValidator with default settings
    pub fn new() -> Self {
        Self {
            allowed_extensions: HashSet::new(),
            max_file_size: Some(DEFAULT_MAX_FILE_SIZE),
            allow_directories: true,
            follow_symlinks: true,
        }
    }

    /// Create a PathValidator configured for grid data imports
    pub fn for_grid_imports() -> Self {
        Self::new()
            .allow_extensions(GRID_EXTENSIONS)
            .max_file_size(DEFAULT_MAX_FILE_SIZE)
    }

    /// Add allowed file extensions
    pub fn allow_extensions(mut self, extensions: &[&str]) -> Self {
        for ext in extensions {
            self.allowed_extensions.insert(ext.to_lowercase());
        }
        self
    }

    /// Set maximum file size (in bytes)
    pub fn max_file_size(mut self, max: u64) -> Self {
        self.max_file_size = Some(max);
        self
    }

    /// Remove file size limit
    pub fn no_size_limit(mut self) -> Self {
        self.max_file_size = None;
        self
    }

    /// Allow or disallow directories
    pub fn allow_directories(mut self, allow: bool) -> Self {
        self.allow_directories = allow;
        self
    }

    /// Allow or disallow following symlinks
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Validate a path without requiring a base directory constraint
    ///
    /// This performs basic security checks but does not enforce that the path
    /// stays within a specific directory. Use `validate_within` for stricter security.
    pub fn validate(&self, path: &Path) -> Result<SecurePath, PathSecurityError> {
        // Check for obvious path traversal in the input
        self.check_traversal_patterns(path)?;

        // Resolve the path (follows symlinks if enabled)
        let canonical = self.resolve_path(path)?;

        // Get metadata
        let metadata = self.get_metadata(&canonical)?;

        // Validate file vs directory
        if metadata.is_dir && !self.allow_directories {
            return Err(PathSecurityError::NotAFile(path.to_path_buf()));
        }

        // Validate extension for files
        if metadata.is_file && !self.allowed_extensions.is_empty() {
            self.check_extension(&canonical)?;
        }

        // Validate size for files
        if metadata.is_file {
            if let Some(max) = self.max_file_size {
                if metadata.size > max {
                    return Err(PathSecurityError::FileTooLarge {
                        size: metadata.size,
                        max,
                    });
                }
            }
        }

        Ok(SecurePath {
            canonical,
            original: path.to_path_buf(),
            metadata: Some(metadata),
        })
    }

    /// Validate a path ensuring it stays within an allowed base directory
    ///
    /// This is the most secure option - it ensures the resolved path cannot
    /// escape the specified base directory through traversal or symlinks.
    pub fn validate_within(
        &self,
        path: &Path,
        allowed_base: &Path,
    ) -> Result<SecurePath, PathSecurityError> {
        // First do basic validation
        let secure = self.validate(path)?;

        // Resolve the base directory
        let canonical_base = allowed_base.canonicalize().map_err(|e| {
            PathSecurityError::IoError(format!(
                "Failed to resolve base directory '{}': {}",
                allowed_base.display(),
                e
            ))
        })?;

        // Ensure the path is within the base directory
        if !secure.canonical.starts_with(&canonical_base) {
            return Err(PathSecurityError::EscapedAllowedDir {
                path: path.to_path_buf(),
                allowed: allowed_base.to_path_buf(),
            });
        }

        Ok(secure)
    }

    /// Check for path traversal patterns in the raw path string
    fn check_traversal_patterns(&self, path: &Path) -> Result<(), PathSecurityError> {
        // Check each component
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    return Err(PathSecurityError::PathTraversal(path.display().to_string()));
                }
                Component::Normal(s) => {
                    // Check for null bytes or other dangerous characters
                    if let Some(s) = s.to_str() {
                        if s.contains('\0') {
                            return Err(PathSecurityError::PathTraversal(format!(
                                "Null byte in path component: {}",
                                path.display()
                            )));
                        }
                    }
                }
                _ => {}
            }
        }

        // Also check the string representation for encoded traversal
        if let Some(path_str) = path.to_str() {
            // URL-encoded traversal
            if path_str.contains("%2e%2e") || path_str.contains("%2E%2E") {
                return Err(PathSecurityError::PathTraversal(format!(
                    "URL-encoded traversal in path: {}",
                    path_str
                )));
            }
            // Double-encoded
            if path_str.contains("%252e") || path_str.contains("%252E") {
                return Err(PathSecurityError::PathTraversal(format!(
                    "Double-encoded traversal in path: {}",
                    path_str
                )));
            }
        }

        Ok(())
    }

    /// Resolve a path to its canonical form
    fn resolve_path(&self, path: &Path) -> Result<PathBuf, PathSecurityError> {
        if !self.follow_symlinks && path.is_symlink() {
            return Err(PathSecurityError::SymlinkError(format!(
                "Symlinks not allowed: {}",
                path.display()
            )));
        }

        path.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PathSecurityError::NotFound(path.to_path_buf())
            } else {
                PathSecurityError::IoError(format!(
                    "Failed to resolve path '{}': {}",
                    path.display(),
                    e
                ))
            }
        })
    }

    /// Get and validate metadata
    fn get_metadata(&self, path: &Path) -> Result<SecurePathMetadata, PathSecurityError> {
        let metadata = fs::metadata(path).map_err(|e| {
            PathSecurityError::IoError(format!(
                "Failed to read metadata for '{}': {}",
                path.display(),
                e
            ))
        })?;

        Ok(SecurePathMetadata {
            size: metadata.len(),
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
        })
    }

    /// Check file extension against allowlist
    fn check_extension(&self, path: &Path) -> Result<(), PathSecurityError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !self.allowed_extensions.contains(&ext) {
            return Err(PathSecurityError::DisallowedExtension {
                extension: ext,
                allowed: self.allowed_extensions.iter().cloned().collect(),
            });
        }

        Ok(())
    }
}

/// Validate a ZIP entry name for path traversal before extraction
///
/// ZIP files can contain malicious entry names like `../../../etc/passwd`.
/// This function validates that a ZIP entry name is safe.
pub fn validate_zip_entry_name(entry_name: &str) -> Result<PathBuf, PathSecurityError> {
    let path = Path::new(entry_name);

    // Check for absolute paths
    if path.is_absolute() {
        return Err(PathSecurityError::PathTraversal(format!(
            "Absolute path in ZIP entry: {}",
            entry_name
        )));
    }

    // Check each component
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err(PathSecurityError::PathTraversal(format!(
                    "Path traversal in ZIP entry: {}",
                    entry_name
                )));
            }
            Component::Normal(s) => {
                if let Some(s) = s.to_str() {
                    // Check for null bytes
                    if s.contains('\0') {
                        return Err(PathSecurityError::PathTraversal(format!(
                            "Null byte in ZIP entry name: {}",
                            entry_name
                        )));
                    }
                    // Check for hidden files (optional, depends on policy)
                    // if s.starts_with('.') { ... }
                }
            }
            _ => {}
        }
    }

    Ok(path.to_path_buf())
}

/// Convenience function to validate a file path for import operations
pub fn validate_import_path(path: &Path) -> Result<SecurePath> {
    PathValidator::for_grid_imports()
        .validate(path)
        .map_err(|e| anyhow!(e))
}

/// Convenience function to validate a file path within a specific directory
pub fn validate_import_path_within(path: &Path, base_dir: &Path) -> Result<SecurePath> {
    PathValidator::for_grid_imports()
        .validate_within(path, base_dir)
        .map_err(|e| anyhow!(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create a test file
        let test_file = dir.path().join("test.json");
        let mut f = File::create(&test_file).unwrap();
        writeln!(f, r#"{{"test": true}}"#).unwrap();

        // Create a subdirectory with a file
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let sub_file = subdir.join("data.m");
        File::create(&sub_file).unwrap();

        dir
    }

    #[test]
    fn test_validate_simple_file() {
        let dir = setup_test_dir();
        let validator = PathValidator::for_grid_imports();

        let result = validator.validate(&dir.path().join("test.json"));
        assert!(result.is_ok());

        let secure = result.unwrap();
        assert!(secure.is_file());
    }

    #[test]
    fn test_reject_path_traversal() {
        let dir = setup_test_dir();
        let validator = PathValidator::for_grid_imports();

        // Direct traversal
        let traversal_path = dir
            .path()
            .join("subdir")
            .join("..")
            .join("..")
            .join("etc")
            .join("passwd");
        let result = validator.validate(&traversal_path);
        assert!(matches!(result, Err(PathSecurityError::PathTraversal(_))));
    }

    #[test]
    fn test_reject_disallowed_extension() {
        let dir = setup_test_dir();

        // Create a file with disallowed extension
        let bad_file = dir.path().join("script.sh");
        File::create(&bad_file).unwrap();

        let validator = PathValidator::for_grid_imports();
        let result = validator.validate(&bad_file);

        assert!(matches!(
            result,
            Err(PathSecurityError::DisallowedExtension { .. })
        ));
    }

    #[test]
    fn test_validate_within_rejects_escape() {
        let dir = setup_test_dir();
        let allowed = dir.path().join("subdir");

        // Try to access file outside allowed directory
        let validator = PathValidator::for_grid_imports();
        let result = validator.validate_within(&dir.path().join("test.json"), &allowed);

        assert!(matches!(
            result,
            Err(PathSecurityError::EscapedAllowedDir { .. })
        ));
    }

    #[test]
    fn test_validate_within_allows_nested() {
        let dir = setup_test_dir();
        let validator = PathValidator::for_grid_imports();

        // File within allowed directory should work
        let result =
            validator.validate_within(&dir.path().join("subdir").join("data.m"), dir.path());

        assert!(result.is_ok());
    }

    #[test]
    fn test_file_size_limit() {
        let dir = setup_test_dir();

        // Create a file that exceeds limit
        let big_file = dir.path().join("big.json");
        let mut f = File::create(&big_file).unwrap();
        // Write 2KB
        for _ in 0..2048 {
            f.write_all(b"X").unwrap();
        }

        // Validator with 1KB limit
        let validator = PathValidator::new()
            .allow_extensions(&["json"])
            .max_file_size(1024);

        let result = validator.validate(&big_file);
        assert!(matches!(
            result,
            Err(PathSecurityError::FileTooLarge { .. })
        ));
    }

    #[test]
    fn test_validate_zip_entry_safe() {
        assert!(validate_zip_entry_name("data/case14.m").is_ok());
        assert!(validate_zip_entry_name("network.json").is_ok());
    }

    #[test]
    fn test_validate_zip_entry_traversal() {
        assert!(matches!(
            validate_zip_entry_name("../../../etc/passwd"),
            Err(PathSecurityError::PathTraversal(_))
        ));
        assert!(matches!(
            validate_zip_entry_name("data/../../../etc/shadow"),
            Err(PathSecurityError::PathTraversal(_))
        ));
    }

    #[test]
    fn test_validate_zip_entry_absolute() {
        assert!(matches!(
            validate_zip_entry_name("/etc/passwd"),
            Err(PathSecurityError::PathTraversal(_))
        ));
    }

    #[test]
    fn test_url_encoded_traversal() {
        let validator = PathValidator::new();

        // These paths contain URL-encoded traversal sequences
        let path = Path::new("data%2e%2e%2fetc/passwd");
        let result = validator.check_traversal_patterns(path);
        assert!(matches!(result, Err(PathSecurityError::PathTraversal(_))));
    }

    #[test]
    fn test_directory_validation() {
        let dir = setup_test_dir();
        let validator = PathValidator::for_grid_imports();

        let result = validator.validate(&dir.path().join("subdir"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_dir());
    }

    #[test]
    fn test_reject_directory_when_file_required() {
        let dir = setup_test_dir();
        let validator = PathValidator::for_grid_imports().allow_directories(false);

        let result = validator.validate(&dir.path().join("subdir"));
        assert!(matches!(result, Err(PathSecurityError::NotAFile(_))));
    }
}
