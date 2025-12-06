//! Native solver dispatch integration tests.
//!
//! These tests require the native solver binaries to be built.
//! Run: cargo build -p gat-clp --release

#![cfg(feature = "native-dispatch")]

use gat_algo::opf::native_dispatch::{
    is_clp_available, network_to_problem, solution_to_opf, solve_dc_opf_native,
};
use gat_algo::opf::OpfMethod;
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
};
use gat_solver_common::problem::ProblemType;
use gat_solver_common::solution::{SolutionBatch, SolutionStatus};

/// Create a simple 2-bus test network.
/// Bus 0: Generator (0-100 MW, $10/MWh linear cost)
/// Bus 1: Load (50 MW)
/// Branch 0-1: x = 0.1 pu
fn two_bus_network() -> Network {
    let mut network = Network::new();

    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        base_kv: gat_core::Kilovolts(138.0),
        voltage_pu: gat_core::PerUnit(1.0),
        angle_rad: gat_core::Radians(0.0),
        vmin_pu: Some(gat_core::PerUnit(0.9)),
        vmax_pu: Some(gat_core::PerUnit(1.1)),
        ..Bus::default()
    }));

    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        base_kv: gat_core::Kilovolts(138.0),
        voltage_pu: gat_core::PerUnit(1.0),
        angle_rad: gat_core::Radians(0.0),
        vmin_pu: Some(gat_core::PerUnit(0.9)),
        vmax_pu: Some(gat_core::PerUnit(1.1)),
        ..Bus::default()
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
            charging_b: gat_core::PerUnit(0.0),
            rating_a: Some(gat_core::MegavoltAmperes(100.0)),
            tap_ratio: 1.0,
            phase_shift: gat_core::Radians(0.0),
            status: true,
            ..Branch::default()
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        pmin: gat_core::Megawatts(0.0),
        pmax: gat_core::Megawatts(100.0),
        qmin: gat_core::Megavars(-50.0),
        qmax: gat_core::Megavars(50.0),
        cost_model: CostModel::linear(0.0, 10.0),
        voltage_setpoint_pu: Some(1.0),
        status: true,
        ..Gen::default()
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power: gat_core::Megawatts(50.0),
        reactive_power: gat_core::Megavars(0.0),
    }));

    network
}

#[test]
fn test_network_to_problem_conversion() {
    let network = two_bus_network();
    let problem = network_to_problem(&network, ProblemType::DcOpf, 60);

    // Verify basic structure
    assert_eq!(problem.bus_id.len(), 2, "Should have 2 buses");
    assert_eq!(problem.gen_id.len(), 1, "Should have 1 generator");
    assert_eq!(problem.branch_id.len(), 1, "Should have 1 branch");

    // Verify bus data
    assert_eq!(problem.bus_id[0], 0, "First bus ID");
    assert_eq!(problem.bus_id[1], 1, "Second bus ID");
    assert_eq!(problem.bus_name[0], "bus1");
    assert_eq!(problem.bus_name[1], "bus2");
    assert_eq!(problem.bus_type[0], 3, "First bus should be slack");
    assert_eq!(problem.bus_type[1], 1, "Second bus should be PQ");

    // Verify load was aggregated to bus
    assert_eq!(problem.bus_p_load[0], 0.0, "Bus 0 should have no load");
    assert_eq!(problem.bus_p_load[1], 50.0, "Bus 1 should have 50 MW load");

    // Verify generator data
    assert_eq!(problem.gen_id[0], 0);
    assert_eq!(problem.gen_bus_id[0], 0, "Gen should be on bus 0");
    assert_eq!(problem.gen_p_max[0], 100.0, "Gen max should be 100 MW");
    assert_eq!(
        problem.gen_cost_c1[0], 10.0,
        "Linear cost should be $10/MWh"
    );

    // Verify branch data
    assert_eq!(problem.branch_id[0], 0);
    assert_eq!(problem.branch_from[0], 0);
    assert_eq!(problem.branch_to[0], 1);
    assert_eq!(problem.branch_x[0], 0.1, "Reactance should be 0.1");
}

#[test]
fn test_solution_to_opf_conversion() {
    let network = two_bus_network();

    // Create a mock solution
    let mut solution = SolutionBatch::default();
    solution.status = SolutionStatus::Optimal;
    solution.iterations = 10;
    solution.solve_time_ms = 123;
    solution.objective = 500.0;

    // Bus results
    solution.bus_id = vec![0, 1];
    solution.bus_v_mag = vec![1.0, 0.98];
    solution.bus_v_ang = vec![0.0, -0.1];
    solution.bus_lmp = vec![10.0, 11.0];

    // Generator results
    solution.gen_id = vec![0];
    solution.gen_p = vec![50.0];
    solution.gen_q = vec![5.0];

    // Branch results
    solution.branch_id = vec![0];
    solution.branch_p_from = vec![50.0];
    solution.branch_q_from = vec![5.0];

    // Convert to OpfSolution
    let opf_solution = solution_to_opf(&solution, &network, OpfMethod::DcOpf);

    // Verify conversion
    assert!(opf_solution.converged, "Should be marked as converged");
    assert_eq!(opf_solution.method_used, OpfMethod::DcOpf);
    assert_eq!(opf_solution.iterations, 10);
    assert_eq!(opf_solution.solve_time_ms, 123);
    assert_eq!(opf_solution.objective_value, 500.0);

    // Check bus results
    assert_eq!(
        opf_solution.bus_voltage_mag.get("bus1"),
        Some(&1.0),
        "Bus1 voltage magnitude"
    );
    assert_eq!(
        opf_solution.bus_voltage_mag.get("bus2"),
        Some(&0.98),
        "Bus2 voltage magnitude"
    );
    assert_eq!(opf_solution.bus_lmp.get("bus1"), Some(&10.0), "Bus1 LMP");
    assert_eq!(opf_solution.bus_lmp.get("bus2"), Some(&11.0), "Bus2 LMP");

    // Check generator results
    assert_eq!(
        opf_solution.generator_p.get("gen1"),
        Some(&50.0),
        "Gen1 active power"
    );
    assert_eq!(
        opf_solution.generator_q.get("gen1"),
        Some(&5.0),
        "Gen1 reactive power"
    );

    // Check branch results
    assert_eq!(
        opf_solution.branch_p_flow.get("line1_2"),
        Some(&50.0),
        "Branch active power flow"
    );
    assert_eq!(
        opf_solution.branch_q_flow.get("line1_2"),
        Some(&5.0),
        "Branch reactive power flow"
    );
}

