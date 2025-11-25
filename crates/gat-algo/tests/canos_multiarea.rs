use gat_algo::{
    AreaId, Corridor, MultiAreaMonteCarlo, MultiAreaOutageScenario, MultiAreaSystem, OutageScenario,
};
use gat_core::{Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node};

fn create_simple_network(name: &str, gen_capacity: f64, load_capacity: f64) -> Network {
    let mut network = Network::new();

    let bus1_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: format!("{}_bus1", name),
        voltage_kv: 100.0,
    }));

    let bus2_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: format!("{}_bus2", name),
        voltage_kv: 100.0,
    }));

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: format!("{}_gen1", name),
        bus: BusId::new(0),
        active_power_mw: gen_capacity,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 1000.0,
        qmin_mvar: -1000.0,
        qmax_mvar: 1000.0,
        cost_model: CostModel::NoCost,
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: format!("{}_load2", name),
        bus: BusId::new(1),
        active_power_mw: load_capacity,
        reactive_power_mvar: 0.0,
    }));

    network.graph.add_edge(
        bus1_idx,
        bus2_idx,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: format!("{}_br1_2", name),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.05,
        }),
    );

    network
}

#[test]
fn test_corridor_creation() {
    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    assert_eq!(corridor.id, 0);
    assert_eq!(corridor.area_a, AreaId(0));
    assert_eq!(corridor.area_b, AreaId(1));
    assert_eq!(corridor.capacity_mw, 100.0);
    assert_eq!(corridor.failure_rate, 0.01);
}

#[test]
fn test_corridor_failure_rate() {
    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0).with_failure_rate(0.05);
    assert_eq!(corridor.failure_rate, 0.05);
}

#[test]
fn test_corridor_online_check() {
    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    let mut offline_corridors = std::collections::HashSet::new();

    assert!(corridor.is_online(&offline_corridors));

    offline_corridors.insert(0);
    assert!(!corridor.is_online(&offline_corridors));
}

#[test]
fn test_multiarea_system_creation() {
    let system = MultiAreaSystem::new();
    assert_eq!(system.num_areas(), 0);
    assert_eq!(system.num_corridors(), 0);
}

#[test]
fn test_multiarea_system_add_area() {
    let mut system = MultiAreaSystem::new();
    let network = create_simple_network("area_a", 100.0, 80.0);

    assert!(system.add_area(AreaId(0), network).is_ok());
    assert_eq!(system.num_areas(), 1);

    // Adding same area twice should fail
    let network2 = create_simple_network("area_a", 100.0, 80.0);
    assert!(system.add_area(AreaId(0), network2).is_err());
}

#[test]
fn test_multiarea_system_add_corridor() {
    let mut system = MultiAreaSystem::new();
    let network1 = create_simple_network("area_a", 100.0, 80.0);
    let network2 = create_simple_network("area_b", 120.0, 90.0);

    system.add_area(AreaId(0), network1).unwrap();
    system.add_area(AreaId(1), network2).unwrap();

    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    assert!(system.add_corridor(corridor).is_ok());
    assert_eq!(system.num_corridors(), 1);
}

#[test]
fn test_multiarea_system_corridor_validation() {
    let mut system = MultiAreaSystem::new();
    let network = create_simple_network("area_a", 100.0, 80.0);
    system.add_area(AreaId(0), network).unwrap();

    // Try to add corridor to non-existent area
    let corridor = Corridor::new(0, AreaId(0), AreaId(99), 100.0);
    assert!(system.add_corridor(corridor).is_err());
}

#[test]
fn test_multiarea_system_validate() {
    let mut system = MultiAreaSystem::new();

    // Single area should fail validation
    let network = create_simple_network("area_a", 100.0, 80.0);
    system.add_area(AreaId(0), network).unwrap();
    assert!(system.validate().is_err());

    // Two areas should pass
    let network2 = create_simple_network("area_b", 120.0, 90.0);
    system.add_area(AreaId(1), network2).unwrap();
    assert!(system.validate().is_ok());
}

#[test]
fn test_multiarea_outage_scenario() {
    let scenario = MultiAreaOutageScenario::new(0.1);
    assert_eq!(scenario.probability, 0.1);
    assert!(scenario.area_scenarios.is_empty());
    assert!(scenario.offline_corridors.is_empty());
}

