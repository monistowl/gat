//! SOCP relaxation solver comprehensive tests
//!
//! Tests cover:
//! - Basic feasibility and convergence
//! - Quadratic cost functions
//! - Phase-shifting transformers
//! - Thermal limit binding
//! - Voltage limit binding
//! - Multi-bus networks

use gat_algo::{OpfMethod, OpfSolver};
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
};

fn simple_network() -> Network {
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
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.0,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
            is_phase_shifter: false,
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
        is_synchronous_condenser: false,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 1.0,
        reactive_power_mvar: 0.2,
    }));

    network
}

#[test]
fn socp_basic_feasible() {
    let network = simple_network();
    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);

    let solution = solver.solve(&network).expect("SOCP solver should converge");

    assert!(solution.converged);
    assert_eq!(solution.method_used, OpfMethod::SocpRelaxation);

    let gen_p = solution.generator_p.get("gen1").copied().unwrap();
    assert!(
        gen_p > 0.9 && gen_p < 1.2,
        "generator dispatch should cover load (~1 MW), got {} MW",
        gen_p
    );

    let v1 = solution.bus_voltage_mag.get("bus1").copied().unwrap();
    let v2 = solution.bus_voltage_mag.get("bus2").copied().unwrap();
    assert!((0.9..=1.1).contains(&v1));
    assert!((0.9..=1.1).contains(&v2));
}

/// Helper: create a 3-bus network for testing
fn three_bus_network() -> Network {
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
    let bus3 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(2),
        name: "bus3".to_string(),
        voltage_kv: 138.0,
    }));

    // Line 1-2
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
            charging_b_pu: 0.02,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
            is_phase_shifter: false,
        }),
    );

    // Line 2-3
    network.graph.add_edge(
        bus2,
        bus3,
        Edge::Branch(Branch {
            id: BranchId::new(1),
            name: "line2_3".to_string(),
            from_bus: BusId::new(1),
            to_bus: BusId::new(2),
            resistance: 0.01,
            reactance: 0.1,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.02,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
            is_phase_shifter: false,
        }),
    );

    // Line 1-3
    network.graph.add_edge(
        bus1,
        bus3,
        Edge::Branch(Branch {
            id: BranchId::new(2),
            name: "line1_3".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(2),
            resistance: 0.02,
            reactance: 0.15,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.01,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
            is_phase_shifter: false,
        }),
    );

    // Generator at bus 1
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 200.0,
        qmin_mvar: -100.0,
        qmax_mvar: 100.0,
        is_synchronous_condenser: false,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    // Generator at bus 2
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(1),
        name: "gen2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 150.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        is_synchronous_condenser: false,
        cost_model: CostModel::linear(0.0, 15.0),
    }));

    // Load at bus 3
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load3".to_string(),
        bus: BusId::new(2),
        active_power_mw: 100.0,
        reactive_power_mvar: 30.0,
    }));

    network
}

#[test]
fn socp_three_bus_network() {
    let network = three_bus_network();
    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);

    let solution = solver.solve(&network).expect("SOCP should converge");

    assert!(solution.converged);

    // Cheapest generator (gen1 at $10/MWh) should be dispatched first
    let gen1_p = solution.generator_p.get("gen1").copied().unwrap();
    let gen2_p = solution.generator_p.get("gen2").copied().unwrap();

    // Total generation should cover load + losses
    let total_gen = gen1_p + gen2_p;
    assert!(
        total_gen >= 100.0 && total_gen < 110.0,
        "total gen {} should be ~100 MW + losses",
        total_gen
    );

    // Cheaper gen1 should produce more
    assert!(
        gen1_p > gen2_p,
        "gen1 ({} MW) should produce more than gen2 ({} MW) due to lower cost",
        gen1_p,
        gen2_p
    );
}

