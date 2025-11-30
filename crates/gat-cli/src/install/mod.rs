//! GAT installation and component management

pub mod component;
pub mod config;
pub mod gat_home;
pub mod github;
pub mod installer;
pub mod solvers_state;

pub use component::Component;
pub use config::{
    ensure_gat_config, ensure_gui_config, ensure_tui_config, gat_config_path, gui_config_path,
    load_gat_config, save_gat_config, tui_config_path,
};
pub use gat_home::{ensure_gat_dirs, gat_home, GatDirs};
pub use github::{build_download_url, detect_arch, detect_os, fetch_latest_release};
pub use installer::{install_component, upgrade_all};
pub use solvers_state::{
    get_solver_info, is_solver_available, list_installed_solvers, load_solvers_state,
    register_solver, save_solvers_state, set_solver_enabled, solvers_state_path, unregister_solver,
    InstalledSolver, SolversState,
};
