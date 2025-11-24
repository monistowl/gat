use gat_algo::{AcOpfError, AcOpfSolver};
use gat_core::{Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node};

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
        voltage_kv: -100.0, // Invalid: negative voltage
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
        active_power_mw: -10.0, // Invalid: negative
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
            resistance: -0.01, // Invalid: negative
            reactance: 0.05,
        }),
    );

    let result = solver.solve(&network);
    assert!(result.is_err());

    if let Err(e) = result {
        match e {
            AcOpfError::DataValidation(msg) => {
                assert!(msg.contains("resistance and reactance must be non-negative"))
            }
            _ => panic!("Expected DataValidation error"),
        }
    }
}

#[test]
fn test_ac_opf_simple_network_accepts_valid_data() {
    let solver = AcOpfSolver::new();
    let network = create_simple_network();

    // Should solve successfully
    let result = solver.solve(&network);
    assert!(result.is_ok());

    let solution = result.unwrap();
    assert!(solution.converged);

    // Generator should produce ~100 MW (load) + small losses
    assert!(!solution.generator_outputs.is_empty());
    let gen_output = solution.generator_outputs.get("gen1").unwrap();
    assert!(
        *gen_output >= 99.0 && *gen_output <= 102.0,
        "Generator output {} not in expected range [99, 102]",
        gen_output
    );

    // Voltage should be nominal (1.0 pu)
    let bus_voltage = solution.bus_voltages.get("bus1").unwrap();
    assert!(
        (*bus_voltage - 1.0).abs() < 0.01,
        "Bus voltage {} not close to 1.0",
        bus_voltage
    );
}

#[test]
fn test_ac_opf_infeasible_case() {
    let solver = AcOpfSolver::new();
    let mut network = create_simple_network();

    // Add more load to exceed generator capacity (assumed max 200 MW per gen)
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(1),
        name: "load_large".to_string(),
        bus: BusId::new(1),
        active_power_mw: 200.0, // Total load now 300 MW, exceeds 200 MW capacity
        reactive_power_mvar: 0.0,
    }));

    let result = solver.solve(&network);
    assert!(result.is_err());

    if let Err(e) = result {
        match e {
            AcOpfError::Infeasible(msg) => {
                assert!(msg.contains("insufficient") || msg.contains("capacity"))
            }
            _ => panic!("Expected Infeasible error, got: {:?}", e),
        }
    }
}

#[test]
fn test_ac_opf_error_display() {
    let err = AcOpfError::Infeasible("demand exceeds supply".to_string());
    assert!(format!("{}", err).contains("infeasible"));

    let err = AcOpfError::Unbounded;
    assert!(format!("{}", err).contains("unbounded"));

    let err = AcOpfError::ConvergenceFailure {
        iterations: 100,
        residual: 0.001,
    };
    assert!(format!("{}", err).contains("100"));
}

#[test]
fn test_ac_opf_ieee_30bus_feasibility() {
    // IEEE 30-bus test case (simplified)
    // This validates that the solver can handle a larger network
    // The DC approximation should produce feasible solutions

    let mut network = Network::new();

    // Create 30 buses (simplified: just validate solver handles it)
    let mut bus_indices = Vec::new();
    for i in 1..=30 {
        let bus_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(i - 1),
            name: format!("bus{}", i),
            voltage_kv: if i == 1 { 138.0 } else { 69.0 },
        }));
        bus_indices.push(bus_idx);
    }

    // Add 6 generators (at buses 1, 2, 5, 8, 11, 13)
    for (idx, bus_num) in vec![1, 2, 5, 8, 11, 13].iter().enumerate() {
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(idx),
            name: format!("gen{}", idx + 1),
            bus: BusId::new(bus_num - 1),
            active_power_mw: 40.0, // ~200 MW total capacity
            reactive_power_mvar: 0.0,
        }));
    }

    // Add loads at various buses (total ~190 MW)
    let load_buses = vec![
        (2, 21.7),
        (3, 2.4),
        (4, 7.6),
        (5, 94.2),
        (6, 0.0),
        (7, 22.8),
        (8, 30.0),
        (9, 5.8),
        (10, 9.2),
        (12, 11.2),
        (14, 6.2),
        (15, 8.2),
        (16, 3.5),
        (17, 9.0),
        (18, 3.2),
        (19, 9.5),
        (20, 2.2),
        (21, 17.5),
        (23, 3.2),
        (24, 8.7),
        (26, 3.5),
        (29, 2.4),
        (30, 10.6),
    ];

    for (load_idx, (bus_num, load_mw)) in load_buses.iter().enumerate() {
        if *load_mw > 0.0 {
            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(load_idx),
                name: format!("load{}", bus_num),
                bus: BusId::new(bus_num - 1),
                active_power_mw: *load_mw,
                reactive_power_mvar: 0.0,
            }));
        }
    }

    // Add branches (simplified: just enough to connect network)
    network.graph.add_edge(
        bus_indices[0],
        bus_indices[1],
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "br1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.0192,
            reactance: 0.0575,
        }),
    );
    network.graph.add_edge(
        bus_indices[1],
        bus_indices[2],
        Edge::Branch(Branch {
            id: BranchId::new(1),
            name: "br2_3".to_string(),
            from_bus: BusId::new(1),
            to_bus: BusId::new(2),
            resistance: 0.0452,
            reactance: 0.1652,
        }),
    );
    // Add more branches to connect the network
    for i in 3..10 {
        network.graph.add_edge(
            bus_indices[i - 1],
            bus_indices[i],
            Edge::Branch(Branch {
                id: BranchId::new(i - 1),
                name: format!("br{}_{}", i, i + 1),
                from_bus: BusId::new(i - 1),
                to_bus: BusId::new(i),
                resistance: 0.01,
                reactance: 0.05,
            }),
        );
    }

    let solver = AcOpfSolver::new();
    let result = solver.solve(&network);

    // Should succeed for a feasible case
    assert!(
        result.is_ok(),
        "IEEE 30-bus should produce feasible solution"
    );

    let solution = result.unwrap();
    assert!(solution.converged, "Solver should converge");

    // Total generation should equal total load (within losses)
    let total_gen: f64 = solution.generator_outputs.values().sum();
    let total_load: f64 = load_buses.iter().map(|(_, load_mw)| load_mw).sum();

    // Allow 3% difference for losses and approximation
    let load_mismatch = (total_gen - total_load).abs() / total_load;
    assert!(
        load_mismatch < 0.03,
        "Generation should match load (±3%): gen={}, load={}",
        total_gen,
        total_load
    );

    // Voltages should be reasonable (0.9-1.1 pu)
    for voltage in solution.bus_voltages.values() {
        assert!(
            voltage >= &0.9 && voltage <= &1.1,
            "Voltage {} outside normal range",
            voltage
        );
    }

    // Cost should be positive
    assert!(
        solution.objective_value > 0.0,
        "Objective should be positive"
    );

    println!(
        "IEEE 30-bus test: {} MW generation, {} MW load, cost ${:.2}/hr",
        total_gen, total_load, solution.objective_value
    );
}