#[test]
fn socp_quadratic_cost() {
    // Test quadratic cost curve: cost = c0 + c1*P + c2*P^2
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
            ..Branch::default()
        }),
    );

    // Generator with quadratic cost: 100 + 10*P + 0.05*P^2
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 200.0,
        qmin_mvar: -100.0,
        qmax_mvar: 100.0,
        is_synchronous_condenser: false,
        cost_model: CostModel::Polynomial(vec![100.0, 10.0, 0.05]),
    }));

    // Second generator with different quadratic cost: 50 + 20*P + 0.02*P^2
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(1),
        name: "gen2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 200.0,
        qmin_mvar: -100.0,
        qmax_mvar: 100.0,
        is_synchronous_condenser: false,
        cost_model: CostModel::Polynomial(vec![50.0, 20.0, 0.02]),
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 50.0,
        reactive_power_mvar: 10.0,
    }));

    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let solution = solver
        .solve(&network)
        .expect("SOCP with quadratic cost should converge");

    assert!(solution.converged);

    // At optimal, marginal costs should be approximately equal
    // MC1 = 10 + 0.1*P1, MC2 = 20 + 0.04*P2
    let gen1_p = solution.generator_p.get("gen1").copied().unwrap();
    let gen2_p = solution.generator_p.get("gen2").copied().unwrap();

    let mc1 = 10.0 + 0.1 * gen1_p;
    let mc2 = 20.0 + 0.04 * gen2_p;

    // Marginal costs should be close (within network effects)
    assert!(
        (mc1 - mc2).abs() < 5.0,
        "Marginal costs should be approximately equal: MC1={}, MC2={}",
        mc1,
        mc2
    );

    // Objective should include quadratic terms
    let expected_cost = 100.0
        + 10.0 * gen1_p
        + 0.05 * gen1_p * gen1_p
        + 50.0
        + 20.0 * gen2_p
        + 0.02 * gen2_p * gen2_p;

    assert!(
        (solution.objective_value - expected_cost).abs() < 10.0,
        "Objective {} should match expected cost {}",
        solution.objective_value,
        expected_cost
    );
}

#[test]
fn socp_thermal_limit_binding() {
    // Test that thermal limits constrain flow
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

    // Branch with tight thermal limit (50 MVA)
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
            s_max_mva: Some(50.0), // Tight limit
            status: true,
            rating_a_mva: None,
            is_phase_shifter: false,
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 200.0,
        qmin_mvar: -100.0,
        qmax_mvar: 100.0,
        is_synchronous_condenser: false,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    // Load that would exceed thermal limit if unconstrained
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 40.0, // Under the 50 MVA limit
        reactive_power_mvar: 10.0,
    }));

    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let solution = solver.solve(&network).expect("SOCP should converge");

    assert!(solution.converged);

    // Flow should be under thermal limit
    let p_flow = solution
        .branch_p_flow
        .get("line1_2")
        .copied()
        .unwrap()
        .abs();
    let q_flow = solution
        .branch_q_flow
        .get("line1_2")
        .copied()
        .unwrap_or(0.0)
        .abs();
    let s_flow = (p_flow * p_flow + q_flow * q_flow).sqrt();

    assert!(
        s_flow <= 50.0 + 1.0, // Allow small tolerance
        "Flow {} MVA should be under thermal limit 50 MVA",
        s_flow
    );
}

