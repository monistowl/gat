//! Common types and IPC protocol for GAT solver plugins.
//!
//! This crate defines the Arrow IPC schema for communication between the main
//! `gat` binary and external solver plugins (e.g., `gat-ipopt`, `gat-cbc`).
//!
//! # Architecture
//!
//! The plugin system uses a subprocess model with Arrow IPC for zero-copy data
//! transfer. This design isolates solver failures, allows mixing Rust and C++
//! code safely, and enables parallel solver execution.
//!
//! ```text
//! gat (main) ──stdin──> gat-ipopt (subprocess)
//!            <─stdout──
//!            <─stderr── (logs/errors)
//! ```
//!
//! # Supported Solvers
//!
//! ## Native Solvers (require installation)
//!
//! | Solver | Problem Type | Reference |
//! |--------|--------------|-----------|
//! | IPOPT  | NLP | Wächter & Biegler (2006) doi:[10.1007/s10107-004-0559-y] |
//! | HiGHS  | LP/MIP | Huangfu & Hall (2018) doi:[10.1007/s12532-017-0130-5] |
//! | CBC    | MIP | COIN-OR Branch & Cut |
//! | Bonmin | MINLP | Bonami et al. (2008) doi:[10.1016/j.disopt.2006.10.011] |
//!
//! ## Pure-Rust Solvers (always available)
//!
//! | Solver | Problem Type | Reference |
//! |--------|--------------|-----------|
//! | Clarabel | SOCP/SDP | Goulart et al. (2024) |
//! | L-BFGS | NLP | Liu & Nocedal (1989) doi:[10.1007/BF01589116] |
//!
//! # Protocol Version
//!
//! The IPC protocol is versioned to ensure compatibility between `gat` and
//! solver plugins. Breaking changes increment [`PROTOCOL_VERSION`].
//!
//! [10.1007/s10107-004-0559-y]: https://doi.org/10.1007/s10107-004-0559-y
//! [10.1007/s12532-017-0130-5]: https://doi.org/10.1007/s12532-017-0130-5
//! [10.1016/j.disopt.2006.10.011]: https://doi.org/10.1016/j.disopt.2006.10.011
//! [10.1007/BF01589116]: https://doi.org/10.1007/BF01589116

pub mod error;
pub mod ipc;
pub mod plugin;
pub mod problem;
pub mod solution;
pub mod subprocess;

pub use error::{ExitCode, SolverError};
pub use plugin::{run_solver_plugin, SolverPlugin};
pub use problem::{ProblemBatch, ProblemType};
pub use solution::{SolutionBatch, SolutionStatus};
pub use subprocess::SolverProcess;

/// Protocol version for IPC compatibility checking.
/// Increment when making breaking changes to the schema.
pub const PROTOCOL_VERSION: i32 = 1;

