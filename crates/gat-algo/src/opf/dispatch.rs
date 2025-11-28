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

use crate::OpfError;

/// Represents the available solver backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolverBackend {
    // === Pure-Rust solvers (always available) ===
    /// Clarabel - Conic solver for SOCP relaxation
    Clarabel,
    /// L-BFGS penalty method for NLP
    Lbfgs,

    // === Native solvers (require installation) ===
    /// IPOPT - Interior point optimizer for AC-OPF
    #[cfg(feature = "native-dispatch")]
    Ipopt,
    /// HiGHS - High-performance LP/MIP solver
    #[cfg(feature = "native-dispatch")]
    Highs,
    /// CBC - Branch and cut MIP solver
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemClass {
    /// Linear programming (DC-OPF, economic dispatch)
    LinearProgram,
    /// Second-order cone programming (SOCP relaxation)
    ConicProgram,
    /// Nonlinear programming (AC-OPF)
    NonlinearProgram,
    /// Mixed-integer programming
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
