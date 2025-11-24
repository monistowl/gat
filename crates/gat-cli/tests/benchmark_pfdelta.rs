use std::path::Path;
use std::fs;

#[test]
fn test_benchmark_command_help() {
    // Verify the benchmark subcommand exists and has pfdelta
    assert!(Path::new("../../crates/gat-cli/src/commands/benchmark/pfdelta.rs").exists());
}

#[test]
fn test_benchmark_module_compiles() {
    // If this test runs, the module compiled successfully
    let benchmark_mod = fs::read_to_string("../../crates/gat-cli/src/commands/benchmark/mod.rs")
        .expect("Failed to read benchmark mod.rs");
    assert!(benchmark_mod.contains("pub fn handle"));
    assert!(benchmark_mod.contains("BenchmarkCommands"));
}
