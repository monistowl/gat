//! Integration tests for power flow output format options

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
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
fn test_dc_pf_output_format_parquet() {
    let tmp = tempdir().unwrap();
    let arrow_path = tmp.path().join("case.arrow");
    let out = tmp.path().join("flows.parquet");
    let case_file = repo_path("test_data/matpower/ieee14.case");

    // Import test case to Arrow format
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

    // Run DC power flow with explicit Parquet output format
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "pf",
        "dc",
        arrow_path.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--output-format",
        "parquet",
    ])
    .assert()
    .success();

    // Check that output file exists and is a valid Parquet file
    assert!(out.exists(), "Parquet output should exist");
    assert!(
        out.metadata().unwrap().len() > 0,
        "Parquet should have content"
    );
}

#[test]
fn test_dc_pf_output_format_json() {
    let tmp = tempdir().unwrap();
    let arrow_path = tmp.path().join("case.arrow");
    let out = tmp.path().join("flows.json");
    let case_file = repo_path("test_data/matpower/ieee14.case");

    // Import test case
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

    // Run DC power flow with JSON output format
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "pf",
        "dc",
        arrow_path.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--output-format",
        "json",
    ])
    .assert()
    .success();

    // Check that output file exists and is valid JSON
    assert!(out.exists(), "JSON output should exist");
    let content = fs::read_to_string(&out).unwrap();
    assert!(
        content.contains("branch_id"),
        "JSON should contain branch_id"
    );
    assert!(content.contains("flow_mw"), "JSON should contain flow_mw");

    // Verify it's valid JSON
    let _parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[test]
fn test_dc_pf_output_format_csv() {
    let tmp = tempdir().unwrap();
    let arrow_path = tmp.path().join("case.arrow");
    let out = tmp.path().join("flows.csv");
    let case_file = repo_path("test_data/matpower/ieee14.case");

    // Import test case
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

    // Run DC power flow with CSV output format
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "pf",
        "dc",
        arrow_path.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--output-format",
        "csv",
    ])
    .assert()
    .success();

    // Check that output file exists and is valid CSV
    assert!(out.exists(), "CSV output should exist");
    let content = fs::read_to_string(&out).unwrap();
    assert!(
        content.contains("branch_id"),
        "CSV should contain branch_id header"
    );
    assert!(
        content.contains("flow_mw"),
        "CSV should contain flow_mw header"
    );

    // Verify CSV has multiple lines
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() > 1, "CSV should have header + data rows");
}

#[test]
fn test_dc_pf_output_format_default_parquet() {
    let tmp = tempdir().unwrap();
    let arrow_path = tmp.path().join("case.arrow");
    let out = tmp.path().join("flows.parquet");
    let case_file = repo_path("test_data/matpower/ieee14.case");

    // Import test case
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

    // Test without --output-format flag (should default to parquet)
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "pf",
        "dc",
        arrow_path.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ])
    .assert()
    .success();

    assert!(out.exists(), "Default output should be Parquet");
}

#[test]
fn test_dc_pf_help_shows_output_format() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["pf", "dc", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--output-format"));
}
