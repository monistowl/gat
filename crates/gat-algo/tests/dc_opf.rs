//! DC-OPF solver tests

use gat_algo::{OpfMethod, OpfSolver};
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
};

/// Create a simple 2-bus network for testing
/// Bus 1: Generator (cheap, 0-100 MW, $10/MWh)
/// Bus 2: Load (50 MW)
/// Branch 1-2: x = 0.1 pu
fn create_2bus_network() -> Network {
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

    network.graph.add_edge(
        bus1_idx,
        bus2_idx,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 50.0,
        reactive_power_mvar: 0.0,
    }));

    network
}

#[test]
fn test_dc_opf_2bus_basic() {
    let network = create_2bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    assert!(solution.converged);
    assert_eq!(solution.method_used, OpfMethod::DcOpf);

    // Generator should produce ~50 MW (matching load)
    let gen_p = solution.generator_p.get("gen1").expect("gen1 output");
    assert!((*gen_p - 50.0).abs() < 1.0, "gen1 should produce ~50 MW, got {}", gen_p);

    // Objective = 50 MW * $10/MWh = $500/hr
    assert!((solution.objective_value - 500.0).abs() < 10.0,
        "objective should be ~$500/hr, got {}", solution.objective_value);
}

#[test]
fn test_dc_opf_2bus_lmp() {
    let network = create_2bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    // Both buses should have LMPs (dual of power balance)
    assert!(solution.bus_lmp.contains_key("bus1"), "bus1 should have LMP");
    assert!(solution.bus_lmp.contains_key("bus2"), "bus2 should have LMP");

    // Without congestion, LMPs should be close to marginal cost ($10/MWh)
    let lmp1 = *solution.bus_lmp.get("bus1").unwrap();
    let lmp2 = *solution.bus_lmp.get("bus2").unwrap();
    assert!((lmp1 - 10.0).abs() < 1.0, "bus1 LMP should be ~$10/MWh, got {}", lmp1);
    assert!((lmp2 - 10.0).abs() < 1.0, "bus2 LMP should be ~$10/MWh, got {}", lmp2);
}

#[test]
fn test_dc_opf_2bus_angles() {
    let network = create_2bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    // Reference bus (bus1) should have angle = 0
    let theta1 = *solution.bus_voltage_ang.get("bus1").unwrap_or(&f64::NAN);
    assert!(theta1.abs() < 1e-6, "bus1 angle should be 0 (reference), got {}", theta1);

    // Bus2 angle should be negative (power flowing from 1 to 2)
    // θ2 = θ1 - P_12 * x = 0 - 50 * 0.1 = -5 radians (in per-unit base)
    let theta2 = *solution.bus_voltage_ang.get("bus2").unwrap_or(&f64::NAN);
    assert!(theta2 < 0.0, "bus2 angle should be negative, got {}", theta2);
}

/// Create a 3-bus network to test cost ordering
/// Bus 1: Cheap generator ($10/MWh, 0-100 MW)
/// Bus 2: Expensive generator ($30/MWh, 0-100 MW)
/// Bus 3: Load (80 MW)
/// Branches: 1-2 (x=0.1), 2-3 (x=0.1), 1-3 (x=0.1)
fn create_3bus_network() -> Network {
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));

    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 100.0,
    }));

    let bus3 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(2),
        name: "bus3".to_string(),
        voltage_kv: 100.0,
    }));

    // Triangle topology
    network.graph.add_edge(bus1, bus2, Edge::Branch(Branch {
        id: BranchId::new(0),
        name: "line1_2".to_string(),
        from_bus: BusId::new(0),
        to_bus: BusId::new(1),
        resistance: 0.01,
        reactance: 0.1,
    }));

    network.graph.add_edge(bus2, bus3, Edge::Branch(Branch {
        id: BranchId::new(1),
        name: "line2_3".to_string(),
        from_bus: BusId::new(1),
        to_bus: BusId::new(2),
        resistance: 0.01,
        reactance: 0.1,
    }));

    network.graph.add_edge(bus1, bus3, Edge::Branch(Branch {
        id: BranchId::new(2),
        name: "line1_3".to_string(),
        from_bus: BusId::new(0),
        to_bus: BusId::new(2),
        resistance: 0.01,
        reactance: 0.1,
    }));

    // Cheap generator at bus 1
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1_cheap".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    // Expensive generator at bus 2
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(1),
        name: "gen2_expensive".to_string(),
        bus: BusId::new(1),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 30.0),
    }));

    // Load at bus 3
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load3".to_string(),
        bus: BusId::new(2),
        active_power_mw: 80.0,
        reactive_power_mvar: 0.0,
    }));

    network
}

#[test]
fn test_dc_opf_3bus_merit_order() {
    let network = create_3bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    assert!(solution.converged);

    // Cheap generator should be dispatched first
    let gen1_p = *solution.generator_p.get("gen1_cheap").unwrap_or(&0.0);
    let gen2_p = *solution.generator_p.get("gen2_expensive").unwrap_or(&0.0);

    // Total generation should match load (~80 MW)
    let total_gen = gen1_p + gen2_p;
    assert!((total_gen - 80.0).abs() < 1.0, "total gen should be ~80 MW, got {}", total_gen);

    // Cheap generator should produce more than expensive one
    assert!(gen1_p > gen2_p, "cheap gen ({}) should produce more than expensive ({})", gen1_p, gen2_p);

    // If no congestion, cheap generator should produce all 80 MW
    assert!(gen1_p > 70.0, "cheap gen should produce most of the load, got {}", gen1_p);
}

#[test]
fn test_dc_opf_3bus_flows() {
    let network = create_3bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);

    let solution = solver.solve(&network).expect("DC-OPF should converge");

    // All branches should have computed flows
    assert!(solution.branch_p_flow.contains_key("line1_2"));
    assert!(solution.branch_p_flow.contains_key("line2_3"));
    assert!(solution.branch_p_flow.contains_key("line1_3"));

    // Power should flow from gen (bus 1) toward load (bus 3)
    let flow_1_3 = *solution.branch_p_flow.get("line1_3").unwrap_or(&0.0);
    // Flow should be positive (from bus 1 to bus 3) or at least non-trivial
    assert!(flow_1_3.abs() > 1.0, "flow on line1_3 should be significant, got {}", flow_1_3);
}