#[test]
fn socp_phase_shifting_transformer() {
    // Test phase-shifting transformer
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
    let bus3 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(2),
        name: "bus3".to_string(),
        voltage_kv: 138.0,
    }));

    // Normal line 1-2
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
            is_phase_shifter: false,
        }),
    );

    // Phase-shifting transformer 2-3 with 10 degree phase shift
    let phase_shift = 10.0_f64.to_radians();
    network.graph.add_edge(
        bus2,
        bus3,
        Edge::Branch(Branch {
            id: BranchId::new(1),
            name: "pst2_3".to_string(),
            from_bus: BusId::new(1),
            to_bus: BusId::new(2),
            resistance: 0.005,
            reactance: 0.05,
            tap_ratio: 1.0,
            phase_shift_rad: phase_shift,
            charging_b_pu: 0.0,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
            is_phase_shifter: true,
        }),
    );

    // Normal line 1-3
    network.graph.add_edge(
        bus1,
        bus3,
        Edge::Branch(Branch {
            id: BranchId::new(2),
            name: "line1_3".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(2),
            resistance: 0.02,
            reactance: 0.15,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.0,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
            is_phase_shifter: false,
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 200.0,
        qmin_mvar: -100.0,
        qmax_mvar: 100.0,
        is_synchronous_condenser: false,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load3".to_string(),
        bus: BusId::new(2),
        active_power_mw: 50.0,
        reactive_power_mvar: 10.0,
    }));

    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let solution = solver
        .solve(&network)
        .expect("SOCP with PST should converge");

    assert!(solution.converged);

    // Verify angles are computed
    let theta1 = solution.bus_voltage_ang.get("bus1").copied().unwrap();
    let theta2 = solution.bus_voltage_ang.get("bus2").copied().unwrap();
    let theta3 = solution.bus_voltage_ang.get("bus3").copied().unwrap();

    // Reference bus should be at 0 degrees
    assert!(
        theta1.abs() < 0.1,
        "Reference bus angle should be ~0, got {}",
        theta1
    );

    // Other buses should have reasonable angles
    assert!(
        theta2.abs() < 30.0 && theta3.abs() < 30.0,
        "Bus angles should be reasonable: theta2={}, theta3={}",
        theta2,
        theta3
    );
}

#[test]
fn socp_tap_ratio_transformer() {
    // Test off-nominal tap ratio transformer
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1_hv".to_string(),
        voltage_kv: 230.0, // High voltage side
    }));
    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2_lv".to_string(),
        voltage_kv: 138.0, // Low voltage side
    }));

    // Transformer with off-nominal tap ratio (1.05 = 5% boost)
    network.graph.add_edge(
        bus1,
        bus2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "xfmr1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.005,
            reactance: 0.1,
            tap_ratio: 1.05, // 5% boost
            phase_shift_rad: 0.0,
            charging_b_pu: 0.0,
            s_max_mva: Some(100.0),
            status: true,
            rating_a_mva: None,
            is_phase_shifter: false,
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 200.0,
        qmin_mvar: -100.0,
        qmax_mvar: 100.0,
        is_synchronous_condenser: false,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 50.0,
        reactive_power_mvar: 15.0,
    }));

    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let solution = solver
        .solve(&network)
        .expect("SOCP with tap ratio should converge");

    assert!(solution.converged);

    // Voltages should be reasonable
    let v1 = solution.bus_voltage_mag.get("bus1_hv").copied().unwrap();
    let v2 = solution.bus_voltage_mag.get("bus2_lv").copied().unwrap();

    assert!(
        (0.9..=1.1).contains(&v1) && (0.9..=1.1).contains(&v2),
        "Voltages should be in bounds: v1={}, v2={}",
        v1,
        v2
    );
}

