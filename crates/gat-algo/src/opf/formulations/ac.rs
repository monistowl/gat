//! Full nonlinear AC-OPF formulation wrapper.

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use crate::OpfError;
use gat_core::Network;

/// Full nonlinear AC-OPF formulation (nonlinear program).
///
/// Wraps the existing `ac_nlp::AcOpfProblem` implementation.
pub struct AcOpfFormulation;

impl OpfFormulation for AcOpfFormulation {
    fn id(&self) -> &str {
        "ac-opf"
    }

    fn problem_class(&self) -> ProblemClass {
        ProblemClass::NonlinearProgram
    }

    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError> {
        let stats = network.stats();
        Ok(OpfProblem {
            n_bus: stats.num_buses,
            n_gen: stats.num_gens,
            problem_class: ProblemClass::NonlinearProgram,
            data: Box::new(()),
        })
    }

    fn accepts_warm_start(&self) -> &[WarmStartKind] {
        // AC-OPF benefits from DC and SOCP warm-starts
        &[WarmStartKind::Flat, WarmStartKind::Dc, WarmStartKind::Socp]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ac_formulation_id() {
        let form = AcOpfFormulation;
        assert_eq!(form.id(), "ac-opf");
    }

    #[test]
    fn test_ac_formulation_problem_class() {
        let form = AcOpfFormulation;
        assert_eq!(form.problem_class(), ProblemClass::NonlinearProgram);
    }

    #[test]
    fn test_ac_formulation_accepts_all_warm_starts() {
        let form = AcOpfFormulation;
        let warm_starts = form.accepts_warm_start();
        assert!(warm_starts.contains(&WarmStartKind::Flat));
        assert!(warm_starts.contains(&WarmStartKind::Dc));
        assert!(warm_starts.contains(&WarmStartKind::Socp));
    }
}