#[test]
#[ignore = "requires gat-clp binary (cargo build -p gat-clp --release)"]
fn test_solve_dc_opf_native_two_bus() {
    if !is_clp_available() {
        eprintln!("Skipping: gat-clp not available. Build with: cargo build -p gat-clp --release");
        return;
    }

    let network = two_bus_network();
    let solution = solve_dc_opf_native(&network, 60).expect("Should solve successfully");

    // The solver should complete without errors
    assert!(solution.converged, "Solution should converge");
    assert_eq!(solution.method_used, OpfMethod::DcOpf);

    // Full DC-OPF formulation is implemented.
    // This test verifies:
    // 1. The IPC protocol works (problem → Arrow → solver → Arrow → solution)
    // 2. The binary can be found and executed
    // 3. The DC-OPF formulation (power balance, line limits) is correct
    // 4. The conversion functions handle the roundtrip correctly

    // Verify solution structure is populated
    assert!(
        solution.generator_p.contains_key("gen1"),
        "Solution should include gen1"
    );
    assert!(
        solution.bus_lmp.contains_key("bus1"),
        "Solution should include bus1 LMP"
    );
    assert!(
        solution.bus_lmp.contains_key("bus2"),
        "Solution should include bus2 LMP"
    );

    // Verify generator output matches load (50 MW)
    let gen_p = solution.generator_p.get("gen1").copied().unwrap_or(0.0);
    assert!(
        gen_p > 45.0 && gen_p < 55.0,
        "Gen should supply ~50 MW (matching load), got {}",
        gen_p
    );

    // Verify objective value (~$500/hr for 50 MW at $10/MWh)
    assert!(
        solution.objective_value > 400.0 && solution.objective_value < 600.0,
        "Objective should be ~$500/hr, got {}",
        solution.objective_value
    );

    // Verify bus angles: bus 0 (ref) should be 0, bus 1 should be negative
    // (power flows from bus 0 → bus 1, so θ_0 > θ_1)
    let theta_0 = solution.bus_voltage_ang.get("bus1").copied().unwrap_or(0.0);
    let theta_1 = solution.bus_voltage_ang.get("bus2").copied().unwrap_or(0.0);
    assert!(
        theta_0.abs() < 1e-6,
        "Reference bus angle should be ~0, got {}",
        theta_0
    );
    assert!(
        theta_1 < 0.0,
        "Load bus angle should be negative (power flows to it), got {}",
        theta_1
    );
}

#[test]
fn test_network_to_problem_with_offline_components() {
    let mut network = Network::new();

    // Add buses
    network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        ..Bus::default()
    }));

    network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        ..Bus::default()
    }));

    // Add online generator
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        pmax: gat_core::Megawatts(100.0),
        status: true,
        ..Gen::default()
    }));

    // Add offline generator (should be excluded)
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(1),
        name: "gen2".to_string(),
        bus: BusId::new(1),
        pmax: gat_core::Megawatts(50.0),
        status: false, // OFFLINE
        ..Gen::default()
    }));

    let problem = network_to_problem(&network, ProblemType::DcOpf, 60);

    // Should only include the online generator
    assert_eq!(
        problem.gen_id.len(),
        1,
        "Should only have 1 online generator"
    );
    assert_eq!(problem.gen_id[0], 0, "Should be gen1 (id=0)");
}

#[test]
fn test_network_to_problem_polynomial_cost() {
    let mut network = Network::new();

    network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        ..Bus::default()
    }));

    // Generator with quadratic cost: c0 + c1*P + c2*P^2
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        pmax: gat_core::Megawatts(100.0),
        cost_model: CostModel::Polynomial(vec![100.0, 20.0, 0.5]), // c0, c1, c2
        status: true,
        ..Gen::default()
    }));

    let problem = network_to_problem(&network, ProblemType::DcOpf, 60);

    assert_eq!(problem.gen_cost_c0[0], 100.0, "Constant term");
    assert_eq!(problem.gen_cost_c1[0], 20.0, "Linear term");
    assert_eq!(problem.gen_cost_c2[0], 0.5, "Quadratic term");
}

#[test]
fn test_network_to_problem_timeout() {
    let network = two_bus_network();
    let timeout = 120u64;
    let problem = network_to_problem(&network, ProblemType::DcOpf, timeout);

    assert_eq!(
        problem.timeout_seconds, timeout,
        "Timeout should be set correctly"
    );
}
