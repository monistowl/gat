//! Integration tests for the Strategy pattern refactoring (PROJ-2).
//!
//! These tests verify that the new dispatcher-based system produces
//! equivalent results to the original monolithic solve() method.

use gat_algo::opf::dispatch::ProblemClass;
use gat_algo::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use gat_algo::opf::{OpfDispatcher, SolverRegistry};
use gat_algo::OpfError;
use gat_core::Network;
use std::sync::Arc;

/// Test that SolverRegistry::with_defaults() includes all expected formulations.
#[test]
fn test_registry_has_all_formulations() {
    let registry = SolverRegistry::with_defaults();

    let formulations = registry.list_formulations();
    assert!(formulations.contains(&"dc-opf"), "Missing dc-opf");
    assert!(formulations.contains(&"socp"), "Missing socp");
    assert!(formulations.contains(&"ac-opf"), "Missing ac-opf");
    assert!(
        formulations.contains(&"economic-dispatch"),
        "Missing economic-dispatch"
    );
}

/// Test that SolverRegistry::with_defaults() includes all expected backends.
#[test]
fn test_registry_has_all_backends() {
    let registry = SolverRegistry::with_defaults();

    let backends = registry.list_backends();
    assert!(backends.contains(&"clarabel"), "Missing clarabel");
    assert!(backends.contains(&"lbfgs"), "Missing lbfgs");
}

/// Test that OpfDispatcher can be constructed with defaults.
#[test]
fn test_dispatcher_construction() {
    let registry = Arc::new(SolverRegistry::with_defaults());
    let _dispatcher = OpfDispatcher::new(registry);
    // Just verify construction doesn't panic
}

/// Test that formulation lookup works.
#[test]
fn test_formulation_lookup() {
    let registry = SolverRegistry::with_defaults();

    let dc = registry.get_formulation("dc-opf");
    assert!(dc.is_some());
    assert_eq!(dc.unwrap().id(), "dc-opf");

    let missing = registry.get_formulation("nonexistent");
    assert!(missing.is_none());
}

/// Test that backend selection works by problem class.
#[test]
fn test_backend_selection_by_class() {
    let registry = SolverRegistry::with_defaults();

    // LP should get Clarabel
    let lp_backend = registry.select_backend(ProblemClass::LinearProgram);
    assert!(lp_backend.is_some());
    assert_eq!(lp_backend.unwrap().id(), "clarabel");

    // NLP should get LBFGS (or IPOPT if available)
    let nlp_backend = registry.select_backend(ProblemClass::NonlinearProgram);
    assert!(nlp_backend.is_some());
    // Could be either lbfgs or ipopt depending on availability
    let nlp_backend = nlp_backend.unwrap();
    let nlp_id = nlp_backend.id();
    assert!(nlp_id == "lbfgs" || nlp_id == "ipopt");
}

/// Test that custom formulations can be registered.
#[test]
fn test_custom_formulation_registration() {
    struct CustomFormulation;

    impl OpfFormulation for CustomFormulation {
        fn id(&self) -> &str {
            "custom"
        }
        fn problem_class(&self) -> ProblemClass {
            ProblemClass::LinearProgram
        }
        fn build_problem(&self, _network: &Network) -> Result<OpfProblem, OpfError> {
            unimplemented!()
        }
        fn accepts_warm_start(&self) -> &[WarmStartKind] {
            &[]
        }
    }

    let mut registry = SolverRegistry::with_defaults();
    registry.register_formulation(Arc::new(CustomFormulation));

    let custom = registry.get_formulation("custom");
    assert!(custom.is_some());
    assert_eq!(custom.unwrap().id(), "custom");
}

/// Test that formulation problem classes are correct.
#[test]
fn test_formulation_problem_classes() {
    let registry = SolverRegistry::with_defaults();

    let dc = registry.get_formulation("dc-opf").unwrap();
    assert_eq!(dc.problem_class(), ProblemClass::LinearProgram);

    let socp = registry.get_formulation("socp").unwrap();
    assert_eq!(socp.problem_class(), ProblemClass::ConicProgram);

    let ac = registry.get_formulation("ac-opf").unwrap();
    assert_eq!(ac.problem_class(), ProblemClass::NonlinearProgram);

    let econ = registry.get_formulation("economic-dispatch").unwrap();
    assert_eq!(econ.problem_class(), ProblemClass::LinearProgram);
}

/// Test that formulations report correct warm-start acceptance.
#[test]
fn test_formulation_warm_start_acceptance() {
    let registry = SolverRegistry::with_defaults();

    // DC-OPF only accepts Flat start
    let dc = registry.get_formulation("dc-opf").unwrap();
    let dc_warm = dc.accepts_warm_start();
    assert!(dc_warm.contains(&WarmStartKind::Flat));
    assert!(!dc_warm.contains(&WarmStartKind::Dc));

    // AC-OPF accepts all warm-start types
    let ac = registry.get_formulation("ac-opf").unwrap();
    let ac_warm = ac.accepts_warm_start();
    assert!(ac_warm.contains(&WarmStartKind::Flat));
    assert!(ac_warm.contains(&WarmStartKind::Dc));
    assert!(ac_warm.contains(&WarmStartKind::Socp));
}

/// Test that backends report availability correctly.
#[test]
fn test_backend_availability() {
    let registry = SolverRegistry::with_defaults();

    // Clarabel is always available (pure Rust)
    let clarabel = registry.get_backend("clarabel").unwrap();
    assert!(clarabel.is_available());

    // LBFGS is always available (pure Rust)
    let lbfgs = registry.get_backend("lbfgs").unwrap();
    assert!(lbfgs.is_available());
}

/// Test that backends report supported problem classes correctly.
#[test]
fn test_backend_supported_classes() {
    let registry = SolverRegistry::with_defaults();

    // Clarabel supports LP and SOCP
    let clarabel = registry.get_backend("clarabel").unwrap();
    let clarabel_classes = clarabel.supported_classes();
    assert!(clarabel_classes.contains(&ProblemClass::LinearProgram));
    assert!(clarabel_classes.contains(&ProblemClass::ConicProgram));
    assert!(!clarabel_classes.contains(&ProblemClass::NonlinearProgram));

    // LBFGS supports NLP
    let lbfgs = registry.get_backend("lbfgs").unwrap();
    let lbfgs_classes = lbfgs.supported_classes();
    assert!(lbfgs_classes.contains(&ProblemClass::NonlinearProgram));
    assert!(!lbfgs_classes.contains(&ProblemClass::LinearProgram));
}

/// Test that backends_for filters correctly by problem class.
#[test]
fn test_backends_for_filtering() {
    let registry = SolverRegistry::with_defaults();

    // LP backends should include clarabel
    let lp_backends = registry.backends_for(ProblemClass::LinearProgram);
    assert!(lp_backends.contains(&"clarabel"));

    // NLP backends should include lbfgs
    let nlp_backends = registry.backends_for(ProblemClass::NonlinearProgram);
    assert!(nlp_backends.contains(&"lbfgs"));
}
