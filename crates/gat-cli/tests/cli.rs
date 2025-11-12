use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::tempdir;

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
