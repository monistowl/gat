//! Diagnostic tests for AC-OPF convergence issues on case118_ieee.
//!
//! This file investigates why case118_ieee fails to converge with our AC-NLP IPOPT solver.
//! Key analysis includes:
//! - Per-bus constraint violation analysis at flat start
//! - Component breakdown (P_inj, P_gen, P_load, P_shunt) per bus
//! - Comparison with working cases (case14, case30)
//!
//! Run with:
//! ```bash
//! cargo test -p gat-algo debug_case118 --release --features solver-ipopt -- --nocapture
//! ```

use gat_algo::opf::ac_nlp::{AcOpfProblem, PowerEquations};
use gat_io::importers::parse_matpower;
use std::path::{Path, PathBuf};

/// Get the workspace root directory (gat/)
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points to crates/gat-algo
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // gat/
        .expect("Failed to find workspace root")
        .to_path_buf()
}

/// Load a PGLib case from the standard data directory.
fn load_pglib_case(case_name: &str) -> gat_core::Network {
    let case_path = workspace_root()
        .join("data")
        .join("pglib-opf")
        .join(case_name)
        .join("case.m");

    if !case_path.exists() {
        panic!(
            "Case file not found: {}",
            case_path.display()
        );
    }

    let result = parse_matpower(case_path.to_str().unwrap()).expect("Failed to parse MATPOWER file");
    result.network
}

/// Analyze constraint violations at a given point.
struct ConstraintAnalysis {
    /// Per-bus P balance violation (should be ~0 at feasible point)
    p_violations: Vec<f64>,
    /// Per-bus Q balance violation (should be ~0 at feasible point)
    q_violations: Vec<f64>,
    /// Component breakdown for debugging
    bus_components: Vec<BusComponents>,
}

/// Breakdown of power balance components at each bus.
#[derive(Debug, Clone)]
struct BusComponents {
    bus_idx: usize,
    bus_name: String,
    /// Power injection from network (P_inj from Y-bus)
    p_inj: f64,
    /// Generator injection (sum of Pg at this bus)
    p_gen: f64,
    /// Load at this bus (MW converted to p.u.)
    p_load_pu: f64,
    /// Shunt power at this bus (gs * V^2)
    p_shunt_pu: f64,
    /// Net P violation: P_inj - P_gen + P_load + P_shunt
    p_violation: f64,
    /// Same for Q
    q_inj: f64,
    q_gen: f64,
    q_load_pu: f64,
    q_shunt_pu: f64,
    q_violation: f64,
    /// Voltage at this bus (from initial point)
    v: f64,
}

fn analyze_constraints(problem: &AcOpfProblem, x: &[f64]) -> ConstraintAnalysis {
    // Extract voltages and angles
    let (v, theta) = problem.extract_v_theta(x);

    // Compute power injections from AC power flow equations
    let (p_inj, q_inj) = PowerEquations::compute_injections(&problem.ybus, &v, &theta);

    // Sum generator injections at each bus
    let mut pg_bus = vec![0.0; problem.n_bus];
    let mut qg_bus = vec![0.0; problem.n_bus];

    for (i, &bus_idx) in problem.gen_bus_idx.iter().enumerate() {
        pg_bus[bus_idx] += x[problem.pg_offset + i];
        qg_bus[bus_idx] += x[problem.qg_offset + i];
    }

    let mut bus_components = Vec::with_capacity(problem.n_bus);
    let mut p_violations = Vec::with_capacity(problem.n_bus);
    let mut q_violations = Vec::with_capacity(problem.n_bus);

    for (i, bus) in problem.buses.iter().enumerate() {
        let p_load_pu = bus.p_load / problem.base_mva;
        let q_load_pu = bus.q_load / problem.base_mva;
        let v_sq = v[i] * v[i];
        let p_shunt_pu = bus.gs_pu * v_sq;
        let q_shunt_pu = bus.bs_pu * v_sq;

        // P balance: P_inj - P_gen + P_load + P_shunt = 0
        let p_viol = p_inj[i] - pg_bus[i] + p_load_pu + p_shunt_pu;
        // Q balance: Q_inj - Q_gen + Q_load - Q_shunt = 0
        let q_viol = q_inj[i] - qg_bus[i] + q_load_pu - q_shunt_pu;

        p_violations.push(p_viol);
        q_violations.push(q_viol);

        bus_components.push(BusComponents {
            bus_idx: i,
            bus_name: bus.name.clone(),
            p_inj: p_inj[i],
            p_gen: pg_bus[i],
            p_load_pu,
            p_shunt_pu,
            p_violation: p_viol,
            q_inj: q_inj[i],
            q_gen: qg_bus[i],
            q_load_pu,
            q_shunt_pu,
            q_violation: q_viol,
            v: v[i],
        });
    }

    ConstraintAnalysis {
        p_violations,
        q_violations,
        bus_components,
    }
}

