//! GAT installation and component management

pub mod component;
pub mod config;
pub mod gat_home;
pub mod github;
pub mod installer;

pub use component::Component;
pub use config::{
    ensure_gat_config, ensure_gui_config, ensure_tui_config, gat_config_path, gui_config_path,
    tui_config_path,
};
pub use gat_home::{ensure_gat_dirs, gat_home, GatDirs};
pub use github::{build_download_url, detect_arch, detect_os, fetch_latest_release};
pub use installer::{install_component, upgrade_all};
