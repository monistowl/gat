//! L-BFGS solver backend for NLP problems.

use std::collections::HashMap;

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfBackend, OpfProblem, SolverConfig};
use crate::opf::OpfSolution;
use crate::OpfError;

/// L-BFGS backend for nonlinear programming problems.
///
/// Uses the argmin crate's L-BFGS implementation with augmented Lagrangian.
/// Always available as a pure-Rust fallback for AC-OPF.
pub struct LbfgsBackend;

impl OpfBackend for LbfgsBackend {
    fn id(&self) -> &str {
        "lbfgs"
    }

    fn supported_classes(&self) -> &[ProblemClass] {
        &[ProblemClass::NonlinearProgram]
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
        // TODO: Delegate to actual ac_nlp::solve_ac_opf()
        Err(OpfError::NotImplemented(format!(
            "LbfgsBackend::solve not yet implemented for {:?}",
            problem.problem_class
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lbfgs_backend_id() {
        let backend = LbfgsBackend;
        assert_eq!(backend.id(), "lbfgs");
    }

    #[test]
    fn test_lbfgs_is_always_available() {
        let backend = LbfgsBackend;
        assert!(backend.is_available());
    }

    #[test]
    fn test_lbfgs_supports_nlp() {
        let backend = LbfgsBackend;
        let classes = backend.supported_classes();
        assert!(classes.contains(&ProblemClass::NonlinearProgram));
        assert!(!classes.contains(&ProblemClass::LinearProgram));
    }
}
