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
                title: "Data validation and cleanup",
                description: "Validate topology, catch islands, and prepare a clean grid artifact for studies.",
                path: "notebooks/demos/validation.md",
            },
            Demo {
                title: "RAG + context building",
                description: "Curate context assets and summarize decisions for downstream assistants.",
                path: "notebooks/demos/rag-context.md",
            },
            Demo {
                title: "Experiment tracking + reproducibility",
                description: "Capture runs, decisions, and quick metadata for iterative research.",
                path: "notebooks/demos/research-tracking.md",
            },
            Demo {
                title: "Time-series and forecasting",
                description: "Run time-coupled OPF, stats, and forecasts with reusable Parquet outputs.",
                path: "notebooks/demos/time-series.md",
            },
            Demo {
                title: "Spatial joins and equity features",
                description: "Map buses to regions and featurize for planning or ML workflows.",
                path: "notebooks/demos/geo-features.md",
            },
            Demo {
                title: "Reliability, deliverability, and hosting",
                description: "Screen contingencies, capacity value, and DER hosting capacity in one loop.",
                path: "notebooks/demos/reliability-hosting.md",
            },
            Demo {
                title: "Contingency analysis + grid hardening",
                description: "Run N-1 screening, triage violations, and capture remediation ideas.",
                path: "notebooks/demos/contingency-resilience.md",
            },
            Demo {
                title: "Sensitivity sweeps + post-processing",
                description: "Vary load and limits, run sweeps, and summarize violatons by scenario.",
                path: "notebooks/demos/sensitivity-sweeps.md",
            },
            Demo {
                title: "Solver benchmarking + regression",
                description: "Compare OPF solvers, capture runtimes, and persist benchmarks for CI.",
                path: "notebooks/demos/solver-benchmarks.md",
            },
            Demo {
                title: "Data ingestion + format conversion",
                description: "Convert RAW/CIM inputs to Arrow, lint metadata, and prep shareable assets.",
                path: "notebooks/demos/data-ingestion.md",
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

    let research_tracking = path.join("notebooks/demos/research-tracking.md");
    write_if_absent(&research_tracking, render_research_tracking_demo())?;

    let validation = path.join("notebooks/demos/validation.md");
    write_if_absent(&validation, render_validation_demo())?;

    let time_series = path.join("notebooks/demos/time-series.md");
    write_if_absent(&time_series, render_time_series_demo())?;

    let geo_features = path.join("notebooks/demos/geo-features.md");
    write_if_absent(&geo_features, render_geo_features_demo())?;

    let reliability_hosting = path.join("notebooks/demos/reliability-hosting.md");
    write_if_absent(&reliability_hosting, render_reliability_hosting_demo())?;

    let contingency_resilience = path.join("notebooks/demos/contingency-resilience.md");
    write_if_absent(
        &contingency_resilience,
        render_contingency_resilience_demo(),
    )?;

    let sensitivity_sweeps = path.join("notebooks/demos/sensitivity-sweeps.md");
    write_if_absent(&sensitivity_sweeps, render_sensitivity_sweeps_demo())?;

    let solver_benchmarks = path.join("notebooks/demos/solver-benchmarks.md");
    write_if_absent(&solver_benchmarks, render_solver_benchmarks_demo())?;

    let data_ingestion = path.join("notebooks/demos/data-ingestion.md");
    write_if_absent(&data_ingestion, render_data_ingestion_demo())?;

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
- Explore the curated demos in `notebooks/demos/` for power flow, scenarios, time-series,
  spatial analysis, and reliability tooling.

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

Time-series friendly assets:
```bash
gat ts forecast --grid datasets/ieee14.arrow --historical datasets/hist.parquet --out datasets/forecast.parquet
gat ts solve --grid datasets/ieee14.arrow --timeseries datasets/forecast.parquet --out datasets/ts_results.parquet
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

# Run a short time-series slice
gat ts solve --grid datasets/ieee14.arrow --timeseries datasets/forecast.parquet --out notebooks/ts_results.parquet

# Map buses to a polygon layer
gat geo join --grid datasets/ieee14.arrow --polygons datasets/tracts.parquet --out datasets/bus_to_tract.parquet
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

fn render_validation_demo() -> String {
    let content = r#"# Data validation and cleanup

Keep the grid artifact healthy before running larger studies.

## 1) Validate schema + topology
```bash
gat validate --file datasets/ieee14.arrow --schema grid
gat graph stats datasets/ieee14.arrow
gat graph islands datasets/ieee14.arrow
gat graph connectivity datasets/ieee14.arrow
```

## 2) Catch limits + slack issues
```bash
gat validate --file datasets/ieee14.arrow --schema branch_limits
gat validate --file datasets/ieee14.arrow --schema bus_voltage
```

## 3) Regenerate a clean artifact
```bash
gat import psse --file data/ieee14.raw --out datasets/ieee14_clean.arrow
gat validate --file datasets/ieee14_clean.arrow --schema grid
```

## Notes
- [ ] Track fixes in `context/validation_log.md`
- [ ] Re-run after editing external CSV inputs
"#;

    content.to_string()
}

fn render_research_tracking_demo() -> String {
    let content = r#"# Experiment tracking + reproducibility

Keep quick notes, metadata, and run artifacts together while iterating.

## 1) Stamp a run folder
```bash
RUN=$(date +%Y%m%d-%H%M%S)
mkdir -p notebooks/runs/$RUN
```

## 2) Capture the command + outputs
```bash
gat pf dc datasets/ieee14.arrow --out notebooks/runs/$RUN/pf.parquet --limits datasets/limits.csv
md5sum datasets/ieee14.arrow > notebooks/runs/$RUN/input_checksums.txt
```

## 3) Log decisions + prompts
```bash
cat >> context/experiment_log.md << 'EOF'
## $RUN
- why: testing updated limits CSV
- command: gat pf dc datasets/ieee14.arrow --out notebooks/runs/$RUN/pf.parquet --limits datasets/limits.csv
- notes: rerun after refreshing DER interconnect data
EOF
```

## 4) Summarize quickly
```bash
duckdb "SELECT COUNT(*) AS branches, SUM(overload) AS total_violation FROM read_parquet('notebooks/runs/$RUN/pf.parquet')"
```

## Follow-ups
- [ ] Commit `context/experiment_log.md` as a checkpoint
- [ ] Copy timings into `context/run_metadata.csv`
"#;

    content.to_string()
}

fn render_time_series_demo() -> String {
    let content = r#"# Time-series and forecasting

Tie together forecasting, time-coupled OPF, and rolling statistics with reusable Parquet outputs.

## 1) Forecast loads or renewables
```bash
gat ts forecast --grid datasets/ieee14.arrow --historical datasets/hist.parquet --out datasets/forecast.parquet
```

## 2) Solve time-series OPF
```bash
gat ts solve --grid datasets/ieee14.arrow --timeseries datasets/forecast.parquet --out notebooks/ts_results.parquet
```

## 3) Summaries and pivots
```bash
duckdb "SELECT hour, SUM(load_mw) FROM read_parquet('notebooks/ts_results.parquet') GROUP BY 1 ORDER BY 1"

gat ts stats --timeseries notebooks/ts_results.parquet --window 24h --out notebooks/ts_stats.parquet
```

## Follow-ups
- [ ] Compare solver choices on a subset of hours
- [ ] Export key findings to `context/ts_notes.md`
"#;

    content.to_string()
}

fn render_sensitivity_sweeps_demo() -> String {
    let content = r#"# Sensitivity sweeps + post-processing

Vary inputs, sweep scenarios, and summarize violations per case.

## 1) Generate scenario variations
```bash
cat > datasets/sweeps.yaml << 'EOF'
grid: datasets/ieee14.arrow
scenarios:
  - name: load_up_5
    load_multiplier: 1.05
  - name: load_down_5
    load_multiplier: 0.95
  - name: limit_tight
    branch_limit_multiplier: 0.9
EOF
gat scenarios materialize --spec datasets/sweeps.yaml --out-dir datasets/sweeps
```

## 2) Run batch flows
```bash
gat batch pf --manifest datasets/sweeps/manifest.json --max-jobs 8 --threads 4 --out notebooks/sweeps
```

## 3) Post-process
```bash
duckdb "SELECT scenario, SUM(overload) AS overload_mw FROM read_parquet('notebooks/sweeps/*.parquet') GROUP BY 1 ORDER BY overload_mw DESC"
```

## 4) Track diffs
```bash
ls notebooks/sweeps > context/sweep_runs.txt
```

## Next ideas
- [ ] Add reactive power sensitivity by adjusting generator Q limits
- [ ] Plot overload distribution per scenario
"#;

    content.to_string()
}

fn render_geo_features_demo() -> String {
    let content = r#"# Spatial joins and equity features

Use spatial joins to connect network assets with external data for equity or planning.

## 1) Map buses to geography
```bash
gat geo join --grid datasets/ieee14.arrow --polygons datasets/tracts.parquet --method point_in_polygon --out datasets/bus_to_tract.parquet
```

## 2) Featurize with time-series results
```bash
gat geo featurize --mapping datasets/bus_to_tract.parquet --timeseries notebooks/ts_results.parquet --lags 1,24,168 --windows 24,168 --seasonal true --out notebooks/tract_features.parquet
```

## 3) Blend with demographics
```bash
duckdb "SELECT tract_id, load_mw, median_income FROM read_parquet('notebooks/tract_features.parquet') LIMIT 10"
```

## Notes
- [ ] Keep joined tables in `context/` for later RAG retrieval
- [ ] Try alternative polygon sources (utility districts, zip codes)
"#;

    content.to_string()
}

fn render_reliability_hosting_demo() -> String {
    let content = r#"# Reliability, deliverability, and hosting

Bundle contingency screening, deliverability, and DER hosting capacity to stress-test a case.

## 1) N-1 screening
```bash
gat nminus1 dc datasets/ieee14.arrow --spec datasets/contingencies.yaml --out notebooks/nminus1.parquet
```

## 2) Deliverability and ELCC
```bash
gat analytics deliverability --grid datasets/ieee14.arrow --assets datasets/critical_loads.csv --out notebooks/deliverability.parquet
gat analytics elcc --grid datasets/ieee14.arrow --scenarios 200 --out notebooks/elcc.parquet
```

## 3) Hosting capacity for DER planning
```bash
gat derms hosting-capacity --grid datasets/ieee14.arrow --der-type solar --voltage-band 0.95,1.05 --penetration-max 5.0 --out notebooks/hosting.parquet
```

## Decisions
- [ ] Track worst-case outages
- [ ] Capture summary tables in `context/reliability_notes.md`
"#;

    content.to_string()
}

fn render_contingency_resilience_demo() -> String {
    let content = r#"# Contingency analysis + grid hardening

Quickly scan N-1 cases, catch overloads, and note corrective options.

## 1) Define contingencies
Create `datasets/contingencies.yaml` with branch and generator outages.

## 2) Screen in bulk
```bash
gat nminus1 dc datasets/ieee14.arrow --spec datasets/contingencies.yaml --out notebooks/nminus1.parquet
duckdb "SELECT contingency_id, COUNT(*) AS n_violations FROM 'notebooks/nminus1.parquet' GROUP BY 1 ORDER BY 2 DESC"
```

## 3) Re-run with OPF and limits
```bash
gat opf dc datasets/ieee14.arrow --limits datasets/limits.csv --out notebooks/base_opf.parquet
gat nminus1 dc datasets/ieee14.arrow --spec datasets/contingencies.yaml --limits datasets/limits.csv --out notebooks/nminus1_limited.parquet
```

## 4) Capture fixes
- [ ] Note candidate re-dispatch strategies in `context/hardening.md`
- [ ] Keep violation pivots under `context/nminus1_summary.csv`
- [ ] Tag follow-up "storm hardening" scenarios for batch reruns
"#;

    content.to_string()
}

fn render_data_ingestion_demo() -> String {
    let content = r#"# Data ingestion + format conversion

Convert common formats to Arrow, lint metadata, and stage assets for collaboration.

## 1) Convert RAW/PSS/E/CIM to Arrow
```bash
gat import matpower --file data/ieee14.raw --out datasets/ieee14.arrow
gat import psse --file data/ieee14.raw --out datasets/ieee14_psse.arrow
gat import cim --file data/network.rdf --out datasets/cim.arrow
```

## 2) Validate inputs and metadata
```bash
gat validate --file datasets/ieee14.arrow --schema grid
gat validate --file datasets/ieee14.arrow --schema branch_limits
gat graph stats datasets/ieee14.arrow
```

## 3) Produce shareable parquet summaries
```bash
gat graph stats datasets/ieee14.arrow --out datasets/grid_stats.parquet
duckdb "COPY (SELECT * FROM read_parquet('datasets/grid_stats.parquet')) TO 'context/grid_stats.csv' (FORMAT CSV)"
```

## 4) Track provenance
- [ ] Record source filenames and hashes in `context/ingestion_log.md`
- [ ] Keep validated Arrow copies under `datasets/validated/`
"#;

    content.to_string()
}

fn render_solver_benchmarks_demo() -> String {
    let content = r#"# Solver benchmarking + regression

Compare solver choices for OPF runs and keep timing snapshots for later CI checks.

## 1) Prepare inputs
```bash
gat import matpower --file data/ieee14.raw --out datasets/ieee14.arrow
gat opf dc datasets/ieee14.arrow --solver highs --cost datasets/costs.csv --out notebooks/opf_highs.parquet
```

## 2) Benchmark alternatives
```bash
gat opf dc datasets/ieee14.arrow --solver clarabel --cost datasets/costs.csv --out notebooks/opf_clarabel.parquet
gat opf dc datasets/ieee14.arrow --solver ipopt --cost datasets/costs.csv --out notebooks/opf_ipopt.parquet
```

## 3) Capture timings
```bash
/usr/bin/time -v gat opf dc datasets/ieee14.arrow --solver highs --cost datasets/costs.csv --out notebooks/bench_highs.parquet
```

## 4) Summarize results
```bash
duckdb "SELECT solver, COUNT(*) AS n_records FROM read_parquet('notebooks/opf_*.parquet') GROUP BY 1"
```

## Follow-ups
- [ ] Store timing tables in `context/solver_benchmarks.csv`
- [ ] Re-run benchmarks after upgrading dependencies
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
        assert!(manifest.contains("Data validation and cleanup"));
        assert!(manifest.contains("Experiment tracking + reproducibility"));
        assert!(manifest.contains("Time-series and forecasting"));
        assert!(manifest.contains("Reliability, deliverability, and hosting"));
        assert!(manifest.contains("Contingency analysis + grid hardening"));
        assert!(manifest.contains("Sensitivity sweeps + post-processing"));
        assert!(manifest.contains("Solver benchmarking + regression"));
        assert!(manifest.contains("Data ingestion + format conversion"));
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

        let research_tracking = fs::read_to_string(
            workspace.join("notebooks/demos/research-tracking.md"),
        )
        .unwrap();
        assert!(research_tracking.contains("experiment_log"));
        assert!(research_tracking.contains("md5sum"));

        let validation = fs::read_to_string(workspace.join("notebooks/demos/validation.md"))
            .unwrap();
        assert!(validation.contains("gat graph islands"));
        assert!(validation.contains("gat validate"));

        let time_series = fs::read_to_string(workspace.join("notebooks/demos/time-series.md")).unwrap();
        assert!(time_series.contains("gat ts solve"));
        assert!(time_series.contains("gat ts stats"));

        let geo = fs::read_to_string(workspace.join("notebooks/demos/geo-features.md")).unwrap();
        assert!(geo.contains("gat geo join"));
        assert!(geo.contains("gat geo featurize"));

        let reliability =
            fs::read_to_string(workspace.join("notebooks/demos/reliability-hosting.md")).unwrap();
        assert!(reliability.contains("gat derms hosting-capacity"));
        assert!(reliability.contains("gat analytics elcc"));

        let contingency = fs::read_to_string(
            workspace.join("notebooks/demos/contingency-resilience.md"),
        )
        .unwrap();
        assert!(contingency.contains("gat nminus1 dc"));
        assert!(contingency.contains("gat opf dc"));

        let sensitivity_sweeps = fs::read_to_string(
            workspace.join("notebooks/demos/sensitivity-sweeps.md"),
        )
        .unwrap();
        assert!(sensitivity_sweeps.contains("scenarios materialize"));
        assert!(sensitivity_sweeps.contains("batch pf"));

        let solver_benchmarks = fs::read_to_string(
            workspace.join("notebooks/demos/solver-benchmarks.md"),
        )
        .unwrap();
        assert!(solver_benchmarks.contains("gat opf dc"));
        assert!(solver_benchmarks.contains("/usr/bin/time"));

        let data_ingestion =
            fs::read_to_string(workspace.join("notebooks/demos/data-ingestion.md")).unwrap();
        assert!(data_ingestion.contains("gat import"));
        assert!(data_ingestion.contains("gat validate"));
    }
}
