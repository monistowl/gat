/// Settings/Preferences Pane for gat-tui
///
/// Manages user preferences, application settings, and configuration options
/// across multiple categories: Display, Data, Execution, and Advanced.
use serde::{Deserialize, Serialize};

/// Settings tab enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Display,
    Data,
    Execution,
    Advanced,
}

impl SettingsTab {
    pub fn label(&self) -> &str {
        match self {
            SettingsTab::Display => "Display",
            SettingsTab::Data => "Data",
            SettingsTab::Execution => "Execution",
            SettingsTab::Advanced => "Advanced",
        }
    }
}

/// Display preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplaySettings {
    pub theme: String, // "dark", "light", "auto"
    pub show_grid_lines: bool,
    pub compact_mode: bool,
    pub font_size: u8,
    pub status_bar_position: String, // "top", "bottom"
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            show_grid_lines: true,
            compact_mode: false,
            font_size: 12,
            status_bar_position: "bottom".to_string(),
        }
    }
}

/// Data handling preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataSettings {
    pub auto_refresh: bool,
    pub refresh_interval_secs: u32,
    pub cache_enabled: bool,
    pub cache_size_mb: u32,
    pub default_dataset_path: String,
}

impl Default for DataSettings {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            refresh_interval_secs: 30,
            cache_enabled: true,
            cache_size_mb: 512,
            default_dataset_path: "./datasets".to_string(),
        }
    }
}

/// Execution preferences
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionSettings {
    pub max_parallel_jobs: u32,
    pub job_timeout_secs: u32,
    pub auto_save_results: bool,
    pub results_output_path: String,
    pub verbose_logging: bool,
}

impl Default for ExecutionSettings {
    fn default() -> Self {
        Self {
            max_parallel_jobs: 4,
            job_timeout_secs: 3600,
            auto_save_results: true,
            results_output_path: "./results".to_string(),
            verbose_logging: false,
        }
    }
}

/// Advanced settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdvancedSettings {
    pub enable_debug_mode: bool,
    pub enable_telemetry: bool,
    pub connection_timeout_secs: u32,
    pub retry_attempts: u32,
    pub custom_config_path: String,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            enable_debug_mode: false,
            enable_telemetry: true,
            connection_timeout_secs: 30,
            retry_attempts: 3,
            custom_config_path: String::new(),
        }
    }
}

/// Settings Pane State
#[derive(Debug, Clone)]
pub struct SettingsPaneState {
    pub current_tab: SettingsTab,
    pub display_settings: DisplaySettings,
    pub data_settings: DataSettings,
    pub execution_settings: ExecutionSettings,
    pub advanced_settings: AdvancedSettings,
    pub selected_setting_index: usize,
    pub unsaved_changes: bool,
}

impl Default for SettingsPaneState {
    fn default() -> Self {
        Self {
            current_tab: SettingsTab::Display,
            display_settings: DisplaySettings::default(),
            data_settings: DataSettings::default(),
            execution_settings: ExecutionSettings::default(),
            advanced_settings: AdvancedSettings::default(),
            selected_setting_index: 0,
            unsaved_changes: false,
        }
    }
}

