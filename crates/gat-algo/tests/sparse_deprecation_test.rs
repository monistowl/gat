//! Test that deprecated PTDF/LODF APIs still work but new APIs are preferred

use gat_algo::sparse::SparsePtdf;
use gat_core::{Branch, BranchId, Bus, BusId, Edge, Network, Node};

fn create_test_network() -> Network {
    let mut network = Network::new();

    let b1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "Bus1".to_string(),
        base_kv: gat_core::Kilovolts(138.0),
        ..Default::default()
    }));
    let b2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(2),
        name: "Bus2".to_string(),
        base_kv: gat_core::Kilovolts(138.0),
        ..Default::default()
    }));

    network.graph.add_edge(
        b1,
        b2,
        Edge::Branch(Branch {
            id: BranchId::new(1),
            name: "Line1-2".to_string(),
            from_bus: BusId::new(1),
            to_bus: BusId::new(2),
            reactance: 0.1,
            ..Default::default()
        }),
    );

    network
}

#[test]
fn test_new_sparse_ptdf_api() {
    let network = create_test_network();
    let ptdf = SparsePtdf::compute_ptdf(&network).unwrap();

    assert_eq!(ptdf.num_branches(), 1);
    assert_eq!(ptdf.num_buses(), 2);

    // Type-safe access
    let val = ptdf.get(BranchId::new(1), BusId::new(2));
    assert!(val.is_some());
}

#[test]
fn test_new_sparse_lodf_api() {
    let network = create_test_network();
    let ptdf = SparsePtdf::compute_ptdf(&network).unwrap();
    let lodf = SparsePtdf::compute_lodf(&network, &ptdf).unwrap();

    assert_eq!(lodf.num_branches(), 1);

    // Type-safe access
    let val = lodf.get(BranchId::new(1), BranchId::new(1));
    assert!(val.is_some());
    // Diagonal should be -1
    assert!((val.unwrap() + 1.0).abs() < 1e-10);
}