fn print_top_violations(analysis: &ConstraintAnalysis, problem: &AcOpfProblem, top_n: usize) {
    println!("\n=== TOP {} BUSES BY |P| VIOLATION ===", top_n);
    println!(
        "{:<8} {:<15} {:>10} {:>10} {:>10} {:>10} {:>12}",
        "Index", "Name", "P_inj", "P_gen", "P_load", "P_shunt", "VIOLATION"
    );
    println!("{}", "-".repeat(80));

    let mut sorted: Vec<_> = analysis.bus_components.iter().collect();
    sorted.sort_by(|a, b| {
        b.p_violation
            .abs()
            .partial_cmp(&a.p_violation.abs())
            .unwrap()
    });

    for comp in sorted.iter().take(top_n) {
        println!(
            "{:<8} {:<15} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>12.4}",
            comp.bus_idx,
            &comp.bus_name[..comp.bus_name.len().min(15)],
            comp.p_inj,
            comp.p_gen,
            comp.p_load_pu,
            comp.p_shunt_pu,
            comp.p_violation
        );
    }

    println!("\n=== TOP {} BUSES BY |Q| VIOLATION ===", top_n);
    println!(
        "{:<8} {:<15} {:>10} {:>10} {:>10} {:>10} {:>12}",
        "Index", "Name", "Q_inj", "Q_gen", "Q_load", "Q_shunt", "VIOLATION"
    );
    println!("{}", "-".repeat(80));

    sorted.sort_by(|a, b| {
        b.q_violation
            .abs()
            .partial_cmp(&a.q_violation.abs())
            .unwrap()
    });

    for comp in sorted.iter().take(top_n) {
        println!(
            "{:<8} {:<15} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>12.4}",
            comp.bus_idx,
            &comp.bus_name[..comp.bus_name.len().min(15)],
            comp.q_inj,
            comp.q_gen,
            comp.q_load_pu,
            comp.q_shunt_pu,
            comp.q_violation
        );
    }

    // Summary statistics
    let total_p_viol: f64 = analysis.p_violations.iter().map(|v| v.abs()).sum();
    let total_q_viol: f64 = analysis.q_violations.iter().map(|v| v.abs()).sum();
    let max_p_viol = analysis
        .p_violations
        .iter()
        .map(|v| v.abs())
        .fold(0.0f64, f64::max);
    let max_q_viol = analysis
        .q_violations
        .iter()
        .map(|v| v.abs())
        .fold(0.0f64, f64::max);

    println!("\n=== SUMMARY ===");
    println!("Total |P| violation: {:.4} p.u.", total_p_viol);
    println!("Total |Q| violation: {:.4} p.u.", total_q_viol);
    println!("Max |P| violation:   {:.4} p.u.", max_p_viol);
    println!("Max |Q| violation:   {:.4} p.u.", max_q_viol);

    // Check generation vs load balance
    let total_p_gen: f64 = analysis.bus_components.iter().map(|c| c.p_gen).sum();
    let total_p_load: f64 = analysis.bus_components.iter().map(|c| c.p_load_pu).sum();
    let total_p_shunt: f64 = analysis.bus_components.iter().map(|c| c.p_shunt_pu).sum();
    let total_q_gen: f64 = analysis.bus_components.iter().map(|c| c.q_gen).sum();
    let total_q_load: f64 = analysis.bus_components.iter().map(|c| c.q_load_pu).sum();
    let total_q_shunt: f64 = analysis.bus_components.iter().map(|c| c.q_shunt_pu).sum();

    println!("\n=== GENERATION VS LOAD BALANCE ===");
    println!("Total P_gen:   {:>10.4} p.u.", total_p_gen);
    println!("Total P_load:  {:>10.4} p.u.", total_p_load);
    println!("Total P_shunt: {:>10.4} p.u.", total_p_shunt);
    println!(
        "P balance (gen - load - shunt): {:>10.4} p.u.",
        total_p_gen - total_p_load - total_p_shunt
    );
    println!();
    println!("Total Q_gen:   {:>10.4} p.u.", total_q_gen);
    println!("Total Q_load:  {:>10.4} p.u.", total_q_load);
    println!("Total Q_shunt: {:>10.4} p.u.", total_q_shunt);
    println!(
        "Q balance (gen - load + shunt): {:>10.4} p.u.",
        total_q_gen - total_q_load + total_q_shunt
    );

    // Count buses with shunts
    let buses_with_shunts: Vec<_> = analysis
        .bus_components
        .iter()
        .filter(|c| c.p_shunt_pu.abs() > 1e-6 || c.q_shunt_pu.abs() > 1e-6)
        .collect();

    if !buses_with_shunts.is_empty() {
        println!("\n=== BUSES WITH SHUNTS ===");
        println!(
            "{:<8} {:<15} {:>10} {:>10}",
            "Index", "Name", "Gs (p.u.)", "Bs (p.u.)"
        );
        for comp in &buses_with_shunts {
            let bus = &problem.buses[comp.bus_idx];
            println!(
                "{:<8} {:<15} {:>10.4} {:>10.4}",
                comp.bus_idx,
                &comp.bus_name[..comp.bus_name.len().min(15)],
                bus.gs_pu,
                bus.bs_pu
            );
        }
    }
}

