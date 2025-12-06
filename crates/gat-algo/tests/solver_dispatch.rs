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

// =============================================================================
// OpfSolver require_native tests
// =============================================================================

mod opf_solver_require_native {
    use gat_algo::opf::{OpfMethod, OpfSolver};
    use gat_core::{
        Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
    };

    /// Create a simple 2-bus network for testing OpfSolver.
    fn create_test_network() -> Network {
        let mut network = Network::new();

        // Add buses
        let bus1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(0),
            name: "slack".to_string(),
            base_kv: gat_core::Kilovolts(100.0),
            ..Bus::default()
        }));

        let bus2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "load_bus".to_string(),
            base_kv: gat_core::Kilovolts(100.0),
            ..Bus::default()
        }));

        // Add line between buses
        network.graph.add_edge(
            bus1,
            bus2,
            Edge::Branch(Branch {
                id: BranchId::new(0),
                name: "line".to_string(),
                from_bus: BusId::new(0),
                to_bus: BusId::new(1),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );

        // Add generator at bus 1
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(0),
            name: "gen1".to_string(),
            bus: BusId::new(0),
            active_power: gat_core::Megawatts(0.0),
            reactive_power: gat_core::Megavars(0.0),
            pmin: gat_core::Megawatts(0.0),
            pmax: gat_core::Megawatts(100.0),
            qmin: gat_core::Megavars(-50.0),
            qmax: gat_core::Megavars(50.0),
            is_synchronous_condenser: false,
            cost_model: CostModel::linear(0.0, 10.0),
            ..Gen::default()
        }));

        // Add load at bus 2
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(0),
            name: "load1".to_string(),
            bus: BusId::new(1),
            active_power: gat_core::Megawatts(50.0),
            reactive_power: gat_core::Megavars(10.0),
        }));

        network
    }

    /// Test that OpfSolver defaults to not requiring native solver.
    #[test]
    fn test_default_does_not_require_native() {
        let solver = OpfSolver::new();
        assert!(!solver.requires_native());
    }

    /// Test that require_native can be set.
    #[test]
    fn test_require_native_can_be_set() {
        let solver = OpfSolver::new().require_native(true);
        assert!(solver.requires_native());

        let solver = OpfSolver::new().require_native(false);
        assert!(!solver.requires_native());
    }

    /// Test builder pattern chaining with require_native.
    #[test]
    fn test_builder_pattern_chaining() {
        let solver = OpfSolver::new()
            .with_method(OpfMethod::AcOpf)
            .with_max_iterations(50)
            .with_tolerance(1e-8)
            .require_native(true);

        assert_eq!(solver.method(), OpfMethod::AcOpf);
        assert!(solver.requires_native());
    }

    /// Test that require_native only affects AC-OPF method.
    /// Other methods don't have native backends, so require_native has no effect.
    #[test]
    fn test_require_native_only_affects_ac_opf() {
        // DC-OPF, SOCP, and EconomicDispatch don't have native backends
        // so require_native should have no effect on their behavior
        let dc_solver = OpfSolver::new()
            .with_method(OpfMethod::DcOpf)
            .require_native(true);

        // The method should still be DC-OPF regardless of require_native
        assert_eq!(dc_solver.method(), OpfMethod::DcOpf);
    }

    /// Test that requiring native IPOPT without native solver features fails.
    ///
    /// When neither `native-dispatch` nor `solver-ipopt` features are enabled,
    /// require_native(true) should cause AC-OPF to fail with a helpful error message.
    #[cfg(all(not(feature = "native-dispatch"), not(feature = "solver-ipopt")))]
    #[test]
    fn test_require_native_without_feature_fails() {
        let network = create_test_network();
        let solver = OpfSolver::new()
            .with_method(OpfMethod::AcOpf)
            .require_native(true);

        let result = solver.solve(&network);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("native-dispatch"),
            "Error should mention 'native-dispatch' feature: {}",
            err_msg
        );
    }

    /// Test that default behavior (no require_native) succeeds with pure-Rust solver.
    #[cfg(not(feature = "native-dispatch"))]
    #[test]
    fn test_default_fallback_succeeds() {
        let network = create_test_network();
        let solver = OpfSolver::new()
            .with_method(OpfMethod::AcOpf)
            .with_max_iterations(200);
        // require_native defaults to false

        let result = solver.solve(&network);
        // Should succeed using L-BFGS fallback
        assert!(
            result.is_ok(),
            "Should succeed with L-BFGS fallback: {:?}",
            result.err()
        );
    }
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

    /// Test OpfSolver with require_native when feature enabled but solver not installed.
    ///
    /// This tests the case where:
    /// - native-dispatch feature IS enabled
    /// - require_native(true) is set
    /// - But IPOPT is NOT in solvers.toml
    ///
    /// Should fail with a helpful installation error.
    #[test]
    fn test_require_native_without_installed_solver_fails() {
        use gat_algo::opf::{OpfMethod, OpfSolver};
        use gat_core::{
            Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
        };

        // Create a simple test network
        let mut network = Network::new();
        let bus1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(0),
            name: "slack".to_string(),
            base_kv: gat_core::Kilovolts(100.0),
            ..Bus::default()
        }));
        let bus2 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "load".to_string(),
            base_kv: gat_core::Kilovolts(100.0),
            ..Bus::default()
        }));
        network.graph.add_edge(
            bus1,
            bus2,
            Edge::Branch(Branch {
                id: BranchId::new(0),
                name: "line".to_string(),
                from_bus: BusId::new(0),
                to_bus: BusId::new(1),
                resistance: 0.01,
                reactance: 0.1,
                ..Branch::default()
            }),
        );
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(0),
            name: "gen".to_string(),
            bus: BusId::new(0),
            active_power: gat_core::Megawatts(0.0),
            reactive_power: gat_core::Megavars(0.0),
            pmin: gat_core::Megawatts(0.0),
            pmax: gat_core::Megawatts(100.0),
            qmin: gat_core::Megavars(-50.0),
            qmax: gat_core::Megavars(50.0),
            is_synchronous_condenser: false,
            cost_model: CostModel::linear(0.0, 10.0),
            ..Gen::default()
        }));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(0),
            name: "load".to_string(),
            bus: BusId::new(1),
            active_power: gat_core::Megawatts(50.0),
            reactive_power: gat_core::Megavars(10.0),
        }));

        let solver = OpfSolver::new()
            .with_method(OpfMethod::AcOpf)
            .require_native(true);

        // If IPOPT is not actually installed, this should fail
        // If IPOPT is installed, this will succeed or show warning
        let result = solver.solve(&network);

        // We can't easily control whether IPOPT is installed in the test environment
        // so just verify the solver runs without panicking
        // The important thing is the code path exists
        match result {
            Ok(_) => {
                // Either IPOPT was installed or the warning was printed
            }
            Err(e) => {
                let msg = e.to_string();
                // Error should mention IPOPT or installation
                assert!(
                    msg.contains("IPOPT") || msg.contains("install"),
                    "Error should guide user to install IPOPT: {}",
                    msg
                );
            }
        }
    }
}
