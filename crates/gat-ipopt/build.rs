//! Build script for gat-ipopt binary.
//!
//! Sets the rpath so the binary can find bundled shared libraries
//! relative to its installation location.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Set rpath for finding shared libraries at runtime
    // $ORIGIN/../lib finds libs relative to the binary's location
    // This works when binary is in solvers/ and libs are in lib/
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../lib");
}
