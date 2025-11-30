//! Solver dispatch and selection logic.
//!
//! This module determines which solver backend to use for OPF problems:
//! - Pure-Rust solvers (Clarabel for SOCP, L-BFGS for NLP) are always available
//! - Native solvers (IPOPT, CBC, HiGHS) require installation and user consent
//!
//! # Solver Selection Priority
//!
//! 1. User-specified solver (if installed and enabled)
//! 2. Best available solver for the problem type
//! 3. Fallback to pure-Rust solver
//!
//! # Problem Class Mapping
//!
//! | OPF Method | Problem Class | Recommended Solver |
//! |------------|---------------|-------------------|
//! | DC-OPF | LinearProgram | Clarabel, HiGHS |
//! | SOCP Relaxation | ConicProgram | Clarabel |
//! | AC-OPF | NonlinearProgram | IPOPT, L-BFGS |
//! | Unit Commitment | MixedInteger | CBC, HiGHS |
//!
//! # Algorithm References
//!
//! - **Interior-point methods:** Wächter & Biegler (2006), doi:[10.1007/s10107-004-0559-y]
//! - **Simplex method:** Dantzig (1963), *Linear Programming and Extensions*
//! - **Branch-and-cut:** Padberg & Rinaldi (1991), doi:[10.1137/1033004]
//! - **SOCP relaxation for OPF:** Jabr (2006), doi:[10.1109/TPWRS.2006.876672]
//!
//! [10.1007/s10107-004-0559-y]: https://doi.org/10.1007/s10107-004-0559-y
//! [10.1137/1033004]: https://doi.org/10.1137/1033004
//! [10.1109/TPWRS.2006.876672]: https://doi.org/10.1109/TPWRS.2006.876672

use crate::OpfError;

/// Represents the available solver backends.
///
/// Solver backends are divided into two categories:
///
/// 1. **Pure-Rust** - Ship with GAT, no external dependencies
/// 2. **Native** - Require installation, better performance for large problems
///
/// The dispatch system automatically selects the best available solver for
/// each problem type, preferring native solvers when available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolverBackend {
    // === Pure-Rust solvers (always available) ===
    /// Clarabel - Interior-point conic solver for SOCP/SDP.
    ///
    /// Uses a homogeneous self-dual embedding with Nesterov-Todd scaling.
    /// Primary solver for SOCP relaxation of AC-OPF.
    ///
    /// **Complexity:** O(n³) per iteration, O(√n log(1/ε)) iterations
    /// **Best for:** SOCP relaxation, moderate-size networks (< 10k buses)
    Clarabel,

    /// L-BFGS with augmented Lagrangian for constrained NLP.
    ///
    /// Limited-memory BFGS approximates the Hessian using m recent gradient
    /// vectors (typically m=10). Combined with augmented Lagrangian method
    /// to handle power flow constraints.
    ///
    /// **Complexity:** O(mn) per iteration where m = memory depth
    /// **Best for:** AC-OPF fallback when IPOPT unavailable
    /// **Reference:** Nocedal, J. (1980). Updating quasi-Newton matrices with
    /// limited storage. *Mathematics of Computation*, 35(151), 773-782.
    /// doi:[10.1090/S0025-5718-1980-0572855-7](https://doi.org/10.1090/S0025-5718-1980-0572855-7)
    Lbfgs,

    // === Native solvers (require installation) ===
    /// IPOPT - Interior Point OPTimizer for large-scale NLP.
    ///
    /// State-of-the-art primal-dual interior-point method with filter
    /// line-search. Industry standard for AC-OPF.
    ///
    /// **Complexity:** O(n²) per iteration (with sparse linear algebra)
    /// **Best for:** AC-OPF on large networks (> 1000 buses)
    /// **Reference:** Wächter & Biegler (2006), doi:10.1007/s10107-004-0559-y
    #[cfg(feature = "native-dispatch")]
    Ipopt,

    /// HiGHS - High-performance LP/MIP solver.
    ///
    /// Dual revised simplex and interior-point for LP, branch-and-cut for MIP.
    /// Open-source successor to CPLEX's academic algorithms.
    ///
    /// **Best for:** DC-OPF, unit commitment
    /// **Reference:** Huangfu & Hall (2018), doi:10.1007/s12532-017-0130-5
    #[cfg(feature = "native-dispatch")]
    Highs,

    /// CBC - COIN-OR Branch and Cut for MIP.
    ///
    /// Uses LP relaxation with Gomory cuts, mixed-integer rounding,
    /// and clique cuts. Good general-purpose MIP solver.
    ///
    /// **Best for:** Unit commitment, network design with discrete decisions
    #[cfg(feature = "native-dispatch")]
    Cbc,
}

impl SolverBackend {
    /// Check if this is a native (non-Rust) solver.
    pub fn is_native(&self) -> bool {
        match self {
            SolverBackend::Clarabel | SolverBackend::Lbfgs => false,
            #[cfg(feature = "native-dispatch")]
            _ => true,
        }
    }