/// Available native solvers that can be installed.
///
/// Each solver implements specific optimization algorithms suitable for
/// different problem classes in power systems optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SolverId {
    /// IPOPT - Interior Point OPTimizer for large-scale NLP.
    ///
    /// Uses a primal-dual interior-point filter line-search algorithm.
    /// Primary solver for AC-OPF problems.
    ///
    /// **Algorithm:** Primal-dual interior-point with filter line-search
    /// **Reference:** Wächter, A., & Biegler, L. T. (2006). On the implementation
    /// of an interior-point filter line-search algorithm for large-scale nonlinear
    /// programming. *Mathematical Programming*, 106(1), 25-57.
    /// **DOI:** [10.1007/s10107-004-0559-y](https://doi.org/10.1007/s10107-004-0559-y)
    Ipopt,

    /// CLP - COIN-OR LP solver for linear programs.
    ///
    /// Efficient dual revised simplex method for large sparse LP problems.
    /// Used for DC-OPF and other linear relaxations.
    ///
    /// **Algorithm:** Dual revised simplex with presolve
    /// **Reference:** Forrest, J., & Lougee-Heimer, R. (2005). CBC User's Guide.
    /// In *Emerging Theory, Methods, and Applications* (pp. 257-277). INFORMS.
    /// **DOI:** [10.1287/educ.1053.0020](https://doi.org/10.1287/educ.1053.0020)
    Clp,

    /// CBC - COIN-OR Branch and Cut for MIP.
    ///
    /// Solves mixed-integer linear programs using branch-and-cut with
    /// cutting planes (Gomory, MIR, clique cuts).
    ///
    /// **Algorithm:** Branch-and-cut with LP relaxation
    /// **Reference:** COIN-OR Foundation. [github.com/coin-or/Cbc](https://github.com/coin-or/Cbc)
    Cbc,

    /// HiGHS - High-performance LP/MIP solver.
    ///
    /// Uses dual revised simplex for LP and branch-and-cut for MIP.
    /// Successor to CPLEX's academic algorithms.
    ///
    /// **Algorithm:** Dual revised simplex, interior-point (LP); branch-and-cut (MIP)
    /// **Reference:** Huangfu, Q., & Hall, J. A. J. (2018). Parallelizing the dual
    /// revised simplex method. *Mathematical Programming Computation*, 10(1), 119-142.
    /// **DOI:** [10.1007/s12532-017-0130-5](https://doi.org/10.1007/s12532-017-0130-5)
    Highs,

    /// Bonmin - Basic Open-source Nonlinear Mixed INteger programming.
    ///
    /// Solves convex MINLP using branch-and-bound with NLP relaxations
    /// solved by IPOPT at each node.
    ///
    /// **Algorithm:** NLP-based branch-and-bound, outer approximation
    /// **Reference:** Bonami, P., et al. (2008). An algorithmic framework for convex
    /// mixed integer nonlinear programs. *Discrete Optimization*, 5(2), 186-204.
    /// **DOI:** [10.1016/j.disopt.2006.10.011](https://doi.org/10.1016/j.disopt.2006.10.011)
    Bonmin,

    /// Couenne - Convex Over and Under ENvelopes for Nonlinear Estimation.
    ///
    /// Global optimizer for non-convex MINLPs using spatial branch-and-bound
    /// with convex relaxations.
    ///
    /// **Algorithm:** Spatial branch-and-bound with McCormick relaxations
    /// **Reference:** Belotti, P., et al. (2009). Branching and bounds tightening
    /// techniques for non-convex MINLP. *Optimization Methods and Software*, 24(4-5), 597-634.
    /// **DOI:** [10.1080/10556780903087124](https://doi.org/10.1080/10556780903087124)
    Couenne,

    /// SYMPHONY - COIN-OR parallel MIP solver.
    ///
    /// Designed for distributed computing with master-worker parallelism.
    ///
    /// **Algorithm:** Parallel branch-cut-price
    /// **Reference:** Ralphs, T. K., & Güzelsoy, M. (2005). The SYMPHONY callable
    /// library for mixed integer programming. *The Next Wave in Computing, Optimization,
    /// and Decision Technologies*, 61-76.
    /// **DOI:** [10.1007/0-387-23529-9_5](https://doi.org/10.1007/0-387-23529-9_5)
    Symphony,
}

