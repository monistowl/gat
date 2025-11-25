//! Exact mimic of the DC-OPF logic to isolate the issue

use good_lp::solvers::clarabel::clarabel;
use good_lp::*;
use std::collections::HashMap;

#[test]
fn test_exact_dc_opf_logic() {
    // Same as above but build expressions incrementally
    let mut vars = variables!();

    let p_gen = vars.add(variable().min(0.0).max(100.0));
    let theta = vars.add(variable().min(-10.0).max(10.0));

    let cost = 10.0 * p_gen;

    // Build flow expression for bus 0 incrementally
    let mut flow0 = Expression::from(0.0);
    flow0 += -10.0 * theta;

    // Build flow expression for bus 1 incrementally
    let mut flow1 = Expression::from(0.0);
    flow1 += 10.0 * theta;

    // Build generator expression for bus 0
    let mut gen0 = Expression::from(0.0);
    gen0 += p_gen;

    // Test with cloning
    let gen0_clone = gen0.clone();
    let flow0_clone = flow0.clone();
    let flow1_clone = flow1.clone();

    let problem = vars
        .minimise(cost)
        .using(clarabel)
        .with(constraint!(gen0_clone - flow0_clone == 0.0))
        .with(constraint!(-flow1_clone == 50.0));

    let solution = problem.solve().expect("Should be feasible");

    let p_val = solution.value(p_gen);
    let theta_val = solution.value(theta);

    println!("P_gen = {}", p_val);
    println!("theta = {}", theta_val);

    assert!((theta_val + 5.0).abs() < 0.01, "theta should be -5");
    assert!((p_val - 50.0).abs() < 0.01, "P_gen should be 50");
}
