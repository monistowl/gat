//! Y-Bus construction tests

use gat_algo::opf::ac_nlp::YBusBuilder;
use gat_core::{Branch, BranchId, Bus, BusId, Edge, Network, Node};
use num_complex::Complex64;

/// Helper: create a simple 2-bus network
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

    // Line with R=0.01, X=0.1 (per unit)
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
            charging_b_pu: 0.02, // Small line charging
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
        }),
    );

    network
}

#[test]
fn ybus_two_bus_admittance() {
    let network = two_bus_network();
    let ybus = YBusBuilder::from_network(&network).expect("should build Y-bus");

    // For R=0.01, X=0.1: y = 1/(0.01 + j0.1) = (0.01 - j0.1) / (0.01² + 0.1²)
    //                      = (0.01 - j0.1) / 0.0101 ≈ 0.99 - j9.9
    let y_series = Complex64::new(0.01, 0.1).inv();

    // Off-diagonal: Y_12 = Y_21 = -y_series
    let y12 = ybus.get(0, 1);
    assert!(
        (y12.re - (-y_series.re)).abs() < 0.01,
        "Y_12 real part mismatch: got {}, expected {}",
        y12.re,
        -y_series.re
    );
    assert!(
        (y12.im - (-y_series.im)).abs() < 0.1,
        "Y_12 imag part mismatch: got {}, expected {}",
        y12.im,
        -y_series.im
    );

    // Diagonal: Y_11 = y_series + j*B_shunt/2
    let y11 = ybus.get(0, 0);
    let expected_y11 = y_series + Complex64::new(0.0, 0.02 / 2.0);
    assert!(
        (y11.re - expected_y11.re).abs() < 0.01,
        "Y_11 real mismatch"
    );
    assert!(
        (y11.im - expected_y11.im).abs() < 0.1,
        "Y_11 imag mismatch"
    );
}

#[test]
fn ybus_symmetry() {
    let network = two_bus_network();
    let ybus = YBusBuilder::from_network(&network).expect("should build Y-bus");

    // Y-bus should be symmetric for networks without phase shifters
    let y12 = ybus.get(0, 1);
    let y21 = ybus.get(1, 0);

    assert!(
        (y12.re - y21.re).abs() < 1e-10,
        "Y-bus should be symmetric"
    );
    assert!(
        (y12.im - y21.im).abs() < 1e-10,
        "Y-bus should be symmetric"
    );
}
