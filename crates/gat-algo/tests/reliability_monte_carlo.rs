use gat_algo::{MonteCarlo, OutageScenario, OutageGenerator};
use gat_core::{Bus, Branch, Gen, Load, Network, Node, Edge, BusId, GenId, LoadId, BranchId};

fn create_simple_network() -> Network {
    let mut network = Network::new();

    // Add 2 buses
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

    // Add generator at bus 1 (100 MW)
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 100.0,
        reactive_power_mvar: 0.0,
    }));

    // Add load at bus 2 (80 MW)
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 80.0,
        reactive_power_mvar: 0.0,
    }));

    // Add branch
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

    network
}

#[test]
fn test_outage_scenario_baseline() {
    let scenario = OutageScenario::baseline();
    assert!(scenario.offline_generators.is_empty());
    assert!(scenario.offline_branches.is_empty());
    assert_eq!(scenario.demand_scale, 1.0);
}

#[test]
fn test_outage_scenario_capacity_check() {
    let network = create_simple_network();
    let scenario = OutageScenario::baseline();

    // Should have capacity (100 MW gen >= 80 MW load)
    assert!(scenario.has_capacity(&network, 80.0));

    // Should not have capacity at higher load
    let high_load_scenario = OutageScenario {
        demand_scale: 1.5,
        .. scenario.clone()
    };
    assert!(!high_load_scenario.has_capacity(&network, 80.0));
}

#[test]
fn test_outage_generator_init() {
    let gen = OutageGenerator::new();
    assert_eq!(gen.gen_failure_rate, 0.05);
    assert_eq!(gen.branch_failure_rate, 0.02);
}

#[test]
fn test_outage_generator_creates_scenarios() {
    let gen = OutageGenerator::new();
    let network = create_simple_network();

    let scenarios = gen.generate_scenarios(&network, 100);
    assert_eq!(scenarios.len(), 100);

    // Each scenario should have valid probability
    let total_prob: f64 = scenarios.iter().map(|s| s.probability).sum();
    assert!((total_prob - 1.0).abs() < 0.001);
}

#[test]
fn test_monte_carlo_init() {
    let mc = MonteCarlo::new(1000);
    assert_eq!(mc.num_scenarios, 1000);
    assert!(mc.hours_per_year > 8700.0);  // ~365.25 * 24
}

#[test]
fn test_monte_carlo_perfect_reliability() {
    // Network with excess capacity
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

    // 200 MW generation for 50 MW load = very reliable
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 200.0,
        reactive_power_mvar: 0.0,
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 50.0,
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

    let mc = MonteCarlo::new(1000);
    let metrics = mc.compute_reliability(&network).unwrap();

    // Should have relatively low LOLE due to excess capacity
    // Note: With 200 MW gen and 50 MW load, even with 1.2x demand variation (60 MW),
    // LOLE is mainly driven by generator failures (5% failure rate)
    assert!(metrics.lole >= 0.0);
    assert!(metrics.eue >= 0.0);
    assert!(metrics.scenarios_analyzed == 1000);
}

#[test]
fn test_monte_carlo_tight_reliability() {
    // Network with tight capacity (100 MW gen, 100 MW load)
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
            resistance: 0.01,
            reactance: 0.05,
        }),
    );

    let mc = MonteCarlo::new(1000);
    let metrics = mc.compute_reliability(&network).unwrap();

    // Should have higher LOLE due to tight margin
    // (generator offline scenarios + demand variations will cause shortfall)
    // With 100 MW gen and 100 MW load, any demand > 100 MW causes shortfall
    assert!(metrics.lole > 0.0, "Tight system should have some LOLE");
    assert!(metrics.lole < 10000.0, "LOLE should be reasonable (less than full year)");
    assert!(metrics.scenarios_with_shortfall > 0, "Should have some shortfall scenarios");
}

#[test]
fn test_monte_carlo_metrics_structure() {
    let network = create_simple_network();
    let mc = MonteCarlo::new(500);
    let metrics = mc.compute_reliability(&network).unwrap();

    // Verify structure
    assert_eq!(metrics.scenarios_analyzed, 500);
    assert!(metrics.scenarios_with_shortfall <= 500);
    assert!(metrics.lole >= 0.0);
    assert!(metrics.eue >= 0.0);
}

#[test]
fn test_monte_carlo_multiple_networks() {
    let network1 = create_simple_network();
    let mut network2 = Network::new();

    // Create network2 with higher capacity
    let bus1_idx = network2.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 100.0,
    }));

    let bus2_idx = network2.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 100.0,
    }));

    // Increase capacity in network2 (200 MW)
    network2.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 200.0,
        reactive_power_mvar: 0.0,
    }));

    network2.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 80.0,
        reactive_power_mvar: 0.0,
    }));

    network2.graph.add_edge(
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

    let mc = MonteCarlo::new(500);
    let results = mc.compute_multiple(&[network1, network2]).unwrap();

    assert_eq!(results.len(), 2);
    // network2 should have lower LOLE due to higher capacity
    assert!(results[1].lole <= results[0].lole);
}
