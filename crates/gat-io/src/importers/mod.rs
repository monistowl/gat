//! Power system format importers and exporters.
//!
//! This module provides parsers for all supported power system formats:
//! - **MATPOWER** (.m files) - MATLAB-based power system format (most common in academia)
//! - **PSS/E RAW** (.raw files) - Siemens PSS/E proprietary format
//! - **CIM RDF/XML** - IEC 61970 CIM standard format
//! - **pandapower JSON** - Python pandapower format
//!
//! All importers return a [`gat_core::Network`] graph along with [`ImportDiagnostics`]
//! containing any warnings or errors encountered during parsing.
//!
//! ## Quick Import Example
//!
//! ```no_run
//! use gat_io::importers::{Format, parse_matpower};
//!
//! // Auto-detect format from extension
//! if let Some((format, _confidence)) = Format::detect(std::path::Path::new("case14.m")) {
//!     let result = format.parse("case14.m")?;
//!     let network = result.network;
//!     println!("Imported {} buses", network.graph.node_count());
//! }
//!
//! // Or use format-specific function directly
//! let result = parse_matpower("case14.m")?;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Format Comparison
//!
//! | Format | Strength | Limitation | Best For |
//! |--------|----------|-----------|----------|
//! | **MATPOWER** | Simple, widely used in academia | No transformer detail | Academic studies |
//! | **PSS/E** | Industrial standard, detailed | Complex format | Large-scale utility studies |
//! | **CIM RDF** | Standardized, extensible | Verbose, slow parsing | Smart grid applications |
//! | **pandapower** | Modern Python ecosystem | JSON size overhead | Integration with Python tools |
//!
//! ## Validation Pipeline
//!
//! All importers follow this pipeline:
//!
//! 1. **Format Detection** - Identify file type from extension and content
//! 2. **Raw Parsing** - Parse format-specific syntax (MATLAB, XML, JSON, fixed-width)
//! 3. **Data Extraction** - Map format fields to standard element types
//! 4. **Network Construction** - Build graph with [`gat_core::Node`] and [`gat_core::Edge`]
//! 5. **Post-Import Validation** - Check structure, references, physical sanity
//! 6. **Diagnostics** - Report warnings/errors without aborting
//!
//! ## Format-Specific Notes
//!
//! ### MATPOWER
//! - Supports versions 1-2.1 (all field variations)
//! - Preserves **generator cost models** (polynomial and piecewise linear)
//! - Note: Cost models are critical for OPF; loss of cost data breaks economic dispatch
//!
//! ### PSS/E
//! - Supports RAW format versions 29-35
//! - Handles buses, loads, generators, and branches
//! - Partial support for transformers and phase shifters
//!
//! ### CIM RDF
//! - Full IEC 61970 CIM standard
//! - Most verbose format; parsing slower than others
//! - Best for interchange with utility SCADA systems
//!
//! ### pandapower
//! - Native JSON format from Python pandapower library
//! - Supports all element types (buses, gens, loads, lines, transformers)
//! - Round-trips cleanly with Python code
//!
//! ## Error Handling
//!
//! All importers use **error recovery** rather than failing on first error:
//!
//! ```rust,no_run
//! # fn main() -> anyhow::Result<()> {
//! use gat_io::importers::{parse_matpower, ArrowDirectoryWriter, load_grid_from_arrow};
//!
//! let result = parse_matpower("case14.m")?;
//! let network = result.network;
//! let diag = result.diagnostics;
//!
//! if diag.has_errors() {
//!     eprintln!("Import had {} errors", diag.error_count());
//!     for issue in &diag.issues {
//!         eprintln!("  - {:?}: {}", issue.severity, issue.message);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Integration with Arrow Export
//!
//! After importing, export to the new directory-based Arrow format for lossless storage:
//!
//! ```no_run
//! # use gat_io::importers::{load_grid_from_arrow, parse_matpower, ArrowDirectoryWriter};
//! # use std::path::Path;
//! let result = parse_matpower("case14.m")?;
//! let network = result.network;
//!
//! // Export to Arrow directory
//! let writer = ArrowDirectoryWriter::new("case14_arrow_dir")?;
//! writer.write_network(&network, None, None)?;
//!
//! // Load from Arrow directory
//! let loaded_network = load_grid_from_arrow("case14_arrow_dir")?;
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Public API
//!
//! - [`Format`] - Format detection and unified import interface
//! - [`parse_matpower`] - Import MATPOWER .m files
//! - [`parse_psse`] - Import PSS/E RAW files
//! - [`parse_cim`] - Import CIM RDF/XML files
//! - [`parse_pandapower`] - Import pandapower JSON files
//! - [`ArrowDirectoryReader`] - Read networks from Arrow directory format
//! - [`ArrowDirectoryWriter`] - Write networks to Arrow directory format
//!
//! ## Module Organization
//!
//! - [`format`] - Format detection and unified interface
//! - [`matpower`]/[`matpower_parser`] - MATPOWER importer
//! - [`psse`] - PSS/E RAW importer
//! - [`cim`]/[`cim_validator`] - CIM RDF importer
//! - [`pandapower`] - pandapower JSON importer
//! - [`arrow`] - Arrow export (IPC mode only)

#[cfg(feature = "ipc")]
mod arrow;
#[cfg(not(feature = "ipc"))]
mod arrow_disabled;
#[cfg(not(feature = "ipc"))]
use arrow_disabled as arrow;
pub mod cim;
mod cim_validator;
mod format;
pub mod matpower;
pub mod matpower_parser;
pub mod pandapower;
pub mod psse;

pub use crate::exporters::{ArrowDirectoryReader, ArrowDirectoryWriter};
pub use arrow::{
    export_network_to_arrow, load_grid_from_arrow, load_grid_from_arrow_with_manifest,
};
pub use cim_validator::{
    validate_cim_with_warnings, validate_network_from_cim, CimValidationError,
};
pub use format::{Confidence, Format};

pub use cim::{import_cim_rdf, parse_cim};
pub use matpower::{import_matpower_case, load_matpower_network, parse_matpower};
pub use pandapower::{load_pandapower_network, parse_pandapower};
pub use psse::{import_psse_raw, parse_psse};

#[cfg(all(test, feature = "ipc"))]
mod tests;
