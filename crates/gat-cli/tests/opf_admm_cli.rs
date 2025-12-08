//! Integration tests for `gat opf admm` command

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn test_opf_admm_help() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["opf", "admm", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ADMM"));
}

#[test]
fn test_opf_admm_requires_grid_file() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["opf", "admm"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("GRID_FILE"));
}
