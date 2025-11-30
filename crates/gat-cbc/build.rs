//! Build script for gat-cbc.
//!
//! This script links against CBC libraries with the following priority:
//! 1. System CBC via pkg-config (recommended, most compatible)
//! 2. Pre-built libraries from target/coinor or vendor/local
//! 3. Build from vendored sources (fallback, may have compatibility issues)

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Try system CBC first via pkg-config (most reliable)
    if try_system_cbc() {
        return;
    }

    // Determine paths
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let prebuilt_dir = workspace_root.join("target/coinor");
    let vendor_dir = workspace_root.join("vendor");

    // Check for pre-built libraries
    // Priority: 1) target/coinor (from xtask), 2) vendor/local (from build-cbc.sh)
    let vendor_local_dir = workspace_root.join("vendor/local");

    for check_dir in [&prebuilt_dir, &vendor_local_dir] {
        if let Some(artifacts) = gat_coinor_build::find_prebuilt(check_dir) {
            // Check that CBC is actually built
            if artifacts.libraries.iter().any(|l| l == "Cbc") {
                println!(
                    "cargo:warning=Using pre-built CBC from {}",
                    check_dir.display()
                );
                emit_link_flags(&artifacts);
                return;
            }
        }
    }

    // Fallback: build from vendored sources
    println!("cargo:warning=Building CBC from vendored sources...");
    println!("cargo:warning=For faster builds, run: cargo xtask build-solvers --cbc");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let config = gat_coinor_build::CoinorBuildConfig {
        vendor_dir,
        build_dir: out_dir.join("coinor-build"),
        install_dir: out_dir.join("coinor"),
        components: gat_coinor_build::Component::cbc_deps().to_vec(),
    };

    let artifacts = gat_coinor_build::build(&config).expect("Failed to build CBC from source");
    emit_link_flags(&artifacts);
}

/// Try to link against system CBC via pkg-config.
fn try_system_cbc() -> bool {
    // Check if pkg-config can find cbc
    let output = Command::new("pkg-config")
        .args(["--libs", "--cflags", "cbc"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return false,
    };

    let flags = String::from_utf8_lossy(&output.stdout);
    println!("cargo:warning=Using system CBC via pkg-config");

    // Parse and emit the flags
    for flag in flags.split_whitespace() {
        if let Some(lib) = flag.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={}", lib);
        } else if let Some(path) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={}", path);
        }
    }

    // Link C++ standard library (COIN-OR is written in C++)
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");

    // Link additional dependencies that CoinUtils requires
    // (bz2/zlib for compressed file I/O, LAPACK/BLAS for dense factorization)
    println!("cargo:rustc-link-lib=bz2");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=lapack");
    println!("cargo:rustc-link-lib=blas");
    println!("cargo:rustc-link-lib=m");

    true
}

fn emit_link_flags(artifacts: &gat_coinor_build::BuildArtifacts) {
    println!(
        "cargo:rustc-link-search=native={}",
        artifacts.lib_dir.display()
    );

    // Link in reverse dependency order
    // Full chain: Cbc -> OsiCbc -> Cgl -> OsiClp -> Clp -> Osi -> CoinUtils
    // Note: OsiCbc and OsiClp are adapter libraries that connect CBC/CLP to the Osi interface
    println!("cargo:rustc-link-lib=static=Cbc");
    println!("cargo:rustc-link-lib=static=OsiCbc");
    println!("cargo:rustc-link-lib=static=Cgl");
    println!("cargo:rustc-link-lib=static=OsiClp");
    println!("cargo:rustc-link-lib=static=Clp");
    println!("cargo:rustc-link-lib=static=Osi");
    println!("cargo:rustc-link-lib=static=CoinUtils");

    // Link C++ standard library
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");

    // Link system libraries required by CoinUtils
    // - bz2: compressed file I/O
    // - z (zlib): compressed file I/O (gzopen, gzread, etc.)
    // - lapack/blas: dense linear algebra (dgetrf_, dgetrs_)
    // - readline: interactive parameter input (CoinParamUtils)
    println!("cargo:rustc-link-lib=bz2");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=lapack");
    println!("cargo:rustc-link-lib=blas");
    println!("cargo:rustc-link-lib=readline");
    println!("cargo:rustc-link-lib=m");
}