/// Debug test for case118 - show per-bus constraint violations
#[test]
fn debug_case118_violations() {
    println!("\n");
    println!("============================================================");
    println!("  CASE118 CONSTRAINT VIOLATION ANALYSIS");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case118_ieee");
    let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    println!("\nProblem dimensions:");
    println!("  Buses:      {}", problem.n_bus);
    println!("  Generators: {}", problem.n_gen);
    println!("  Branches:   {}", problem.n_branch);
    println!("  Variables:  {}", problem.n_var);
    println!("  Base MVA:   {}", problem.base_mva);

    // Use flat start (V=1.0, θ=0, Pg/Qg at midpoint)
    let x0 = problem.initial_point();

    let analysis = analyze_constraints(&problem, &x0);
    print_top_violations(&analysis, &problem, 15);
}

/// Debug test for case14 (working case) - baseline comparison
#[test]
fn debug_case14_violations() {
    println!("\n");
    println!("============================================================");
    println!("  CASE14 CONSTRAINT VIOLATION ANALYSIS (BASELINE)");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case14_ieee");
    let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    println!("\nProblem dimensions:");
    println!("  Buses:      {}", problem.n_bus);
    println!("  Generators: {}", problem.n_gen);
    println!("  Branches:   {}", problem.n_branch);
    println!("  Variables:  {}", problem.n_var);
    println!("  Base MVA:   {}", problem.base_mva);

    let x0 = problem.initial_point();

    let analysis = analyze_constraints(&problem, &x0);
    print_top_violations(&analysis, &problem, 10);
}

