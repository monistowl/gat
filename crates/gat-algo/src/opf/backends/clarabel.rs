//! Clarabel solver backend for LP and SOCP problems.

use std::collections::HashMap;

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfBackend, OpfProblem, SolverConfig};
use crate::opf::OpfSolution;
use crate::OpfError;

/// Clarabel backend for LP and SOCP problems.
///
/// Clarabel is a pure-Rust interior-point solver that's always available.
pub struct ClarabelBackend;

impl OpfBackend for ClarabelBackend {
    fn id(&self) -> &str {
        "clarabel"
    }

    fn supported_classes(&self) -> &[ProblemClass] {
        &[ProblemClass::LinearProgram, ProblemClass::ConicProgram]
    }

    fn is_available(&self) -> bool {
        true // Always available (pure Rust)
    }

    fn solve(
        &self,
        problem: &OpfProblem,
        _config: &SolverConfig,
        _warm_start: Option<&HashMap<String, f64>>,
    ) -> Result<OpfSolution, OpfError> {
        // TODO: Delegate to actual dc_opf::solve() or socp::solve() based on problem class
        // For now, return a placeholder error
        Err(OpfError::NotImplemented(format!(
            "ClarabelBackend::solve not yet implemented for {:?}",
            problem.problem_class
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clarabel_backend_id() {
        let backend = ClarabelBackend;
        assert_eq!(backend.id(), "clarabel");
    }

    #[test]
    fn test_clarabel_is_always_available() {
        let backend = ClarabelBackend;
        assert!(backend.is_available());
    }

    #[test]
    fn test_clarabel_supports_lp_and_socp() {
        let backend = ClarabelBackend;
        let classes = backend.supported_classes();
        assert!(classes.contains(&ProblemClass::LinearProgram));
        assert!(classes.contains(&ProblemClass::ConicProgram));
        assert!(!classes.contains(&ProblemClass::NonlinearProgram));
    }
}
