/// Configuration management for gat-tui
///
/// Provides a convenient wrapper around config-rs for loading and managing
/// application configuration from TOML files and environment variables.
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Application configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: ThemeConfig,
    pub cli: CliConfig,
    pub logging: LoggingConfig,
    pub ui: UiConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme")]
    pub mode: String, // "dark" or "light"
    #[serde(default)]
    pub accent_color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CliConfig {
    #[serde(default = "default_cli_path")]
    pub gat_cli_path: String,
    #[serde(default = "default_timeout")]
    pub command_timeout_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub log_dir: Option<String>,
    #[serde(default)]
    pub enable_file_logging: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default)]
    pub auto_save_on_pane_switch: bool,
    #[serde(default)]
    pub confirm_on_delete: bool,
    #[serde(default = "default_animation_enabled")]
    pub enable_animations: bool,
    #[serde(default)]
    pub recent_parameters: std::collections::HashMap<String, Vec<String>>,
}

// Default values
fn default_theme() -> String {
    "dark".to_string()
}

fn default_cli_path() -> String {
    "gat-cli".to_string()
}

fn default_timeout() -> u64 {
    300
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_animation_enabled() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            theme: ThemeConfig {
                mode: default_theme(),
                accent_color: String::new(),
            },
            cli: CliConfig {
                gat_cli_path: default_cli_path(),
                command_timeout_secs: default_timeout(),
            },
            logging: LoggingConfig {
                level: default_log_level(),
                log_dir: None,
                enable_file_logging: false,
            },
            ui: UiConfig {
                auto_save_on_pane_switch: true,
                confirm_on_delete: true,
                enable_animations: true,
                recent_parameters: std::collections::HashMap::new(),
            },
        }
    }
}

/// Configuration manager for loading and managing app config
pub struct ConfigManager {
    config: AppConfig,
}

impl ConfigManager {
    /// Load configuration from default locations
    ///
    /// Attempts to load configuration in this order:
    /// 1. ./gat-tui.toml (project root)
    /// 2. ~/.config/gat-tui/config.toml (user home)
    /// 3. Built-in defaults
    ///
    /// Environment variables override file settings:
    /// - GAT_TUI_THEME_MODE
    /// - GAT_TUI_CLI_GAT_CLI_PATH
    /// - GAT_TUI_LOGGING_LEVEL
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            // Start with defaults
            .add_source(Config::try_from(&AppConfig::default()).unwrap())
            // Try loading from project root
            .add_source(File::with_name("gat-tui").required(false))
            // Try loading from user home config directory
            .add_source(
                File::new(
                    &format!(
                        "{}/.config/gat-tui/config",
                        std::env::var("HOME").unwrap_or_default()
                    ),
                    config::FileFormat::Toml,
                )
                .required(false),
            )
            // Allow environment variables to override settings
            .add_source(
                Environment::with_prefix("GAT_TUI")
                    .try_parsing(true)
                    .separator("_"),
            )
            .build()?
            .try_deserialize::<AppConfig>()?;

        Ok(ConfigManager { config })
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(Config::try_from(&AppConfig::default()).unwrap())
            .add_source(File::from(path.as_ref()))
            .build()?
            .try_deserialize::<AppConfig>()?;

        Ok(ConfigManager { config })
    }

    /// Get the current configuration
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Get mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    /// Record a recent parameter set for a pane and persist it alongside UI
    /// preferences so operators can re-run jobs quickly when they return to a
    /// pane.
    pub fn record_recent_parameters(
        &mut self,
        pane_id: impl Into<String>,
        params: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;

        let pane_key = pane_id.into();
        let entry = self
            .config
            .ui
            .recent_parameters
            .entry(pane_key)
            .or_default();

        let mut new_params = params;
        new_params.retain(|value| !entry.contains(value));
        entry.splice(0..0, new_params);
        entry.truncate(5);

        let path = Self::default_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.save_to_file(&path)
    }

    /// Retrieve stored parameters for a pane, if any.
    pub fn recent_parameters_for(&self, pane_id: &str) -> Vec<String> {
        self.config
            .ui
            .recent_parameters
            .get(pane_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Standard on-disk config path used by gat-tui.
    pub fn default_config_path() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config/gat-tui/config.toml")
    }

    /// Save configuration to a file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(&self.config)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }

    /// Update theme mode
    pub fn set_theme_mode(&mut self, mode: String) {
        self.config.theme.mode = mode;
    }

    /// Update CLI path
    pub fn set_cli_path(&mut self, path: String) {
        self.config.cli.gat_cli_path = path;
    }

    /// Update logging level
    pub fn set_log_level(&mut self, level: String) {
        self.config.logging.level = level;
    }

    /// Update command timeout
    pub fn set_timeout(&mut self, secs: u64) {
        self.config.cli.command_timeout_secs = secs;
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        ConfigManager {
            config: AppConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.theme.mode, "dark");
        assert_eq!(config.cli.gat_cli_path, "gat-cli");
        assert_eq!(config.cli.command_timeout_secs, 300);
        assert_eq!(config.logging.level, "info");
        assert!(config.ui.auto_save_on_pane_switch);
        assert!(config.ui.confirm_on_delete);
        assert!(config.ui.enable_animations);
        assert!(config.ui.recent_parameters.is_empty());
    }

    #[test]
    fn test_config_manager_load() {
        // This will use defaults and any env vars that are set
        let mgr = ConfigManager::load().expect("should load config");
        assert!(!mgr.config().cli.gat_cli_path.is_empty());
    }

    #[test]
    fn test_config_mutation() {
        let mut mgr = ConfigManager::load().expect("should load");
        mgr.set_theme_mode("light".to_string());
        assert_eq!(mgr.config().theme.mode, "light");

        mgr.set_cli_path("/usr/bin/gat-cli".to_string());
        assert_eq!(mgr.config().cli.gat_cli_path, "/usr/bin/gat-cli");

        mgr.set_timeout(600);
        assert_eq!(mgr.config().cli.command_timeout_secs, 600);
    }

    #[test]
    fn test_recent_parameter_recording() {
        let mut mgr = ConfigManager::default();
        mgr.record_recent_parameters("commands", vec!["--grid case33bw.arrow".into()])
            .expect("should persist recent params");

        let params = mgr.recent_parameters_for("commands");
        assert_eq!(
            params.first().map(String::as_str),
            Some("--grid case33bw.arrow")
        );
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).expect("should serialize");
        assert!(toml_str.contains("theme"));
        assert!(toml_str.contains("cli"));
        assert!(toml_str.contains("logging"));
        assert!(toml_str.contains("ui"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
[theme]
mode = "light"

[cli]
gat_cli_path = "/usr/bin/gat-cli"
command_timeout_secs = 600

[logging]
level = "debug"

[ui]
auto_save_on_pane_switch = false
confirm_on_delete = true
enable_animations = false
"#;

        let config: AppConfig = toml::from_str(toml_str).expect("should deserialize");
        assert_eq!(config.theme.mode, "light");
        assert_eq!(config.cli.gat_cli_path, "/usr/bin/gat-cli");
        assert_eq!(config.cli.command_timeout_secs, 600);
        assert_eq!(config.logging.level, "debug");
        assert!(!config.ui.auto_save_on_pane_switch);
        assert!(!config.ui.enable_animations);
    }

    #[test]
    fn test_config_manager_get_config() {
        let mgr = ConfigManager::load().expect("should load");
        let config = mgr.config();
        assert!(!config.cli.gat_cli_path.is_empty());
    }
}
