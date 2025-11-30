//! Build infrastructure for COIN-OR libraries from vendored sources.
//!
//! This crate provides reusable build logic for extracting and compiling
//! COIN-OR optimization libraries (CoinUtils, Osi, Clp, Cgl, Cbc) from
//! vendored ZIP archives.
//!
//! # Usage
//!
//! In a `build.rs`:
//!
//! ```ignore
//! use gat_coinor_build::{CoinorBuildConfig, Component};
//!
//! fn main() {
//!     // Check for prebuilt libraries first
//!     if let Some(artifacts) = gat_coinor_build::find_prebuilt(&prebuilt_dir) {
//!         artifacts.emit_cargo_metadata();
//!         return;
//!     }
//!
//!     // Build from source
//!     let config = CoinorBuildConfig {
//!         vendor_dir: PathBuf::from("../../vendor"),
//!         build_dir: PathBuf::from(&std::env::var("OUT_DIR").unwrap()),
//!         install_dir: PathBuf::from(&std::env::var("OUT_DIR").unwrap()).join("coinor"),
//!         components: vec![Component::CoinUtils, Component::Osi, Component::Clp],
//!     };
//!
//!     let artifacts = gat_coinor_build::build(&config).expect("COIN-OR build failed");
//!     artifacts.emit_cargo_metadata();
//! }
//! ```
//!
//! # Build Order
//!
//! COIN-OR libraries have the following dependency order:
//!
//! ```text
//! CoinUtils (base)
//!     └── Osi (solver interface)
//!         └── Clp (LP solver)
//!             └── Cgl (cut generators)
//!                 └── Cbc (MIP solver)
//! ```

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for building COIN-OR libraries.
#[derive(Debug, Clone)]
pub struct CoinorBuildConfig {
    /// Path to vendor/ directory containing COIN-OR ZIP archives.
    pub vendor_dir: PathBuf,
    /// Directory for extracting and building sources.
    pub build_dir: PathBuf,
    /// Directory for installing compiled libraries and headers.
    pub install_dir: PathBuf,
    /// Components to build (in dependency order).
    pub components: Vec<Component>,
}

/// COIN-OR library component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    /// CoinUtils - Base utilities (vectors, matrices, I/O).
    CoinUtils,
    /// Osi - Open Solver Interface (abstract LP interface).
    Osi,
    /// Clp - COIN-OR LP solver.
    Clp,
    /// Cgl - Cut Generator Library.
    Cgl,
    /// Cbc - COIN-OR Branch and Cut MIP solver.
    Cbc,
}

impl Component {
    /// Get the ZIP archive filename for this component.
    pub fn zip_name(&self) -> &'static str {
        match self {
            Component::CoinUtils => "CoinUtils-master.zip",
            Component::Osi => "Osi-master.zip",
            Component::Clp => "Clp-master.zip",
            Component::Cgl => "Cgl-master.zip",
            Component::Cbc => "Cbc-master.zip",
        }
    }

    /// Get the extracted directory name.
    pub fn extracted_dir(&self) -> &'static str {
        match self {
            Component::CoinUtils => "CoinUtils-master",
            Component::Osi => "Osi-master",
            Component::Clp => "Clp-master",
            Component::Cgl => "Cgl-master",
            Component::Cbc => "Cbc-master",
        }
    }

    /// Get the library name (without lib prefix or extension).
    pub fn lib_name(&self) -> &'static str {
        match self {
            Component::CoinUtils => "CoinUtils",
            Component::Osi => "Osi",
            Component::Clp => "Clp",
            Component::Cgl => "Cgl",
            Component::Cbc => "Cbc",
        }
    }

    /// Get required dependencies for this component.
    pub fn dependencies(&self) -> &'static [Component] {
        match self {
            Component::CoinUtils => &[],
            Component::Osi => &[Component::CoinUtils],
            Component::Clp => &[Component::CoinUtils, Component::Osi],
            Component::Cgl => &[Component::CoinUtils, Component::Osi, Component::Clp],
            Component::Cbc => &[
                Component::CoinUtils,
                Component::Osi,
                Component::Clp,
                Component::Cgl,
            ],
        }
    }

    /// Get all components in proper build order.
    pub fn all_in_order() -> &'static [Component] {
        &[
            Component::CoinUtils,
            Component::Osi,
            Component::Clp,
            Component::Cgl,
            Component::Cbc,
        ]
    }

    /// Get components needed for CLP (LP solver only).
    pub fn clp_deps() -> &'static [Component] {
        &[Component::CoinUtils, Component::Osi, Component::Clp]
    }

    /// Get components needed for CBC (full MIP solver).
    pub fn cbc_deps() -> &'static [Component] {
        Component::all_in_order()
    }
}