/// Debug test for case30 (working case) - another baseline
#[test]
fn debug_case30_violations() {
    println!("\n");
    println!("============================================================");
    println!("  CASE30 CONSTRAINT VIOLATION ANALYSIS (BASELINE)");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case30_ieee");
    let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    println!("\nProblem dimensions:");
    println!("  Buses:      {}", problem.n_bus);
    println!("  Generators: {}", problem.n_gen);
    println!("  Branches:   {}", problem.n_branch);
    println!("  Variables:  {}", problem.n_var);
    println!("  Base MVA:   {}", problem.base_mva);

    let x0 = problem.initial_point();

    let analysis = analyze_constraints(&problem, &x0);
    print_top_violations(&analysis, &problem, 10);
}

/// Debug test for case162_ieee_dtc - the other failing case
#[test]
fn debug_case162_violations() {
    println!("\n");
    println!("============================================================");
    println!("  CASE162 CONSTRAINT VIOLATION ANALYSIS");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case162_ieee_dtc");
    let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    println!("\nProblem dimensions:");
    println!("  Buses:      {}", problem.n_bus);
    println!("  Generators: {}", problem.n_gen);
    println!("  Branches:   {}", problem.n_branch);
    println!("  Variables:  {}", problem.n_var);
    println!("  Base MVA:   {}", problem.base_mva);

    let x0 = problem.initial_point();

    let analysis = analyze_constraints(&problem, &x0);
    print_top_violations(&analysis, &problem, 15);
}

/// Compare generator mappings between cases
#[test]
fn debug_generator_mapping() {
    println!("\n");
    println!("============================================================");
    println!("  GENERATOR-TO-BUS MAPPING ANALYSIS");
    println!("============================================================");

    for case_name in &[
        "pglib_opf_case14_ieee",
        "pglib_opf_case30_ieee",
        "pglib_opf_case118_ieee",
    ] {
        let network = load_pglib_case(case_name);
        let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

        println!("\n=== {} ===", case_name);
        println!(
            "{:<20} {:<15} {:>10} {:>10} {:>10} {:>10}",
            "Generator", "Bus", "Pmin", "Pmax", "Qmin", "Qmax"
        );

        for (i, gen) in problem.generators.iter().enumerate() {
            let bus_idx = problem.gen_bus_idx[i];
            let bus_name = &problem.buses[bus_idx].name;
            println!(
                "{:<20} {:<15} {:>10.2} {:>10.2} {:>10.2} {:>10.2}",
                gen.name, bus_name, gen.pmin_mw, gen.pmax_mw, gen.qmin_mvar, gen.qmax_mvar
            );
        }

        // Check for buses with multiple generators
        let mut gen_count_per_bus = vec![0usize; problem.n_bus];
        for &bus_idx in &problem.gen_bus_idx {
            gen_count_per_bus[bus_idx] += 1;
        }

        let multi_gen_buses: Vec<_> = gen_count_per_bus
            .iter()
            .enumerate()
            .filter(|(_, &count)| count > 1)
            .collect();

        if !multi_gen_buses.is_empty() {
            println!("\nBuses with multiple generators:");
            for (bus_idx, &count) in &multi_gen_buses {
                println!("  {} (idx {}): {} generators", problem.buses[*bus_idx].name, bus_idx, count);
            }
        }
    }
}