impl SettingsPaneState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Switch to a specific tab
    pub fn switch_tab(&mut self, tab: SettingsTab) {
        self.current_tab = tab;
        self.selected_setting_index = 0;
    }

    /// Navigate to next tab
    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            SettingsTab::Display => SettingsTab::Data,
            SettingsTab::Data => SettingsTab::Execution,
            SettingsTab::Execution => SettingsTab::Advanced,
            SettingsTab::Advanced => SettingsTab::Display,
        };
        self.selected_setting_index = 0;
    }

    /// Navigate to previous tab
    pub fn prev_tab(&mut self) {
        self.current_tab = match self.current_tab {
            SettingsTab::Display => SettingsTab::Advanced,
            SettingsTab::Data => SettingsTab::Display,
            SettingsTab::Execution => SettingsTab::Data,
            SettingsTab::Advanced => SettingsTab::Execution,
        };
        self.selected_setting_index = 0;
    }

    /// Get number of settings in current tab
    pub fn settings_count(&self) -> usize {
        match self.current_tab {
            SettingsTab::Display => 5,
            SettingsTab::Data => 5,
            SettingsTab::Execution => 5,
            SettingsTab::Advanced => 5,
        }
    }

    /// Select next setting
    pub fn next_setting(&mut self) {
        let count = self.settings_count();
        if count > 0 {
            self.selected_setting_index = (self.selected_setting_index + 1) % count;
        }
    }

    /// Select previous setting
    pub fn prev_setting(&mut self) {
        let count = self.settings_count();
        if count > 0 {
            self.selected_setting_index = if self.selected_setting_index == 0 {
                count - 1
            } else {
                self.selected_setting_index - 1
            };
        }
    }

    /// Get currently selected setting name and value
    pub fn selected_setting(&self) -> Option<(String, String)> {
        match self.current_tab {
            SettingsTab::Display => match self.selected_setting_index {
                0 => Some(("Theme".to_string(), self.display_settings.theme.clone())),
                1 => Some((
                    "Grid Lines".to_string(),
                    self.display_settings.show_grid_lines.to_string(),
                )),
                2 => Some((
                    "Compact Mode".to_string(),
                    self.display_settings.compact_mode.to_string(),
                )),
                3 => Some((
                    "Font Size".to_string(),
                    self.display_settings.font_size.to_string(),
                )),
                4 => Some((
                    "Status Bar Position".to_string(),
                    self.display_settings.status_bar_position.clone(),
                )),
                _ => None,
            },
            SettingsTab::Data => match self.selected_setting_index {
                0 => Some((
                    "Auto Refresh".to_string(),
                    self.data_settings.auto_refresh.to_string(),
                )),
                1 => Some((
                    "Refresh Interval (s)".to_string(),
                    self.data_settings.refresh_interval_secs.to_string(),
                )),
                2 => Some((
                    "Cache Enabled".to_string(),
                    self.data_settings.cache_enabled.to_string(),
                )),
                3 => Some((
                    "Cache Size (MB)".to_string(),
                    self.data_settings.cache_size_mb.to_string(),
                )),
                4 => Some((
                    "Default Dataset Path".to_string(),
                    self.data_settings.default_dataset_path.clone(),
                )),
                _ => None,
            },
            SettingsTab::Execution => match self.selected_setting_index {
                0 => Some((
                    "Max Parallel Jobs".to_string(),
                    self.execution_settings.max_parallel_jobs.to_string(),
                )),
                1 => Some((
                    "Job Timeout (s)".to_string(),
                    self.execution_settings.job_timeout_secs.to_string(),
                )),
                2 => Some((
                    "Auto Save Results".to_string(),
                    self.execution_settings.auto_save_results.to_string(),
                )),
                3 => Some((
                    "Results Output Path".to_string(),
                    self.execution_settings.results_output_path.clone(),
                )),
                4 => Some((
                    "Verbose Logging".to_string(),
                    self.execution_settings.verbose_logging.to_string(),
                )),
                _ => None,
            },
            SettingsTab::Advanced => match self.selected_setting_index {
                0 => Some((
                    "Debug Mode".to_string(),
                    self.advanced_settings.enable_debug_mode.to_string(),
                )),
                1 => Some((
                    "Telemetry".to_string(),
                    self.advanced_settings.enable_telemetry.to_string(),
                )),
                2 => Some((
                    "Connection Timeout (s)".to_string(),
                    self.advanced_settings.connection_timeout_secs.to_string(),
                )),
                3 => Some((
                    "Retry Attempts".to_string(),
                    self.advanced_settings.retry_attempts.to_string(),
                )),
                4 => Some((
                    "Config Path".to_string(),
                    self.advanced_settings.custom_config_path.clone(),
                )),
                _ => None,
            },
        }
    }

    /// Get detailed description of current setting
    pub fn get_setting_description(&self) -> String {
        match self.current_tab {
            SettingsTab::Display => match self.selected_setting_index {
                0 => "Color scheme: 'dark', 'light', or 'auto' for system default".to_string(),
                1 => "Display grid lines in data tables and visualizations".to_string(),
                2 => "Use compact layout to reduce whitespace".to_string(),
                3 => "Font size in points (8-16)".to_string(),
                4 => "Position of status bar: 'top' or 'bottom'".to_string(),
                _ => String::new(),
            },
            SettingsTab::Data => match self.selected_setting_index {
                0 => "Automatically refresh data from sources".to_string(),
                1 => "Interval between refresh operations (seconds)".to_string(),
                2 => "Enable in-memory caching of datasets".to_string(),
                3 => "Maximum cache size in megabytes".to_string(),
                4 => "Default directory for dataset operations".to_string(),
                _ => String::new(),
            },
            SettingsTab::Execution => match self.selected_setting_index {
                0 => "Maximum concurrent batch jobs (1-16)".to_string(),
                1 => "Maximum time for a job to complete (seconds)".to_string(),
                2 => "Save job results automatically after completion".to_string(),
                3 => "Directory for saving operation results".to_string(),
                4 => "Enable detailed operation logging".to_string(),
                _ => String::new(),
            },
            SettingsTab::Advanced => match self.selected_setting_index {
                0 => "Enable debug mode for development and troubleshooting".to_string(),
                1 => "Allow anonymized telemetry collection".to_string(),
                2 => "Network connection timeout (seconds)".to_string(),
                3 => "Number of retry attempts for failed operations".to_string(),
                4 => "Path to custom configuration file (if any)".to_string(),
                _ => String::new(),
            },
        }
    }

    /// Get all settings for current tab as formatted strings
    pub fn get_all_settings(&self) -> Vec<(String, String)> {
        match self.current_tab {
            SettingsTab::Display => vec![
                ("Theme".to_string(), self.display_settings.theme.clone()),
                (
                    "Grid Lines".to_string(),
                    self.display_settings.show_grid_lines.to_string(),
                ),
                (
                    "Compact Mode".to_string(),
                    self.display_settings.compact_mode.to_string(),
                ),
                (
                    "Font Size".to_string(),
                    self.display_settings.font_size.to_string(),
                ),
                (
                    "Status Bar Position".to_string(),
                    self.display_settings.status_bar_position.clone(),
                ),
            ],
            SettingsTab::Data => vec![
                (
                    "Auto Refresh".to_string(),
                    self.data_settings.auto_refresh.to_string(),
                ),
                (
                    "Refresh Interval (s)".to_string(),
                    self.data_settings.refresh_interval_secs.to_string(),
                ),
                (
                    "Cache Enabled".to_string(),
                    self.data_settings.cache_enabled.to_string(),
                ),
                (
                    "Cache Size (MB)".to_string(),
                    self.data_settings.cache_size_mb.to_string(),
                ),
                (
                    "Default Dataset Path".to_string(),
                    self.data_settings.default_dataset_path.clone(),
                ),
            ],
            SettingsTab::Execution => vec![
                (
                    "Max Parallel Jobs".to_string(),
                    self.execution_settings.max_parallel_jobs.to_string(),
                ),
                (
                    "Job Timeout (s)".to_string(),
                    self.execution_settings.job_timeout_secs.to_string(),
                ),
                (
                    "Auto Save Results".to_string(),
                    self.execution_settings.auto_save_results.to_string(),
                ),
                (
                    "Results Output Path".to_string(),
                    self.execution_settings.results_output_path.clone(),
                ),
                (
                    "Verbose Logging".to_string(),
                    self.execution_settings.verbose_logging.to_string(),
                ),
            ],
            SettingsTab::Advanced => vec![
                (
                    "Debug Mode".to_string(),
                    self.advanced_settings.enable_debug_mode.to_string(),
                ),
                (
                    "Telemetry".to_string(),
                    self.advanced_settings.enable_telemetry.to_string(),
                ),
                (
                    "Connection Timeout (s)".to_string(),
                    self.advanced_settings.connection_timeout_secs.to_string(),
                ),
                (
                    "Retry Attempts".to_string(),
                    self.advanced_settings.retry_attempts.to_string(),
                ),
                (
                    "Config Path".to_string(),
                    self.advanced_settings.custom_config_path.clone(),
                ),
            ],
        }
    }

    /// Save current settings to memory (mark as not changed)
    pub fn save_settings(&mut self) {
        self.unsaved_changes = false;
    }

    /// Reset settings to default values
    pub fn reset_to_defaults(&mut self) {
        self.display_settings = DisplaySettings::default();
        self.data_settings = DataSettings::default();
        self.execution_settings = ExecutionSettings::default();
        self.advanced_settings = AdvancedSettings::default();
        self.unsaved_changes = true;
    }

    /// Mark that changes have been made
    pub fn mark_changed(&mut self) {
        self.unsaved_changes = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_tab_label() {
        assert_eq!(SettingsTab::Display.label(), "Display");
        assert_eq!(SettingsTab::Data.label(), "Data");
        assert_eq!(SettingsTab::Execution.label(), "Execution");
        assert_eq!(SettingsTab::Advanced.label(), "Advanced");
    }

    #[test]
    fn test_display_settings_default() {
        let settings = DisplaySettings::default();
        assert_eq!(settings.theme, "dark");
        assert!(settings.show_grid_lines);
        assert!(!settings.compact_mode);
        assert_eq!(settings.font_size, 12);
        assert_eq!(settings.status_bar_position, "bottom");
    }

    #[test]
    fn test_data_settings_default() {
        let settings = DataSettings::default();
        assert!(settings.auto_refresh);
        assert_eq!(settings.refresh_interval_secs, 30);
        assert!(settings.cache_enabled);
        assert_eq!(settings.cache_size_mb, 512);
        assert_eq!(settings.default_dataset_path, "./datasets");
    }

    #[test]
    fn test_execution_settings_default() {
        let settings = ExecutionSettings::default();
        assert_eq!(settings.max_parallel_jobs, 4);
        assert_eq!(settings.job_timeout_secs, 3600);
        assert!(settings.auto_save_results);
        assert_eq!(settings.results_output_path, "./results");
        assert!(!settings.verbose_logging);
    }

    #[test]
    fn test_advanced_settings_default() {
        let settings = AdvancedSettings::default();
        assert!(!settings.enable_debug_mode);
        assert!(settings.enable_telemetry);
        assert_eq!(settings.connection_timeout_secs, 30);
        assert_eq!(settings.retry_attempts, 3);
        assert_eq!(settings.custom_config_path, "");
    }

    #[test]
    fn test_settings_pane_state_init() {
        let state = SettingsPaneState::new();
        assert_eq!(state.current_tab, SettingsTab::Display);
        assert_eq!(state.selected_setting_index, 0);
        assert!(!state.unsaved_changes);
    }

    #[test]
    fn test_switch_tab() {
        let mut state = SettingsPaneState::new();
        state.switch_tab(SettingsTab::Execution);
        assert_eq!(state.current_tab, SettingsTab::Execution);
        assert_eq!(state.selected_setting_index, 0);
    }

    #[test]
    fn test_next_tab() {
        let mut state = SettingsPaneState::new();
        assert_eq!(state.current_tab, SettingsTab::Display);
        state.next_tab();
        assert_eq!(state.current_tab, SettingsTab::Data);
        state.next_tab();
        assert_eq!(state.current_tab, SettingsTab::Execution);
        state.next_tab();
        assert_eq!(state.current_tab, SettingsTab::Advanced);
        state.next_tab();
        assert_eq!(state.current_tab, SettingsTab::Display);
    }

    #[test]
    fn test_prev_tab() {
        let mut state = SettingsPaneState::new();
        assert_eq!(state.current_tab, SettingsTab::Display);
        state.prev_tab();
        assert_eq!(state.current_tab, SettingsTab::Advanced);
        state.prev_tab();
        assert_eq!(state.current_tab, SettingsTab::Execution);
        state.prev_tab();
        assert_eq!(state.current_tab, SettingsTab::Data);
        state.prev_tab();
        assert_eq!(state.current_tab, SettingsTab::Display);
    }

    #[test]
    fn test_settings_count() {
        let state = SettingsPaneState::new();
        assert_eq!(state.settings_count(), 5); // Display tab

        let mut state = state;
        state.switch_tab(SettingsTab::Data);
        assert_eq!(state.settings_count(), 5);

        state.switch_tab(SettingsTab::Execution);
        assert_eq!(state.settings_count(), 5);

        state.switch_tab(SettingsTab::Advanced);
        assert_eq!(state.settings_count(), 5);
    }

    #[test]
    fn test_next_setting() {
        let mut state = SettingsPaneState::new();
        assert_eq!(state.selected_setting_index, 0);
        state.next_setting();
        assert_eq!(state.selected_setting_index, 1);
        state.next_setting();
        assert_eq!(state.selected_setting_index, 2);
    }

    #[test]
    fn test_next_setting_wraps() {
        let mut state = SettingsPaneState::new();
        for _ in 0..5 {
            state.next_setting();
        }
        assert_eq!(state.selected_setting_index, 0); // Wraps around
    }

    #[test]
    fn test_prev_setting() {
        let mut state = SettingsPaneState::new();
        state.selected_setting_index = 2;
        state.prev_setting();
        assert_eq!(state.selected_setting_index, 1);
        state.prev_setting();
        assert_eq!(state.selected_setting_index, 0);
    }

    #[test]
    fn test_prev_setting_wraps() {
        let mut state = SettingsPaneState::new();
        state.selected_setting_index = 0;
        state.prev_setting();
        assert_eq!(state.selected_setting_index, 4); // Wraps to end
    }

    #[test]
    fn test_selected_setting_display_tab() {
        let state = SettingsPaneState::new();
        let (name, value) = state.selected_setting().unwrap();
        assert_eq!(name, "Theme");
        assert_eq!(value, "dark");
    }

    #[test]
    fn test_selected_setting_navigation() {
        let mut state = SettingsPaneState::new();
        state.next_setting();
        let (name, _value) = state.selected_setting().unwrap();
        assert_eq!(name, "Grid Lines");
    }

    #[test]
    fn test_selected_setting_data_tab() {
        let mut state = SettingsPaneState::new();
        state.switch_tab(SettingsTab::Data);
        let (name, _value) = state.selected_setting().unwrap();
        assert_eq!(name, "Auto Refresh");
    }

    #[test]
    fn test_selected_setting_execution_tab() {
        let mut state = SettingsPaneState::new();
        state.switch_tab(SettingsTab::Execution);
        let (name, _value) = state.selected_setting().unwrap();
        assert_eq!(name, "Max Parallel Jobs");
    }

    #[test]
    fn test_selected_setting_advanced_tab() {
        let mut state = SettingsPaneState::new();
        state.switch_tab(SettingsTab::Advanced);
        let (name, _value) = state.selected_setting().unwrap();
        assert_eq!(name, "Debug Mode");
    }

    #[test]
    fn test_get_setting_description() {
        let state = SettingsPaneState::new();
        let desc = state.get_setting_description();
        assert!(desc.contains("Color scheme"));
    }

    #[test]
    fn test_get_all_settings() {
        let state = SettingsPaneState::new();
        let settings = state.get_all_settings();
        assert_eq!(settings.len(), 5);
        assert_eq!(settings[0].0, "Theme");
        assert_eq!(settings[1].0, "Grid Lines");
    }

    #[test]
    fn test_get_all_settings_different_tab() {
        let mut state = SettingsPaneState::new();
        state.switch_tab(SettingsTab::Data);
        let settings = state.get_all_settings();
        assert_eq!(settings[0].0, "Auto Refresh");
    }

    #[test]
    fn test_save_settings() {
        let mut state = SettingsPaneState::new();
        state.unsaved_changes = true;
        state.save_settings();
        assert!(!state.unsaved_changes);
    }

    #[test]
    fn test_reset_to_defaults() {
        let mut state = SettingsPaneState::new();
        state.display_settings.theme = "light".to_string();
        state.data_settings.auto_refresh = false;
        state.reset_to_defaults();
        assert_eq!(state.display_settings.theme, "dark");
        assert!(state.data_settings.auto_refresh);
        assert!(state.unsaved_changes);
    }

    #[test]
    fn test_mark_changed() {
        let mut state = SettingsPaneState::new();
        assert!(!state.unsaved_changes);
        state.mark_changed();
        assert!(state.unsaved_changes);
    }

    #[test]
    fn test_settings_persistence_across_tabs() {
        let mut state = SettingsPaneState::new();
        state.display_settings.theme = "light".to_string();
        state.switch_tab(SettingsTab::Data);
        state.switch_tab(SettingsTab::Display);
        assert_eq!(state.display_settings.theme, "light");
    }

    #[test]
    fn test_get_all_settings_execution_tab() {
        let mut state = SettingsPaneState::new();
        state.switch_tab(SettingsTab::Execution);
        let settings = state.get_all_settings();
        assert_eq!(settings.len(), 5);
        assert_eq!(settings[0].0, "Max Parallel Jobs");
        assert_eq!(settings[1].0, "Job Timeout (s)");
        assert_eq!(settings[2].0, "Auto Save Results");
        assert_eq!(settings[3].0, "Results Output Path");
        assert_eq!(settings[4].0, "Verbose Logging");
    }

    #[test]
    fn test_get_all_settings_advanced_tab() {
        let mut state = SettingsPaneState::new();
        state.switch_tab(SettingsTab::Advanced);
        let settings = state.get_all_settings();
        assert_eq!(settings.len(), 5);
        assert_eq!(settings[0].0, "Debug Mode");
        assert_eq!(settings[4].0, "Config Path");
    }
}
