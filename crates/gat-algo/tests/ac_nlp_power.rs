//! AC power flow equation tests

use gat_algo::opf::ac_nlp::{PowerEquations, YBusBuilder};
use gat_core::{Branch, BranchId, Bus, BusId, Edge, Network, Node};

fn two_bus_network() -> Network {
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 138.0,
    }));
    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 138.0,
    }));

    network.graph.add_edge(
        bus1,
        bus2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.0,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
        }),
    );

    network
}

#[test]
fn power_injection_flat_start() {
    let network = two_bus_network();
    let ybus = YBusBuilder::from_network(&network).unwrap();

    // Flat start: V = [1.0, 1.0], θ = [0.0, 0.0]
    let v = vec![1.0, 1.0];
    let theta = vec![0.0, 0.0];

    let (p_inj, q_inj) = PowerEquations::compute_injections(&ybus, &v, &theta);

    // At flat start with no angle difference, power flow should be zero
    assert!(p_inj[0].abs() < 1e-10, "P1 should be ~0 at flat start");
    assert!(p_inj[1].abs() < 1e-10, "P2 should be ~0 at flat start");
    assert!(q_inj[0].abs() < 1e-10, "Q1 should be ~0 at flat start");
    assert!(q_inj[1].abs() < 1e-10, "Q2 should be ~0 at flat start");
}

#[test]
fn power_injection_with_angle() {
    let network = two_bus_network();
    let ybus = YBusBuilder::from_network(&network).unwrap();

    // V = [1.0, 1.0], θ = [0.0, -0.1 rad] (bus 2 lagging)
    // Power should flow from bus 1 to bus 2
    let v = vec![1.0, 1.0];
    let theta = vec![0.0, -0.1];

    let (p_inj, _q_inj) = PowerEquations::compute_injections(&ybus, &v, &theta);

    // P1 should be positive (injecting into network = sending)
    // P2 should be negative (withdrawing from network = receiving)
    assert!(p_inj[0] > 0.0, "P1 should be positive (sending)");
    assert!(p_inj[1] < 0.0, "P2 should be negative (receiving)");

    // Conservation: P1 + P2 ≈ losses (small for this case)
    let total_p = p_inj[0] + p_inj[1];
    assert!(total_p.abs() < 0.01, "Power should be nearly conserved");
}
