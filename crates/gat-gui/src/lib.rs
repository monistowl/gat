use std::fmt;
use std::path::PathBuf;

use anyhow::Result;

/// Metadata describing an available GUI application that can be launched from this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub command: &'static str,
}

/// Parameters for each supported GUI app.
#[derive(Debug, Clone)]
pub enum GuiApp {
    Notebook(gat_notebook::NotebookOptions),
}

/// Result of launching a GUI experience.
#[derive(Debug, Clone)]
pub struct LaunchReport {
    pub app: &'static str,
    pub url: String,
    pub workspace: Option<PathBuf>,
}

impl LaunchReport {
    pub fn summary(&self) -> String {
        let workspace_line = self
            .workspace
            .as_ref()
            .map(|ws| format!("workspace: {}", ws.display()));

        match workspace_line {
            Some(ws) => format!(
                "{app} ready at {url} ({ws})",
                app = self.app,
                url = self.url
            ),
            None => format!("{app} ready at {url}", app = self.app, url = self.url),
        }
    }
}

impl fmt::Display for LaunchReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Enumerate the composite set of GUI apps.
pub fn available_apps() -> Vec<AppMetadata> {
    vec![AppMetadata {
        name: "notebook",
        description: "Twinsong-inspired notebook tailored for GAT scenarios and research.",
        command: "gat-gui notebook",
    }]
}

/// Launch the requested GUI app.
pub fn launch(app: GuiApp) -> Result<LaunchReport> {
    match app {
        GuiApp::Notebook(options) => {
            let notebook = gat_notebook::launch(options)?;
            Ok(LaunchReport {
                app: "gat-notebook",
                url: notebook.url,
                workspace: Some(notebook.workspace),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn list_includes_notebook() {
        let apps = available_apps();
        assert!(apps.iter().any(|app| app.name == "notebook"));
    }

    #[test]
    fn launch_notebook_wires_through() {
        let workspace = tempdir().unwrap();
        let report = launch(GuiApp::Notebook(gat_notebook::NotebookOptions {
            workspace: workspace.path().join("workspace"),
            port: 7878,
            open_browser: false,
        }))
        .expect("launch should succeed");

        assert!(report.url.contains("7878"));
        assert_eq!(report.app, "gat-notebook");
    }
}
