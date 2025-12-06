//! Unified configuration for GAT user interfaces.
//!
//! The [`GatConfig`] provides a single configuration system for all UIs,
//! with sections for core settings, TUI-specific options, and GUI-specific options.
//!
//! Configuration is stored in `~/.gat/config.toml` and supports partial configs
//! where unspecified values use sensible defaults.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Main configuration for all GAT UIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatConfig {
    /// Core settings shared across all interfaces.
    pub core: CoreConfig,

    /// TUI-specific configuration.
    pub tui: TuiConfig,

    /// GUI-specific configuration.
    pub gui: GuiConfig,
}

impl Default for GatConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            tui: TuiConfig::default(),
            gui: GuiConfig::default(),
        }
    }
}

/// Core settings shared across all UIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CoreConfig {
    /// Default directory for loading/saving case files.
    pub default_case_dir: Option<PathBuf>,

    /// Recently opened files (up to 10).
    pub recent_files: Vec<PathBuf>,

    /// Maximum entries in recent files list.
    pub max_recent: usize,

    /// Default power flow solver tolerance.
    pub pf_tolerance: f64,

    /// Maximum power flow iterations.
    pub pf_max_iter: usize,

    /// Enable parallel solver execution where supported.
    pub parallel_solvers: bool,

    /// Number of worker threads (0 = auto-detect).
    pub worker_threads: usize,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            default_case_dir: None,
            recent_files: Vec::new(),
            max_recent: 10,
            pf_tolerance: 1e-8,
            pf_max_iter: 100,
            parallel_solvers: true,
            worker_threads: 0,
        }
    }
}

/// TUI-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    /// Color theme name.
    pub theme: String,

    /// Show line numbers in tables.
    pub show_line_numbers: bool,

    /// Table row highlighting style.
    pub highlight_style: HighlightStyle,

    /// Default number of decimal places for display.
    pub decimal_places: usize,

    /// Show status bar at bottom.
    pub show_status_bar: bool,

    /// Enable mouse support.
    pub mouse_enabled: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            show_line_numbers: true,
            highlight_style: HighlightStyle::default(),
            decimal_places: 4,
            show_status_bar: true,
            mouse_enabled: true,
        }
    }
}

/// GUI-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiConfig {
    /// Window width on startup.
    pub window_width: u32,

    /// Window height on startup.
    pub window_height: u32,

    /// Remember window position.
    pub remember_position: bool,

    /// Last window X position.
    pub last_x: Option<i32>,

    /// Last window Y position.
    pub last_y: Option<i32>,

    /// Theme (light/dark/system).
    pub theme: GuiTheme,

    /// Font size for data tables.
    pub table_font_size: u32,

    /// Show toolbar.
    pub show_toolbar: bool,

    /// Auto-run power flow on network load.
    pub auto_run_pf: bool,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            window_width: 1280,
            window_height: 800,
            remember_position: true,
            last_x: None,
            last_y: None,
            theme: GuiTheme::System,
            table_font_size: 14,
            show_toolbar: true,
            auto_run_pf: false,
        }
    }
}

/// Row highlighting style for TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HighlightStyle {
    /// No highlighting.
    None,
    /// Highlight current row.
    #[default]
    Row,
    /// Highlight current cell.
    Cell,
}

/// GUI theme options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GuiTheme {
    /// Light theme.
    Light,
    /// Dark theme.
    Dark,
    /// Follow system preference.
    #[default]
    System,
}