impl SolverId {
    /// Get the binary name for this solver.
    pub fn binary_name(&self) -> &'static str {
        match self {
            SolverId::Ipopt => "gat-ipopt",
            SolverId::Clp => "gat-clp",
            SolverId::Cbc => "gat-cbc",
            SolverId::Highs => "gat-highs",
            SolverId::Bonmin => "gat-bonmin",
            SolverId::Couenne => "gat-couenne",
            SolverId::Symphony => "gat-symphony",
        }
    }

    /// Get the display name for this solver.
    pub fn display_name(&self) -> &'static str {
        match self {
            SolverId::Ipopt => "IPOPT",
            SolverId::Clp => "CLP",
            SolverId::Cbc => "CBC",
            SolverId::Highs => "HiGHS",
            SolverId::Bonmin => "Bonmin",
            SolverId::Couenne => "Couenne",
            SolverId::Symphony => "SYMPHONY",
        }
    }

    /// Get a description of what this solver does.
    pub fn description(&self) -> &'static str {
        match self {
            SolverId::Ipopt => "NLP interior-point optimizer",
            SolverId::Clp => "LP dual simplex",
            SolverId::Cbc => "MIP branch-and-cut",
            SolverId::Highs => "LP/MIP high-performance",
            SolverId::Bonmin => "MINLP branch-and-bound",
            SolverId::Couenne => "Global optimization",
            SolverId::Symphony => "Parallel MIP",
        }
    }

    /// Get all available solver IDs.
    pub fn all() -> &'static [SolverId] {
        &[
            SolverId::Ipopt,
            SolverId::Clp,
            SolverId::Cbc,
            SolverId::Highs,
            SolverId::Bonmin,
            SolverId::Couenne,
            SolverId::Symphony,
        ]
    }
}

impl std::fmt::Display for SolverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for SolverId {
    type Err = SolverError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ipopt" => Ok(SolverId::Ipopt),
            "clp" => Ok(SolverId::Clp),
            "cbc" => Ok(SolverId::Cbc),
            "highs" => Ok(SolverId::Highs),
            "bonmin" => Ok(SolverId::Bonmin),
            "couenne" => Ok(SolverId::Couenne),
            "symphony" => Ok(SolverId::Symphony),
            _ => Err(SolverError::UnknownSolver(s.to_string())),
        }
    }
}

/// Pure-Rust solvers that are always available (no native code).
///
/// These solvers require no external dependencies and work on any platform.
/// They provide reliable fallback behavior when native solvers aren't installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PureRustSolver {
    /// Clarabel - Interior-point solver for conic programs (SOCP, SDP).
    ///
    /// Solves second-order cone programs (SOCP) used in convex relaxations
    /// of AC-OPF. Supports both SOCP and semidefinite programming (SDP).
    ///
    /// **Algorithm:** Homogeneous self-dual interior-point method
    /// **Reference:** Goulart, P., Chen, Y., & Schwan, M. (2024). Clarabel:
    /// An interior-point solver for conic programs with quadratic objectives.
    /// [github.com/oxfordcontrol/Clarabel.rs](https://github.com/oxfordcontrol/Clarabel.rs)
    Clarabel,

    /// L-BFGS - Limited-memory BFGS quasi-Newton method for NLP.
    ///
    /// Memory-efficient approximation of Newton's method that stores only
    /// a limited number of gradient vectors. Used with penalty/barrier methods
    /// for constrained AC-OPF.
    ///
    /// **Algorithm:** Limited-memory BFGS with line search
    /// **Reference:** Liu, D. C., & Nocedal, J. (1989). On the limited memory
    /// BFGS method for large scale optimization. *Mathematical Programming*,
    /// 45(1-3), 503-528.
    /// **DOI:** [10.1007/BF01589116](https://doi.org/10.1007/BF01589116)
    ///
    /// **Note:** For AC-OPF, we combine L-BFGS with an augmented Lagrangian
    /// method to handle equality and inequality constraints.
    Lbfgs,
}

impl PureRustSolver {
    pub fn display_name(&self) -> &'static str {
        match self {
            PureRustSolver::Clarabel => "Clarabel",
            PureRustSolver::Lbfgs => "L-BFGS",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PureRustSolver::Clarabel => "Conic (SOCP, SDP)",
            PureRustSolver::Lbfgs => "NLP penalty method",
        }
    }
}

/// The resolved solver choice for a problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverChoice {
    /// Use a native solver plugin (subprocess).
    Native(SolverId),
    /// Use a pure-Rust solver (in-process).
    PureRust(PureRustSolver),
}

impl std::fmt::Display for SolverChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverChoice::Native(id) => write!(f, "{}", id),
            SolverChoice::PureRust(solver) => write!(f, "{}", solver.display_name()),
        }
    }
}
