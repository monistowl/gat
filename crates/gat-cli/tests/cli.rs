use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::{json, to_string_pretty};
use std::fs;
#[cfg(feature = "full-io")]
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;

#[cfg(feature = "full-io")]
fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(relative)
}

#[cfg(feature = "full-io")]
fn import_ieee14_arrow(tmp: &Path) -> PathBuf {
    let arrow_path = tmp.join("case.arrow");
    let case_file = repo_path("test_data/matpower/ieee14.case");
    // Import the canonical IEEE-14 case into Arrow so downstream CLI commands have a stable grid input.
    let mut import = cargo_bin_cmd!("gat-cli");
    import
        .args([
            "import",
            "matpower",
            "--m",
            case_file.to_str().unwrap(),
            "-o",
            arrow_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    arrow_path
}

#[cfg(feature = "full-io")]
#[test]
fn gat_ts_resample_runs() {
    let out_dir = tempdir().unwrap();
    let out = out_dir.path().join("resampled.parquet");
    let input = repo_path("test_data/ts/telemetry.parquet");
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "ts",
        "resample",
        input.to_str().unwrap(),
        "--rule",
        "5s",
        "-o",
        out.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Resampling"));
    assert!(out.exists());
}

#[cfg(feature = "full-io")]
#[test]
fn gat_ts_agg_runs() {
    let out_dir = tempdir().unwrap();
    let out = out_dir.path().join("aggregated.parquet");
    let input = repo_path("test_data/ts/telemetry.parquet");
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "ts",
        "agg",
        input.to_str().unwrap(),
        "--group",
        "sensor",
        "--value",
        "value",
        "--agg",
        "sum",
        "-o",
        out.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Aggregating"));
    assert!(out.exists());
}

#[cfg(feature = "full-io")]
#[test]
fn gat_import_and_pf_dc_runs() {
    let tmp = tempdir().unwrap();
    let arrow_path = tmp.path().join("case.arrow");
    let pf_out = tmp.path().join("dc.parquet");
    let case_file = repo_path("test_data/matpower/ieee14.case");

    let mut import = cargo_bin_cmd!("gat-cli");
    import
        .args([
            "import",
            "matpower",
            "--m",
            case_file.to_str().unwrap(),
            "-o",
            arrow_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Importing MATPOWER"));
    assert!(arrow_path.exists());

    let mut pf = cargo_bin_cmd!("gat-cli");
    pf.args([
        "pf",
        "dc",
        arrow_path.to_str().unwrap(),
        "--out",
        pf_out.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("DC power flow summary"));
    assert!(pf_out.exists());
}

#[cfg(feature = "full-io")]
#[test]
fn gat_analytics_ptdf_runs() {
    let tmp = tempdir().unwrap();
    let arrow = import_ieee14_arrow(tmp.path());
    let out = tmp.path().join("ptdf.parquet");
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "analytics",
        "ptdf",
        arrow.to_str().unwrap(),
        "--source",
        "1",
        "--sink",
        "2",
        "-o",
        out.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("PTDF analysis"))
    .stdout(predicate::str::contains("persisted"));
    assert!(out.exists());
}

#[test]
fn gat_runs_resume_displays_manifest() {
    let entry = json!({
        "run_id": Uuid::new_v4().to_string(),
        "command": "pf dc",
        "version": "0.1.0",
        "timestamp": "1980-01-01T00:00:00Z",
        "outputs": ["/tmp/fake.parquet"],
        "params": [
            {"name": "grid_file", "value": "grid.arrow"},
            {"name": "out", "value": "/tmp/fake.parquet"},
            {"name": "threads", "value": "auto"}
        ]
    });
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("run.json");
    fs::write(&path, to_string_pretty(&entry).unwrap()).unwrap();

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["runs", "resume", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manifest"))
        .stdout(predicate::str::contains("grid.arrow"));
}

#[test]
fn gat_runs_list_reports_manifests() {
    let entry = json!({
        "run_id": Uuid::new_v4().to_string(),
        "command": "pf ac",
        "version": "0.1.0",
        "timestamp": "1980-01-01T00:00:00Z",
        "outputs": ["/tmp/dc.parquet"],
        "params": []
    });
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("run.json");
    fs::write(&path, to_string_pretty(&entry).unwrap()).unwrap();

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["runs", "list", "--root", tmp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("RUN ID"))
        .stdout(predicate::str::contains("pf ac"));
}

#[test]
fn gat_runs_describe_accepts_run_id_alias() {
    let run_id = Uuid::new_v4().to_string();
    let entry = json!({
        "run_id": run_id,
        "command": "pf dc",
        "version": "0.1.0",
        "timestamp": "1980-01-01T00:00:00Z",
        "outputs": ["/tmp/dc.parquet"],
        "params": []
    });
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("run-0.json");
    fs::write(&path, to_string_pretty(&entry).unwrap()).unwrap();

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "runs",
        "describe",
        run_id.as_str(),
        "--root",
        tmp.path().to_str().unwrap(),
        "--format",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"command\""))
    .stdout(predicate::str::contains("pf dc"));
}

#[test]
fn gat_dataset_rts_gmlc_fetches_files() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("rts");
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "dataset",
        "rts-gmlc",
        "fetch",
        "--out",
        out.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("RTS-GMLC dataset ready"));
    assert!(out.join("grid.matpower").exists());
    assert!(out.join("timeseries.csv").exists());
}

#[test]
fn gat_dataset_hiren_list_and_fetch() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("hiren");
    let mut list_cmd = cargo_bin_cmd!("gat-cli");
    list_cmd
        .args(["dataset", "hiren", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("case_big"));
    let mut fetch_cmd = cargo_bin_cmd!("gat-cli");
    fetch_cmd
        .args([
            "dataset",
            "hiren",
            "fetch",
            "case_big",
            "--out",
            out.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("HIREN case case_big copied"));
    assert!(out.join("case_big.matpower").exists());
}

#[test]
fn gat_dataset_public_list_shows_catalog() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    // Verifies that the embedded public dataset catalog prints the expected IDs without spinning up downloads.
    cmd.args(["dataset", "public", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("opsd-time-series-2020"))
        .stdout(predicate::str::contains("airtravel"));
}

#[test]
fn gat_dataset_public_list_filters_by_tag() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    // Tag filtering ensures the lightweight catalog list remains queryable.
    cmd.args(["dataset", "public", "list", "--tag", "tutorial"])
        .assert()
        .success()
        .stdout(predicate::str::contains("airtravel"))
        .stdout(predicate::str::contains("opsd-time-series-2020").not());
}

#[test]
fn gat_dataset_public_describe_outputs_metadata() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    // Describe ensures the CLI surfaces the full metadata before triggering downloads.
    cmd.args(["dataset", "public", "describe", "airtravel"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dataset: airtravel"))
        .stdout(predicate::str::contains("Tags: time-series, tutorial"));
}

#[cfg(feature = "full-io")]
#[test]
fn gat_graph_stats_runs() {
    let tmp = tempdir().unwrap();
    let arrow = import_ieee14_arrow(tmp.path());

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["graph", "stats", arrow.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Graph statistics"))
        .stdout(predicate::str::contains("Nodes"));
}

#[cfg(feature = "full-io")]
#[test]
fn gat_graph_islands_runs() {
    let tmp = tempdir().unwrap();
    let arrow = import_ieee14_arrow(tmp.path());

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["graph", "islands", arrow.to_str().unwrap(), "--emit"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Island"))
        .stdout(predicate::str::contains("Node"));
}

#[test]
fn gat_completions_outputs_script() {
    cargo_bin_cmd!("gat-cli")
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_gat()"));
}

#[cfg(feature = "full-io")]
#[test]
fn gat_graph_export_writes_file() {
    let tmp = tempdir().unwrap();
    let arrow = import_ieee14_arrow(tmp.path());
    let out_file = tmp.path().join("topo.dot");

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "graph",
        "export",
        arrow.to_str().unwrap(),
        "--format",
        "graphviz",
        "--out",
        out_file.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Graph exported to"));
    let contents = fs::read_to_string(&out_file).unwrap();
    assert!(contents.contains("graph"));
}

#[cfg(all(feature = "full-io", feature = "viz"))]
#[test]
fn gat_graph_visualize_writes_file() {
    let tmp = tempdir().unwrap();
    let arrow = import_ieee14_arrow(tmp.path());
    let out = tmp.path().join("layout.json");

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "graph",
        "visualize",
        arrow.to_str().unwrap(),
        "--iterations",
        "20",
        "--out",
        out.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Layout written to"));
    let text = fs::read_to_string(&out).unwrap();
    assert!(text.contains("\"nodes\""));
}

#[cfg(feature = "full-io")]
#[test]
fn gat_scenarios_validate_runs() {
    let tmp = tempdir().unwrap();
    let spec_path = tmp.path().join("scenario.yaml");
    let scenario_content = r#"
version: 1
grid_file: "grid.arrow"
defaults:
  time_slices:
    - "2025-01-01T00:00:00Z"
scenarios:
  - scenario_id: "base"
    description: "base case"
"#;
    fs::write(&spec_path, scenario_content).unwrap();

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "scenarios",
        "validate",
        "--spec",
        spec_path.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Scenario spec validated successfully"));
}

#[cfg(all(feature = "full-io", feature = "viz"))]
#[test]
fn gat_graph_visualize_prints_json() {
    let tmp = tempdir().unwrap();
    let arrow = import_ieee14_arrow(tmp.path());

    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "graph",
        "visualize",
        arrow.to_str().unwrap(),
        "--iterations",
        "10",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"edges\""))
    .stdout(predicate::str::contains("\"nodes\""));
}
