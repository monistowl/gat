use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, to_string_pretty};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use uuid::Uuid;

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(relative)
}

#[test]
fn gat_ts_resample_runs() {
    let out_dir = tempdir().unwrap();
    let out = out_dir.path().join("resampled.parquet");
    let input = repo_path("test_data/ts/telemetry.parquet");
    let mut cmd = Command::cargo_bin("gat-cli").unwrap();
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

#[test]
fn gat_ts_agg_runs() {
    let out_dir = tempdir().unwrap();
    let out = out_dir.path().join("aggregated.parquet");
    let input = repo_path("test_data/ts/telemetry.parquet");
    let mut cmd = Command::cargo_bin("gat-cli").unwrap();
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

#[test]
fn gat_import_and_pf_dc_runs() {
    let tmp = tempdir().unwrap();
    let arrow_path = tmp.path().join("case.arrow");
    let pf_out = tmp.path().join("dc.parquet");
    let case_file = repo_path("test_data/matpower/ieee14.case");

    let mut import = Command::cargo_bin("gat-cli").unwrap();
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

    let mut pf = Command::cargo_bin("gat-cli").unwrap();
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

    let mut cmd = Command::cargo_bin("gat-cli").unwrap();
    cmd.args(["runs", "resume", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manifest"))
        .stdout(predicate::str::contains("grid.arrow"));
}

#[test]
fn gat_dataset_rts_gmlc_fetches_files() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("rts");
    let mut cmd = Command::cargo_bin("gat-cli").unwrap();
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
    let mut list_cmd = Command::cargo_bin("gat-cli").unwrap();
    list_cmd
        .args(["dataset", "hiren", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("case_big"));
    let mut fetch_cmd = Command::cargo_bin("gat-cli").unwrap();
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
