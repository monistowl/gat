//! Economic dispatch formulation wrapper.

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use crate::OpfError;
use gat_core::Network;

/// Economic dispatch formulation (linear program, no network).
///
/// Wraps the existing `economic::solve()` implementation.
pub struct EconomicDispatchFormulation;

impl OpfFormulation for EconomicDispatchFormulation {
    fn id(&self) -> &str {
        "economic-dispatch"
    }

    fn problem_class(&self) -> ProblemClass {
        ProblemClass::LinearProgram
    }

    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError> {
        let stats = network.stats();
        Ok(OpfProblem {
            n_bus: stats.num_buses,
            n_gen: stats.num_gens,
            problem_class: ProblemClass::LinearProgram,
            data: Box::new(()),
        })
    }

    fn accepts_warm_start(&self) -> &[WarmStartKind] {
        &[WarmStartKind::Flat]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_economic_formulation_id() {
        let form = EconomicDispatchFormulation;
        assert_eq!(form.id(), "economic-dispatch");
    }

    #[test]
    fn test_economic_formulation_problem_class() {
        let form = EconomicDispatchFormulation;
        assert_eq!(form.problem_class(), ProblemClass::LinearProgram);
    }

    #[test]
    fn test_economic_formulation_warm_start() {
        let form = EconomicDispatchFormulation;
        let warm_starts = form.accepts_warm_start();
        assert_eq!(warm_starts.len(), 1);
        assert_eq!(warm_starts[0], WarmStartKind::Flat);
    }
}
