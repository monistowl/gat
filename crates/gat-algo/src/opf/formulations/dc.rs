//! DC-OPF formulation wrapper.

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use crate::OpfError;
use gat_core::Network;

/// DC-OPF formulation (linear program).
///
/// Wraps the existing `dc_opf::solve()` implementation.
pub struct DcOpfFormulation;

impl OpfFormulation for DcOpfFormulation {
    fn id(&self) -> &str {
        "dc-opf"
    }

    fn problem_class(&self) -> ProblemClass {
        ProblemClass::LinearProgram
    }

    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError> {
        // Store the network reference for the backend to use
        // In a real implementation, we'd precompute the B' matrix here
        let stats = network.stats();
        Ok(OpfProblem {
            n_bus: stats.num_buses,
            n_gen: stats.num_gens,
            problem_class: ProblemClass::LinearProgram,
            data: Box::new(()),
        })
    }

    fn accepts_warm_start(&self) -> &[WarmStartKind] {
        // DC-OPF is LP, doesn't benefit from warm-start
        &[WarmStartKind::Flat]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_formulation_id() {
        let form = DcOpfFormulation;
        assert_eq!(form.id(), "dc-opf");
    }

    #[test]
    fn test_dc_formulation_problem_class() {
        let form = DcOpfFormulation;
        assert_eq!(form.problem_class(), ProblemClass::LinearProgram);
    }

    #[test]
    fn test_dc_formulation_warm_start() {
        let form = DcOpfFormulation;
        let warm_starts = form.accepts_warm_start();
        assert_eq!(warm_starts.len(), 1);
        assert_eq!(warm_starts[0], WarmStartKind::Flat);
    }
}
