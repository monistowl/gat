//! SOCP relaxation formulation wrapper.

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use crate::OpfError;
use gat_core::Network;

/// SOCP relaxation formulation (conic program).
///
/// Wraps the existing `socp::solve()` implementation.
pub struct SocpFormulation;

impl OpfFormulation for SocpFormulation {
    fn id(&self) -> &str {
        "socp"
    }

    fn problem_class(&self) -> ProblemClass {
        ProblemClass::ConicProgram
    }

    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError> {
        let stats = network.stats();
        Ok(OpfProblem {
            n_bus: stats.num_buses,
            n_gen: stats.num_gens,
            problem_class: ProblemClass::ConicProgram,
            data: Box::new(()),
        })
    }

    fn accepts_warm_start(&self) -> &[WarmStartKind] {
        // SOCP can use DC warm-start for initial point
        &[WarmStartKind::Flat, WarmStartKind::Dc]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socp_formulation_id() {
        let form = SocpFormulation;
        assert_eq!(form.id(), "socp");
    }

    #[test]
    fn test_socp_formulation_problem_class() {
        let form = SocpFormulation;
        assert_eq!(form.problem_class(), ProblemClass::ConicProgram);
    }

    #[test]
    fn test_socp_formulation_warm_start() {
        let form = SocpFormulation;
        let warm_starts = form.accepts_warm_start();
        assert_eq!(warm_starts.len(), 2);
        assert!(warm_starts.contains(&WarmStartKind::Flat));
        assert!(warm_starts.contains(&WarmStartKind::Dc));
    }
}
