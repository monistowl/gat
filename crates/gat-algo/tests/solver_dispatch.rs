//! Integration tests for solver dispatch system.
//!
//! Tests the solver selection logic, fallback behavior, and
//! feature-gated native solver support.

use gat_algo::opf::{DispatchConfig, ProblemClass, SolverBackend, SolverDispatcher};

/// Test that the default dispatcher always selects pure-Rust solvers.
#[test]
fn test_default_dispatcher_uses_pure_rust() {
    let dispatcher = SolverDispatcher::new();

    // LP should use Clarabel
    let lp_solver = dispatcher.select(ProblemClass::LinearProgram).unwrap();
    assert_eq!(lp_solver, SolverBackend::Clarabel);

    // SOCP should use Clarabel
    let socp_solver = dispatcher.select(ProblemClass::ConicProgram).unwrap();
    assert_eq!(socp_solver, SolverBackend::Clarabel);

    // NLP should use L-BFGS (native disabled by default)
    let nlp_solver = dispatcher.select(ProblemClass::NonlinearProgram).unwrap();
    assert_eq!(nlp_solver, SolverBackend::Lbfgs);
}

/// Test that MIP problems fail without native solvers.
#[test]
fn test_mip_requires_native_solver() {
    let dispatcher = SolverDispatcher::new();
    let result = dispatcher.select(ProblemClass::MixedInteger);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("MIP") || err_msg.contains("solver"),
        "Error should mention MIP solver: {}",
        err_msg
    );
}

/// Test that pure-Rust solvers are always in the available list.
#[test]
fn test_pure_rust_solvers_always_available() {
    let dispatcher = SolverDispatcher::new();
    let available = dispatcher.list_available();

    assert!(
        available.contains(&SolverBackend::Clarabel),
        "Clarabel should always be available"
    );
    assert!(
        available.contains(&SolverBackend::Lbfgs),
        "L-BFGS should always be available"
    );
}

/// Test solver backend properties.
#[test]
fn test_solver_backend_properties() {
    // Pure-Rust solvers should not be native
    assert!(!SolverBackend::Clarabel.is_native());
    assert!(!SolverBackend::Lbfgs.is_native());

    // Display names
    assert_eq!(SolverBackend::Clarabel.display_name(), "Clarabel");
    assert_eq!(SolverBackend::Lbfgs.display_name(), "L-BFGS");

    // Descriptions should be non-empty
    assert!(!SolverBackend::Clarabel.description().is_empty());
    assert!(!SolverBackend::Lbfgs.description().is_empty());
}

/// Test custom dispatch configuration.
#[test]
fn test_custom_dispatch_config() {
    let config = DispatchConfig {
        native_enabled: false,
        preferred_lp: Some(SolverBackend::Clarabel),
        preferred_nlp: Some(SolverBackend::Lbfgs),
        timeout_seconds: 600,
    };

    let dispatcher = SolverDispatcher::with_config(config);

    // Should use preferred solvers
    let lp = dispatcher.select(ProblemClass::LinearProgram).unwrap();
    assert_eq!(lp, SolverBackend::Clarabel);

    let nlp = dispatcher.select(ProblemClass::NonlinearProgram).unwrap();
    assert_eq!(nlp, SolverBackend::Lbfgs);
}

/// Test default dispatch config values.
#[test]
fn test_default_dispatch_config() {
    let config = DispatchConfig::default();

    assert!(!config.native_enabled);
    assert!(config.preferred_lp.is_none());
    assert!(config.preferred_nlp.is_none());
    assert_eq!(config.timeout_seconds, 300); // 5 minutes
}

/// Test that problem class selection is deterministic.
#[test]
fn test_solver_selection_determinism() {
    let dispatcher = SolverDispatcher::new();

    // Multiple calls should return same solver
    for _ in 0..10 {
        let solver = dispatcher.select(ProblemClass::ConicProgram).unwrap();
        assert_eq!(solver, SolverBackend::Clarabel);
    }
}

/// Test that list_available returns at least the pure-Rust solvers.
#[test]
fn test_list_available_minimum_solvers() {
    let dispatcher = SolverDispatcher::new();
    let available = dispatcher.list_available();

    // Should have at least 2 (Clarabel and L-BFGS)
    assert!(
        available.len() >= 2,
        "Should have at least 2 solvers, got {}",
        available.len()
    );
}

#[cfg(feature = "native-dispatch")]
mod native_dispatch_tests {
    use super::*;
    use gat_solver_common::SolverId;

    /// Test that enabling native dispatch allows IPOPT selection.
    #[test]
    fn test_native_dispatch_with_ipopt() {
        let config = DispatchConfig {
            native_enabled: true,
            preferred_lp: None,
            preferred_nlp: None,
            timeout_seconds: 300,
        };

        let mut dispatcher = SolverDispatcher::with_config(config);
        dispatcher.set_installed_solvers(vec![SolverId::Ipopt]);

        let nlp_solver = dispatcher.select(ProblemClass::NonlinearProgram).unwrap();
        assert_eq!(nlp_solver, SolverBackend::Ipopt);
    }

    /// Test that MIP works with CBC installed.
    #[test]
    fn test_native_dispatch_with_cbc() {
        let config = DispatchConfig {
            native_enabled: true,
            preferred_lp: None,
            preferred_nlp: None,
            timeout_seconds: 300,
        };

        let mut dispatcher = SolverDispatcher::with_config(config);
        dispatcher.set_installed_solvers(vec![SolverId::Cbc]);

        let mip_solver = dispatcher.select(ProblemClass::MixedInteger).unwrap();
        assert_eq!(mip_solver, SolverBackend::Cbc);
    }

    /// Test fallback when native enabled but solver not installed.
    #[test]
    fn test_native_dispatch_fallback() {
        let config = DispatchConfig {
            native_enabled: true,
            preferred_lp: None,
            preferred_nlp: None,
            timeout_seconds: 300,
        };

        // No installed solvers
        let dispatcher = SolverDispatcher::with_config(config);

        // Should fall back to L-BFGS for NLP
        let nlp_solver = dispatcher.select(ProblemClass::NonlinearProgram).unwrap();
        assert_eq!(nlp_solver, SolverBackend::Lbfgs);
    }

    /// Test that native solvers are marked as native.
    #[test]
    fn test_native_solver_is_native() {
        assert!(SolverBackend::Ipopt.is_native());
        assert!(SolverBackend::Highs.is_native());
        assert!(SolverBackend::Cbc.is_native());
    }
}
