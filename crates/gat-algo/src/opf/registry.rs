//! Registry for OPF formulations and backends.
//!
//! The registry holds all registered components and provides lookup
//! by ID and filtering by problem class.

use std::collections::HashMap;
use std::sync::Arc;

use super::dispatch::ProblemClass;
use super::traits::{OpfBackend, OpfFormulation};

/// Holds all registered formulations and backends.
///
/// Create with `SolverRegistry::new()` for empty or
/// `SolverRegistry::with_defaults()` for built-in solvers.
#[derive(Default)]
pub struct SolverRegistry {
    formulations: HashMap<String, Arc<dyn OpfFormulation>>,
    backends: HashMap<String, Arc<dyn OpfBackend>>,
}

impl SolverRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create registry with built-in formulations and backends.
    ///
    /// Registers:
    /// - Formulations: dc-opf, socp, ac-opf, economic-dispatch
    /// - Backends: clarabel, lbfgs (+ ipopt if available)
    pub fn with_defaults() -> Self {
        let registry = Self::new();
        // TODO: Register built-in formulations and backends in Task 5
        registry
    }

    /// Register a custom formulation.
    pub fn register_formulation(&mut self, f: Arc<dyn OpfFormulation>) {
        self.formulations.insert(f.id().to_string(), f);
    }

    /// Register a custom backend.
    pub fn register_backend(&mut self, b: Arc<dyn OpfBackend>) {
        self.backends.insert(b.id().to_string(), b);
    }

    /// Get a formulation by ID.
    pub fn get_formulation(&self, id: &str) -> Option<Arc<dyn OpfFormulation>> {
        self.formulations.get(id).cloned()
    }

    /// Get a backend by ID.
    pub fn get_backend(&self, id: &str) -> Option<Arc<dyn OpfBackend>> {
        self.backends.get(id).cloned()
    }

    /// List all formulation IDs.
    pub fn list_formulations(&self) -> Vec<&str> {
        self.formulations.keys().map(|s| s.as_str()).collect()
    }

    /// List all backend IDs.
    pub fn list_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// List available backends for a problem class.
    ///
    /// Returns IDs of backends that:
    /// 1. Support the given problem class
    /// 2. Are currently available (runtime check)
    pub fn backends_for(&self, class: ProblemClass) -> Vec<&str> {
        self.backends
            .iter()
            .filter(|(_, b)| b.supported_classes().contains(&class) && b.is_available())
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Select the best available backend for a problem class.
    ///
    /// Priority:
    /// 1. Native solvers (IPOPT for NLP, HiGHS for LP/MIP)
    /// 2. Pure-Rust fallbacks (Clarabel for LP/SOCP, L-BFGS for NLP)
    pub fn select_backend(&self, class: ProblemClass) -> Option<Arc<dyn OpfBackend>> {
        // Preference order by problem class
        let preferred = match class {
            ProblemClass::LinearProgram => vec!["highs", "clarabel"],
            ProblemClass::ConicProgram => vec!["clarabel"],
            ProblemClass::NonlinearProgram => vec!["ipopt", "lbfgs"],
            ProblemClass::MixedInteger => vec!["highs", "cbc"],
        };

        // Find first available backend in preference order
        for id in preferred {
            if let Some(backend) = self.backends.get(id) {
                if backend.supported_classes().contains(&class) && backend.is_available() {
                    return Some(backend.clone());
                }
            }
        }

        // Fall back to any available backend for this class
        self.backends
            .values()
            .find(|b| b.supported_classes().contains(&class) && b.is_available())
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opf::traits::{OpfProblem, SolverConfig, WarmStartKind};
    use crate::opf::OpfSolution;
    use crate::OpfError;
    use gat_core::Network;

    /// Mock formulation for testing.
    struct MockFormulation {
        id: String,
        class: ProblemClass,
    }

    impl OpfFormulation for MockFormulation {
        fn id(&self) -> &str {
            &self.id
        }
        fn problem_class(&self) -> ProblemClass {
            self.class
        }
        fn build_problem(&self, _network: &Network) -> Result<OpfProblem, OpfError> {
            unimplemented!("mock")
        }
        fn accepts_warm_start(&self) -> &[WarmStartKind] {
            &[WarmStartKind::Flat]
        }
    }

    /// Mock backend for testing.
    struct MockBackend {
        id: String,
        classes: Vec<ProblemClass>,
        available: bool,
    }

    impl OpfBackend for MockBackend {
        fn id(&self) -> &str {
            &self.id
        }
        fn supported_classes(&self) -> &[ProblemClass] {
            &self.classes
        }
        fn is_available(&self) -> bool {
            self.available
        }
        fn solve(
            &self,
            _problem: &OpfProblem,
            _config: &SolverConfig,
            _warm_start: Option<&std::collections::HashMap<String, f64>>,
        ) -> Result<OpfSolution, OpfError> {
            unimplemented!("mock")
        }
    }

    #[test]
    fn test_register_and_get_formulation() {
        let mut registry = SolverRegistry::new();
        let form = Arc::new(MockFormulation {
            id: "test-form".to_string(),
            class: ProblemClass::LinearProgram,
        });

        registry.register_formulation(form.clone());

        let retrieved = registry.get_formulation("test-form");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-form");
    }

    #[test]
    fn test_register_and_get_backend() {
        let mut registry = SolverRegistry::new();
        let backend = Arc::new(MockBackend {
            id: "test-backend".to_string(),
            classes: vec![ProblemClass::LinearProgram],
            available: true,
        });

        registry.register_backend(backend);

        let retrieved = registry.get_backend("test-backend");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-backend");
    }

    #[test]
    fn test_backends_for_filters_by_class() {
        let mut registry = SolverRegistry::new();

        registry.register_backend(Arc::new(MockBackend {
            id: "lp-solver".to_string(),
            classes: vec![ProblemClass::LinearProgram],
            available: true,
        }));
        registry.register_backend(Arc::new(MockBackend {
            id: "nlp-solver".to_string(),
            classes: vec![ProblemClass::NonlinearProgram],
            available: true,
        }));

        let lp_backends = registry.backends_for(ProblemClass::LinearProgram);
        assert_eq!(lp_backends.len(), 1);
        assert!(lp_backends.contains(&"lp-solver"));

        let nlp_backends = registry.backends_for(ProblemClass::NonlinearProgram);
        assert_eq!(nlp_backends.len(), 1);
        assert!(nlp_backends.contains(&"nlp-solver"));
    }

    #[test]
    fn test_backends_for_excludes_unavailable() {
        let mut registry = SolverRegistry::new();

        registry.register_backend(Arc::new(MockBackend {
            id: "available".to_string(),
            classes: vec![ProblemClass::LinearProgram],
            available: true,
        }));
        registry.register_backend(Arc::new(MockBackend {
            id: "unavailable".to_string(),
            classes: vec![ProblemClass::LinearProgram],
            available: false,
        }));

        let backends = registry.backends_for(ProblemClass::LinearProgram);
        assert_eq!(backends.len(), 1);
        assert!(backends.contains(&"available"));
    }

    #[test]
    fn test_select_backend_returns_available() {
        let mut registry = SolverRegistry::new();

        registry.register_backend(Arc::new(MockBackend {
            id: "clarabel".to_string(),
            classes: vec![ProblemClass::LinearProgram, ProblemClass::ConicProgram],
            available: true,
        }));

        let selected = registry.select_backend(ProblemClass::LinearProgram);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id(), "clarabel");
    }

    #[test]
    fn test_select_backend_returns_none_when_empty() {
        let registry = SolverRegistry::new();
        let selected = registry.select_backend(ProblemClass::MixedInteger);
        assert!(selected.is_none());
    }

    #[test]
    fn test_list_formulations() {
        let mut registry = SolverRegistry::new();
        registry.register_formulation(Arc::new(MockFormulation {
            id: "form-a".to_string(),
            class: ProblemClass::LinearProgram,
        }));
        registry.register_formulation(Arc::new(MockFormulation {
            id: "form-b".to_string(),
            class: ProblemClass::NonlinearProgram,
        }));

        let ids = registry.list_formulations();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"form-a"));
        assert!(ids.contains(&"form-b"));
    }
}
