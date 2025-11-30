//! Configuration management for GAT components
//! All configuration is centralized in ~/.gat/config/

use crate::install::gat_home;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main GAT configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatConfig {
    /// Data directory configuration
    #[serde(default)]
    pub data: DataConfig,
    /// Solver configuration
    #[serde(default)]
    pub solvers: SolverConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Data directory paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    /// Cache grid models here
    #[serde(default = "default_grid_cache")]
    pub grid_cache: String,
    /// Store results here
    #[serde(default = "default_results_dir")]
    pub results_dir: String,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            grid_cache: default_grid_cache(),
            results_dir: default_results_dir(),
        }
    }
}

fn default_grid_cache() -> String {
    "~/.gat/cache/grids".to_string()
}

fn default_results_dir() -> String {
    "~/.gat/cache/results".to_string()
}

/// Native solver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    /// Enable native solver plugins (requires user consent)
    #[serde(default)]
    pub native_enabled: bool,
    /// Default solver for LP/MIP problems
    #[serde(default = "default_lp_solver")]
    pub default_lp: String,
    /// Default solver for NLP problems
    #[serde(default = "default_nlp_solver")]
    pub default_nlp: String,
    /// Solver timeout in seconds (0 = no timeout)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Maximum number of iterations
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            native_enabled: false,
            default_lp: default_lp_solver(),
            default_nlp: default_nlp_solver(),
            timeout_seconds: default_timeout(),
            max_iterations: default_max_iterations(),
        }
    }
}

fn default_lp_solver() -> String {
    "clarabel".to_string()
}

fn default_nlp_solver() -> String {
    "lbfgs".to_string()
}

fn default_timeout() -> u64 {
    300 // 5 minutes
}

fn default_max_iterations() -> u32 {
    1000
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Load the GAT configuration from ~/.gat/config/gat.toml
pub fn load_gat_config() -> Result<GatConfig> {
    let config_path = gat_config_path()?;

    if !config_path.exists() {
        return Ok(GatConfig::default());
    }

    let contents = std::fs::read_to_string(&config_path)?;
    let config: GatConfig = toml::from_str(&contents)?;
    Ok(config)
}

/// Save the GAT configuration to ~/.gat/config/gat.toml
pub fn save_gat_config(config: &GatConfig) -> Result<()> {
    let config_path = gat_config_path()?;
    let config_dir = config_path.parent().unwrap();
    std::fs::create_dir_all(config_dir)?;

    let contents = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, contents)?;
    Ok(())
}

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

    // Write default config using the typed struct
    save_gat_config(&GatConfig::default())
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
