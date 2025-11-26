//! Performance benchmarks for Arrow I/O operations
//!
//! This benchmark suite measures the performance of:
//! - Loading networks from Arrow directory format
//! - Writing networks to Arrow directory format
//! - Validating Arrow manifests and checksums
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench -p gat-io
//!
//! # Run specific benchmark
//! cargo bench -p gat-io -- load_network
//!
//! # Generate HTML reports
//! cargo bench -p gat-io -- --save-baseline my-baseline
//! ```
//!
//! ## Performance Targets
//!
//! | Operation | 14 bus | 118 bus | 1354 bus | Target |
//! |-----------|--------|---------|----------|--------|
//! | Load | <5ms | <20ms | <100ms | <200ms for any |
//! | Write | <10ms | <50ms | <200ms | <500ms for any |
//! | Import MATPOWER | <10ms | <50ms | <200ms | <500ms for any |
//!
//! These targets are conservative and should be easily met on modern hardware.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gat_io::exporters::{open_arrow_directory, write_network_to_arrow_directory};
use gat_io::importers::parse_matpower;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test cases with different network sizes
struct TestCase {
    name: &'static str,
    matpower_file: &'static str,
    buses: usize,
}

impl TestCase {
    const fn new(name: &'static str, matpower_file: &'static str, buses: usize) -> Self {
        Self {
            name,
            matpower_file,
            buses,
        }
    }
}

const TEST_CASES: &[TestCase] = &[
    TestCase::new(
        "14_bus",
        "test_data/matpower/pglib/pglib_opf_case14_ieee.m",
        14,
    ),
    TestCase::new(
        "30_bus",
        "test_data/matpower/pglib/pglib_opf_case30_ieee.m",
        30,
    ),
    TestCase::new(
        "57_bus",
        "test_data/matpower/pglib/pglib_opf_case57_ieee.m",
        57,
    ),
    TestCase::new(
        "118_bus",
        "test_data/matpower/pglib/pglib_opf_case118_ieee.m",
        118,
    ),
    TestCase::new(
        "300_bus",
        "test_data/matpower/pglib/pglib_opf_case300_ieee.m",
        300,
    ),
    TestCase::new(
        "1354_bus",
        "test_data/matpower/pglib/pglib_opf_case1354_pegase.m",
        1354,
    ),
];

/// Benchmark: Import MATPOWER files of various sizes
fn bench_import_matpower(c: &mut Criterion) {
    let mut group = c.benchmark_group("import_matpower");

    for test_case in TEST_CASES {
        let path = PathBuf::from(test_case.matpower_file);

        // Skip if file doesn't exist (some test cases may not be available)
        if !path.exists() {
            eprintln!("Skipping {}: file not found", test_case.name);
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("parse", test_case.name),
            &path,
            |b, path| {
                b.iter(|| {
                    let result = parse_matpower(path.to_str().unwrap()).unwrap();
                    black_box(result.network)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Write networks to Arrow directory format
fn bench_write_arrow(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_arrow");

    for test_case in TEST_CASES {
        let path = PathBuf::from(test_case.matpower_file);

        // Skip if file doesn't exist
        if !path.exists() {
            continue;
        }

        // Pre-load the network
        let result = parse_matpower(path.to_str().unwrap()).unwrap();
        let network = result.network;

        group.bench_with_input(
            BenchmarkId::new("write", test_case.name),
            &network,
            |b, network| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let output_path = temp_dir.path().join("network");
                    write_network_to_arrow_directory(network, &output_path, None).unwrap();
                    black_box(output_path)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Read networks from Arrow directory format
fn bench_read_arrow(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_arrow");

    // Pre-create Arrow directories for each test case
    let temp_dir = TempDir::new().unwrap();
    let mut arrow_paths = Vec::new();

    for test_case in TEST_CASES {
        let matpower_path = PathBuf::from(test_case.matpower_file);

        // Skip if file doesn't exist
        if !matpower_path.exists() {
            continue;
        }

        // Import and write to Arrow
        let result = parse_matpower(matpower_path.to_str().unwrap()).unwrap();
        let arrow_path = temp_dir.path().join(test_case.name);
        write_network_to_arrow_directory(&result.network, &arrow_path, None).unwrap();

        arrow_paths.push((test_case.name, arrow_path));
    }

    // Benchmark reading
    for (name, arrow_path) in &arrow_paths {
        group.bench_with_input(BenchmarkId::new("read", name), arrow_path, |b, path| {
            b.iter(|| {
                let network = open_arrow_directory(path).unwrap();
                black_box(network)
            })
        });
    }

    group.finish();
}

/// Benchmark: Full roundtrip (MATPOWER -> Arrow -> Network)
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    for test_case in TEST_CASES {
        let path = PathBuf::from(test_case.matpower_file);

        // Skip if file doesn't exist
        if !path.exists() {
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("matpower_arrow_network", test_case.name),
            &path,
            |b, path| {
                b.iter(|| {
                    // Import from MATPOWER
                    let result = parse_matpower(path.to_str().unwrap()).unwrap();

                    // Write to Arrow
                    let temp_dir = TempDir::new().unwrap();
                    let arrow_path = temp_dir.path().join("network");
                    write_network_to_arrow_directory(&result.network, &arrow_path, None).unwrap();

                    // Read back from Arrow
                    let network = open_arrow_directory(&arrow_path).unwrap();
                    black_box(network)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Manifest validation (if available)
fn bench_manifest_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_validation");

    // Pre-create Arrow directories
    let temp_dir = TempDir::new().unwrap();
    let mut arrow_paths = Vec::new();

    for test_case in TEST_CASES {
        let matpower_path = PathBuf::from(test_case.matpower_file);

        if !matpower_path.exists() {
            continue;
        }

        let result = parse_matpower(matpower_path.to_str().unwrap()).unwrap();
        let arrow_path = temp_dir.path().join(test_case.name);
        write_network_to_arrow_directory(&result.network, &arrow_path, None).unwrap();

        arrow_paths.push((test_case.name, arrow_path));
    }

    // Benchmark manifest reading/validation
    for (name, arrow_path) in &arrow_paths {
        group.bench_with_input(BenchmarkId::new("validate", name), arrow_path, |b, path| {
            b.iter(|| {
                // Reading the network validates the manifest
                let network = open_arrow_directory(path).unwrap();
                black_box(network)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_import_matpower,
    bench_write_arrow,
    bench_read_arrow,
    bench_roundtrip,
    bench_manifest_validation
);
criterion_main!(benches);
