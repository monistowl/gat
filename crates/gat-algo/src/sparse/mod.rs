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
//! ## Type Safety
//!
//! Unlike older implementations in [`crate::contingency::lodf`] and
//! [`crate::opf::ac_nlp::sparse_ybus`], this module uses typed IDs
//! ([`gat_core::BranchId`], [`gat_core::BusId`]) throughout for compile-time safety.
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
//! use gat_core::BusId;
//!
//! let network = Network::from_file("case9241.arrow")?;
//!
//! // Build sparse B' matrix
//! let b_prime = SparseSusceptance::from_network(&network)?;
//! println!("Non-zeros: {} ({:.4}% density)",
//!          b_prime.nnz(),
//!          b_prime.density() * 100.0);
//!
//! // Compute PTDF for contingency screening
//! let ptdf = SparsePtdf::compute_ptdf(&network)?;
//! let sens = ptdf.get(branch_id, BusId::new(5));
//!
//! // Compute LODF for N-1 analysis
//! let lodf = SparsePtdf::compute_lodf(&network, &ptdf)?;
//! let post_flow = lodf.estimate_post_outage_flow(branch_l, branch_m, flow_l, flow_m);
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
