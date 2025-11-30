//! Build script for gat-ipopt-sys.
//!
//! Links against IPOPT with the following priority:
//! 1. Pre-built libraries from vendor/local (preferred for CI reproducibility)
//! 2. System IPOPT via pkg-config (fallback for user convenience)
//!
//! # Installing IPOPT
//!
//! Build from vendored sources (preferred):
//! ```sh
//! cargo xtask build-solvers --ipopt
//! ```
//!
//! Or install system package (fallback):
//! - Ubuntu: sudo apt install coinor-libipopt-dev
//! - macOS: brew install ipopt

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Determine paths for vendor/local (preferred)
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let vendor_local = workspace_root.join("vendor/local");

    // Try vendor/local FIRST (from pre-built or xtask build)
    // This ensures CI builds use our vendored IPOPT for reproducibility
    if try_vendor_local(&vendor_local) {
        return;
    }

    // Fallback to system IPOPT via pkg-config (user convenience)
    if try_system_ipopt() {
        return;
    }

    // No IPOPT found
    panic!(
        "IPOPT not found!\n\n\
         Options to install IPOPT:\n\
         1. Install system package:\n\
            - Ubuntu: sudo apt install coinor-libipopt-dev\n\
            - macOS: brew install ipopt\n\
         2. Run: cargo xtask build-solvers --ipopt\n"
    );
}

/// Try to link against vendor/local pre-built IPOPT.
fn try_vendor_local(vendor_local: &PathBuf) -> bool {
    let lib_dir = vendor_local.join("lib");
    let include_dir = vendor_local.join("include/coin-or");

    // Check if IPOPT library exists
    let has_shared = lib_dir.join("libipopt.so").exists();
    let has_static = lib_dir.join("libipopt.a").exists();

    if !has_shared && !has_static {
        return false;
    }

    // Check for header
    if !include_dir.join("IpStdCInterface.h").exists() {
        println!("cargo:warning=IPOPT library found but headers missing in vendor/local");
        return false;
    }

    println!(
        "cargo:warning=Using IPOPT from {}",
        vendor_local.display()
    );

    // Emit linker flags
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Prefer shared library (fewer dependency issues)
    if has_shared {
        println!("cargo:rustc-link-lib=ipopt");
        // Set rpath for finding shared library at runtime
        // Note: -rpath takes a directory, not a file
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath,{}",
            lib_dir.display()
        );
    } else {
        println!("cargo:rustc-link-lib=static=ipopt");
        // Static linking needs all dependencies
        emit_static_deps(&lib_dir);
    }

    true
}

/// Emit additional link flags for static IPOPT build.
fn emit_static_deps(lib_dir: &PathBuf) {
    // IPOPT depends on MUMPS for sparse linear algebra
    if lib_dir.join("libcoinmumps.a").exists() {
        println!("cargo:rustc-link-lib=static=coinmumps");
    }

    // METIS for sparse matrix ordering
    if lib_dir.join("libcoinmetis.a").exists() {
        println!("cargo:rustc-link-lib=static=coinmetis");
    }

    // System dependencies
    println!("cargo:rustc-link-lib=lapack");
    println!("cargo:rustc-link-lib=blas");
    println!("cargo:rustc-link-lib=gfortran");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=dl");

    // C++ standard library
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
}

/// Try to link against system IPOPT via pkg-config.
fn try_system_ipopt() -> bool {
    let output = Command::new("pkg-config")
        .args(["--libs", "--cflags", "ipopt"])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return false,
    };

    let flags = String::from_utf8_lossy(&output.stdout);
    println!("cargo:warning=Using system IPOPT via pkg-config");

    // Parse and emit the flags
    for flag in flags.split_whitespace() {
        if let Some(lib) = flag.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={}", lib);
        } else if let Some(path) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={}", path);
        }
    }

    true
}
