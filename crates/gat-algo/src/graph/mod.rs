//! Graph algorithms for power network analysis.
//!
//! This module provides graph-theoretic algorithms for power systems, including:
//! - **Partitioning**: Split networks into regions for distributed optimization
//! - **Connectivity**: Find islands, articulation points, and bridges
//!
//! # Partitioning for Distributed OPF
//!
//! The [`partition`] module implements network partitioning for ADMM-based
//! distributed optimal power flow. Partitions are designed to:
//! - Minimize tie-line (boundary) cuts
//! - Balance computational load across partitions
//! - Preserve electrical coherence
//!
//! ```ignore
//! use gat_algo::graph::{partition_network, PartitionStrategy};
//!
//! let partitions = partition_network(
//!     &network,
//!     PartitionStrategy::Spectral { num_partitions: 4 },
//! )?;
//!
//! for p in &partitions {
//!     println!("Partition {}: {} buses, {} tie lines",
//!         p.id, p.buses.len(), p.tie_lines.len());
//! }
//! ```

pub mod partition;

pub use partition::{
    partition_network, NetworkPartition, PartitionError, PartitionStrategy, TieLine,
};