    /// Get the display name for this solver.
    pub fn display_name(&self) -> &'static str {
        match self {
            SolverBackend::Clarabel => "Clarabel",
            SolverBackend::Lbfgs => "L-BFGS",
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Ipopt => "IPOPT",
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Highs => "HiGHS",
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Cbc => "CBC",
        }
    }

    /// Get a description of this solver's capabilities.
    pub fn description(&self) -> &'static str {
        match self {
            SolverBackend::Clarabel => "Conic solver for SOCP/SDP (pure Rust)",
            SolverBackend::Lbfgs => "Quasi-Newton NLP with penalty method (pure Rust)",
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Ipopt => "Interior point optimizer for large-scale NLP",
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Highs => "High-performance LP/MIP solver",
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Cbc => "Branch and cut MIP solver",
        }
    }
}

/// Problem class for solver selection.
///
/// Each OPF formulation maps to a mathematical optimization problem class,
/// which determines which solvers can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemClass {
    /// Linear programming (LP) - DC-OPF, economic dispatch.
    ///
    /// The DC power flow approximation linearizes the AC equations by assuming:
    /// - Voltage magnitudes fixed at 1.0 p.u.
    /// - Small angle differences (sin θ ≈ θ)
    /// - Negligible line losses (R << X)
    ///
    /// **Reference:** Stott, B., Jardim, J., & Alsaç, O. (2009). DC power flow
    /// revisited. *IEEE Trans. Power Systems*, 24(3), 1290-1300.
    /// doi:[10.1109/TPWRS.2009.2021235](https://doi.org/10.1109/TPWRS.2009.2021235)
    LinearProgram,

    /// Second-order cone programming (SOCP) - convex relaxation of AC-OPF.
    ///
    /// Relaxes the nonconvex power flow equations into second-order cone
    /// constraints. Provides a lower bound on the true AC-OPF objective.
    /// Exact for radial networks under mild conditions.
    ///
    /// **Reference:** Jabr, R. A. (2006). Radial distribution load flow using
    /// conic programming. *IEEE Trans. Power Systems*, 21(3), 1458-1459.
    /// doi:[10.1109/TPWRS.2006.879234](https://doi.org/10.1109/TPWRS.2006.879234)
    ConicProgram,

    /// Nonlinear programming (NLP) - full AC-OPF.
    ///
    /// Solves the nonconvex AC power flow equations exactly using interior-point
    /// methods. Most accurate but may find local optima.
    ///
    /// Standard AC-OPF formulation:
    /// - min  Σᵢ fᵢ(Pgᵢ)           (generator costs)
    /// - s.t. Pᵢ = Σⱼ |Vᵢ||Vⱼ|Yᵢⱼcos(θᵢ-θⱼ-φᵢⱼ)  (real power balance)
    ///        Qᵢ = Σⱼ |Vᵢ||Vⱼ|Yᵢⱼsin(θᵢ-θⱼ-φᵢⱼ)  (reactive power balance)
    ///
    /// **Reference:** Carpentier, J. (1962). Contribution à l'étude du dispatching
    /// économique. *Bulletin de la Société Française des Électriciens*, 3(8), 431-447.
    NonlinearProgram,

    /// Mixed-integer programming (MIP) - unit commitment, network design.
    ///
    /// Includes binary/integer variables for on/off decisions. Used for:
    /// - Unit commitment (generator scheduling)
    /// - Transmission expansion planning
    /// - Network reconfiguration
    ///
    /// **Reference:** Padberg, M., & Rinaldi, G. (1991). A branch-and-cut algorithm
    /// for the resolution of large-scale symmetric traveling salesman problems.
    /// *SIAM Review*, 33(1), 60-100. doi:[10.1137/1033004](https://doi.org/10.1137/1033004)
    MixedInteger,
}

/// Configuration for solver dispatch.
#[derive(Debug, Clone)]
pub struct DispatchConfig {
    /// Allow native solvers (requires user consent).
    pub native_enabled: bool,
    /// Preferred solver for LP problems.
    pub preferred_lp: Option<SolverBackend>,
    /// Preferred solver for NLP problems.
    pub preferred_nlp: Option<SolverBackend>,
    /// Timeout in seconds (0 = no timeout).
    pub timeout_seconds: u64,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            native_enabled: false,
            preferred_lp: None,
            preferred_nlp: None,
            timeout_seconds: 300,
        }
    }
}

/// Solver dispatcher that selects the best available solver.
pub struct SolverDispatcher {
    config: DispatchConfig,
    #[cfg(feature = "native-dispatch")]
    installed_native: Vec<gat_solver_common::SolverId>,
}

impl SolverDispatcher {
    /// Create a new dispatcher with default config.
    pub fn new() -> Self {
        Self {
            config: DispatchConfig::default(),
            #[cfg(feature = "native-dispatch")]
            installed_native: Vec::new(),
        }
    }

