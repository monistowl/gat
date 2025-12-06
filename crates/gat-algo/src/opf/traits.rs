//! Core traits for the extensible OPF solver architecture.
//!
//! This module defines the Strategy pattern traits that allow new formulations
//! and backends to be added without modifying existing code.

use crate::OpfError;
use gat_core::Network;
use std::collections::HashMap;

use super::dispatch::ProblemClass;
use super::OpfSolution;

/// Kinds of warm-start data that can initialize a solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarmStartKind {
    /// Flat start: V=1.0, θ=0, Pg=Pmax/2
    Flat,
    /// From DC-OPF: angles and Pg only
    Dc,
    /// From SOCP: V, θ, Pg, Qg
    Socp,
}

/// Configuration passed to backend solvers.
#[derive(Debug, Clone)]
pub struct SolverConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
            timeout_seconds: 300,
        }
    }
}

/// Intermediate problem representation built from a Network.
///
/// This allows formulations to precompute data structures (Y-bus, etc.)
/// that backends can use for solving.
#[derive(Debug)]
pub struct OpfProblem {
    /// Number of buses
    pub n_bus: usize,
    /// Number of generators
    pub n_gen: usize,
    /// Problem class for solver matching
    pub problem_class: ProblemClass,
    /// Opaque data for the backend (formulation-specific)
    pub data: Box<dyn std::any::Any + Send + Sync>,
}

/// Defines a mathematical OPF formulation (what to solve).
///
/// Implementations include DC-OPF, SOCP relaxation, and full AC-OPF.
/// Each formulation knows how to build its problem representation from
/// a Network and what warm-start types it can accept.
pub trait OpfFormulation: Send + Sync {
    /// Unique identifier (e.g., "dc-opf", "ac-opf", "socp")
    fn id(&self) -> &str;

    /// Problem class for solver matching
    fn problem_class(&self) -> ProblemClass;

    /// Build the problem from a network
    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError>;

    /// Warm-start types this formulation can accept
    fn accepts_warm_start(&self) -> &[WarmStartKind];
}

/// Implements the actual solving (how to solve).
///
/// Backends are matched to formulations via ProblemClass. Multiple backends
/// may support the same class (e.g., Clarabel and HiGHS both solve LP).
pub trait OpfBackend: Send + Sync {
    /// Unique identifier (e.g., "clarabel", "ipopt", "lbfgs")
    fn id(&self) -> &str;

    /// Problem classes this backend can solve
    fn supported_classes(&self) -> &[ProblemClass];

    /// Check if this backend is available at runtime
    fn is_available(&self) -> bool;

    /// Solve the problem
    fn solve(
        &self,
        problem: &OpfProblem,
        config: &SolverConfig,
        warm_start: Option<&HashMap<String, f64>>,
    ) -> Result<OpfSolution, OpfError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that traits are object-safe (can be used with dyn).
    #[test]
    fn test_traits_are_object_safe() {
        // This test passes if it compiles - traits must be object-safe
        fn _accepts_formulation(_f: &dyn OpfFormulation) {}
        fn _accepts_backend(_b: &dyn OpfBackend) {}
    }

    /// Test that trait objects can be Send + Sync (required for Arc).
    #[test]
    fn test_traits_are_send_sync() {
        fn _assert_send<T: Send>() {}
        fn _assert_sync<T: Sync>() {}

        // These compile only if the trait objects are Send + Sync
        _assert_send::<Box<dyn OpfFormulation>>();
        _assert_sync::<Box<dyn OpfFormulation>>();
        _assert_send::<Box<dyn OpfBackend>>();
        _assert_sync::<Box<dyn OpfBackend>>();
    }

    /// Test default SolverConfig values.
    #[test]
    fn test_solver_config_defaults() {
        let config = SolverConfig::default();
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.tolerance, 1e-6);
        assert_eq!(config.timeout_seconds, 300);
    }

    /// Test WarmStartKind equality.
    #[test]
    fn test_warm_start_kind_eq() {
        assert_eq!(WarmStartKind::Flat, WarmStartKind::Flat);
        assert_ne!(WarmStartKind::Flat, WarmStartKind::Dc);
    }
}