/// Build artifacts from a successful COIN-OR compilation.
#[derive(Debug, Clone)]
pub struct BuildArtifacts {
    /// Directory containing compiled static libraries.
    pub lib_dir: PathBuf,
    /// Directory containing header files.
    pub include_dir: PathBuf,
    /// List of library names that were built.
    pub libraries: Vec<String>,
}

impl BuildArtifacts {
    /// Emit cargo metadata to link against the built libraries.
    pub fn emit_cargo_metadata(&self) {
        println!("cargo:rustc-link-search=native={}", self.lib_dir.display());
        for lib in &self.libraries {
            println!("cargo:rustc-link-lib=static={}", lib);
        }
        // Link C++ standard library
        #[cfg(target_os = "linux")]
        println!("cargo:rustc-link-lib=stdc++");
        #[cfg(target_os = "macos")]
        println!("cargo:rustc-link-lib=c++");
    }
}

/// Check for pre-built COIN-OR libraries at the given path.
///
/// Returns `Some(BuildArtifacts)` if libraries are found, `None` otherwise.
pub fn find_prebuilt(install_dir: &Path) -> Option<BuildArtifacts> {
    let lib_dir = install_dir.join("lib");
    let include_dir = install_dir.join("include");

    if !lib_dir.exists() || !include_dir.exists() {
        return None;
    }

    // Check for at least CoinUtils library
    let coinutils_lib = lib_dir.join("libCoinUtils.a");
    if !coinutils_lib.exists() {
        return None;
    }

    // Discover all available libraries
    let mut libraries = Vec::new();
    for entry in fs::read_dir(&lib_dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("lib") && name_str.ends_with(".a") {
            let lib_name = name_str
                .strip_prefix("lib")
                .unwrap()
                .strip_suffix(".a")
                .unwrap();
            libraries.push(lib_name.to_string());
        }
    }

    Some(BuildArtifacts {
        lib_dir,
        include_dir,
        libraries,
    })
}

/// Build COIN-OR libraries from vendored sources.
///
/// This function:
/// 1. Extracts ZIP archives to the build directory
/// 2. Compiles C++ sources using the `cc` crate
/// 3. Installs libraries and headers to the install directory
pub fn build(config: &CoinorBuildConfig) -> Result<BuildArtifacts> {
    // Ensure directories exist
    fs::create_dir_all(&config.build_dir).context("Failed to create build directory")?;
    fs::create_dir_all(&config.install_dir).context("Failed to create install directory")?;

    let lib_dir = config.install_dir.join("lib");
    let include_dir = config.install_dir.join("include");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&include_dir)?;

    // Resolve full dependency list
    let components = resolve_dependencies(&config.components);

    let mut libraries = Vec::new();

    for component in &components {
        println!("cargo:warning=Building {:?}...", component);

        // Extract if needed
        let src_dir = extract_component(config, *component)?;

        // Build the component
        build_component(config, *component, &src_dir, &lib_dir, &include_dir)?;

        libraries.push(component.lib_name().to_string());
    }

    Ok(BuildArtifacts {
        lib_dir,
        include_dir,
        libraries,
    })
}

/// Resolve component list to include all dependencies in build order.
fn resolve_dependencies(requested: &[Component]) -> Vec<Component> {
    let mut needed: HashSet<Component> = HashSet::new();

    // Collect all needed components
    for comp in requested {
        needed.insert(*comp);
        for dep in comp.dependencies() {
            needed.insert(*dep);
        }
    }

    // Return in proper build order
    Component::all_in_order()
        .iter()
        .copied()
        .filter(|c| needed.contains(c))
        .collect()
}