    /// Create a dispatcher with the given config.
    pub fn with_config(config: DispatchConfig) -> Self {
        let mut dispatcher = Self::new();
        dispatcher.config = config;
        dispatcher
    }

    /// Update the list of installed native solvers.
    #[cfg(feature = "native-dispatch")]
    pub fn set_installed_solvers(&mut self, solvers: Vec<gat_solver_common::SolverId>) {
        self.installed_native = solvers;
    }

    /// Select the best solver for the given problem class.
    pub fn select(&self, problem_class: ProblemClass) -> Result<SolverBackend, OpfError> {
        match problem_class {
            ProblemClass::LinearProgram | ProblemClass::ConicProgram => {
                // For LP/SOCP, prefer Clarabel (pure Rust, always available)
                if let Some(preferred) = self.config.preferred_lp {
                    if self.is_available(preferred) {
                        return Ok(preferred);
                    }
                }
                Ok(SolverBackend::Clarabel)
            }
            ProblemClass::NonlinearProgram => {
                // For NLP, prefer IPOPT if native is enabled and installed
                #[cfg(feature = "native-dispatch")]
                if self.config.native_enabled {
                    if let Some(preferred) = self.config.preferred_nlp {
                        if self.is_available(preferred) {
                            return Ok(preferred);
                        }
                    }
                    // Check if IPOPT is installed
                    if self
                        .installed_native
                        .contains(&gat_solver_common::SolverId::Ipopt)
                    {
                        return Ok(SolverBackend::Ipopt);
                    }
                }

                // Fallback to L-BFGS (pure Rust)
                Ok(SolverBackend::Lbfgs)
            }
            ProblemClass::MixedInteger => {
                // For MIP, we need a native solver
                #[cfg(feature = "native-dispatch")]
                if self.config.native_enabled {
                    if self
                        .installed_native
                        .contains(&gat_solver_common::SolverId::Cbc)
                    {
                        return Ok(SolverBackend::Cbc);
                    }
                    if self
                        .installed_native
                        .contains(&gat_solver_common::SolverId::Highs)
                    {
                        return Ok(SolverBackend::Highs);
                    }
                }

                Err(OpfError::NotImplemented(
                    "No MIP solver available. Install CBC or HiGHS: `gat install cbc`".to_string(),
                ))
            }
        }
    }

    /// Check if a solver backend is available.
    fn is_available(&self, backend: SolverBackend) -> bool {
        match backend {
            SolverBackend::Clarabel | SolverBackend::Lbfgs => true,
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Ipopt => {
                self.config.native_enabled
                    && self
                        .installed_native
                        .contains(&gat_solver_common::SolverId::Ipopt)
            }
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Highs => {
                self.config.native_enabled
                    && self
                        .installed_native
                        .contains(&gat_solver_common::SolverId::Highs)
            }
            #[cfg(feature = "native-dispatch")]
            SolverBackend::Cbc => {
                self.config.native_enabled
                    && self
                        .installed_native
                        .contains(&gat_solver_common::SolverId::Cbc)
            }
        }
    }

    /// List all available solvers.
    pub fn list_available(&self) -> Vec<SolverBackend> {
        #[allow(unused_mut)]
        let mut solvers = vec![SolverBackend::Clarabel, SolverBackend::Lbfgs];

        #[cfg(feature = "native-dispatch")]
        if self.config.native_enabled {
            for solver_id in &self.installed_native {
                match solver_id {
                    gat_solver_common::SolverId::Ipopt => solvers.push(SolverBackend::Ipopt),
                    gat_solver_common::SolverId::Highs => solvers.push(SolverBackend::Highs),
                    gat_solver_common::SolverId::Cbc => solvers.push(SolverBackend::Cbc),
                    _ => {}
                }
            }
        }

        solvers
    }
}

impl Default for SolverDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_dispatcher_selects_pure_rust() {
        let dispatcher = SolverDispatcher::new();

        // LP should use Clarabel
        let lp_solver = dispatcher.select(ProblemClass::LinearProgram).unwrap();
        assert_eq!(lp_solver, SolverBackend::Clarabel);

        // SOCP should use Clarabel
        let socp_solver = dispatcher.select(ProblemClass::ConicProgram).unwrap();
        assert_eq!(socp_solver, SolverBackend::Clarabel);

        // NLP should use L-BFGS (native disabled by default)
        let nlp_solver = dispatcher.select(ProblemClass::NonlinearProgram).unwrap();
        assert_eq!(nlp_solver, SolverBackend::Lbfgs);
    }

    #[test]
    fn test_mip_fails_without_native() {
        let dispatcher = SolverDispatcher::new();
        let result = dispatcher.select(ProblemClass::MixedInteger);
        assert!(result.is_err());
    }

    #[test]
    fn test_solver_backend_properties() {
        assert!(!SolverBackend::Clarabel.is_native());
        assert!(!SolverBackend::Lbfgs.is_native());
        assert_eq!(SolverBackend::Clarabel.display_name(), "Clarabel");
    }
}
