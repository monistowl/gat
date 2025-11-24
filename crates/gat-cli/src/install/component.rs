//! Available GAT components that can be installed on demand

use std::fmt;

/// Available GAT components that can be installed on demand
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    /// Terminal User Interface
    Tui,
    /// GUI Dashboard (future)
    Gui,
    /// Additional solvers (CBC, HIGHS, future proprietary)
    Solvers,
}

impl Component {
    /// Get the binary name for this component
    pub fn binary_name(&self) -> &'static str {
        match self {
            Component::Tui => "gat-tui",
            Component::Gui => "gat-gui",
            Component::Solvers => "solvers",
        }
    }

    /// Get the release artifact prefix for this component
    pub fn artifact_prefix(&self) -> &'static str {
        match self {
            Component::Tui => "gat-tui",
            Component::Gui => "gat-gui",
            Component::Solvers => "gat-solvers",
        }
    }

    /// Check if this component is currently installed
    pub fn is_installed(&self, gat_bin: &std::path::Path) -> bool {
        match self {
            Component::Solvers => {
                // Solvers are checked in lib directory
                gat_bin
                    .parent()
                    .map(|p| p.parent().map(|pp| pp.join("lib/solvers").exists()).unwrap_or(false))
                    .unwrap_or(false)
            }
            _ => {
                // Other components are binaries
                gat_bin
                    .parent()
                    .map(|p| p.join(self.binary_name()).exists())
                    .unwrap_or(false)
            }
        }
    }

    /// Get all available components
    pub fn all() -> &'static [Component] {
        &[Component::Tui, Component::Gui, Component::Solvers]
    }

    /// Parse a component from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "gat-tui" => Some(Component::Tui),
            "gat-gui" => Some(Component::Gui),
            "solvers" => Some(Component::Solvers),
            _ => None,
        }
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.binary_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_names() {
        assert_eq!(Component::Tui.binary_name(), "gat-tui");
        assert_eq!(Component::Gui.binary_name(), "gat-gui");
        assert_eq!(Component::Solvers.binary_name(), "solvers");
    }

    #[test]
    fn test_component_artifact_prefixes() {
        assert_eq!(Component::Tui.artifact_prefix(), "gat-tui");
        assert_eq!(Component::Gui.artifact_prefix(), "gat-gui");
        assert_eq!(Component::Solvers.artifact_prefix(), "gat-solvers");
    }

    #[test]
    fn test_all_components() {
        let all = Component::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&Component::Tui));
        assert!(all.contains(&Component::Gui));
        assert!(all.contains(&Component::Solvers));
    }

    #[test]
    fn test_component_from_str() {
        assert_eq!(Component::from_str("gat-tui"), Some(Component::Tui));
        assert_eq!(Component::from_str("gat-gui"), Some(Component::Gui));
        assert_eq!(Component::from_str("solvers"), Some(Component::Solvers));
        assert_eq!(Component::from_str("invalid"), None);
    }

    #[test]
    fn test_component_display() {
        assert_eq!(format!("{}", Component::Tui), "gat-tui");
        assert_eq!(format!("{}", Component::Gui), "gat-gui");
        assert_eq!(format!("{}", Component::Solvers), "solvers");
    }
}