/// Extract a component's ZIP archive to the build directory.
fn extract_component(config: &CoinorBuildConfig, component: Component) -> Result<PathBuf> {
    let zip_path = config.vendor_dir.join(component.zip_name());
    let extract_dir = config.build_dir.join(component.extracted_dir());

    // Skip if already extracted
    if extract_dir.exists() {
        return Ok(extract_dir);
    }

    println!("cargo:warning=Extracting {}...", component.zip_name());

    let file = fs::File::open(&zip_path)
        .with_context(|| format!("Failed to open {}", zip_path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP: {}", zip_path.display()))?;

    archive
        .extract(&config.build_dir)
        .with_context(|| format!("Failed to extract {}", zip_path.display()))?;

    Ok(extract_dir)
}

/// Build a single COIN-OR component.
fn build_component(
    config: &CoinorBuildConfig,
    component: Component,
    src_dir: &Path,
    lib_dir: &Path,
    include_dir: &Path,
) -> Result<()> {
    // Copy headers first
    copy_headers(component, src_dir, include_dir)?;

    // Find and compile source files
    let sources = find_sources(component, src_dir)?;

    if sources.is_empty() {
        anyhow::bail!("No source files found for {:?}", component);
    }

    println!(
        "cargo:warning=Compiling {} source files for {:?}",
        sources.len(),
        component
    );

    // Ensure OUT_DIR is set (cc crate requires it)
    // When called from xtask, we set it to our build dir
    let out_dir =
        std::env::var("OUT_DIR").unwrap_or_else(|_| config.build_dir.to_string_lossy().to_string());
    std::env::set_var("OUT_DIR", &out_dir);

    // Also set TARGET and HOST for cc crate if not set
    if std::env::var("TARGET").is_err() {
        #[cfg(target_arch = "x86_64")]
        std::env::set_var("TARGET", "x86_64-unknown-linux-gnu");
        #[cfg(target_arch = "aarch64")]
        std::env::set_var("TARGET", "aarch64-unknown-linux-gnu");
    }
    if std::env::var("HOST").is_err() {
        #[cfg(target_arch = "x86_64")]
        std::env::set_var("HOST", "x86_64-unknown-linux-gnu");
        #[cfg(target_arch = "aarch64")]
        std::env::set_var("HOST", "aarch64-unknown-linux-gnu");
    }

    // Build using cc crate
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .opt_level(2)
        .warnings(false)
        // Output directly to our lib_dir
        .out_dir(lib_dir)
        // Include paths for this component and dependencies
        .include(include_dir)
        .include(include_dir.join("coin")) // Standard COIN-OR include path
        .include(src_dir.join("src"))
        // Standard COIN-OR defines
        .define("HAVE_CMATH", None)
        .define("HAVE_CFLOAT", None)
        .define("HAVE_CSTDIO", None)
        .define("HAVE_CSTDLIB", None)
        .define("HAVE_CSTRING", None)
        .define("HAVE_CASSERT", None);

    // Add component-specific include paths
    match component {
        Component::CoinUtils => {
            build.include(src_dir.join("src"));
        }
        Component::Osi => {
            build.include(src_dir.join("src/Osi"));
        }
        Component::Clp => {
            build.include(src_dir.join("src"));
        }
        Component::Cgl => {
            build.include(src_dir.join("src"));
            // Cgl has many subdirectories
            for entry in fs::read_dir(src_dir.join("src"))? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    build.include(entry.path());
                }
            }
        }
        Component::Cbc => {
            build.include(src_dir.join("src"));
            // CBC shares CbcOrClpParam with CLP - add CLP source path
            let clp_src = config.build_dir.join("Clp-master/src");
            if clp_src.exists() {
                build.include(&clp_src);
            }
        }
    }

    // Add all source files
    for src in &sources {
        build.file(src);
    }

    // Compile to static library - cc will put it in out_dir (now lib_dir)
    build.compile(component.lib_name());

    Ok(())
}

/// Copy header files to the include directory.
fn copy_headers(component: Component, src_dir: &Path, include_dir: &Path) -> Result<()> {
    let headers_src = match component {
        Component::CoinUtils => src_dir.join("src"),
        Component::Osi => src_dir.join("src/Osi"),
        Component::Clp => src_dir.join("src"),
        Component::Cgl => src_dir.join("src"),
        Component::Cbc => src_dir.join("src"),
    };

    // Create component-specific include dir
    let comp_include = include_dir.join(format!("coin-or/{}", component.lib_name()));
    fs::create_dir_all(&comp_include)?;

    // Also create flat coin/ directory for traditional includes
    let flat_include = include_dir.join("coin");
    fs::create_dir_all(&flat_include)?;

    // Copy all .h and .hpp files
    copy_headers_recursive(&headers_src, &comp_include, &flat_include)?;

    Ok(())
}

