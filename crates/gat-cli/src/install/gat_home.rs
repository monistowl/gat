//! GAT home directory structure and helpers

use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// Represents the ~/.gat directory structure
#[derive(Debug, Clone)]
pub struct GatDirs {
    /// Root ~/.gat directory
    pub root: PathBuf,
    /// ~/.gat/bin/ - executable binaries
    pub bin: PathBuf,
    /// ~/.gat/config/ - configuration files
    pub config: PathBuf,
    /// ~/.gat/lib/ - libraries and solvers
    pub lib: PathBuf,
    /// ~/.gat/cache/ - runtime caches
    pub cache: PathBuf,
}

impl GatDirs {
    /// Create GatDirs from root path
    pub fn from_root(root: PathBuf) -> Self {
        Self {
            bin: root.join("bin"),
            config: root.join("config"),
            lib: root.join("lib"),
            cache: root.join("cache"),
            root,
        }
    }
}

/// Get the GAT home directory (defaults to ~/.gat)
pub fn gat_home() -> Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| anyhow!("Cannot determine home directory"))
        .map(|h| h.join(".gat"))
}

/// Ensure all GAT directories exist
pub fn ensure_gat_dirs() -> Result<GatDirs> {
    let root = gat_home()?;
    let dirs = GatDirs::from_root(root);

    std::fs::create_dir_all(&dirs.bin)?;
    std::fs::create_dir_all(&dirs.config)?;
    std::fs::create_dir_all(&dirs.lib)?;
    std::fs::create_dir_all(&dirs.cache)?;

    Ok(dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gat_dirs_structure() {
        let root = PathBuf::from("/tmp/test-gat");
        let dirs = GatDirs::from_root(root.clone());

        assert_eq!(dirs.root, root);
        assert_eq!(dirs.bin, root.join("bin"));
        assert_eq!(dirs.config, root.join("config"));
        assert_eq!(dirs.lib, root.join("lib"));
        assert_eq!(dirs.cache, root.join("cache"));
    }

    #[test]
    fn test_gat_home_returns_path() {
        let home = gat_home();
        assert!(home.is_ok());
        let path = home.unwrap();
        assert!(path.ends_with(".gat"));
    }
}
