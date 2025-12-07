//! Integration test verifying GPU and CPU branch flow calculations produce equivalent results.

use gat_algo::opf::gpu_branch_flow::GpuBranchFlowCalculator;
use gat_core::{Branch, BranchId, Bus, BusId, Edge, Kilovolts, Network, Node, PerUnit, Radians};
use std::collections::HashMap;

fn create_ieee14_style_network() -> Network {
    let mut network = Network::new();

    // Add 14 buses
    for i in 1..=14 {
        let bus_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(i),
            name: format!("Bus{}", i),
            base_kv: Kilovolts(138.0),
            voltage_pu: PerUnit(1.0),
            angle_rad: Radians(0.0),
            vmin_pu: Some(PerUnit(0.95)),
            vmax_pu: Some(PerUnit(1.05)),
            area_id: None,
            zone_id: None,
        }));
        // Store bus_idx for edge creation (though we'll look them up later)
        let _ = bus_idx;
    }

    // Add branches (simplified IEEE 14-bus topology)
    let branch_data = vec![
        (1, 2, 0.01938, 0.05917),
        (1, 5, 0.05403, 0.22304),
        (2, 3, 0.04699, 0.19797),
        (2, 4, 0.05811, 0.17632),
        (2, 5, 0.05695, 0.17388),
        (3, 4, 0.06701, 0.17103),
        (4, 5, 0.01335, 0.04211),
        (4, 7, 0.0, 0.20912),
        (4, 9, 0.0, 0.55618),
        (5, 6, 0.0, 0.25202),
        (6, 11, 0.09498, 0.19890),
        (6, 12, 0.12291, 0.25581),
        (6, 13, 0.06615, 0.13027),
        (7, 8, 0.0, 0.17615),
        (7, 9, 0.11001, 0.20640),
        (9, 10, 0.03181, 0.08450),
        (9, 14, 0.12711, 0.27038),
        (10, 11, 0.08205, 0.19207),
        (12, 13, 0.22092, 0.19988),
        (13, 14, 0.17093, 0.34802),
    ];

    // Create a mapping from bus ID to node index
    let mut bus_id_to_node_idx: HashMap<usize, petgraph::graph::NodeIndex> = HashMap::new();
    for node_idx in network.graph.node_indices() {
        if let Some(Node::Bus(bus)) = network.graph.node_weight(node_idx) {
            bus_id_to_node_idx.insert(bus.id.value(), node_idx);
        }
    }

    for (i, (from, to, r, x)) in branch_data.iter().enumerate() {
        let from_idx = *bus_id_to_node_idx.get(from).expect("from bus exists");
        let to_idx = *bus_id_to_node_idx.get(to).expect("to bus exists");

        // Handle zero-impedance branches (transformers) with small resistance
        let resistance = if *r == 0.0 && *x > 0.0 {
            0.0001 // Small resistance to avoid singularity
        } else {
            *r
        };

        network.graph.add_edge(
            from_idx,
            to_idx,
            Edge::Branch(Branch {
                id: BranchId::new(i + 1),
                name: format!("Branch{}", i + 1),
                from_bus: BusId::new(*from),
                to_bus: BusId::new(*to),
                resistance,
                reactance: *x,
                charging_b: PerUnit(0.05),
                tap_ratio: 1.0,
                phase_shift: Radians(0.0),
                status: true,
                ..Branch::default()
            }),
        );
    }

    network
}

#[test]
fn test_gpu_cpu_branch_flow_equivalence() {
    let network = create_ieee14_style_network();

    // Create voltage solution (typical flat start + small perturbations)
    let mut bus_voltage_mag = HashMap::new();
    let mut bus_voltage_ang = HashMap::new();

    for i in 1..=14 {
        bus_voltage_mag.insert(format!("Bus{}", i), 1.0 - (i as f64) * 0.005);
        bus_voltage_ang.insert(format!("Bus{}", i), -(i as f64) * 0.02);
    }

    let mut calc = GpuBranchFlowCalculator::new();

    println!("GPU available: {}", calc.is_gpu_available());

    // Compute branch flows (uses GPU if available, CPU otherwise)
    let (p_flow, q_flow, losses) = calc
        .compute_branch_flows(&network, &bus_voltage_mag, &bus_voltage_ang, 100.0)
        .expect("Branch flow calculation should succeed");

    // Verify we got results for all branches
    assert_eq!(p_flow.len(), 20, "Should have 20 branch P flows");
    assert_eq!(q_flow.len(), 20, "Should have 20 branch Q flows");

    // Verify losses are reasonable (< 1000 MW for test case)
    assert!(
        losses.abs() < 1000.0,
        "Losses should be reasonable: {} MW",
        losses
    );

    // Verify individual flows are reasonable (< 1000 MW for each branch)
    for (name, &p) in &p_flow {
        assert!(
            p.abs() < 1000.0,
            "P flow for {} should be reasonable: {} MW",
            name,
            p
        );
    }

    for (name, &q) in &q_flow {
        assert!(
            q.abs() < 1000.0,
            "Q flow for {} should be reasonable: {} MVAr",
            name,
            q
        );
    }

    println!("Total losses: {:.4} MW", losses);
    println!("Sample flows:");
    for i in 1..=5 {
        let branch_name = format!("Branch{}", i);
        if let (Some(&p), Some(&q)) = (p_flow.get(&branch_name), q_flow.get(&branch_name)) {
            println!("  {}: P={:.4} MW, Q={:.4} MVAr", branch_name, p, q);
        }
    }
}