#[test]
fn test_multiarea_outage_scenario_set_area() {
    let mut scenario = MultiAreaOutageScenario::new(0.1);
    let area_scenario = OutageScenario::baseline();

    scenario.set_area(AreaId(0), area_scenario.clone());
    assert_eq!(scenario.area_scenarios.len(), 1);
    assert!(scenario.area_scenarios.contains_key(&AreaId(0)));
}

#[test]
fn test_multiarea_outage_scenario_corridor_offline() {
    let mut scenario = MultiAreaOutageScenario::new(0.1);

    scenario.mark_corridor_offline(0);
    scenario.mark_corridor_offline(1);

    assert_eq!(scenario.offline_corridors.len(), 2);
    assert!(scenario.offline_corridors.contains(&0));
    assert!(scenario.offline_corridors.contains(&1));
}

#[test]
fn test_multiarea_outage_standalone_feasibility() {
    let mut system = MultiAreaSystem::new();
    let network1 = create_simple_network("area_a", 100.0, 80.0);
    let network2 = create_simple_network("area_b", 120.0, 90.0);

    system.add_area(AreaId(0), network1).unwrap();
    system.add_area(AreaId(1), network2).unwrap();

    // Scenario with no outages should be feasible
    let mut scenario = MultiAreaOutageScenario::new(0.5);
    scenario.set_area(AreaId(0), OutageScenario::baseline());
    scenario.set_area(AreaId(1), OutageScenario::baseline());

    assert!(scenario.is_feasible_standalone(&system));
}

#[test]
fn test_multiarea_monte_carlo_init() {
    let mc = MultiAreaMonteCarlo::new(500);
    assert_eq!(mc.num_scenarios, 500);
    assert!(mc.hours_per_year > 8700.0);
}

#[test]
fn test_multiarea_monte_carlo_scenario_generation() {
    let mut system = MultiAreaSystem::new();
    let network1 = create_simple_network("area_a", 150.0, 80.0);
    let network2 = create_simple_network("area_b", 150.0, 90.0);

    system.add_area(AreaId(0), network1).unwrap();
    system.add_area(AreaId(1), network2).unwrap();

    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    system.add_corridor(corridor).unwrap();

    let mc = MultiAreaMonteCarlo::new(100);
    let scenarios = mc.generate_multiarea_scenarios(&system, 42).unwrap();

    assert_eq!(scenarios.len(), 100);

    // Each scenario should have all areas
    for scenario in &scenarios {
        assert_eq!(scenario.area_scenarios.len(), 2);
        assert!(scenario.area_scenarios.contains_key(&AreaId(0)));
        assert!(scenario.area_scenarios.contains_key(&AreaId(1)));
    }
}

#[test]
fn test_multiarea_monte_carlo_reliable_system() {
    let mut system = MultiAreaSystem::new();
    // Both areas have 2x capacity relative to load = very reliable
    let network1 = create_simple_network("area_a", 160.0, 80.0);
    let network2 = create_simple_network("area_b", 180.0, 90.0);

    system.add_area(AreaId(0), network1).unwrap();
    system.add_area(AreaId(1), network2).unwrap();

    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    system.add_corridor(corridor).unwrap();

    let mc = MultiAreaMonteCarlo::new(500);
    let metrics = mc.compute_multiarea_reliability(&system).unwrap();

    // Reliable system should have low LOLE
    let lole_a = metrics.area_lole[&AreaId(0)];
    let lole_b = metrics.area_lole[&AreaId(1)];

    assert!(lole_a >= 0.0);
    assert!(lole_b >= 0.0);
    assert!(metrics.scenarios_analyzed == 500);
}