impl GatConfig {
    /// Get the default config directory path.
    pub fn config_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".gat"))
    }

    /// Get the default config file path.
    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("config.toml"))
    }

    /// Load configuration from the default location.
    ///
    /// Returns default config if file doesn't exist.
    pub fn load() -> Result<Self> {
        match Self::config_path() {
            Some(path) if path.exists() => Self::load_from(&path),
            _ => Ok(Self::default()),
        }
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to the default location.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()
            .ok_or_else(|| Error::Config("could not determine config directory".to_string()))?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        self.save_to(&path)
    }

    /// Save configuration to a specific path.
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Add a file to the recent files list.
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if already present
        self.core.recent_files.retain(|p| p != &path);

        // Add to front
        self.core.recent_files.insert(0, path);

        // Trim to max
        self.core.recent_files.truncate(self.core.max_recent);
    }

    /// Migrate from legacy config locations.
    ///
    /// Checks for old TUI/GUI-specific configs and merges them.
    pub fn migrate_legacy() -> Result<Option<Self>> {
        let config_dir = match Self::config_dir() {
            Some(d) => d,
            None => return Ok(None),
        };

        // Check for legacy TUI config
        let legacy_tui = config_dir.join("tui.toml");
        let legacy_gui = config_dir.join("gui.toml");

        if !legacy_tui.exists() && !legacy_gui.exists() {
            return Ok(None);
        }

        let mut config = Self::default();

        // Migrate TUI config
        if legacy_tui.exists() {
            if let Ok(contents) = std::fs::read_to_string(&legacy_tui) {
                if let Ok(tui_config) = toml::from_str::<TuiConfig>(&contents) {
                    config.tui = tui_config;
                }
            }
        }

        // Migrate GUI config
        if legacy_gui.exists() {
            if let Ok(contents) = std::fs::read_to_string(&legacy_gui) {
                if let Ok(gui_config) = toml::from_str::<GuiConfig>(&contents) {
                    config.gui = gui_config;
                }
            }
        }

        Ok(Some(config))
    }

    /// Load config, migrating from legacy if needed.
    pub fn load_or_migrate() -> Result<Self> {
        // First try loading existing unified config
        if let Some(path) = Self::config_path() {
            if path.exists() {
                return Self::load_from(&path);
            }
        }

        // Try migrating legacy configs
        if let Some(migrated) = Self::migrate_legacy()? {
            // Save the migrated config
            migrated.save()?;
            return Ok(migrated);
        }

        // Fall back to defaults
        Ok(Self::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = GatConfig::default();
        assert_eq!(config.core.pf_max_iter, 100);
        assert!(config.core.parallel_solvers);
        assert_eq!(config.tui.decimal_places, 4);
        assert_eq!(config.gui.window_width, 1280);
    }

    #[test]
    fn test_partial_config_parsing() {
        let toml = r#"
            [core]
            pf_max_iter = 50

            [gui]
            theme = "dark"
        "#;

        let config: GatConfig = toml::from_str(toml).unwrap();

        // Explicitly set values
        assert_eq!(config.core.pf_max_iter, 50);
        assert_eq!(config.gui.theme, GuiTheme::Dark);

        // Defaults for unset values
        assert_eq!(config.core.pf_tolerance, 1e-8);
        assert_eq!(config.tui.decimal_places, 4);
    }

    #[test]
    fn test_save_and_load() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        let mut config = GatConfig::default();
        config.core.pf_max_iter = 200;
        config.save_to(&path).unwrap();

        let loaded = GatConfig::load_from(&path).unwrap();
        assert_eq!(loaded.core.pf_max_iter, 200);
    }

    #[test]
    fn test_recent_files() {
        let mut config = GatConfig::default();
        config.core.max_recent = 3;

        config.add_recent_file(PathBuf::from("a.m"));
        config.add_recent_file(PathBuf::from("b.m"));
        config.add_recent_file(PathBuf::from("c.m"));
        config.add_recent_file(PathBuf::from("d.m"));

        assert_eq!(config.core.recent_files.len(), 3);
        assert_eq!(config.core.recent_files[0], PathBuf::from("d.m"));
    }

    #[test]
    fn test_recent_files_dedup() {
        let mut config = GatConfig::default();

        config.add_recent_file(PathBuf::from("a.m"));
        config.add_recent_file(PathBuf::from("b.m"));
        config.add_recent_file(PathBuf::from("a.m")); // Re-add a.m

        assert_eq!(config.core.recent_files.len(), 2);
        assert_eq!(config.core.recent_files[0], PathBuf::from("a.m"));
    }
}
