//! OpfDispatcher orchestrates OPF solving with fallback chains.
//!
//! The dispatcher:
//! 1. Looks up the formulation by ID
//! 2. Builds the problem via `formulation.build_problem(network)`
//! 3. Selects the best available backend for the problem class
//! 4. Attempts solve; on failure, tries warm-starts from fallback chain

use std::sync::Arc;

use crate::OpfError;
use gat_core::Network;

use super::registry::SolverRegistry;
use super::traits::{SolverConfig, WarmStartKind};
use super::OpfSolution;

/// Orchestrates OPF solving with configurable fallback chains.
pub struct OpfDispatcher {
    registry: Arc<SolverRegistry>,
}

impl OpfDispatcher {
    /// Create a dispatcher with the given registry.
    pub fn new(registry: Arc<SolverRegistry>) -> Self {
        Self { registry }
    }

    /// Solve OPF for a network using the specified formulation.
    ///
    /// # Arguments
    /// * `network` - The power network to solve
    /// * `formulation_id` - ID of the formulation (e.g., "dc-opf", "ac-opf")
    /// * `config` - Solver configuration (iterations, tolerance, timeout)
    /// * `fallbacks` - Warm-start kinds to try if initial solve fails
    ///
    /// # Returns
    /// The solution, or the first error if all attempts fail.
    ///
    /// # Fallback Chain
    /// If the initial (flat-start) solve fails with a convergence error:
    /// 1. For each warm-start kind in `fallbacks`:
    ///    a. Run the corresponding formulation to get warm-start data
    ///    b. Retry the target solve with warm-start
    /// 2. If all fallbacks fail, return the original error
    pub fn solve(
        &self,
        network: &Network,
        formulation_id: &str,
        config: SolverConfig,
        fallbacks: &[WarmStartKind],
    ) -> Result<OpfSolution, OpfError> {
        // Look up formulation
        let formulation = self
            .registry
            .get_formulation(formulation_id)
            .ok_or_else(|| {
                OpfError::NotImplemented(format!("Unknown formulation: {}", formulation_id))
            })?;

        // Build problem
        let problem = formulation.build_problem(network)?;

        // Select backend
        let backend = self
            .registry
            .select_backend(problem.problem_class)
            .ok_or_else(|| {
                OpfError::NotImplemented(format!(
                    "No available backend for {:?}",
                    problem.problem_class
                ))
            })?;

        // Attempt flat-start solve
        match backend.solve(&problem, &config, None) {
            Ok(solution) => Ok(solution),
            Err(first_error) => {
                // Check if this is a convergence failure worth retrying
                if !Self::is_convergence_failure(&first_error) {
                    return Err(first_error);
                }

                // Try fallback warm-starts
                for &warm_start_kind in fallbacks {
                    // Skip Flat since that's what we just tried
                    if warm_start_kind == WarmStartKind::Flat {
                        continue;
                    }

                    // Get warm-start data from appropriate formulation
                    if let Some(warm_start_data) =
                        self.compute_warm_start(network, warm_start_kind, &config)
                    {
                        // Retry with warm-start
                        if let Ok(solution) =
                            backend.solve(&problem, &config, Some(&warm_start_data))
                        {
                            return Ok(solution);
                        }
                    }
                }

                // All fallbacks failed, return original error
                Err(first_error)
            }
        }
    }

    /// Check if an error is a convergence failure that might benefit from warm-start.
    fn is_convergence_failure(error: &OpfError) -> bool {
        matches!(
            error,
            OpfError::ConvergenceFailure { .. }
                | OpfError::Infeasible(_)
                | OpfError::NumericalIssue(_)
        )
    }

