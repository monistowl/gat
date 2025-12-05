//! # Sparse Matrix Infrastructure for Power System Analysis
//!
//! Power grids are inherently sparse: a 10,000-bus network might have only
//! 15,000 branches, yielding ~0.03% matrix density. This module provides
//! efficient sparse representations that scale to large networks.
//!
//! ## Module Organization
//!
//! - [`ybus`]: Sparse admittance matrix (Y-bus) for AC power flow
//! - [`susceptance`]: Sparse susceptance matrix (B') for DC power flow
//! - [`sensitivity`]: PTDF and LODF matrices for contingency analysis
//! - [`incremental`]: Woodbury-based incremental updates for N-1 analysis
//!
//! ## Memory Comparison
//!
//! | Network Size | Dense (MB) | Sparse (MB) | Ratio |
//! |--------------|------------|-------------|-------|
//! | 1,000 buses  | 8          | 0.2         | 40x   |
//! | 10,000 buses | 800        | 2           | 400x  |
//! | 100,000 buses| 80,000     | 20          | 4000x |
//!
//! ## Usage
//!
//! ```ignore
//! use gat_algo::sparse::{SparseSusceptance, SparsePtdf};
//!
//! let network = Network::from_file("case9241.arrow")?;
//!
//! // Build sparse B' matrix
//! let b_prime = SparseSusceptance::from_network(&network)?;
//! println!("Non-zeros: {} ({:.4}% density)",
//!          b_prime.nnz(),
//!          b_prime.density() * 100.0);
//!
//! // Compute sparse PTDF for contingency screening
//! let ptdf = SparsePtdf::from_network(&network)?;
//! let flow_sensitivity = ptdf.get_branch_sensitivity(branch_id, bus_id);
//! ```

pub mod incremental;
pub mod sensitivity;
pub mod susceptance;
pub mod ybus;

// Re-export main types
pub use incremental::{IncrementalSolver, WoodburyUpdate};
pub use sensitivity::{LodfMatrix, PtdfMatrix, SparsePtdf};
pub use susceptance::{SparseSusceptance, SusceptanceError};
pub use ybus::{SparseYBus, YBusError};