/// Test AC-OPF solver convergence on case118 with IPOPT
#[cfg(feature = "solver-ipopt")]
#[test]
fn test_case118_convergence() {
    use gat_algo::{OpfMethod, OpfSolver};

    println!("\n");
    println!("============================================================");
    println!("  CASE118 AC-OPF SOLVER TEST (IPOPT)");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case118_ieee");

    // IMPORTANT: prefer_native(true) uses IPOPT instead of penalty method
    let solver = OpfSolver::new()
        .with_method(OpfMethod::AcOpf)
        .with_max_iterations(1000)
        .with_tolerance(1e-6)
        .prefer_native(true);

    // Reference objective from MATPOWER: $97,214/hr
    let reference_obj = 97214.0;

    match solver.solve(&network) {
        Ok(solution) => {
            println!("\nAC-OPF CONVERGED!");
            println!("  Objective:  ${:.2}/hr (reference: ${:.2})", solution.objective_value, reference_obj);
            println!("  Obj gap:    {:.2}%", (solution.objective_value - reference_obj) / reference_obj * 100.0);
            println!("  Iterations: {}", solution.iterations);
            println!("  Time:       {:.2}ms", solution.solve_time_ms);

            // Check total generation
            let total_p_gen: f64 = solution.generator_p.values().sum();
            println!("  Total Pgen: {:.2} MW", total_p_gen);

            // Verify objective is within 1% of reference
            let obj_gap = (solution.objective_value - reference_obj).abs() / reference_obj;
            assert!(
                obj_gap < 0.01,
                "Objective ${:.2} should be within 1% of reference ${:.2} (gap: {:.2}%)",
                solution.objective_value, reference_obj, obj_gap * 100.0
            );
        }
        Err(e) => {
            println!("\nAC-OPF FAILED: {:?}", e);
            panic!("AC-OPF should converge on case118");
        }
    }
}

/// Test AC-OPF solver convergence on case14 with IPOPT
#[cfg(feature = "solver-ipopt")]
#[test]
fn test_case14_convergence() {
    use gat_algo::{OpfMethod, OpfSolver};

    println!("\n");
    println!("============================================================");
    println!("  CASE14 AC-OPF SOLVER TEST (IPOPT)");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case14_ieee");

    // IMPORTANT: prefer_native(true) uses IPOPT instead of penalty method
    let solver = OpfSolver::new()
        .with_method(OpfMethod::AcOpf)
        .with_max_iterations(500)
        .with_tolerance(1e-6)
        .prefer_native(true);

    // Reference objective from MATPOWER: $2,178.1/hr
    let reference_obj = 2178.1;

    match solver.solve(&network) {
        Ok(solution) => {
            println!("\nAC-OPF CONVERGED!");
            println!("  Objective:  ${:.2}/hr (reference: ${:.2})", solution.objective_value, reference_obj);
            println!("  Obj gap:    {:.2}%", (solution.objective_value - reference_obj) / reference_obj * 100.0);
            println!("  Iterations: {}", solution.iterations);
            println!("  Time:       {:.2}ms", solution.solve_time_ms);

            // Check total generation
            let total_p_gen: f64 = solution.generator_p.values().sum();
            println!("  Total Pgen: {:.2} MW", total_p_gen);

            // Verify objective is within 1% of reference
            let obj_gap = (solution.objective_value - reference_obj).abs() / reference_obj;
            assert!(
                obj_gap < 0.01,
                "Objective ${:.2} should be within 1% of reference ${:.2} (gap: {:.2}%)",
                solution.objective_value, reference_obj, obj_gap * 100.0
            );
        }
        Err(e) => {
            println!("\nAC-OPF FAILED: {:?}", e);
            panic!("AC-OPF should converge on case14");
        }
    }
}

/// Compare the actual equality constraint values with what we compute manually
#[test]
fn debug_constraint_verification() {
    println!("\n");
    println!("============================================================");
    println!("  CONSTRAINT VALUE VERIFICATION");
    println!("============================================================");

    for case_name in &[
        "pglib_opf_case14_ieee",
        "pglib_opf_case118_ieee",
    ] {
        let network = load_pglib_case(case_name);
        let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

        println!("\n=== {} ===", case_name);

        let x0 = problem.initial_point();

        // Get constraint values from the problem's equality_constraints method
        let g = problem.equality_constraints(&x0);

        // Manual computation for comparison
        let analysis = analyze_constraints(&problem, &x0);

        println!("Comparing problem.equality_constraints() vs manual computation:");
        println!("{:<8} {:>15} {:>15} {:>10}", "Bus", "g (problem)", "manual P", "diff");

        let mut max_diff = 0.0f64;
        for i in 0..problem.n_bus.min(10) {
            let diff = (g[i] - analysis.p_violations[i]).abs();
            max_diff = max_diff.max(diff);
            println!(
                "{:<8} {:>15.6} {:>15.6} {:>10.2e}",
                i, g[i], analysis.p_violations[i], diff
            );
        }

        println!("\nMax P difference: {:.2e}", max_diff);

        // Also check Q constraints
        max_diff = 0.0;
        for i in 0..problem.n_bus.min(10) {
            let diff = (g[problem.n_bus + i] - analysis.q_violations[i]).abs();
            max_diff = max_diff.max(diff);
        }
        println!("Max Q difference: {:.2e}", max_diff);
    }
}