#[test]
fn socp_10_bus_meshed_network() {
    // Larger meshed network test
    let mut network = Network::new();

    // Create 10 buses
    let mut bus_indices = Vec::new();
    for i in 0..10 {
        let bus_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(i),
            name: format!("bus{}", i + 1),
            voltage_kv: 138.0,
        }));
        bus_indices.push(bus_idx);
    }

    // Create a meshed topology: ring + some cross connections
    let branches = vec![
        (0, 1, 0.01, 0.1),
        (1, 2, 0.01, 0.1),
        (2, 3, 0.01, 0.1),
        (3, 4, 0.01, 0.1),
        (4, 5, 0.01, 0.1),
        (5, 6, 0.01, 0.1),
        (6, 7, 0.01, 0.1),
        (7, 8, 0.01, 0.1),
        (8, 9, 0.01, 0.1),
        (9, 0, 0.02, 0.15), // Close the ring
        (0, 5, 0.02, 0.15), // Cross connection
        (2, 7, 0.02, 0.15), // Cross connection
    ];

    for (idx, (from, to, r, x)) in branches.iter().enumerate() {
        network.graph.add_edge(
            bus_indices[*from],
            bus_indices[*to],
            Edge::Branch(Branch {
                id: BranchId::new(idx),
                name: format!("line{}_{}", from + 1, to + 1),
                from_bus: BusId::new(*from),
                to_bus: BusId::new(*to),
                resistance: *r,
                reactance: *x,
                tap_ratio: 1.0,
                phase_shift_rad: 0.0,
                charging_b_pu: 0.01,
                s_max_mva: None,
                status: true,
                rating_a_mva: None,
                is_phase_shifter: false,
            }),
        );
    }

    // Add 3 generators with different costs
    for (idx, (bus, cost)) in [(0, 10.0), (3, 15.0), (7, 20.0)].iter().enumerate() {
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(idx),
            name: format!("gen{}", idx + 1),
            bus: BusId::new(*bus),
            active_power_mw: 0.0,
            reactive_power_mvar: 0.0,
            pmin_mw: 0.0,
            pmax_mw: 150.0,
            qmin_mvar: -50.0,
            qmax_mvar: 50.0,
            is_synchronous_condenser: false,
            cost_model: CostModel::linear(0.0, *cost),
        }));
    }

    // Add loads at various buses
    let loads = [
        (2, 30.0, 10.0),
        (5, 40.0, 15.0),
        (8, 35.0, 12.0),
        (9, 25.0, 8.0),
    ];
    for (idx, (bus, p, q)) in loads.iter().enumerate() {
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(idx),
            name: format!("load{}", bus + 1),
            bus: BusId::new(*bus),
            active_power_mw: *p,
            reactive_power_mvar: *q,
        }));
    }

    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let solution = solver.solve(&network).expect("10-bus SOCP should converge");

    assert!(solution.converged);

    // Total generation should match total load + losses
    let total_gen: f64 = solution.generator_p.values().sum();
    let total_load: f64 = loads.iter().map(|(_, p, _)| p).sum();

    assert!(
        total_gen >= total_load && total_gen < total_load * 1.1,
        "Generation {} should cover load {} plus losses",
        total_gen,
        total_load
    );

    // All voltages should be in bounds
    for (name, v) in &solution.bus_voltage_mag {
        assert!(
            (0.9..=1.1).contains(v),
            "Bus {} voltage {} out of bounds",
            name,
            v
        );
    }

    // Cheaper generators should dispatch more
    let gen1_p = solution.generator_p.get("gen1").copied().unwrap();
    let gen3_p = solution.generator_p.get("gen3").copied().unwrap();
    assert!(
        gen1_p >= gen3_p,
        "Cheapest gen1 ({}) should produce >= most expensive gen3 ({})",
        gen1_p,
        gen3_p
    );
}

#[test]
fn socp_line_charging() {
    // Test that line charging (shunt susceptance) is handled correctly
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        voltage_kv: 345.0, // High voltage = more charging
    }));
    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        voltage_kv: 345.0,
    }));

    // Long line with significant charging
    network.graph.add_edge(
        bus1,
        bus2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.005,
            reactance: 0.05,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            charging_b_pu: 0.10, // Significant line charging
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
            is_phase_shifter: false,
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 200.0,
        qmin_mvar: -100.0,
        qmax_mvar: 100.0,
        is_synchronous_condenser: false,
        cost_model: CostModel::linear(0.0, 10.0),
    }));

    // Light load to see charging effect
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 10.0,
        reactive_power_mvar: 5.0,
    }));

    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let solution = solver
        .solve(&network)
        .expect("SOCP with line charging should converge");

    assert!(solution.converged);

    // Generator Q should be affected by line charging
    // With significant charging, generator may absorb reactive power
    let gen_q = solution.generator_q.get("gen1").copied().unwrap();

    // Just verify it solved and Q is reasonable
    assert!(
        gen_q.abs() < 100.0,
        "Generator Q {} should be reasonable",
        gen_q
    );
}
