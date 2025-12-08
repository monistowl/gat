//! Integration tests for `gat tep` command

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn test_tep_help() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["tep", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Transmission expansion planning"));
}

#[test]
fn test_tep_solve_help() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["tep", "solve", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("network"))
        .stdout(predicate::str::contains("candidates"));
}

#[test]
fn test_tep_validate_help() {
    let mut cmd = cargo_bin_cmd!("gat-cli");
    cmd.args(["tep", "validate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("candidates"));
}
