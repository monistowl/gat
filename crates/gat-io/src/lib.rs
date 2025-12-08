//! # gat-io: Power System Data I/O & Import
//!
//! Comprehensive input/output support for power system datasets, including import from multiple
//! formats (MATPOWER, PSS/E, CIM, pandapower) and export to normalized Arrow columnar format.
//!
//! ## Design Philosophy
//!
//! **Single Responsibility**: Each format parser focuses on format-specific parsing. Generic
//! validation and normalization happens post-import through a shared diagnostics and validation
//! pipeline.
//!
//! **Lossless Roundtrips**: Support all fields from original formats (e.g., MATPOWER gencost
//! models) to enable faithful import/export cycles.
//!
//! **Error Recovery**: Partial imports continue when encountering validation errors, collecting
//! diagnostics for user visibility rather than panicking.
//!
//! ## Quick Start: Import a MATPOWER Case
//!
//! ```rust,no_run
//! use gat_io::importers::parse_matpower;
//!
//! fn main() -> anyhow::Result<()> {
//!     let result = parse_matpower("case14.m")?;
//!     let network = result.network;
//!     let diagnostics = result.diagnostics;
//!
//!     println!("Buses: {}", network.graph.node_count());
//!     println!("Branches: {}", network.graph.edge_count());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Supported Formats
//!
//! | Format | File Extensions | Status | Notes |
//! |--------|-----------------|--------|-------|
//! | MATPOWER | `.m`, `.case`, `.mat` | Stable | Full gencost support |
//! | PSS/E RAW | `.raw` | Stable | Versions 29-35 |
//! | CIM RDF/XML | `.rdf`, `.xml` | Stable | IEC 61970 CIM-RDF |
//! | pandapower JSON | `.json` | Stable | Full element support |
//!
//! ## Module Overview
//!
//! ### Importers ([`importers`])
//! - [`importers::parse_matpower`] - MATLAB/MATPOWER format
//! - [`importers::parse_psse`] - PSS/E RAW format
//! - [`importers::parse_cim`] - CIM RDF/XML format
//! - [`importers::parse_pandapower`] - pandapower JSON format
//! - [`importers::Format`] - Format detection and unified interface
//!
//! ### Arrow Schema & Validation ([`arrow_schema`], [`arrow_validator`])
//! - Normalized multi-file Arrow schema for lossless storage
//! - **Tables**: `system`, `buses`, `generators`, `loads`, `branches`
//! - Referential integrity checks (unique IDs, foreign keys, cost models)
//!
//! ### Helpers ([`helpers`])
//! - `ImportDiagnostics` - Diagnostics collection with severity levels
//! - `ImportResult` - Result type containing network and diagnostics
//! - `ValidationConfig` - Post-import validation configuration
//! - `validate_network` - Network topology and sanity checks
//! - `ArrowValidator` - Arrow dataset referential integrity
//! - `PathValidator` - Security checks for file paths
//!
//! ### Data Sources ([`sources`])
//! - [`sources::eia`] - EIA Electricity Data Browser integration
//! - [`sources::ember`] - Ember electricity data integration
//! - [`sources::opfdata`] - OPFData repository integration
//!
//! ### Validation ([`validate`])
//! - Dataset specification validation
//! - Constraint checking (voltage limits, thermal limits, etc.)
//!
//! ## Feature Flags
//!
//! - **Default**: All import formats enabled
//! - `wasm`: WASM-compatible build (disables file I/O, stubs network access)
//!
//! ## Error Handling
//!
//! All public APIs return `Result<T>` or [`helpers::ImportResult`] with diagnostics:
//!
//! ```rust,no_run
//! use gat_io::importers::parse_matpower;
//!
//! match parse_matpower("cases/case9.m") {
//!     Ok(result) => {
//!         let network = result.network;
//!         let diagnostics = result.diagnostics;
//!         
//!         if diagnostics.has_errors() {
//!             eprintln!("Import errors:\n{}", diagnostics);
//!         }
//!     }
//!     Err(e) => eprintln!("Failed to open file: {}", e),
//! }
//! ```
//!
//! ## Validation Pipeline
//!
//! 1. **Format Detection** (`Format::detect()`) - Identifies file type from extension + content
//! 2. **Format-Specific Parsing** - Parse to intermediate representation
//! 3. **Network Construction** - Build graph from parsed elements
//! 4. **Post-Import Validation** - Check structure, references, physical sanity
//! 5. **Diagnostics Reporting** - Collect warnings/errors for user review
//!
//! ## Integration with gat-core
//!
//! All importers return [`gat_core::Network`] graphs:
//! - Nodes: Buses, Generators, Loads
//! - Edges: Branches, Transformers
//!
//! See [`gat_core`] documentation for graph-based analysis APIs.

pub mod arrow_manifest;
pub mod arrow_schema;
pub mod arrow_validator;
pub mod helpers;

// WASM-compatible parsing module - always available
// Contains string-based parsers that work in both native and WASM environments
pub mod wasm_parsers;

// Modules requiring native I/O (polars, filesystem)
#[cfg(feature = "native-io")]
pub mod exporters;
#[cfg(feature = "native-io")]
pub mod validate;

// Modules requiring non-WASM filesystem access AND native-io (polars)
// These modules use both std::fs and polars DataFrame operations
#[cfg(all(not(target_arch = "wasm32"), feature = "native-io"))]
pub mod importers;
#[cfg(all(not(target_arch = "wasm32"), feature = "native-io"))]
pub mod sources;

// For wasm builds we stub IO; web demo should provide data from host/JS.
#[cfg(target_arch = "wasm32")]
pub mod wasm_stub {
    use anyhow::{bail, Result};

    pub fn load_csv_stub(_data: &str) -> Result<()> {
        bail!("gat-io wasm build: CSV/Parquet IO not available in wasm stub")
    }
}
