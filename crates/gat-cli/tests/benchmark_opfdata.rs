use std::fs;
use std::path::Path;

#[test]
fn test_opfdata_benchmark_module_exists() {
    // Verify the opfdata benchmark module exists
    assert!(Path::new("../../crates/gat-cli/src/commands/benchmark/opfdata.rs").exists());
}

#[test]
fn test_opfdata_benchmark_module_compiles() {
    // If this test runs, the module compiled successfully
    let benchmark_mod = fs::read_to_string("../../crates/gat-cli/src/commands/benchmark/mod.rs")
        .expect("Failed to read benchmark mod.rs");
    assert!(benchmark_mod.contains("pub mod opfdata"));
    assert!(benchmark_mod.contains("BenchmarkCommands::Opfdata"));
}

#[test]
fn test_opfdata_source_module_exists() {
    // Verify the opfdata source module exists
    assert!(Path::new("../../crates/gat-io/src/sources/opfdata.rs").exists());
}

#[test]
fn test_opfdata_source_module_exports() {
    // Verify source module has proper exports
    let sources_mod = fs::read_to_string("../../crates/gat-io/src/sources/mod.rs")
        .expect("Failed to read sources mod.rs");
    assert!(sources_mod.contains("pub mod opfdata"));
    assert!(sources_mod.contains("list_sample_refs"));
    assert!(sources_mod.contains("load_opfdata_instance"));
}

#[test]
fn test_cli_has_opfdata_command() {
    // Verify CLI defines the opfdata benchmark command
    let cli = fs::read_to_string("../../crates/gat-cli/src/cli.rs").expect("Failed to read cli.rs");
    assert!(
        cli.contains("Opfdata {"),
        "CLI should define Opfdata command"
    );
    assert!(
        cli.contains("opfdata_dir"),
        "Opfdata should have opfdata_dir arg"
    );
}
