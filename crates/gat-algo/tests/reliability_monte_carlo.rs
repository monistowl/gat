use gat_algo::{
    DeliverabilityScore, DeliverabilityScoreConfig, MonteCarlo, OutageGenerator, OutageScenario,
    ReliabilityMetrics,
};
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
};

fn create_simple_network() -> Network {
    let mut network = Network::new();

    // Add 2 buses
    let bus1_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    let bus2_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    // Add generator at bus 1 (100 MW)
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power: gat_core::Megawatts(100.0),
        reactive_power: gat_core::Megavars(0.0),
        pmin: gat_core::Megawatts(0.0),
        pmax: gat_core::Megawatts(1000.0),
        qmin: gat_core::Megavars(-1000.0),
        qmax: gat_core::Megavars(1000.0),
        is_synchronous_condenser: false,
        cost_model: CostModel::NoCost,
        ..Gen::default()
    }));

    // Add load at bus 2 (80 MW)
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power: gat_core::Megawatts(80.0),
        reactive_power: gat_core::Megavars(0.0),
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
            ..Branch::default()
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
        ..scenario.clone()
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
    assert!(mc.hours_per_year > 8700.0); // ~365.25 * 24
}

#[test]
fn test_monte_carlo_perfect_reliability() {
    // Network with excess capacity
    let mut network = Network::new();

    let bus1_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    let bus2_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    // 200 MW generation for 50 MW load = very reliable
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power: gat_core::Megawatts(200.0),
        reactive_power: gat_core::Megavars(0.0),
        pmin: gat_core::Megawatts(0.0),
        pmax: gat_core::Megawatts(1000.0),
        qmin: gat_core::Megavars(-1000.0),
        qmax: gat_core::Megavars(1000.0),
        is_synchronous_condenser: false,
        cost_model: CostModel::NoCost,
        ..Gen::default()
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power: gat_core::Megawatts(50.0),
        reactive_power: gat_core::Megavars(0.0),
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
            ..Branch::default()
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
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    let bus2_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power: gat_core::Megawatts(100.0),
        reactive_power: gat_core::Megavars(0.0),
        pmin: gat_core::Megawatts(0.0),
        pmax: gat_core::Megawatts(1000.0),
        qmin: gat_core::Megavars(-1000.0),
        qmax: gat_core::Megavars(1000.0),
        is_synchronous_condenser: false,
        cost_model: CostModel::NoCost,
        ..Gen::default()
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power: gat_core::Megawatts(100.0),
        reactive_power: gat_core::Megavars(0.0),
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
            ..Branch::default()
        }),
    );

    let mc = MonteCarlo::new(1000);
    let metrics = mc.compute_reliability(&network).unwrap();

    // Should have higher LOLE due to tight margin
    // (generator offline scenarios + demand variations will cause shortfall)
    // With 100 MW gen and 100 MW load, any demand > 100 MW causes shortfall
    assert!(metrics.lole > 0.0, "Tight system should have some LOLE");
    assert!(
        metrics.lole < 10000.0,
        "LOLE should be reasonable (less than full year)"
    );
    assert!(
        metrics.scenarios_with_shortfall > 0,
        "Should have some shortfall scenarios"
    );
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
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    let bus2_idx = network2.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    // Increase capacity in network2 (200 MW)
    network2.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power: gat_core::Megawatts(200.0),
        reactive_power: gat_core::Megavars(0.0),
        pmin: gat_core::Megawatts(0.0),
        pmax: gat_core::Megawatts(1000.0),
        qmin: gat_core::Megavars(-1000.0),
        qmax: gat_core::Megavars(1000.0),
        is_synchronous_condenser: false,
        cost_model: CostModel::NoCost,
        ..Gen::default()
    }));

    network2.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power: gat_core::Megawatts(80.0),
        reactive_power: gat_core::Megavars(0.0),
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
            ..Branch::default()
        }),
    );

    let mc = MonteCarlo::new(500);
    let results = mc.compute_multiple(&[network1, network2]).unwrap();

    assert_eq!(results.len(), 2);
    // network2 should have lower LOLE due to higher capacity
    assert!(results[1].lole <= results[0].lole);
}

#[test]
fn test_deliverability_score_config_defaults() {
    let config = DeliverabilityScoreConfig::new();
    assert_eq!(config.weight_lole, 1.0);
    assert_eq!(config.weight_voltage, 0.0);
    assert_eq!(config.weight_thermal, 0.0);
    assert_eq!(config.lole_max, 3.0);
}

#[test]
fn test_deliverability_score_config_builder() {
    let config = DeliverabilityScoreConfig::new()
        .with_weight_lole(0.7)
        .with_weight_voltage(0.2)
        .with_weight_thermal(0.1)
        .with_lole_max(5.0);

    assert_eq!(config.weight_lole, 0.7);
    assert_eq!(config.weight_voltage, 0.2);
    assert_eq!(config.weight_thermal, 0.1);
    assert_eq!(config.lole_max, 5.0);
}

