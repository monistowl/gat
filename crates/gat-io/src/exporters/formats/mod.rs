//! Format-specific exporters for power system data.
//!
//! This module provides exporters that convert in-memory Network representations
//! back to their original file formats (MATPOWER, PSS/E, CIM, pandapower, PowerModels).
//!
//! ## Supported Formats
//!
//! - **MATPOWER** (.m files) - Full support with cost models
//! - **PSS/E** (.raw files) - Full support
//! - **CIM** (RDF/XML) - Full support
//! - **pandapower** (JSON) - Full support
//! - **PowerModels.jl** (JSON) - Full support with cost models
//!
//! ## Usage
//!
//! ```no_run
//! use gat_io::exporters::formats::export_network_to_matpower;
//! use gat_io::importers::load_grid_from_arrow;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Load network from Arrow
//! let network = load_grid_from_arrow("grid.arrow")?;
//!
//! // Export to MATPOWER format
//! export_network_to_matpower(&network, "output.m", None)?;
//! # Ok(())
//! # }
//! ```

pub mod cim;
pub mod matpower;
pub mod pandapower;
pub mod powermodels;
pub mod psse;

#[cfg(test)]
mod tests;

pub use cim::export_network_to_cim;
pub use matpower::export_network_to_matpower;
pub use pandapower::export_network_to_pandapower;
pub use powermodels::{export_network_to_powermodels, export_network_to_powermodels_string};
pub use psse::export_network_to_psse;
