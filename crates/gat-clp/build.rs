//! Build script for gat-clp.
//!
//! This script links against pre-built COIN-OR CLP libraries if available,
//! or builds them from vendored sources as a fallback.

use gat_coinor_build::{CoinorBuildConfig, Component};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Determine paths
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let prebuilt_dir = workspace_root.join("target/coinor");
    let vendor_dir = workspace_root.join("vendor");

    // Check for pre-built libraries first (from `cargo xtask build-solvers`)
    if let Some(artifacts) = gat_coinor_build::find_prebuilt(&prebuilt_dir) {
        println!("cargo:warning=Using pre-built CLP from {}", prebuilt_dir.display());

        // Check that CLP is actually built
        if artifacts.libraries.iter().any(|l| l == "Clp") {
            emit_link_flags(&artifacts);
            return;
        }
        println!("cargo:warning=Pre-built libraries found but missing CLP, building from source...");
    }

    // Fallback: build from vendored sources
    println!("cargo:warning=Building CLP from vendored sources...");
    println!("cargo:warning=For faster builds, run: cargo xtask build-solvers --clp");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let config = CoinorBuildConfig {
        vendor_dir,
        build_dir: out_dir.join("coinor-build"),
        install_dir: out_dir.join("coinor"),
        components: Component::clp_deps().to_vec(),
    };

    let artifacts = gat_coinor_build::build(&config).expect("Failed to build CLP from source");
    emit_link_flags(&artifacts);
}

fn emit_link_flags(artifacts: &gat_coinor_build::BuildArtifacts) {
    println!("cargo:rustc-link-search=native={}", artifacts.lib_dir.display());

    // Link in reverse dependency order (Clp depends on Osi depends on CoinUtils)
    println!("cargo:rustc-link-lib=static=Clp");
    println!("cargo:rustc-link-lib=static=Osi");
    println!("cargo:rustc-link-lib=static=CoinUtils");

    // Link C++ standard library
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");

    // Link math library
    println!("cargo:rustc-link-lib=m");
}