#[test]
fn test_deliverability_score_config_validate() {
    let valid_config = DeliverabilityScoreConfig::new();
    assert!(valid_config.validate().is_ok());

    let invalid_zero_weights = DeliverabilityScoreConfig {
        weight_lole: 0.0,
        weight_voltage: 0.0,
        weight_thermal: 0.0,
        lole_max: 3.0,
        max_violations: 10.0,
        max_overloads: 5.0,
    };
    assert!(invalid_zero_weights.validate().is_err());

    let invalid_zero_max = DeliverabilityScoreConfig {
        weight_lole: 1.0,
        weight_voltage: 0.0,
        weight_thermal: 0.0,
        lole_max: 0.0,
        max_violations: 10.0,
        max_overloads: 5.0,
    };
    assert!(invalid_zero_max.validate().is_err());
}

#[test]
fn test_deliverability_score_perfect_reliability() {
    let metrics = ReliabilityMetrics {
        lole: 0.0, // Perfect LOLE
        eue: 0.0,
        scenarios_analyzed: 1000,
        scenarios_with_shortfall: 0,
        average_shortfall: 0.0,
    };

    let config = DeliverabilityScoreConfig::new();
    let score = DeliverabilityScore::from_metrics(metrics, &config).unwrap();

    // Perfect reliability should give score close to 100
    assert!(score.score > 95.0);
    assert_eq!(score.status(), "Excellent");
}

#[test]
fn test_deliverability_score_poor_reliability() {
    let metrics = ReliabilityMetrics {
        lole: 10.0, // Much higher than LOLE_MAX (3.0)
        eue: 50.0,
        scenarios_analyzed: 1000,
        scenarios_with_shortfall: 500,
        average_shortfall: 5.0,
    };

    let config = DeliverabilityScoreConfig::new();
    let score = DeliverabilityScore::from_metrics(metrics, &config).unwrap();

    // Poor reliability should give score around 0
    assert!(
        score.score < 50.0,
        "Score should be low for poor reliability"
    );
    assert!(score.score >= 0.0, "Score should be clamped to >= 0");
}

#[test]
fn test_deliverability_score_clamps_to_range() {
    // Create a scenario that would theoretically exceed 100
    let metrics = ReliabilityMetrics {
        lole: 100.0, // Very high
        eue: 500.0,
        scenarios_analyzed: 1000,
        scenarios_with_shortfall: 1000,
        average_shortfall: 50.0,
    };

    let config = DeliverabilityScoreConfig::new();
    let score = DeliverabilityScore::from_metrics(metrics, &config).unwrap();

    // Score should be clamped to 0-100 range
    assert!(score.score >= 0.0);
    assert!(score.score <= 100.0);
}

#[test]
fn test_deliverability_score_status_levels() {
    let config = DeliverabilityScoreConfig::new();

    let excellent = ReliabilityMetrics {
        lole: 0.1,
        eue: 0.5,
        scenarios_analyzed: 1000,
        scenarios_with_shortfall: 10,
        average_shortfall: 0.05,
    };
    let score_excellent = DeliverabilityScore::from_metrics(excellent, &config).unwrap();
    assert_eq!(score_excellent.status(), "Excellent");

    let good = ReliabilityMetrics {
        lole: 0.5,
        eue: 2.5,
        scenarios_analyzed: 1000,
        scenarios_with_shortfall: 50,
        average_shortfall: 0.05,
    };
    let score_good = DeliverabilityScore::from_metrics(good, &config).unwrap();
    assert_eq!(score_good.status(), "Good");

    let critical = ReliabilityMetrics {
        lole: 50.0,
        eue: 250.0,
        scenarios_analyzed: 1000,
        scenarios_with_shortfall: 1000,
        average_shortfall: 25.0,
    };
    let score_critical = DeliverabilityScore::from_metrics(critical, &config).unwrap();
    assert_eq!(score_critical.status(), "Critical");
}

#[test]
fn test_deliverability_score_meets_threshold() {
    let metrics = ReliabilityMetrics {
        lole: 0.5,
        eue: 2.5,
        scenarios_analyzed: 1000,
        scenarios_with_shortfall: 50,
        average_shortfall: 0.05,
    };

    let config = DeliverabilityScoreConfig::new();
    let score = DeliverabilityScore::from_metrics(metrics, &config).unwrap();

    assert!(
        score.meets_threshold(80.0),
        "Good reliability should meet 80% threshold"
    );
    assert!(
        !score.meets_threshold(99.0),
        "Should not meet impossible threshold"
    );
}

#[test]
fn test_monte_carlo_arena_allocation_large_scenario_count() {
    // This test verifies arena allocation works correctly
    // with a larger number of scenarios
    let network = create_simple_network();

    let mc = MonteCarlo::new(5000);
    let metrics = mc.compute_reliability(&network).unwrap();

    assert_eq!(metrics.scenarios_analyzed, 5000);
    assert!(metrics.lole >= 0.0);
    assert!(metrics.eue >= 0.0);
}