/// Recursively copy header files.
fn copy_headers_recursive(src: &Path, comp_dest: &Path, flat_dest: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }

    for entry in walkdir::WalkDir::new(src).max_depth(3) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "h" || ext == "hpp" {
                    let filename = path.file_name().unwrap();

                    // Copy to component-specific dir
                    let comp_target = comp_dest.join(filename);
                    if !comp_target.exists() {
                        fs::copy(path, &comp_target)?;
                    }

                    // Copy to flat coin/ dir
                    let flat_target = flat_dest.join(filename);
                    if !flat_target.exists() {
                        fs::copy(path, &flat_target)?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Find C++ source files for a component.
fn find_sources(component: Component, src_dir: &Path) -> Result<Vec<PathBuf>> {
    let search_dir = match component {
        Component::CoinUtils => src_dir.join("src"),
        Component::Osi => src_dir.join("src/Osi"),
        Component::Clp => src_dir.join("src"),
        Component::Cgl => src_dir.join("src"),
        Component::Cbc => src_dir.join("src"),
    };

    let mut sources = Vec::new();

    for entry in walkdir::WalkDir::new(&search_dir).max_depth(3) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "cpp" || ext == "c" {
                    let filename = path.file_name().unwrap().to_string_lossy();

                    // Skip test files and main files
                    if filename.contains("Test")
                        || filename.contains("test")
                        || filename.contains("Main")
                        || filename.contains("main")
                        || filename.contains("unitTest")
                        || filename.contains("Driver")
                    {
                        continue;
                    }

                    // Skip deprecated/attic directories
                    let path_str = path.to_string_lossy();
                    if path_str.contains("/Attic/") || path_str.contains("/attic/") {
                        continue;
                    }

                    // Skip ABC (Advanced Branch and Cut) files - optional feature
                    // These require special configuration that we don't support
                    if filename.starts_with("Abc") || filename.starts_with("CoinAbc") {
                        continue;
                    }

                    // Skip CBC files with complex parameter/config dependencies
                    // These require the full autotools configure which we don't support.
                    // We compile the core CBC library and write our own minimal C wrapper.
                    if component == Component::Cbc {
                        let skip = filename.starts_with("CbcParam")
                            || filename.starts_with("CbcCbcParam")
                            || filename.contains("CbcSolver")
                            || filename.contains("CbcMain")
                            || filename.contains("CbcLinked")
                            || filename.contains("CbcAmpl")
                            || filename.starts_with("CbcGen")  // CbcGenSolvers, CbcGenCtlBlk, etc
                            || filename.starts_with("Cbc_C_Interface")
                            || filename.starts_with("Cbc_ampl")
                            || filename.starts_with("OsiCbc")
                            || filename.starts_with("unitTest")
                            || filename.starts_with("CbcMip")  // CbcMipStartIO needs config
                            || filename.contains("Sos") // Some SOS files have issues
                            || filename.starts_with("CbcBab")  // CbcBab includes CbcParameters with invalid static_cast
                            || filename.starts_with("CbcSolution") // CbcSolution also includes CbcParameters
                            || filename.starts_with("CoinSolve"); // CoinSolve includes CbcSolver.hpp with broken params
                        if skip {
                            continue;
                        }
                    }

                    // Skip OsiClp interface files if building CLP standalone
                    // (they need linking against CLP which creates circular dep)
                    if component == Component::Clp && filename.starts_with("OsiClp") {
                        continue;
                    }

                    // Skip optional external solver interfaces (require external libraries)
                    // These are Cholesky factorization interfaces for interior-point that
                    // need external linear algebra packages we don't bundle.
                    // We keep ClpCholeskyBase and ClpCholeskyDense (built-in implementations).
                    let filename_lower = filename.to_lowercase();
                    if filename_lower.contains("mumps")      // MUMPS linear solver (needs MPI)
                        || filename_lower.contains("wsmp")   // WSMP solver
                        || filename_lower.contains("wssmp")  // WSSMP solver (alt spelling)
                        || filename_lower.contains("ufl")    // University of Florida sparse
                        || filename_lower.contains("taucs")  // TAUCS solver
                        || filename_lower.contains("cholmod")// CHOLMOD from SuiteSparse
                        || filename_lower.contains("pardiso")
                    // Intel Pardiso solver
                    {
                        continue;
                    }

                    sources.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_resolution() {
        let deps = resolve_dependencies(&[Component::Clp]);
        assert_eq!(
            deps,
            vec![Component::CoinUtils, Component::Osi, Component::Clp]
        );
    }

    #[test]
    fn test_cbc_deps() {
        let deps = resolve_dependencies(&[Component::Cbc]);
        assert_eq!(deps, Component::all_in_order().to_vec());
    }

    #[test]
    fn test_component_zip_names() {
        assert_eq!(Component::CoinUtils.zip_name(), "CoinUtils-master.zip");
        assert_eq!(Component::Cbc.zip_name(), "Cbc-master.zip");
    }
}
