use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

/// Default localhost port used by the embedded notebook server stub.
const DEFAULT_PORT: u16 = 8787;

/// Options for launching the GAT notebook experience.
#[derive(Debug, Clone)]
pub struct NotebookOptions {
    pub workspace: PathBuf,
    pub port: u16,
    pub open_browser: bool,
}

impl NotebookOptions {
    /// Builds a configuration using the provided workspace path.
    pub fn with_workspace(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            port: DEFAULT_PORT,
            open_browser: false,
        }
    }
}

impl Default for NotebookOptions {
    fn default() -> Self {
        Self::with_workspace("./gat-notebook")
    }
}

/// Launch result for the notebook, mirroring the data a future GUI server would expose.
#[derive(Debug, Clone)]
pub struct NotebookLaunch {
    pub url: String,
    pub workspace: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Serialize)]
struct Manifest<'a> {
    app: &'a str,
    source: &'a str,
    description: &'a str,
    workspace: &'a str,
    port: u16,
}

/// Initialize a GAT-focused notebook environment inspired by the Twinsong workflow.
///
/// The current implementation seeds a workspace with a manifest and helper README so that
/// downstream tooling (or a real GUI server) can reuse the same layout.
pub fn launch(options: NotebookOptions) -> Result<NotebookLaunch> {
    let workspace = normalize_workspace(&options.workspace)?;
    let manifest_path = workspace.join("notebook.manifest.json");

    seed_workspace(&workspace)?;

    let url = format!(
        "http://localhost:{port}/?workspace={workspace}",
        port = options.port,
        workspace = workspace.display()
    );

    let manifest = Manifest {
        app: "gat-notebook",
        source: "twinsong-inspired",
        description: "A research-grade notebook tuned for GAT runs, outputs, and RAG notes.",
        workspace: workspace.to_str().unwrap_or_default(),
        port: options.port,
    };

    let manifest_body = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_body)
        .with_context(|| format!("failed to write manifest at {}", manifest_path.display()))?;

    Ok(NotebookLaunch {
        url,
        workspace,
        manifest_path,
    })
}

fn normalize_workspace(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read current directory")?
            .join(path)
    };

    Ok(absolute)
}

fn seed_workspace(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create workspace at {}", path.display()))?;
    fs::create_dir_all(path.join("notebooks"))
        .with_context(|| format!("failed to create notebooks folder under {}", path.display()))?;

    let readme = path.join("README.md");
    if !readme.exists() {
        let guidance = r#"# GAT Notebook Workspace

This folder mirrors the layout used by the Twinsong notebook experience, but tuned for
Grid Analysis Toolkit (GAT) workflows:

- Drop Arrow grids, Parquet runs, and YAML scenario specs under `datasets/`.
- Capture exploratory prompts and decisions inside `notebooks/`.
- Persist batch or RAG context in `context/`.

Example workflow snippet:

```bash
# Run a DC power flow and keep the results alongside the notebook session
gat pf dc data/ieee14.arrow --out notebooks/ieee14_flows.parquet
```
"#;
        fs::create_dir_all(path.join("datasets"))?;
        fs::create_dir_all(path.join("context"))?;
        fs::write(&readme, guidance)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn launch_creates_manifest_and_summary() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        let options = NotebookOptions {
            workspace: workspace.clone(),
            port: 9000,
            open_browser: false,
        };

        let launch = launch(options).expect("launch should succeed");
        assert!(launch.url.contains("9000"));
        assert!(launch.manifest_path.exists());

        let manifest = fs::read_to_string(&launch.manifest_path).unwrap();
        assert!(manifest.contains("gat-notebook"));
        assert!(manifest.contains("twinsong"));
    }

    #[test]
    fn seed_workspace_adds_readme_once() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("ws");

        seed_workspace(&workspace).unwrap();
        let readme = workspace.join("README.md");
        assert!(readme.exists());

        let first_contents = fs::read_to_string(&readme).unwrap();
        seed_workspace(&workspace).unwrap();
        let second_contents = fs::read_to_string(&readme).unwrap();

        assert_eq!(first_contents, second_contents);
    }
}
