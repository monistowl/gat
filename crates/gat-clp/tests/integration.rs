//! Integration tests for gat-clp binary.
//!
//! These tests verify the Arrow IPC round-trip by spawning the gat-clp binary
//! and sending/receiving data through stdin/stdout.

use gat_solver_common::ipc::{read_solution, write_problem};
use gat_solver_common::problem::{ProblemBatch, ProblemType};
use gat_solver_common::solution::SolutionStatus;
use std::process::{Command, Stdio};
use std::io::Write;

/// Find the gat-clp binary in the target directory.
fn find_binary() -> std::path::PathBuf {
    // Look for debug binary first
    let debug_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/debug/gat-clp");

    if debug_path.exists() {
        return debug_path;
    }

    // Try release binary
    let release_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/release/gat-clp");

    if release_path.exists() {
        return release_path;
    }

    panic!("Could not find gat-clp binary. Run `cargo build -p gat-clp` first.");
}

#[test]
fn test_clp_empty_problem() {
    let binary = find_binary();

    // Create a minimal problem with no generators (trivial solution)
    let mut problem = ProblemBatch::new(ProblemType::DcOpf);
    problem.bus_id = vec![1];
    problem.bus_v_min = vec![0.95];
    problem.bus_v_max = vec![1.05];
    problem.bus_p_load = vec![0.0];
    problem.bus_q_load = vec![0.0];
    problem.bus_type = vec![3]; // Slack bus
    problem.bus_v_mag = vec![1.0];
    problem.bus_v_ang = vec![0.0];

    // Serialize to Arrow IPC
    let mut problem_bytes = Vec::new();
    write_problem(&problem, &mut problem_bytes).expect("Failed to serialize problem");

    // Spawn gat-clp and pipe data
    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn gat-clp");

    // Write problem to stdin
    {
        let stdin = child.stdin.as_mut().expect("Failed to get stdin");
        stdin.write_all(&problem_bytes).expect("Failed to write problem");
    }

    // Wait for completion
    let output = child.wait_with_output().expect("Failed to wait for gat-clp");

    // Check exit code
    assert!(output.status.success(), "gat-clp failed with status {:?}\nstderr: {}",
            output.status, String::from_utf8_lossy(&output.stderr));

    // Read solution
    let solution = read_solution(&output.stdout[..]).expect("Failed to parse solution");

    // Verify solution
    assert_eq!(solution.status, SolutionStatus::Optimal, "Expected optimal solution");
    assert!((solution.objective - 0.0).abs() < 1e-6, "Expected zero objective for empty problem");
}

#[test]
fn test_clp_simple_dispatch() {
    let binary = find_binary();

    // Create a simple problem with one generator
    let mut problem = ProblemBatch::new(ProblemType::DcOpf);

    // One bus with 100 MW load
    problem.bus_id = vec![1];
    problem.bus_v_min = vec![0.95];
    problem.bus_v_max = vec![1.05];
    problem.bus_p_load = vec![100.0];
    problem.bus_q_load = vec![0.0];
    problem.bus_type = vec![3];
    problem.bus_v_mag = vec![1.0];
    problem.bus_v_ang = vec![0.0];

    // One generator: 50-200 MW capacity, $10/MWh cost
    problem.gen_id = vec![1];
    problem.gen_bus_id = vec![1];
    problem.gen_p_min = vec![50.0];
    problem.gen_p_max = vec![200.0];
    problem.gen_q_min = vec![-100.0];
    problem.gen_q_max = vec![100.0];
    problem.gen_cost_c0 = vec![0.0];
    problem.gen_cost_c1 = vec![10.0];  // $10/MWh
    problem.gen_cost_c2 = vec![0.0];

    // Serialize
    let mut problem_bytes = Vec::new();
    write_problem(&problem, &mut problem_bytes).expect("Failed to serialize problem");

    // Spawn and run
    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn gat-clp");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(&problem_bytes).unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait");

    eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "gat-clp failed: {}",
            String::from_utf8_lossy(&output.stderr));

    let solution = read_solution(&output.stdout[..]).expect("Failed to parse solution");

    // The simple formulation in gat-clp just optimizes generator bounds
    // Without network constraints, it should dispatch at minimum gen (50 MW)
    // to minimize cost (since objective = c1 * P)
    assert_eq!(solution.status, SolutionStatus::Optimal);
    assert!(!solution.gen_p.is_empty(), "Should have generator dispatch");

    // Generator should be at its minimum (50 MW) to minimize cost
    let gen_p = solution.gen_p[0];
    assert!((gen_p - 50.0).abs() < 1e-4,
            "Expected gen_p = 50.0 (minimum), got {}", gen_p);

    // Objective should be 50 * 10 = 500
    assert!((solution.objective - 500.0).abs() < 1e-4,
            "Expected objective = 500.0, got {}", solution.objective);
}

#[test]
fn test_clp_multiple_generators() {
    let binary = find_binary();

    // Create problem with two generators of different costs
    let mut problem = ProblemBatch::new(ProblemType::DcOpf);

    // One bus
    problem.bus_id = vec![1];
    problem.bus_v_min = vec![0.95];
    problem.bus_v_max = vec![1.05];
    problem.bus_p_load = vec![0.0];
    problem.bus_q_load = vec![0.0];
    problem.bus_type = vec![3];
    problem.bus_v_mag = vec![1.0];
    problem.bus_v_ang = vec![0.0];

    // Two generators:
    // Gen 1: 0-100 MW, $20/MWh (expensive)
    // Gen 2: 0-100 MW, $10/MWh (cheap)
    problem.gen_id = vec![1, 2];
    problem.gen_bus_id = vec![1, 1];
    problem.gen_p_min = vec![0.0, 0.0];
    problem.gen_p_max = vec![100.0, 100.0];
    problem.gen_q_min = vec![-50.0, -50.0];
    problem.gen_q_max = vec![50.0, 50.0];
    problem.gen_cost_c0 = vec![0.0, 0.0];
    problem.gen_cost_c1 = vec![20.0, 10.0];  // Gen 2 is cheaper
    problem.gen_cost_c2 = vec![0.0, 0.0];

    // Serialize
    let mut problem_bytes = Vec::new();
    write_problem(&problem, &mut problem_bytes).expect("Failed to serialize");

    // Run solver
    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(&problem_bytes).unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait");
    assert!(output.status.success());

    let solution = read_solution(&output.stdout[..]).expect("Failed to parse");

    // Both should be at minimum (0) to minimize cost
    assert_eq!(solution.status, SolutionStatus::Optimal);
    assert_eq!(solution.gen_p.len(), 2);

    // Both generators should be at 0 MW (minimum)
    assert!((solution.gen_p[0] - 0.0).abs() < 1e-4,
            "Gen 1 should be at min, got {}", solution.gen_p[0]);
    assert!((solution.gen_p[1] - 0.0).abs() < 1e-4,
            "Gen 2 should be at min, got {}", solution.gen_p[1]);

    // Objective should be 0
    assert!((solution.objective - 0.0).abs() < 1e-4,
            "Expected objective = 0, got {}", solution.objective);
}
