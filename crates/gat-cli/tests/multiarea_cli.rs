//! Integration tests for `gat analytics multiarea` command

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(feature = "full-io")]
fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(relative)
}

#[test]
fn test_multiarea_help() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["analytics", "multiarea", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Multi-area reliability"));
}

#[cfg(feature = "full-io")]
#[test]
fn test_multiarea_runs_with_sample_data() {
    let tmp = tempdir().unwrap();
    let areas_dir = tmp.path().join("areas");
    std::fs::create_dir(&areas_dir).unwrap();

    // Import two small networks as area 1 and area 2
    let case_file = repo_path("test_data/matpower/ieee14.case");

    // Create area 1
    let area1_path = areas_dir.join("1");
    let mut import1 = cargo_bin_cmd!("gat-cli");
    import1
        .args([
            "import",
            "matpower",
            "--m",
            case_file.to_str().unwrap(),
            "-o",
            area1_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Create area 2 (same network for simplicity)
    let area2_path = areas_dir.join("2");
    let mut import2 = cargo_bin_cmd!("gat-cli");
    import2
        .args([
            "import",
            "matpower",
            "--m",
            case_file.to_str().unwrap(),
            "-o",
            area2_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Create corridor definitions file
    let corridors_file = tmp.path().join("corridors.json");
    let corridors_json = r#"[
        {
            "id": 0,
            "area_a": 1,
            "area_b": 2,
            "capacity_mw": 100.0,
            "failure_rate": 0.01
        }
    ]"#;
    std::fs::write(&corridors_file, corridors_json).unwrap();

    // Run multiarea analysis
    let out_file = tmp.path().join("results.json");
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args([
        "analytics",
        "multiarea",
        "--areas-dir",
        areas_dir.to_str().unwrap(),
        "--corridors",
        corridors_file.to_str().unwrap(),
        "--samples",
        "10", // Small number for quick test
        "--out",
        out_file.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Multi-Area Reliability Results"));

    // Verify output file was created and is valid JSON
    assert!(out_file.exists());
    let output_json = std::fs::read_to_string(&out_file).unwrap();
    let output: serde_json::Value = serde_json::from_str(&output_json).unwrap();

    // Verify structure
    assert!(output.get("areas").is_some());
    assert!(output.get("system_lole").is_some());
    assert!(output.get("system_eue").is_some());
    assert!(output.get("samples").is_some());
}
