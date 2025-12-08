//! # gat-algo: Advanced Algorithms for Power System Analysis
//!
//! This crate provides optimization and analytics algorithms for power system operations,
//! including multiple optimal power flow (OPF) formulations, reliability analysis, and
//! economic allocation methods.
//!
//! ## Optimal Power Flow (OPF)
//!
//! The [`OpfSolver`] provides a unified interface to multiple OPF solution methods:
//!
//! | Method | Description | Problem Class |
//! |--------|-------------|---------------|
//! | [`OpfMethod::EconomicDispatch`] | Merit-order dispatch without network | Linear |
//! | [`OpfMethod::DcOpf`] | Linear DC approximation with PTDF flows | Linear |
//! | [`OpfMethod::SocpRelaxation`] | Convex SOCP relaxation of AC-OPF | Conic |
//! | [`OpfMethod::AcOpf`] | Full nonlinear AC-OPF | Nonlinear |
//!
//! ### Architecture
//!
//! The OPF system uses a Strategy Pattern for extensibility:
//!
//! - **[`opf::OpfFormulation`]**: Defines the mathematical problem (what to solve)
//! - **[`opf::OpfBackend`]**: Implements the solver algorithm (how to solve it)
//! - **[`opf::SolverRegistry`]**: Service locator for registered components
//! - **[`opf::OpfDispatcher`]**: Orchestrates solving with fallback chains
//!
//! This separation allows adding new formulations or backends without modifying existing code.
//!
//! ### SOCP Relaxation
//!
//! The SOCP solver implements the Baran-Wu / Farivar-Low branch-flow model with:
//! - Quadratic generator cost curves
//! - Phase-shifting and tap-changing transformers
//! - Thermal limits and voltage bounds
//! - LMP computation from dual variables
//!
//! See the [module documentation](opf/socp.rs) for mathematical details and references.
//!
//! ## Reliability Analysis
//!
//! - [`MonteCarlo`]: Sequential Monte Carlo for LOLE/EUE estimation
//! - [`MultiAreaMonteCarlo`]: Multi-area reliability with corridor constraints
//! - [`DeliverabilityScore`]: Transmission-constrained deliverability metrics
//!
//! ## Economic Allocation
//!
//! - [`alloc_kpi`]: Key performance indicator computation
//! - [`alloc_rents`]: Economic rent allocation methods
//! - [`elcc`]: Effective Load Carrying Capability
//!
//! ## Example
//!
//! ```ignore
//! use gat_algo::{OpfSolver, OpfMethod};
//! use gat_core::Network;
//!
//! let network = Network::from_file("case9.arrow")?;
//!
//! // Solve SOCP relaxation
//! let solver = OpfSolver::new()
//!     .with_method(OpfMethod::SocpRelaxation);
//!
//! let solution = solver.solve(&network)?;
//! println!("Cost: ${:.2}/hr", solution.objective_value);
//! println!("Losses: {:.2} MW", solution.total_losses_mw);
//! ```

// Core modules (always available, WASM-compatible)
pub mod ac_opf;
pub mod arena;
pub mod graph;
pub mod opf;
pub mod sparse;
pub mod tep;
pub mod validation;

// Modules that use rayon for parallelism (desktop-only)
#[cfg(feature = "desktop")]
pub mod contingency;
#[cfg(feature = "desktop")]
pub mod test_utils;

// Desktop-only modules (require rayon, polars, or csv)
#[cfg(feature = "desktop")]
pub mod alloc_kpi;
#[cfg(feature = "desktop")]
pub mod alloc_rents;
#[cfg(feature = "desktop")]
pub mod analytics_ds;
#[cfg(feature = "desktop")]
pub mod analytics_reliability;
#[cfg(feature = "desktop")]
pub mod canos_multiarea;
#[cfg(feature = "desktop")]
pub mod elcc;
#[cfg(feature = "desktop")]
pub mod featurize_geo;
#[cfg(feature = "desktop")]
pub mod featurize_gnn;
#[cfg(feature = "desktop")]
pub mod featurize_kpi;
#[cfg(feature = "desktop")]
pub mod geo_join;
#[cfg(feature = "desktop")]
pub mod io;
#[cfg(feature = "desktop")]
pub mod power_flow;
#[cfg(feature = "desktop")]
pub mod reliability_monte_carlo;

// GPU-accelerated modules (optional feature)
#[cfg(feature = "gpu")]
pub mod gpu_monte_carlo;
#[cfg(feature = "gpu")]
pub use gpu_monte_carlo::GpuMonteCarlo;

// Core re-exports (always available)
pub use ac_opf::{AcOpfError, AcOpfSolution, AcOpfSolver, OpfError};
pub use arena::ArenaContext;
pub use graph::{partition_network, NetworkPartition, PartitionError, PartitionStrategy, TieLine};
pub use opf::{ConstraintInfo, ConstraintType, OpfMethod, OpfSolution, OpfSolver};
pub use sparse::{
    IncrementalSolver, LodfMatrix, PtdfMatrix, SparsePtdf, SparseSusceptance, SparseYBus,
    SusceptanceError, WoodburyUpdate, YBusError,
};
pub use tep::{
    solve_tep, CandidateId, CandidateLine, LineBuildDecision, TepError, TepProblem,
    TepProblemBuilder, TepSolution, TepSolverConfig,
};
pub use validation::{
    compute_opf_violations, compute_opf_violations_from_solution, compute_pf_errors,
    OPFViolationMetrics, ObjectiveGap, PFErrorMetrics, PFReferenceSolution,
};

// Desktop-only re-exports
#[cfg(feature = "desktop")]
pub use alloc_kpi::*;
#[cfg(feature = "desktop")]
pub use alloc_rents::*;
#[cfg(feature = "desktop")]
pub use analytics_ds::*;
#[cfg(feature = "desktop")]
pub use analytics_reliability::*;
#[cfg(feature = "desktop")]
pub use canos_multiarea::{
    AreaId, AreaLoleMetrics, Corridor, MultiAreaMonteCarlo, MultiAreaOutageScenario,
    MultiAreaSystem,
};
#[cfg(feature = "desktop")]
pub use elcc::*;
#[cfg(feature = "desktop")]
pub use featurize_geo::*;
#[cfg(feature = "desktop")]
pub use featurize_gnn::*;
#[cfg(feature = "desktop")]
pub use featurize_kpi::*;
#[cfg(feature = "desktop")]
pub use geo_join::*;
#[cfg(feature = "desktop")]
pub use io::*;
#[cfg(feature = "desktop")]
pub use power_flow::*;
#[cfg(feature = "desktop")]
pub use reliability_monte_carlo::{
    DeliverabilityScore, DeliverabilityScoreConfig, MonteCarlo, OutageGenerator, OutageScenario,
    ReliabilityMetrics,
};
