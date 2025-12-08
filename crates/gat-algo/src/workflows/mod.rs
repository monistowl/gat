//! High-level workflow facades for common power system analysis tasks.
//!
//! These facades provide simplified entry points to the underlying algorithms,
//! handling common configuration and setup patterns.

pub mod power_flow;

pub use power_flow::PowerFlowAnalysis;
