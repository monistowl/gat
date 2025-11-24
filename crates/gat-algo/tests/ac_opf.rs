use gat_algo::{AcOpfSolver, AcOpfError};
use gat_core::{Bus, Branch, Gen, Load, Network, Node, Edge, BusId, GenId, LoadId, BranchId};

/// Helper to create a simple test network
fn create_simple_network() -> Network {
    let mut network = Network::new();

    // Add two buses
    let bus1_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));

    let bus2_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 100.0,
    }));

    // Add branch between buses
    network.graph.add_edge(
        bus1_idx,
        bus2_idx,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "br1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.05,
        }),
    );

    // Add generator at bus 1
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 100.0,
        reactive_power_mvar: 0.0,
    }));

    // Add load at bus 2
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 100.0,
        reactive_power_mvar: 0.0,
    }));

    network
}

#[test]
fn test_ac_opf_solver_init() {
    let solver = AcOpfSolver::new();
    // Check that solver was created (internal fields are private)
    let _ = solver;
}

#[test]
fn test_ac_opf_solver_builder() {
    let solver = AcOpfSolver::new()
        .with_penalty_weights(200.0, 100.0)
        .with_max_iterations(200)
        .with_tolerance(1e-8);

    // Builder pattern works
    let _ = solver;
}

#[test]
fn test_ac_opf_validate_empty_network() {
    let solver = AcOpfSolver::new();
    let network = Network::new();

    let result = solver.solve(&network);
    assert!(result.is_err());

    if let Err(e) = result {
        match e {
            AcOpfError::DataValidation(msg) => assert!(msg.contains("no buses")),
            _ => panic!("Expected DataValidation error"),
        }
    }
}

#[test]
fn test_ac_opf_validate_no_generators() {
    let solver = AcOpfSolver::new();
    let mut network = Network::new();

    network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));

    let result = solver.solve(&network);
    assert!(result.is_err());

    if let Err(e) = result {
        match e {
            AcOpfError::DataValidation(msg) => assert!(msg.contains("no generators")),
            _ => panic!("Expected DataValidation error"),
        }
    }
}

#[test]
fn test_ac_opf_validate_invalid_voltage() {
    let solver = AcOpfSolver::new();
    let mut network = Network::new();

    network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: -100.0,  // Invalid: negative voltage
    }));

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 100.0,
        reactive_power_mvar: 0.0,
    }));

    let result = solver.solve(&network);
    assert!(result.is_err());

    if let Err(e) = result {
        match e {
            AcOpfError::DataValidation(msg) => assert!(msg.contains("voltage_kv must be positive")),
            _ => panic!("Expected DataValidation error"),
        }
    }
}

#[test]
fn test_ac_opf_validate_negative_gen_power() {
    let solver = AcOpfSolver::new();
    let mut network = Network::new();

    network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: -10.0,  // Invalid: negative
        reactive_power_mvar: 0.0,
    }));

    let result = solver.solve(&network);
    assert!(result.is_err());

    if let Err(e) = result {
        match e {
            AcOpfError::DataValidation(msg) => assert!(msg.contains("negative active_power_mw")),
            _ => panic!("Expected DataValidation error"),
        }
    }
}

#[test]
fn test_ac_opf_validate_negative_resistance() {
    let solver = AcOpfSolver::new();
    let mut network = Network::new();

    let bus1_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));

    let bus2_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 100.0,
    }));

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 100.0,
        reactive_power_mvar: 0.0,
    }));

    network.graph.add_edge(
        bus1_idx,
        bus2_idx,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "br1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: -0.01,  // Invalid: negative
            reactance: 0.05,
        }),
    );

    let result = solver.solve(&network);
    assert!(result.is_err());

    if let Err(e) = result {
        match e {
            AcOpfError::DataValidation(msg) => assert!(msg.contains("resistance and reactance must be non-negative")),
            _ => panic!("Expected DataValidation error"),
        }
    }
}

#[test]
fn test_ac_opf_simple_network_accepts_valid_data() {
    let solver = AcOpfSolver::new();
    let network = create_simple_network();

    // Should succeed validation (placeholder solve returns false but no error)
    let result = solver.solve(&network);
    assert!(result.is_ok());

    let solution = result.unwrap();
    assert!(!solution.converged);  // Placeholder doesn't converge yet
}

#[test]
fn test_ac_opf_error_display() {
    let err = AcOpfError::Infeasible("demand exceeds supply".to_string());
    assert!(format!("{}", err).contains("infeasible"));

    let err = AcOpfError::Unbounded;
    assert!(format!("{}", err).contains("unbounded"));

    let err = AcOpfError::ConvergenceFailure { iterations: 100, residual: 0.001 };
    assert!(format!("{}", err).contains("100"));
}