#[test]
fn test_ac_opf_10bus_accuracy() {
    // Simplified 10-bus test case
    let mut network = Network::new();

    // Create 10 buses
    let mut bus_indices = Vec::new();
    for i in 1..=10 {
        let bus_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(i - 1),
            name: format!("bus{}", i),
            voltage_kv: 100.0,
        }));
        bus_indices.push(bus_idx);
    }

    // Add 3 generators (buses 1, 3, 6)
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 150.0,
        reactive_power_mvar: 0.0,
    }));
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(1),
        name: "gen2".to_string(),
        bus: BusId::new(2),
        active_power_mw: 100.0,
        reactive_power_mvar: 0.0,
    }));
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(2),
        name: "gen3".to_string(),
        bus: BusId::new(5),
        active_power_mw: 80.0,
        reactive_power_mvar: 0.0,
    }));

    // Add loads (total ~150 MW)
    let loads = vec![(4, 50.0), (5, 40.0), (7, 30.0), (9, 30.0)];
    for (load_idx, (bus_num, load_mw)) in loads.iter().enumerate() {
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(load_idx),
            name: format!("load{}", bus_num),
            bus: BusId::new(bus_num - 1),
            active_power_mw: *load_mw,
            reactive_power_mvar: 0.0,
        }));
    }

    // Add simple branch structure
    for i in 0..9 {
        network.graph.add_edge(
            bus_indices[i],
            bus_indices[i + 1],
            Edge::Branch(Branch {
                id: BranchId::new(i),
                name: format!("br{}_{}", i + 1, i + 2),
                from_bus: BusId::new(i),
                to_bus: BusId::new(i + 1),
                resistance: 0.01,
                reactance: 0.05,
            }),
        );
    }

    let solver = AcOpfSolver::new();
    let result = solver.solve(&network);

    assert!(result.is_ok());
    let solution = result.unwrap();

    // Verify solution properties
    assert!(solution.converged);
    assert!(solution.objective_value > 0.0);

    let total_gen: f64 = solution.generator_outputs.values().sum();
    let total_load: f64 = loads.iter().map(|(_, load_mw)| load_mw).sum();

    println!(
        "10-bus test: {} MW generation, {} MW load",
        total_gen, total_load
    );
    assert!((total_gen - total_load).abs() / total_load < 0.05);
}

#[test]
fn test_ac_opf_capacity_boundary() {
    // Test behavior at capacity boundary
    // Note: Current solver uses 200 MW as hardcoded max per generator
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

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 195.0, // Near capacity (200 MW max - 1% losses)
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
            resistance: 0.01,
            reactance: 0.05,
        }),
    );

    let solver = AcOpfSolver::new();
    let result = solver.solve(&network);

    // Should succeed near capacity
    assert!(result.is_ok());
    let solution = result.unwrap();
    assert!(solution.converged);

    // Now test with load exceeding capacity - should fail
    let mut network2 = Network::new();

    let bus1_idx2 = network2.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));
    let bus2_idx2 = network2.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 100.0,
    }));

    network2.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 100.0,
        reactive_power_mvar: 0.0,
    }));

    network2.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 210.0, // Exceeds 200 MW capacity + losses
        reactive_power_mvar: 0.0,
    }));

    network2.graph.add_edge(
        bus1_idx2,
        bus2_idx2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "br1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.05,
        }),
    );

    let result2 = solver.solve(&network2);
    assert!(
        result2.is_err(),
        "Should be infeasible at 210 MW demand with 200 MW generator capacity"
    );
}