#[test]
fn test_multiarea_monte_carlo_tight_system() {
    let mut system = MultiAreaSystem::new();
    // Both areas have tight capacity (100% = 1.0x) = less reliable
    let network1 = create_simple_network("area_a", 80.0, 80.0);
    let network2 = create_simple_network("area_b", 90.0, 90.0);

    system.add_area(AreaId(0), network1).unwrap();
    system.add_area(AreaId(1), network2).unwrap();

    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    system.add_corridor(corridor).unwrap();

    let mc = MultiAreaMonteCarlo::new(500);
    let metrics = mc.compute_multiarea_reliability(&system).unwrap();

    // Tight system should have higher LOLE
    let lole_a = metrics.area_lole[&AreaId(0)];
    let lole_b = metrics.area_lole[&AreaId(1)];

    assert!(lole_a > 0.0, "Area A should have LOLE > 0");
    assert!(lole_b > 0.0, "Area B should have LOLE > 0");
    assert!(metrics.scenarios_with_shortfall > 0);
}

#[test]
fn test_multiarea_monte_carlo_corridor_utilization() {
    let mut system = MultiAreaSystem::new();
    let network1 = create_simple_network("area_a", 100.0, 80.0);
    let network2 = create_simple_network("area_b", 120.0, 90.0);

    system.add_area(AreaId(0), network1).unwrap();
    system.add_area(AreaId(1), network2).unwrap();

    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    system.add_corridor(corridor).unwrap();

    let mc = MultiAreaMonteCarlo::new(500);
    let metrics = mc.compute_multiarea_reliability(&system).unwrap();

    // Should track corridor utilization
    assert!(metrics.corridor_utilization.contains_key(&0));
    let util = metrics.corridor_utilization[&0];
    assert!(util >= 0.0 && util <= 100.0);
}

#[test]
fn test_multiarea_compute_area_reliability() {
    let mut system = MultiAreaSystem::new();
    let network1 = create_simple_network("area_a", 150.0, 80.0);
    let network2 = create_simple_network("area_b", 150.0, 90.0);

    system.add_area(AreaId(0), network1).unwrap();
    system.add_area(AreaId(1), network2).unwrap();

    let corridor = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    system.add_corridor(corridor).unwrap();

    let mc = MultiAreaMonteCarlo::new(200);
    let area_metrics = mc.compute_area_reliability(&system, AreaId(0)).unwrap();

    // Should return ReliabilityMetrics for specific area
    assert!(area_metrics.lole >= 0.0);
    assert!(area_metrics.eue >= 0.0);
    assert_eq!(area_metrics.scenarios_analyzed, 200);
}

#[test]
fn test_multiarea_compute_area_reliability_nonexistent() {
    let mut system = MultiAreaSystem::new();
    let network1 = create_simple_network("area_a", 150.0, 80.0);
    let network2 = create_simple_network("area_b", 150.0, 90.0);

    system.add_area(AreaId(0), network1).unwrap();
    system.add_area(AreaId(1), network2).unwrap();

    let mc = MultiAreaMonteCarlo::new(100);

    // Query non-existent area should fail
    assert!(mc.compute_area_reliability(&system, AreaId(99)).is_err());
}

#[test]
fn test_multiarea_system_three_areas() {
    let mut system = MultiAreaSystem::new();
    let net1 = create_simple_network("area_a", 100.0, 80.0);
    let net2 = create_simple_network("area_b", 120.0, 90.0);
    let net3 = create_simple_network("area_c", 110.0, 85.0);

    system.add_area(AreaId(0), net1).unwrap();
    system.add_area(AreaId(1), net2).unwrap();
    system.add_area(AreaId(2), net3).unwrap();

    let corr1 = Corridor::new(0, AreaId(0), AreaId(1), 100.0);
    let corr2 = Corridor::new(1, AreaId(1), AreaId(2), 100.0);

    system.add_corridor(corr1).unwrap();
    system.add_corridor(corr2).unwrap();

    assert_eq!(system.num_areas(), 3);
    assert_eq!(system.num_corridors(), 2);

    let mc = MultiAreaMonteCarlo::new(300);
    let metrics = mc.compute_multiarea_reliability(&system).unwrap();

    // All three areas should have metrics
    assert!(metrics.area_lole.contains_key(&AreaId(0)));
    assert!(metrics.area_lole.contains_key(&AreaId(1)));
    assert!(metrics.area_lole.contains_key(&AreaId(2)));

    // Both corridors should be tracked
    assert!(metrics.corridor_utilization.contains_key(&0));
    assert!(metrics.corridor_utilization.contains_key(&1));
}
