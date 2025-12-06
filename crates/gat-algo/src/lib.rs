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

pub mod ac_opf;
pub mod alloc_kpi;
pub mod alloc_rents;
pub mod analytics_ds;
pub mod analytics_reliability;
pub mod arena;
pub mod canos_multiarea;
pub mod contingency;
pub mod elcc;
pub mod featurize_geo;
pub mod featurize_gnn;
pub mod featurize_kpi;
pub mod geo_join;
pub mod io;
pub mod opf;
pub mod power_flow;
pub mod reliability_monte_carlo;
pub mod sparse;
pub mod tep;
pub mod test_utils;
pub mod validation;

pub use ac_opf::{AcOpfError, AcOpfSolution, AcOpfSolver, OpfError};
pub use alloc_kpi::*;
pub use alloc_rents::*;
pub use analytics_ds::*;
pub use analytics_reliability::*;
pub use arena::ArenaContext;
pub use canos_multiarea::{
    AreaId, AreaLoleMetrics, Corridor, MultiAreaMonteCarlo, MultiAreaOutageScenario,
    MultiAreaSystem,
};
pub use elcc::*;
pub use featurize_geo::*;
pub use featurize_gnn::*;
pub use featurize_kpi::*;
pub use geo_join::*;
pub use io::*;
pub use opf::{ConstraintInfo, ConstraintType, OpfMethod, OpfSolution, OpfSolver};
pub use power_flow::*;
pub use reliability_monte_carlo::{
    DeliverabilityScore, DeliverabilityScoreConfig, MonteCarlo, OutageGenerator, OutageScenario,
    ReliabilityMetrics,
};
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
