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
