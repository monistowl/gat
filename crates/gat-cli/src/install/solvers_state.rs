//! Solver installation state management.
//!
//! Tracks which native solvers are installed, their versions, and paths.
//! This is machine-generated state, separate from user configuration.
//!
//! File: ~/.gat/config/solvers.toml

use crate::install::gat_home::gat_home;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Installed solver state - machine-generated, not user config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolversState {
    /// Protocol version for IPC compatibility.
    #[serde(default = "default_protocol_version")]
    pub protocol_version: u32,
    /// Map of solver name to installation info.
    #[serde(default)]
    pub installed: HashMap<String, InstalledSolver>,
}

fn default_protocol_version() -> u32 {
    1 // Match gat-solver-common::PROTOCOL_VERSION
}

impl Default for SolversState {
    fn default() -> Self {
        Self {
            protocol_version: default_protocol_version(),
            installed: HashMap::new(),
        }
    }
}

/// Information about an installed solver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSolver {
    /// Path to the solver binary.
    pub binary_path: PathBuf,
    /// Version string of the solver.
    pub version: String,
    /// When the solver was installed (ISO 8601).
    pub installed_at: String,
    /// Whether this solver is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Optional notes about the installation.
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_enabled() -> bool {
    true
}

/// Get the path to the solvers state file.
/// Location: ~/.gat/config/solvers.toml
pub fn solvers_state_path() -> Result<PathBuf> {
    let gat_home = gat_home()?;
    Ok(gat_home.join("config").join("solvers.toml"))
}

/// Load the solvers state from ~/.gat/config/solvers.toml
pub fn load_solvers_state() -> Result<SolversState> {
    let state_path = solvers_state_path()?;

    if !state_path.exists() {
        return Ok(SolversState::default());
    }

    let contents = std::fs::read_to_string(&state_path)?;
    let state: SolversState = toml::from_str(&contents)?;
    Ok(state)
}

/// Save the solvers state to ~/.gat/config/solvers.toml
pub fn save_solvers_state(state: &SolversState) -> Result<()> {
    let state_path = solvers_state_path()?;
    let state_dir = state_path.parent().unwrap();
    std::fs::create_dir_all(state_dir)?;

    let contents = toml::to_string_pretty(state)?;
    std::fs::write(&state_path, contents)?;
    Ok(())
}

/// Register a newly installed solver.
pub fn register_solver(
    name: &str,
    binary_path: PathBuf,
    version: &str,
) -> Result<()> {
    let mut state = load_solvers_state()?;

    let installed = InstalledSolver {
        binary_path,
        version: version.to_string(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        enabled: true,
        notes: None,
    };

    state.installed.insert(name.to_string(), installed);
    save_solvers_state(&state)?;

    Ok(())
}

/// Unregister an installed solver.
pub fn unregister_solver(name: &str) -> Result<bool> {
    let mut state = load_solvers_state()?;
    let removed = state.installed.remove(name).is_some();
    if removed {
        save_solvers_state(&state)?;
    }
    Ok(removed)
}

/// Check if a solver is installed and enabled.
pub fn is_solver_available(name: &str) -> Result<bool> {
    let state = load_solvers_state()?;
    Ok(state
        .installed
        .get(name)
        .is_some_and(|s| s.enabled && s.binary_path.exists()))
}

/// Get information about an installed solver.
pub fn get_solver_info(name: &str) -> Result<Option<InstalledSolver>> {
    let state = load_solvers_state()?;
    Ok(state.installed.get(name).cloned())
}

/// List all installed solvers.
pub fn list_installed_solvers() -> Result<Vec<(String, InstalledSolver)>> {
    let state = load_solvers_state()?;
    Ok(state
        .installed
        .into_iter()
        .collect())
}

/// Enable or disable a solver.
pub fn set_solver_enabled(name: &str, enabled: bool) -> Result<bool> {
    let mut state = load_solvers_state()?;
    if let Some(solver) = state.installed.get_mut(name) {
        solver.enabled = enabled;
        save_solvers_state(&state)?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solvers_state_path() {
        let path = solvers_state_path().unwrap();
        assert!(path.to_string_lossy().contains(".gat/config"));
        assert!(path.to_string_lossy().ends_with("solvers.toml"));
    }

    #[test]
    fn test_default_state() {
        let state = SolversState::default();
        assert_eq!(state.protocol_version, 1);
        assert!(state.installed.is_empty());
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut state = SolversState::default();
        state.installed.insert(
            "ipopt".to_string(),
            InstalledSolver {
                binary_path: PathBuf::from("/home/user/.gat/solvers/gat-ipopt"),
                version: "3.14.0".to_string(),
                installed_at: "2024-01-01T00:00:00Z".to_string(),
                enabled: true,
                notes: Some("Built from source".to_string()),
            },
        );

        let serialized = toml::to_string_pretty(&state).unwrap();
        let deserialized: SolversState = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.installed.len(), 1);
        let ipopt = deserialized.installed.get("ipopt").unwrap();
        assert_eq!(ipopt.version, "3.14.0");
        assert!(ipopt.enabled);
    }
}
