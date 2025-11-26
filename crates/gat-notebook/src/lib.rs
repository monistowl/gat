use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub opened_browser: bool,
}

#[derive(Serialize)]
struct Manifest<'a> {
    app: &'a str,
    source: &'a str,
    description: &'a str,
    workspace: &'a str,
    port: u16,
    created_at: String,
    browser_requested: bool,
    notebooks_dir: &'a str,
    datasets_dir: &'a str,
    context_dir: &'a str,
    demos: Vec<Demo<'a>>,
}

#[derive(Serialize)]
struct Demo<'a> {
    title: &'a str,
    description: &'a str,
    path: &'a str,
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

    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    let manifest = Manifest {
        app: "gat-notebook",
        source: "twinsong-inspired",
        description: "A research-grade notebook tuned for GAT runs, outputs, and RAG notes.",
        workspace: workspace.to_str().unwrap_or_default(),
        port: options.port,
        created_at,
        browser_requested: options.open_browser,
        notebooks_dir: "notebooks",
        datasets_dir: "datasets",
        context_dir: "context",
        demos: vec![
            Demo {
                title: "Power flow walkthrough",
                description: "Import a grid, run DC/AC flows, and inspect violations.",
                path: "notebooks/demos/power-flow.md",
            },
            Demo {
                title: "Scenario + batch analysis",
                description: "Materialize scenarios and execute batch studies with limits and solver controls.",
                path: "notebooks/demos/scenario-batch.md",
            },
            Demo {
                title: "RAG + context building",
                description: "Curate context assets and summarize decisions for downstream assistants.",
                path: "notebooks/demos/rag-context.md",
            },
        ],
    };

    let manifest_body = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_body)
        .with_context(|| format!("failed to write manifest at {}", manifest_path.display()))?;

    let opened_browser = options.open_browser && attempt_open_browser(&url);

    Ok(NotebookLaunch {
        url,
        workspace,
        manifest_path,
        opened_browser,
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

    fs::create_dir_all(path.join("datasets"))?;
    fs::create_dir_all(path.join("context"))?;

    fs::create_dir_all(path.join("notebooks/demos"))?;

    let readme = path.join("README.md");
    write_if_absent(&readme, render_readme())?;

    let datasets_readme = path.join("datasets/README.md");
    write_if_absent(&datasets_readme, render_datasets_readme())?;

    let context_readme = path.join("context/README.md");
    write_if_absent(&context_readme, render_context_readme())?;

    let starter = path.join("notebooks/getting-started.md");
    write_if_absent(&starter, render_starter_notebook())?;

    let power_flow = path.join("notebooks/demos/power-flow.md");
    write_if_absent(&power_flow, render_power_flow_demo())?;

    let scenario_batch = path.join("notebooks/demos/scenario-batch.md");
    write_if_absent(&scenario_batch, render_scenario_batch_demo())?;

    let rag_context = path.join("notebooks/demos/rag-context.md");
    write_if_absent(&rag_context, render_rag_context_demo())?;

    Ok(())
}

fn write_if_absent(path: &Path, contents: String) -> Result<()> {
    if !path.exists() {
        fs::write(path, contents)
            .with_context(|| format!("failed to write starter content at {}", path.display()))?;
    }

    Ok(())
}

fn render_readme() -> String {
    let content = r#"# GAT Notebook Workspace

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

    content.to_string()
}

fn render_datasets_readme() -> String {
    let content = r#"# Datasets

Use this folder to track the grid models, Parquet outputs, and YAML specs referenced by your notebook session.

Recommended starters:

```bash
# Convert a MATPOWER RAW file to Arrow
gat import matpower --file data/ieee14.raw --out datasets/ieee14.arrow

# Validate and summarize
gat validate --file datasets/ieee14.arrow --schema grid
gat graph stats datasets/ieee14.arrow
```

Batch-friendly manifests live well here too:

```bash
gat scenarios materialize --spec scenarios.yaml --grid datasets/ieee14.arrow --out-dir datasets/scenario_runs
```
"#;

    content.to_string()
}

fn render_context_readme() -> String {
    let content = r#"# Context assets

Drop CSV/Parquet/YAML files that you want to recall later in downstream RAG workflows.

Ideas:
- Contingency lists: `contingencies.yaml`
- Solver configs: `opf_limits.csv`, `opf_costs.csv`
- Decision logs exported from notebooks or chats

Summaries can stay close:

```bash
cat notebooks/demos/rag-context.md >> context/session_log.md
```
"#;

    content.to_string()
}

fn render_starter_notebook() -> String {
    let content = r#"# Welcome to the GAT Notebook

This starter note mirrors the Twinsong research cadence with slots for goals, context,
and runnable commands. Fill in the prompts below as you explore.

## Session goals
- [ ] Import a grid model
- [ ] Run a power flow or OPF
- [ ] Capture findings and follow-ups

## Quick commands
```bash
# Prepare a dataset
gat import matpower --file data/ieee14.raw --out datasets/ieee14.arrow

# Run a DC power flow and keep the outputs next to this note
gat pf dc datasets/ieee14.arrow --out notebooks/ieee14_flows.parquet
```

## Notes & decisions
- Observation:
- Next step:

## RAG context
Keep any supporting csv/parquet/yaml artifacts in `context/` for retrieval.
"#;

    content.to_string()
}

fn render_power_flow_demo() -> String {
    let content = r#"# Power flow walkthrough

This demo mirrors the common single-case study loop.

## 1) Import a grid model
```bash
gat import matpower --file data/ieee14.raw --out datasets/ieee14.arrow
gat graph stats datasets/ieee14.arrow
```

## 2) Run DC and AC power flow
```bash
# DC: fast screening
gat pf dc datasets/ieee14.arrow --out notebooks/ieee14_dc.parquet

# AC: full solution
gat pf ac datasets/ieee14.arrow --out notebooks/ieee14_ac.parquet
```

## 3) Inspect and compare results
```bash
duckdb "SELECT * FROM 'notebooks/ieee14_ac.parquet' LIMIT 10"
```

## 4) Track follow-ups
- [ ] Re-run with thermal limits
- [ ] Capture violations summary
"#;

    content.to_string()
}

fn render_scenario_batch_demo() -> String {
    let content = r#"# Scenario + batch analysis

Use scenarios and batch execution to explore many cases in one sweep.

## 1) Author scenarios
Create a `scenarios.yaml` in `datasets/` describing load/generation tweaks.

## 2) Materialize inputs
```bash
gat scenarios materialize --spec datasets/scenarios.yaml --grid datasets/ieee14.arrow --out-dir datasets/runs
```

## 3) Execute in batch
```bash
gat batch pf --manifest datasets/runs/manifest.json --max-jobs 8 --threads 4 --out datasets/runs/results
```

## 4) Summarize violations
```bash
duckdb "SELECT scenario_id, COUNT(*) AS n_violations FROM read_parquet('datasets/runs/results/*.parquet') GROUP BY 1"
```

## Next ideas
- Switch to OPF with solver selection: `gat batch opf --solver highs ...`
- Keep congestion pivots in `context/`
"#;

    content.to_string()
}

fn render_rag_context_demo() -> String {
    let content = r#"# RAG + context building

Document discoveries and stash inputs for assistant-ready context.

## 1) Summarize the run
- Dataset: `datasets/ieee14.arrow`
- Commands:
  - `gat pf dc datasets/ieee14.arrow --out notebooks/ieee14_dc.parquet`
  - `gat pf ac datasets/ieee14.arrow --out notebooks/ieee14_ac.parquet`

## 2) Capture artifacts
```bash
cp notebooks/ieee14_ac.parquet context/latest_ac.parquet
cp datasets/scenarios.yaml context/scenarios.yaml
```

## 3) Create retrieval-ready notes
Use this section to keep bullet-point decisions, timestamps, and next questions.

## 4) Plan follow-up experiments
- [ ] Run N-1 screening with `gat nminus1 dc`
- [ ] Evaluate deliverability via `gat analytics deliverability`
"#;

    content.to_string()
}

fn attempt_open_browser(url: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        return Command::new("cmd")
            .args(["/C", "start", url])
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }

    #[cfg(target_os = "macos")]
    {
        return Command::new("open")
            .arg(url)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        return Command::new("xdg-open")
            .arg(url)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }

    #[allow(unreachable_code)]
    false
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
        assert!(!launch.opened_browser);

        let manifest = fs::read_to_string(&launch.manifest_path).unwrap();
        assert!(manifest.contains("gat-notebook"));
        assert!(manifest.contains("twinsong"));
        assert!(manifest.contains("Power flow walkthrough"));
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

    #[test]
    fn data_and_context_guides_are_materialized() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("ws");

        seed_workspace(&workspace).unwrap();

        let datasets_readme = fs::read_to_string(workspace.join("datasets/README.md")).unwrap();
        assert!(datasets_readme.contains("gat import matpower"));
        assert!(datasets_readme.contains("gat scenarios materialize"));

        let context_readme = fs::read_to_string(workspace.join("context/README.md")).unwrap();
        assert!(context_readme.contains("RAG workflows"));
        assert!(context_readme.contains("context/session_log.md"));
    }

    #[test]
    fn starter_notebook_is_materialized_once() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("ws");

        seed_workspace(&workspace).unwrap();
        let starter = workspace.join("notebooks/getting-started.md");
        assert!(starter.exists());

        let first = fs::read_to_string(&starter).unwrap();
        seed_workspace(&workspace).unwrap();
        let second = fs::read_to_string(&starter).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("Welcome to the GAT Notebook"));
    }

    #[test]
    fn demo_notebooks_cover_toolkit_flows() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("ws");

        seed_workspace(&workspace).unwrap();

        let power_flow =
            fs::read_to_string(workspace.join("notebooks/demos/power-flow.md")).unwrap();
        assert!(power_flow.contains("gat pf dc"));
        assert!(power_flow.contains("gat pf ac"));

        let scenario_batch =
            fs::read_to_string(workspace.join("notebooks/demos/scenario-batch.md")).unwrap();
        assert!(scenario_batch.contains("gat batch pf"));
        assert!(scenario_batch.contains("gat scenarios materialize"));

        let rag_context =
            fs::read_to_string(workspace.join("notebooks/demos/rag-context.md")).unwrap();
        assert!(rag_context.contains("gat nminus1 dc"));
        assert!(rag_context.contains("gat analytics deliverability"));
    }
}
