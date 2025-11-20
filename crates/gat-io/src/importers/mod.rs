#[cfg(feature = "ipc")]
mod arrow;
#[cfg(not(feature = "ipc"))]
mod arrow_disabled;
#[cfg(not(feature = "ipc"))]
use arrow_disabled as arrow;
pub mod cim;
pub mod matpower;
pub mod psse;

#[cfg(feature = "ipc")]
pub use arrow::load_grid_from_arrow;
#[cfg(not(feature = "ipc"))]
pub use arrow_disabled::load_grid_from_arrow;
pub use cim::import_cim_rdf;
pub use matpower::import_matpower_case;
pub use psse::import_psse_raw;

#[cfg(all(test, feature = "ipc"))]
mod tests;