/// Test AC-OPF solver convergence on case118 with penalty method (no IPOPT)
#[test]
fn test_case118_penalty_method() {
    use gat_algo::opf::ac_nlp::solve_ac_opf;

    println!("\n");
    println!("============================================================");
    println!("  CASE118 AC-OPF SOLVER TEST (PENALTY METHOD)");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case118_ieee");
    let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    println!("\nProblem dimensions:");
    println!("  Buses:       {}", problem.n_bus);
    println!("  Generators:  {}", problem.n_gen);
    println!("  Branches:    {}", problem.n_branch);
    println!("  Thermal lim: {}", problem.n_thermal_constrained_branches());
    println!("  Variables:   {}", problem.n_var);

    // Try penalty method solver
    let max_iter = 2000;
    let tol = 1e-4;

    match solve_ac_opf(&problem, max_iter, tol) {
        Ok(solution) => {
            println!("\nPENALTY METHOD CONVERGED!");
            println!("  Objective:  ${:.2}/hr", solution.objective_value);
            println!("  Iterations: {}", solution.iterations);
            println!("  Time:       {:.2}ms", solution.solve_time_ms);

            // Reference objective from MATPOWER: $97,214/hr
            let reference_obj = 97214.0;
            let obj_gap = (solution.objective_value - reference_obj).abs() / reference_obj;
            println!("  Obj gap:    {:.2}%", obj_gap * 100.0);
        }
        Err(e) => {
            println!("\nPENALTY METHOD FAILED: {:?}", e);
            // Don't panic - this is diagnostic, we want to see the error
        }
    }
}

/// Test AC-OPF solver convergence on case14 with penalty method (baseline)
#[test]
fn test_case14_penalty_method() {
    use gat_algo::opf::ac_nlp::solve_ac_opf;

    println!("\n");
    println!("============================================================");
    println!("  CASE14 AC-OPF SOLVER TEST (PENALTY METHOD)");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case14_ieee");
    let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    println!("\nProblem dimensions:");
    println!("  Buses:       {}", problem.n_bus);
    println!("  Generators:  {}", problem.n_gen);
    println!("  Branches:    {}", problem.n_branch);
    println!("  Thermal lim: {}", problem.n_thermal_constrained_branches());
    println!("  Variables:   {}", problem.n_var);

    // Try penalty method solver
    let max_iter = 1000;
    let tol = 1e-4;

    match solve_ac_opf(&problem, max_iter, tol) {
        Ok(solution) => {
            println!("\nPENALTY METHOD CONVERGED!");
            println!("  Objective:  ${:.2}/hr", solution.objective_value);
            println!("  Iterations: {}", solution.iterations);
            println!("  Time:       {:.2}ms", solution.solve_time_ms);

            // Reference objective from MATPOWER: $2,178.1/hr
            let reference_obj = 2178.1;
            let obj_gap = (solution.objective_value - reference_obj).abs() / reference_obj;
            println!("  Obj gap:    {:.2}%", obj_gap * 100.0);
        }
        Err(e) => {
            println!("\nPENALTY METHOD FAILED: {:?}", e);
        }
    }
}
