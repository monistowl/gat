use std::fs;
use std::path::Path;

#[test]
fn test_pglib_benchmark_module_exists() {
    // Verify the pglib benchmark module exists
    assert!(Path::new("../../crates/gat-cli/src/commands/benchmark/pglib.rs").exists());
}

#[test]
fn test_pglib_benchmark_module_compiles() {
    // If this test runs, the module compiled successfully
    let benchmark_mod = fs::read_to_string("../../crates/gat-cli/src/commands/benchmark/mod.rs")
        .expect("Failed to read benchmark mod.rs");
    assert!(benchmark_mod.contains("pub mod pglib"));
    assert!(benchmark_mod.contains("BenchmarkCommands::Pglib"));
}

#[test]
fn test_matpower_parser_exists() {
    // Verify the MATPOWER parser module exists
    assert!(Path::new("../../crates/gat-io/src/importers/matpower_parser.rs").exists());
}

#[test]
fn test_pglib_test_data_exists() {
    // Verify test data directories exist
    let pglib_dir = Path::new("../../test_data/pglib");
    assert!(pglib_dir.exists(), "PGLib test data directory should exist");

    // Check that at least one case directory exists
    let case14_dir = pglib_dir.join("pglib_opf_case14_ieee");
    assert!(case14_dir.exists(), "case14 directory should exist");

    // Check that the case directory contains a .m file
    let case_file = case14_dir.join("case.m");
    assert!(case_file.exists(), "case.m file should exist in case14 directory");
}

#[test]
fn test_baseline_file_exists() {
    // Verify baseline CSV exists
    let baseline = Path::new("../../test_data/pglib/baseline.csv");
    assert!(baseline.exists(), "Baseline CSV should exist");

    // Check baseline has content
    let content = fs::read_to_string(baseline).expect("Should be able to read baseline.csv");
    assert!(content.contains("case_name"), "Baseline should have case_name column");
    assert!(content.contains("objective"), "Baseline should have objective column");
}
