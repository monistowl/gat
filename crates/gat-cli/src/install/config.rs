//! Configuration management for GAT components
//! All configuration is centralized in ~/.gat/config/

use crate::install::gat_home;
use anyhow::Result;
use std::path::PathBuf;

/// Get the path to the main GAT configuration file
/// Location: ~/.gat/config/gat.toml
pub fn gat_config_path() -> Result<PathBuf> {
    let gat_home = gat_home()?;
    Ok(gat_home.join("config").join("gat.toml"))
}

/// Get the path to the TUI configuration file
/// Location: ~/.gat/config/tui.toml
pub fn tui_config_path() -> Result<PathBuf> {
    let gat_home = gat_home()?;
    Ok(gat_home.join("config").join("tui.toml"))
}

/// Get the path to the GUI configuration file
/// Location: ~/.gat/config/gui.toml
pub fn gui_config_path() -> Result<PathBuf> {
    let gat_home = gat_home()?;
    Ok(gat_home.join("config").join("gui.toml"))
}

/// Create a default GAT configuration file if it doesn't exist
pub fn ensure_gat_config() -> Result<()> {
    let config_path = gat_config_path()?;

    if config_path.exists() {
        return Ok(());
    }

    let config_dir = config_path.parent().unwrap();
    std::fs::create_dir_all(config_dir)?;

    let default_config = r#"# GAT Configuration
# Location: ~/.gat/config/gat.toml

# Data directories
[data]
# Cache grid models here
grid_cache = "~/.gat/cache/grids"
# Store results here
results_dir = "~/.gat/cache/results"

# Solver preferences
[solvers]
# Default solver: cbc, highs, or custom
default = "cbc"
# CBC binary path (relative to ~/.gat/lib/solvers)
cbc_path = "cbc"
# HIGHS binary path
highs_path = "highs"

# Logging
[logging]
level = "info"
"#;
    std::fs::write(&config_path, default_config)?;

    Ok(())
}

/// Create a default TUI configuration file if it doesn't exist
pub fn ensure_tui_config() -> Result<()> {
    let config_path = tui_config_path()?;

    if config_path.exists() {
        return Ok(());
    }

    let config_dir = config_path.parent().unwrap();
    std::fs::create_dir_all(config_dir)?;

    let default_config = r#"# GAT TUI Configuration
# Location: ~/.gat/config/tui.toml

# Display settings
[display]
theme = "dark"
width = 200
height = 50

# Default panes to show on startup
[startup]
active_pane = "dashboard"

# Command history
[history]
max_items = 100
"#;
    std::fs::write(&config_path, default_config)?;

    Ok(())
}

/// Create a default GUI configuration file if it doesn't exist
pub fn ensure_gui_config() -> Result<()> {
    let config_path = gui_config_path()?;

    if config_path.exists() {
        return Ok(());
    }

    let config_dir = config_path.parent().unwrap();
    std::fs::create_dir_all(config_dir)?;

    let default_config = r#"# GAT GUI Configuration
# Location: ~/.gat/config/gui.toml

# Display settings
[display]
theme = "light"
window_width = 1400
window_height = 900

# Features
[features]
enable_analytics = true
enable_export = true
"#;
    std::fs::write(&config_path, default_config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gat_config_path_contains_gat_config() {
        let path = gat_config_path().unwrap();
        assert!(path.to_string_lossy().contains(".gat/config"));
        assert!(path.to_string_lossy().ends_with("gat.toml"));
    }

    #[test]
    fn test_tui_config_path_contains_tui_config() {
        let path = tui_config_path().unwrap();
        assert!(path.to_string_lossy().contains(".gat/config"));
        assert!(path.to_string_lossy().ends_with("tui.toml"));
    }

    #[test]
    fn test_gui_config_path_contains_gui_config() {
        let path = gui_config_path().unwrap();
        assert!(path.to_string_lossy().contains(".gat/config"));
        assert!(path.to_string_lossy().ends_with("gui.toml"));
    }
}
