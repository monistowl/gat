//! AC-OPF solver tests
//!
//! Tests for full nonlinear AC-OPF using the unified OpfSolver API.
//! These tests validate the AC-OPF implementation (Task 6 from the plan).

use gat_algo::{OpfMethod, OpfSolver};
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
};

/// Helper: create a simple 2-bus network
/// - Bus 1 with generator
/// - Bus 2 with load
/// - One transmission line connecting them
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
        cost_model: CostModel::linear(0.0, 10.0), // $10/MWh
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 10.0,
        reactive_power_mvar: 3.0,
    }));

    network
}

/// Test 1: Basic convergence on 2-bus network
///
/// Verifies that AC-OPF can solve a simple 2-bus case and produces
/// reasonable results for generator dispatch and bus voltages.
#[test]
fn ac_opf_basic_convergence() {
    let network = two_bus_network();
    let solver = OpfSolver::new()
        .with_method(OpfMethod::AcOpf)
        .with_max_iterations(200)
        .with_tolerance(1e-4);

    let solution = solver.solve(&network).expect("AC-OPF should converge");

    // Should converge (or at least produce a result)
    // Due to penalty method, may not be exactly feasible, but should be reasonable

    // Generator should supply approximately the load
    let gen_p = solution.generator_p.get("gen1").copied().unwrap_or(0.0);
    assert!(
        gen_p > 5.0 && gen_p < 20.0,
        "Generator P {} should be near load (10 MW)",
        gen_p
    );

    // Voltages should be reasonable (0.85-1.15 p.u.)
    let v1 = solution.bus_voltage_mag.get("bus1").copied().unwrap_or(0.0);
    let v2 = solution.bus_voltage_mag.get("bus2").copied().unwrap_or(0.0);
    assert!(
        (0.85..=1.15).contains(&v1),
        "V1 {} out of range [0.85, 1.15]",
        v1
    );
    assert!(
        (0.85..=1.15).contains(&v2),
        "V2 {} out of range [0.85, 1.15]",
        v2
    );

    println!(
        "AC-OPF Basic: gen1={:.2} MW, V1={:.3} pu, V2={:.3} pu, cost=${:.2}",
        gen_p, v1, v2, solution.objective_value
    );
}

/// Test 2: Compare AC-OPF to SOCP relaxation
///
/// Runs both SOCP and AC-OPF on the same network and verifies:
/// - AC cost should be >= SOCP cost (SOCP is a lower bound)
/// - Generator dispatch should be similar
#[test]
fn ac_opf_vs_socp_comparison() {
    let network = two_bus_network();

    let socp_solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let ac_solver = OpfSolver::new()
        .with_method(OpfMethod::AcOpf)
        .with_max_iterations(200)
        .with_tolerance(1e-4);

    let socp_sol = socp_solver.solve(&network).expect("SOCP should converge");
    let ac_sol = ac_solver.solve(&network).expect("AC-OPF should converge");

    // Both should give similar objectives (SOCP is a relaxation, so lower bound)
    let socp_cost = socp_sol.objective_value;
    let ac_cost = ac_sol.objective_value;

    // AC cost should be >= SOCP cost (SOCP is relaxation)
    // But for simple networks they should be close
    // Allow 10% tolerance due to penalty method approximation
    assert!(
        ac_cost >= socp_cost * 0.9,
        "AC cost {} should be >= SOCP cost {} (minus tolerance)",
        ac_cost,
        socp_cost
    );

    // Generator dispatch should be similar (within 5 MW)
    let socp_p = socp_sol.generator_p.get("gen1").copied().unwrap_or(0.0);
    let ac_p = ac_sol.generator_p.get("gen1").copied().unwrap_or(0.0);

    assert!(
        (socp_p - ac_p).abs() < 5.0,
        "Generator dispatch should be similar: SOCP={}, AC={}",
        socp_p,
        ac_p
    );

    println!(
        "SOCP vs AC-OPF: SOCP_P={:.2} MW, AC_P={:.2} MW, SOCP_cost=${:.2}, AC_cost=${:.2}",
        socp_p, ac_p, socp_cost, ac_cost
    );
}

/// Helper: create a 3-bus meshed network (triangle topology)
/// - Three buses with 3 lines forming a triangle
/// - Gen at bus1 (cheap, $10/MWh)
/// - Gen at bus2 (expensive, $20/MWh)
/// - Load at bus3 (50 MW)
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
            charging_b_pu: 0.0,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
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
            charging_b_pu: 0.0,
            s_max_mva: None,
            status: true,
            rating_a_mva: None,
        }),
    );

    // Line 1-3 (creates mesh/triangle)
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
        }),
    );

    // Generator at bus 1 (cheap)
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
        cost_model: CostModel::linear(0.0, 10.0), // $10/MWh
    }));

    // Generator at bus 2 (expensive)
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(1),
        name: "gen2".to_string(),
        bus: BusId::new(1),
        active_power_mw: 0.0,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_model: CostModel::linear(0.0, 20.0), // $20/MWh (expensive)
    }));

    // Load at bus 3
    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load3".to_string(),
        bus: BusId::new(2),
        active_power_mw: 50.0,
        reactive_power_mvar: 15.0,
    }));

    network
}

/// Test 3: Economic dispatch on 3-bus meshed network
///
/// Verifies that AC-OPF respects economic dispatch principles:
/// - Cheaper generator should dispatch more than expensive generator
/// - Total generation should cover load plus losses
#[test]
fn ac_opf_three_bus_economic_dispatch() {
    let network = three_bus_network();
    let solver = OpfSolver::new()
        .with_method(OpfMethod::AcOpf)
        .with_max_iterations(300)
        .with_tolerance(1e-4);

    let solution = solver.solve(&network).expect("AC-OPF should converge");

    // Cheaper gen1 should dispatch more than expensive gen2
    let gen1_p = solution.generator_p.get("gen1").copied().unwrap_or(0.0);
    let gen2_p = solution.generator_p.get("gen2").copied().unwrap_or(0.0);

    assert!(
        gen1_p > gen2_p,
        "Cheaper gen1 ({:.2} MW) should dispatch more than gen2 ({:.2} MW)",
        gen1_p,
        gen2_p
    );

    // Total generation should approximately match load + losses
    let total_gen = gen1_p + gen2_p;
    assert!(
        total_gen >= 50.0 && total_gen < 60.0,
        "Total generation {:.2} MW should cover 50 MW load plus losses (< 60 MW)",
        total_gen
    );

    println!(
        "3-bus Economic Dispatch: gen1={:.2} MW ($10/MWh), gen2={:.2} MW ($20/MWh), total={:.2} MW, cost=${:.2}",
        gen1_p, gen2_p, total_gen, solution.objective_value
    );
}