    /// Compute warm-start data from a simpler formulation.
    fn compute_warm_start(
        &self,
        network: &Network,
        kind: WarmStartKind,
        config: &SolverConfig,
    ) -> Option<std::collections::HashMap<String, f64>> {
        let formulation_id = match kind {
            WarmStartKind::Flat => return None,
            WarmStartKind::Dc => "dc-opf",
            WarmStartKind::Socp => "socp",
        };

        // Try to solve the warm-start formulation
        let formulation = self.registry.get_formulation(formulation_id)?;
        let problem = formulation.build_problem(network).ok()?;
        let backend = self.registry.select_backend(problem.problem_class)?;
        let solution = backend.solve(&problem, config, None).ok()?;

        // Convert solution to warm-start map
        // Keys: "Vm:<bus>", "Va:<bus>", "Pg:<gen>", "Qg:<gen>"
        let mut warm_start = std::collections::HashMap::new();

        for (bus, v) in &solution.bus_voltage_mag {
            warm_start.insert(format!("Vm:{}", bus), *v);
        }
        for (bus, a) in &solution.bus_voltage_ang {
            warm_start.insert(format!("Va:{}", bus), *a);
        }
        for (gen, p) in &solution.generator_p {
            warm_start.insert(format!("Pg:{}", gen), *p);
        }
        for (gen, q) in &solution.generator_q {
            warm_start.insert(format!("Qg:{}", gen), *q);
        }

        Some(warm_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opf::dispatch::ProblemClass;
    use crate::opf::traits::{OpfBackend, OpfFormulation, OpfProblem};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock formulation that always succeeds.
    struct SuccessFormulation;

    impl OpfFormulation for SuccessFormulation {
        fn id(&self) -> &str {
            "success"
        }
        fn problem_class(&self) -> ProblemClass {
            ProblemClass::LinearProgram
        }
        fn build_problem(&self, _network: &Network) -> Result<OpfProblem, OpfError> {
            Ok(OpfProblem {
                n_bus: 2,
                n_gen: 1,
                problem_class: ProblemClass::LinearProgram,
                data: Box::new(()),
            })
        }
        fn accepts_warm_start(&self) -> &[WarmStartKind] {
            &[WarmStartKind::Flat]
        }
    }

    /// Mock backend that succeeds on Nth attempt.
    struct MockBackend {
        fail_count: AtomicUsize,
    }

    impl MockBackend {
        fn new(fail_count: usize) -> Self {
            Self {
                fail_count: AtomicUsize::new(fail_count),
            }
        }
    }

    impl OpfBackend for MockBackend {
        fn id(&self) -> &str {
            "mock"
        }
        fn supported_classes(&self) -> &[ProblemClass] {
            &[ProblemClass::LinearProgram]
        }
        fn is_available(&self) -> bool {
            true
        }
        fn solve(
            &self,
            _problem: &OpfProblem,
            _config: &SolverConfig,
            _warm_start: Option<&HashMap<String, f64>>,
        ) -> Result<OpfSolution, OpfError> {
            let remaining = self.fail_count.fetch_sub(1, Ordering::SeqCst);
            if remaining > 0 {
                Err(OpfError::ConvergenceFailure {
                    iterations: 100,
                    residual: 1e-3,
                })
            } else {
                Ok(OpfSolution {
                    converged: true,
                    ..Default::default()
                })
            }
        }
    }

    fn create_test_network() -> Network {
        Network::new()
    }

    #[test]
    fn test_dispatcher_unknown_formulation() {
        let registry = Arc::new(SolverRegistry::new());
        let dispatcher = OpfDispatcher::new(registry);

        let result = dispatcher.solve(
            &create_test_network(),
            "unknown",
            SolverConfig::default(),
            &[],
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown formulation"));
    }

    #[test]
    fn test_dispatcher_no_backend_for_class() {
        let mut registry = SolverRegistry::new();
        registry.register_formulation(Arc::new(SuccessFormulation));
        // No backend registered

        let dispatcher = OpfDispatcher::new(Arc::new(registry));

        let result = dispatcher.solve(
            &create_test_network(),
            "success",
            SolverConfig::default(),
            &[],
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No available backend"));
    }

    #[test]
    fn test_dispatcher_success_on_first_try() {
        let mut registry = SolverRegistry::new();
        registry.register_formulation(Arc::new(SuccessFormulation));
        registry.register_backend(Arc::new(MockBackend::new(0))); // Succeed immediately

        let dispatcher = OpfDispatcher::new(Arc::new(registry));

        let result = dispatcher.solve(
            &create_test_network(),
            "success",
            SolverConfig::default(),
            &[],
        );

        assert!(result.is_ok());
        assert!(result.unwrap().converged);
    }

    #[test]
    fn test_is_convergence_failure() {
        assert!(OpfDispatcher::is_convergence_failure(
            &OpfError::ConvergenceFailure {
                iterations: 100,
                residual: 1e-3,
            }
        ));
        assert!(OpfDispatcher::is_convergence_failure(
            &OpfError::Infeasible("InfeasibleProblemDetected".to_string())
        ));
        assert!(!OpfDispatcher::is_convergence_failure(
            &OpfError::DataValidation("Missing data".to_string())
        ));
    }
}
