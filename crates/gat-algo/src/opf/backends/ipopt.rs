//! IPOPT solver backend for NLP problems.

use std::collections::HashMap;
use std::path::Path;

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfBackend, OpfProblem, SolverConfig};
use crate::opf::OpfSolution;
use crate::OpfError;

/// IPOPT backend for nonlinear programming problems.
///
/// IPOPT is a state-of-the-art interior-point optimizer. This backend
/// requires the `solver-ipopt` feature and checks for IPOPT availability
/// at runtime.
pub struct IpoptBackend;

impl OpfBackend for IpoptBackend {
    fn id(&self) -> &str {
        "ipopt"
    }

    fn supported_classes(&self) -> &[ProblemClass] {
        &[ProblemClass::NonlinearProgram]
    }

    fn is_available(&self) -> bool {
        // Check if gat-ipopt binary exists in ~/.gat/solvers/
        // Note: Full PATH checking would require the `which` crate
        Path::new(&format!(
            "{}/.gat/solvers/gat-ipopt",
            std::env::var("HOME").unwrap_or_default()
        ))
        .exists()
    }

    fn solve(
        &self,
        problem: &OpfProblem,
        _config: &SolverConfig,
        _warm_start: Option<&HashMap<String, f64>>,
    ) -> Result<OpfSolution, OpfError> {
        if !self.is_available() {
            return Err(OpfError::NotImplemented(
                "IPOPT not installed. Run: cargo build -p gat-ipopt --features ipopt-sys --release"
                    .to_string(),
            ));
        }

        // TODO: Delegate to actual ac_nlp::solve_with_ipopt()
        Err(OpfError::NotImplemented(format!(
            "IpoptBackend::solve not yet implemented for {:?}",
            problem.problem_class
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipopt_backend_id() {
        let backend = IpoptBackend;
        assert_eq!(backend.id(), "ipopt");
    }

    #[test]
    fn test_ipopt_supports_nlp() {
        let backend = IpoptBackend;
        let classes = backend.supported_classes();
        assert!(classes.contains(&ProblemClass::NonlinearProgram));
    }

    // Note: is_available() depends on system state, so we don't assert its value
}
