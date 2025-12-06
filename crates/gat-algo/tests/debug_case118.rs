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
        panic!("Case file not found: {}", case_path.display());
    }

    let result =
        parse_matpower(case_path.to_str().unwrap()).expect("Failed to parse MATPOWER file");
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
                gen.name, bus_name, gen.pmin, gen.pmax, gen.qmin, gen.qmax
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
                println!(
                    "  {} (idx {}): {} generators",
                    problem.buses[*bus_idx].name, bus_idx, count
                );
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
            println!(
                "  Objective:  ${:.2}/hr (reference: ${:.2})",
                solution.objective_value, reference_obj
            );
            println!(
                "  Obj gap:    {:.2}%",
                (solution.objective_value - reference_obj) / reference_obj * 100.0
            );
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
                solution.objective_value,
                reference_obj,
                obj_gap * 100.0
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
            println!(
                "  Objective:  ${:.2}/hr (reference: ${:.2})",
                solution.objective_value, reference_obj
            );
            println!(
                "  Obj gap:    {:.2}%",
                (solution.objective_value - reference_obj) / reference_obj * 100.0
            );
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
                solution.objective_value,
                reference_obj,
                obj_gap * 100.0
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

    for case_name in &["pglib_opf_case14_ieee", "pglib_opf_case118_ieee"] {
        let network = load_pglib_case(case_name);
        let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

        println!("\n=== {} ===", case_name);

        let x0 = problem.initial_point();

        // Get constraint values from the problem's equality_constraints method
        let g = problem.equality_constraints(&x0);

        // Manual computation for comparison
        let analysis = analyze_constraints(&problem, &x0);

        println!("Comparing problem.equality_constraints() vs manual computation:");
        println!(
            "{:<8} {:>15} {:>15} {:>10}",
            "Bus", "g (problem)", "manual P", "diff"
        );

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
    println!(
        "  Thermal lim: {}",
        problem.n_thermal_constrained_branches()
    );
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

/// Validate Y-bus construction by printing entries for manual comparison with MATPOWER
#[test]
fn debug_ybus_validation() {
    use gat_algo::opf::ac_nlp::YBus;
    use num_complex::Complex64;

    println!("\n");
    println!("============================================================");
    println!("  Y-BUS VALIDATION TEST");
    println!("============================================================");

    // Use case14 for simpler validation
    let network = load_pglib_case("pglib_opf_case14_ieee");
    let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    println!("\nY-bus dimensions: {}x{}", problem.n_bus, problem.n_bus);

    // Print diagonal entries (self-admittances)
    println!("\n=== DIAGONAL ENTRIES (Self-Admittances) ===");
    println!("{:<5} {:>12} {:>12}  (G_ii + jB_ii)", "Bus", "G_ii", "B_ii");
    for i in 0..problem.n_bus.min(14) {
        let y = problem.ybus.get(i, i);
        println!("{:<5} {:>12.6} {:>12.6}", i, y.re, y.im);
    }

    // Print some off-diagonal entries
    println!("\n=== OFF-DIAGONAL ENTRIES (First 20) ===");
    println!(
        "{:<5} {:<5} {:>12} {:>12}  (G_ij + jB_ij)",
        "From", "To", "G_ij", "B_ij"
    );
    let mut count = 0;
    for i in 0..problem.n_bus.min(14) {
        for j in 0..problem.n_bus.min(14) {
            if i != j {
                let y = problem.ybus.get(i, j);
                if y.re.abs() > 1e-10 || y.im.abs() > 1e-10 {
                    println!("{:<5} {:<5} {:>12.6} {:>12.6}", i, j, y.re, y.im);
                    count += 1;
                    if count >= 20 {
                        break;
                    }
                }
            }
        }
        if count >= 20 {
            break;
        }
    }

    // Check some specific bus data
    println!("\n=== BUS DATA ===");
    println!(
        "{:<5} {:<15} {:>8} {:>8} {:>8} {:>8}",
        "Idx", "Name", "P_load", "Q_load", "gs_pu", "bs_pu"
    );
    for (i, bus) in problem.buses.iter().enumerate().take(14) {
        println!(
            "{:<5} {:<15} {:>8.2} {:>8.2} {:>8.4} {:>8.4}",
            i, bus.name, bus.p_load, bus.q_load, bus.gs_pu, bus.bs_pu
        );
    }

    // Compute total Y-bus row sums (should be small for valid Y-bus)
    println!("\n=== Y-BUS ROW SUMS (should be ~shunt admittance) ===");
    for i in 0..problem.n_bus.min(14) {
        let mut sum = Complex64::new(0.0, 0.0);
        for j in 0..problem.n_bus {
            sum += problem.ybus.get(i, j);
        }
        if sum.norm() > 1e-6 {
            println!("Bus {}: row sum = {:.6} + j{:.6}", i, sum.re, sum.im);
        }
    }
}

/// Finite-difference Jacobian verification test.
/// This is critical for debugging IPOPT convergence issues.
#[cfg(feature = "solver-ipopt")]
#[test]
fn test_jacobian_finite_difference() {
    use gat_algo::opf::ac_nlp::{jacobian_sparsity, jacobian_values};

    println!("\n");
    println!("============================================================");
    println!("  JACOBIAN FINITE DIFFERENCE VERIFICATION");
    println!("============================================================");

    for case_name in &["pglib_opf_case14_ieee", "pglib_opf_case118_ieee"] {
        println!("\n=== {} ===", case_name);

        let network = load_pglib_case(case_name);
        let problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

        // Get initial point
        let x0 = problem.initial_point();
        let n_var = x0.len();

        // Get sparsity pattern
        let (rows, cols) = jacobian_sparsity(&problem);
        let nnz = rows.len();
        println!(
            "Jacobian size: {} constraints x {} variables",
            2 * problem.n_bus + 1,
            n_var
        );
        println!("Non-zeros: {}", nnz);

        // Compute analytical Jacobian
        let jac_analytical = jacobian_values(&problem, &x0);

        // Compute numerical Jacobian via finite differences
        let eps = 1e-7;
        let g0 = problem.equality_constraints(&x0);
        let n_con = g0.len();

        // Build dense numerical Jacobian for comparison
        let mut jac_numerical_map: std::collections::HashMap<(usize, usize), f64> =
            std::collections::HashMap::new();

        for j in 0..n_var {
            let mut x_plus = x0.clone();
            x_plus[j] += eps;
            let g_plus = problem.equality_constraints(&x_plus);

            for i in 0..n_con {
                let deriv = (g_plus[i] - g0[i]) / eps;
                if deriv.abs() > 1e-12 {
                    jac_numerical_map.insert((i, j), deriv);
                }
            }
        }

        // Compare analytical vs numerical for each sparsity entry
        let mut max_abs_error = 0.0f64;
        let mut max_rel_error = 0.0f64;
        let mut worst_entry = (0usize, 0usize, 0.0f64, 0.0f64);
        let mut n_large_errors = 0;

        for (k, (&row, &col)) in rows.iter().zip(cols.iter()).enumerate() {
            if row >= n_con {
                continue; // Skip thermal constraints for now
            }

            let analytical = jac_analytical[k];
            let numerical = *jac_numerical_map.get(&(row, col)).unwrap_or(&0.0);

            let abs_error = (analytical - numerical).abs();
            let rel_error = if numerical.abs() > 1e-8 {
                abs_error / numerical.abs()
            } else if analytical.abs() > 1e-8 {
                abs_error / analytical.abs()
            } else {
                0.0
            };

            if abs_error > max_abs_error {
                max_abs_error = abs_error;
                worst_entry = (row, col, analytical, numerical);
            }
            if rel_error > max_rel_error && abs_error > 1e-6 {
                max_rel_error = rel_error;
            }

            // Count large errors
            if abs_error > 1e-4 && rel_error > 0.01 {
                n_large_errors += 1;
                if n_large_errors <= 10 {
                    println!(
                        "  Error at ({}, {}): analytical={:.6e}, numerical={:.6e}, abs_err={:.2e}, rel_err={:.2e}",
                        row, col, analytical, numerical, abs_error, rel_error
                    );
                }
            }
        }

        println!("\nJacobian verification summary:");
        println!("  Max absolute error: {:.2e}", max_abs_error);
        println!("  Max relative error: {:.2e}", max_rel_error);
        println!(
            "  Entries with >1e-4 abs error and >1% rel error: {}",
            n_large_errors
        );
        println!(
            "  Worst entry: row={}, col={}, analytical={:.6e}, numerical={:.6e}",
            worst_entry.0, worst_entry.1, worst_entry.2, worst_entry.3
        );

        // The Jacobian should match to at least 1e-4 absolute tolerance
        assert!(
            max_abs_error < 1e-3,
            "Jacobian error too large for {}: max_abs_error={:.2e}",
            case_name,
            max_abs_error
        );
    }
}

/// Test disabling thermal constraints to see if power balance alone converges.
#[cfg(feature = "solver-ipopt")]
#[test]
fn test_case118_without_thermal_constraints() {
    use gat_algo::opf::ac_nlp::solve_with_ipopt;

    println!("\n");
    println!("============================================================");
    println!("  CASE118 WITHOUT THERMAL CONSTRAINTS (IPOPT)");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case118_ieee");
    let mut problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    // Disable thermal constraints by setting all rate_mva to 0
    for branch in &mut problem.branches {
        branch.rate_mva = 0.0;
    }

    println!("\nProblem dimensions:");
    println!("  Buses:       {}", problem.n_bus);
    println!("  Generators:  {}", problem.n_gen);
    println!("  Branches:    {}", problem.n_branch);
    println!(
        "  Thermal lim: {} (disabled)",
        problem.n_thermal_constrained_branches()
    );
    println!("  Variables:   {}", problem.n_var);

    // Set IPOPT_PRINT_LEVEL=5 for verbose output
    std::env::set_var("IPOPT_PRINT_LEVEL", "5");

    match solve_with_ipopt(&problem, Some(1000), Some(1e-6)) {
        Ok(solution) => {
            println!("\nIPOPT CONVERGED (without thermal constraints)!");
            println!("  Objective:  ${:.2}/hr", solution.objective_value);
            println!("  Iterations: {}", solution.iterations);

            // Reference objective from MATPOWER: $97,214/hr
            let reference_obj = 97214.0;
            let obj_gap = (solution.objective_value - reference_obj).abs() / reference_obj;
            println!("  Obj gap:    {:.2}%", obj_gap * 100.0);
        }
        Err(e) => {
            println!("\nIPOPT FAILED (without thermal constraints): {:?}", e);
            println!(
                "This indicates the issue is in power balance constraints, not thermal limits."
            );
        }
    }
}

/// Diagnose which thermal constraints are violated at the unconstrained solution.
#[cfg(feature = "solver-ipopt")]
#[test]
fn diagnose_thermal_violations() {
    use gat_algo::opf::ac_nlp::solve_with_ipopt;

    println!("\n");
    println!("============================================================");
    println!("  THERMAL CONSTRAINT VIOLATION DIAGNOSIS");
    println!("============================================================");

    let network = load_pglib_case("pglib_opf_case118_ieee");
    let mut problem = AcOpfProblem::from_network(&network).expect("Failed to build AC-OPF problem");

    // First solve without thermal constraints
    let original_rates: Vec<f64> = problem.branches.iter().map(|b| b.rate_mva).collect();
    for branch in &mut problem.branches {
        branch.rate_mva = 0.0;
    }

    let solution =
        solve_with_ipopt(&problem, Some(500), Some(1e-6)).expect("Should converge without thermal");

    println!(
        "\nUnconstrained solution: ${:.2}/hr",
        solution.objective_value
    );

    // Restore thermal limits and evaluate constraint violations
    for (i, branch) in problem.branches.iter_mut().enumerate() {
        branch.rate_mva = original_rates[i];
    }

    // Build solution vector to evaluate thermal constraints
    let mut x = problem.initial_point();

    // Debug: print some solution values
    println!("\nSolution check:");
    println!(
        "  bus_voltage_mag entries: {}",
        solution.bus_voltage_mag.len()
    );
    println!(
        "  bus_voltage_ang entries: {}",
        solution.bus_voltage_ang.len()
    );
    if let Some(sample_bus) = problem.buses.first() {
        println!("  Sample bus name: {}", sample_bus.name);
        println!(
            "  Sample V from solution: {:?}",
            solution.bus_voltage_mag.get(&sample_bus.name)
        );
    }

    // Copy voltages from solution
    // NOTE: bus_voltage_ang is in DEGREES (see ipopt_solver.rs line 309)
    let mut n_v_found = 0;
    let mut n_theta_found = 0;
    for (i, bus) in problem.buses.iter().enumerate() {
        if let Some(&v) = solution.bus_voltage_mag.get(&bus.name) {
            x[problem.v_offset + i] = v;
            n_v_found += 1;
        }
        if let Some(&theta_deg) = solution.bus_voltage_ang.get(&bus.name) {
            x[problem.theta_offset + i] = theta_deg.to_radians(); // Convert to radians!
            n_theta_found += 1;
        }
    }
    println!("  Matched V: {}/{}", n_v_found, problem.n_bus);
    println!("  Matched θ: {}/{}", n_theta_found, problem.n_bus);

    // Copy generator dispatch from solution
    for (i, gen) in problem.generators.iter().enumerate() {
        if let Some(&p) = solution.generator_p.get(&gen.name) {
            x[problem.pg_offset + i] = p / problem.base_mva;
        }
        if let Some(&q) = solution.generator_q.get(&gen.name) {
            x[problem.qg_offset + i] = q / problem.base_mva;
        }
    }

    // Debug: check voltage values in x
    println!("\nVoltage values in x:");
    for i in 0..5.min(problem.n_bus) {
        println!(
            "  Bus {} ({}): V={:.4}, θ={:.4}",
            i,
            &problem.buses[i].name,
            x[problem.v_offset + i],
            x[problem.theta_offset + i]
        );
    }

    // Evaluate thermal constraints
    let thermal = problem.thermal_constraints(&x);

    println!("\nThermal constraint violations (positive = violated):");
    println!(
        "{:<8} {:<8} {:<8} {:>12} {:>12} {:>10}",
        "Branch", "From", "To", "S_flow (MVA)", "S_max (MVA)", "Violation"
    );
    println!("{}", "-".repeat(70));

    let mut n_violated = 0;
    let mut max_violation = 0.0f64;
    let mut branch_idx = 0;

    for (i, branch) in problem.branches.iter().enumerate() {
        if branch.rate_mva <= 0.0 {
            continue;
        }

        let h_from = thermal[branch_idx * 2];
        let h_to = thermal[branch_idx * 2 + 1];

        let s_max = branch.rate_mva;
        let s_max_pu = s_max / problem.base_mva;

        // h = S² - S²_max, so S = sqrt(h + S²_max)
        let s_from_sq = h_from + s_max_pu * s_max_pu;
        let s_to_sq = h_to + s_max_pu * s_max_pu;
        let s_from = s_from_sq.max(0.0).sqrt() * problem.base_mva;
        let s_to = s_to_sq.max(0.0).sqrt() * problem.base_mva;

        let violation_from = (s_from - s_max).max(0.0);
        let violation_to = (s_to - s_max).max(0.0);

        if violation_from > 0.1 || violation_to > 0.1 {
            n_violated += 1;
            max_violation = max_violation.max(violation_from).max(violation_to);
            println!(
                "{:<8} {:<8} {:<8} {:>12.2} {:>12.2} {:>10.2}",
                i,
                problem.buses[branch.from_idx]
                    .name
                    .split('_')
                    .last()
                    .unwrap_or("?"),
                problem.buses[branch.to_idx]
                    .name
                    .split('_')
                    .last()
                    .unwrap_or("?"),
                s_from.max(s_to),
                s_max,
                violation_from.max(violation_to)
            );
        }

        branch_idx += 1;
    }

    println!("\n{} branches with violations > 0.1 MVA", n_violated);
    println!("Max violation: {:.2} MVA", max_violation);

    if n_violated == 0 {
        println!("\nNo significant thermal violations at unconstrained solution.");
        println!("This suggests the constrained problem should be feasible.");
    } else {
        println!("\nThermal violations exist - constraints are binding.");
        println!("IPOPT should be able to find a feasible solution by backing off generation.");
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
    println!(
        "  Thermal lim: {}",
        problem.n_thermal_constrained_branches()
    );
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
