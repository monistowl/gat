//! IPOPT solver wrapper binary for GAT native solver plugin system.
//!
//! This binary implements the GAT solver IPC protocol:
//! 1. Reads an Arrow IPC stream from stdin containing the optimization problem
//! 2. Solves using IPOPT (Interior Point OPTimizer)
//! 3. Writes an Arrow IPC stream to stdout containing the solution
//!
//! # IPOPT Algorithm
//!
//! IPOPT implements a primal-dual interior-point algorithm with a filter
//! line-search method for nonlinear programming. Key features:
//!
//! - **Barrier method:** Converts inequality constraints to logarithmic barriers
//! - **Newton system:** Solves KKT conditions using sparse symmetric indefinite factorization
//! - **Filter line-search:** Accepts steps that improve objective OR constraint violation
//! - **Restoration phase:** Recovers feasibility when standard steps fail
//!
//! For AC-OPF, the KKT system has the following structure:
//! ```text
//! [H + Σ   Aᵀ ] [Δx] = [-∇f - Aᵀλ]
//! [A       0  ] [Δλ]   [-c(x)     ]
//! ```
//! where H is the Hessian of the Lagrangian and A is the Jacobian of constraints.
//!
//! **Reference:** Wächter, A., & Biegler, L. T. (2006). On the implementation of
//! an interior-point filter line-search algorithm for large-scale nonlinear
//! programming. *Mathematical Programming*, 106(1), 25-57.
//! **DOI:** [10.1007/s10107-004-0559-y](https://doi.org/10.1007/s10107-004-0559-y)
//!
//! # Protocol
//!
//! The binary expects:
//! - Input: Arrow IPC stream with problem data (buses, generators, branches)
//! - Output: Arrow IPC stream with solution (voltages, power outputs, flows)
//!
//! Exit codes are defined in `gat_solver_common::ExitCode`.
//!
//! # Building
//!
//! Requires IPOPT to be installed on the system:
//! - Ubuntu/Debian: `sudo apt install coinor-libipopt-dev`
//! - macOS: `brew install ipopt`
//! - From source: https://coin-or.github.io/Ipopt/INSTALL.html
//!
//! Build with: `cargo build -p gat-ipopt --features ipopt-sys --release`
//!
//! # Performance Notes
//!
//! IPOPT performance depends heavily on the linear solver used:
//! - **MA27/MA57** (HSL): Best for small-medium problems
//! - **MUMPS**: Good open-source option, parallel capable
//! - **MA86/MA97** (HSL): Best for large problems with parallelism
//!
//! For AC-OPF on networks with > 10,000 buses, consider using MA86.

use anyhow::{Context, Result};
use gat_solver_common::{ExitCode, ProblemBatch, SolutionBatch, SolutionStatus, PROTOCOL_VERSION};
use std::io::{self, Read, Write};
use tracing::{debug, error, info};

fn main() {
    // Initialize tracing (respects RUST_LOG env var)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(io::stderr)
        .init();

    info!(
        "gat-ipopt v{} (protocol v{})",
        env!("CARGO_PKG_VERSION"),
        PROTOCOL_VERSION
    );

    let exit_code = match run() {
        Ok(()) => ExitCode::Success,
        Err(e) => {
            error!("Solver error: {:?}", e);
            ExitCode::SolverError
        }
    };

    std::process::exit(exit_code as i32);
}

fn run() -> Result<()> {
    // Read problem from stdin
    debug!("Reading problem from stdin...");
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .context("Failed to read problem from stdin")?;

    if input.is_empty() {
        anyhow::bail!("Empty input - no problem data received");
    }

    debug!("Received {} bytes of problem data", input.len());

    // Parse the Arrow IPC problem
    let problem = gat_solver_common::ipc::read_problem(input.as_slice())
        .context("Failed to parse Arrow IPC problem")?;

    info!(
        "Problem: {} buses, {} generators, {} branches",
        problem.bus_id.len(),
        problem.gen_id.len(),
        problem.branch_id.len()
    );

    // Solve the problem
    let solution = solve_with_ipopt(&problem)?;

    // Write solution to stdout
    debug!("Writing solution to stdout...");
    let mut output = Vec::new();
    gat_solver_common::ipc::write_solution(&solution, &mut output)
        .context("Failed to serialize solution")?;

    io::stdout()
        .write_all(&output)
        .context("Failed to write solution to stdout")?;

    info!(
        "Solution written: status={:?}, objective={:.6}",
        solution.status, solution.objective
    );

    Ok(())
}

/// Solve the optimization problem using IPOPT.
///
/// Converts ProblemBatch to AcOpfProblem, solves using IPOPT, and converts
/// the solution back to SolutionBatch format.
#[cfg(feature = "ipopt-sys")]
fn solve_with_ipopt(problem: &ProblemBatch) -> Result<SolutionBatch> {
    use gat_algo::opf::ac_nlp::{
        compute_branch_flows, solve_with_ipopt as ipopt_solve, AcOpfProblem, BranchData, BusData,
        GenData, YBusBuilder,
    };
    use gat_core::{Branch, BusId, CostModel, Edge, Network, Node};
    use std::collections::HashMap;
    use std::time::Instant;

    let start_time = Instant::now();

    info!("Converting ProblemBatch to AcOpfProblem...");

    // ========================================================================
    // BUILD BUS ID MAPPING (external ID -> internal index)
    // ========================================================================
    let bus_map: HashMap<i64, usize> = problem
        .bus_id
        .iter()
        .enumerate()
        .map(|(idx, &id)| (id, idx))
        .collect();

    // ========================================================================
    // BUILD Y-BUS MATRIX FROM BRANCH DATA
    // ========================================================================
    // We build a temporary Network structure and use YBusBuilder to construct
    // the Y-bus. This reuses the existing code and avoids duplication.
    let n_bus = problem.bus_id.len();
    let mut temp_network = Network::new();

    // Add buses to temporary network
    for i in 0..n_bus {
        temp_network.graph.add_node(Node::Bus(gat_core::Bus {
            id: BusId::new(problem.bus_id[i] as usize),
            name: if i < problem.bus_name.len() {
                problem.bus_name[i].clone()
            } else {
                format!("Bus{}", problem.bus_id[i])
            },
            voltage_kv: 138.0, // Default, not used for Y-bus construction
            voltage_pu: 1.0,
            angle_rad: 0.0,
            vmin_pu: Some(problem.bus_v_min[i]),
            vmax_pu: Some(problem.bus_v_max[i]),
            area_id: None,
            zone_id: None,
        }));
    }

    // Add branches to temporary network
    for i in 0..problem.branch_id.len() {
        if problem.branch_status[i] == 0 {
            continue; // Skip offline branches
        }

        let from_bus_id = BusId::new(problem.branch_from[i] as usize);
        let to_bus_id = BusId::new(problem.branch_to[i] as usize);

        // Find node indices for from and to buses
        let from_idx = temp_network
            .graph
            .node_indices()
            .find(|&idx| {
                if let Node::Bus(bus) = &temp_network.graph[idx] {
                    bus.id == from_bus_id
                } else {
                    false
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Branch {} references unknown from bus {}",
                    i,
                    problem.branch_from[i]
                )
            })?;

        let to_idx = temp_network
            .graph
            .node_indices()
            .find(|&idx| {
                if let Node::Bus(bus) = &temp_network.graph[idx] {
                    bus.id == to_bus_id
                } else {
                    false
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!("Branch {} references unknown to bus {}", i, problem.branch_to[i])
            })?;

        temp_network.graph.add_edge(
            from_idx,
            to_idx,
            Edge::Branch(Branch {
                id: gat_core::BranchId::new(i),
                name: format!("Branch{}", problem.branch_id[i]),
                from_bus: from_bus_id,
                to_bus: to_bus_id,
                resistance: problem.branch_r[i],
                reactance: problem.branch_x[i],
                tap_ratio: problem.branch_tap[i],
                phase_shift_rad: problem.branch_shift[i],
                charging_b_pu: problem.branch_b[i],
                s_max_mva: Some(problem.branch_rate[i]),
                rating_a_mva: Some(problem.branch_rate[i]),
                rating_b_mva: None,
                rating_c_mva: None,
                status: true,
                angle_min_rad: None,
                angle_max_rad: None,
                element_type: "line".to_string(),
                is_phase_shifter: false,
            }),
        );
    }

    // Build Y-bus using YBusBuilder
    let ybus = YBusBuilder::from_network(&temp_network)
        .map_err(|e| anyhow::anyhow!("Failed to build Y-bus: {:?}", e))?;

    debug!("Y-bus built for {} buses", ybus.n_bus());

    // ========================================================================
    // BUILD BUS DATA
    // ========================================================================
    let buses: Vec<BusData> = (0..n_bus)
        .map(|i| BusData {
            id: BusId::new(problem.bus_id[i] as usize),
            name: if i < problem.bus_name.len() {
                problem.bus_name[i].clone()
            } else {
                format!("Bus{}", problem.bus_id[i])
            },
            index: i,
            v_min: problem.bus_v_min[i],
            v_max: problem.bus_v_max[i],
            p_load: problem.bus_p_load[i],
            q_load: problem.bus_q_load[i],
        })
        .collect();

    // ========================================================================
    // BUILD GENERATOR DATA
    // ========================================================================
    let n_gen = problem.gen_id.len();
    let mut generators = Vec::with_capacity(n_gen);

    for i in 0..n_gen {
        if problem.gen_status[i] == 0 {
            continue; // Skip offline generators
        }

        let _bus_idx = *bus_map.get(&problem.gen_bus_id[i]).ok_or_else(|| {
            anyhow::anyhow!("Generator {} references unknown bus {}", i, problem.gen_bus_id[i])
        })?;

        // Build cost model from coefficients
        let c0 = problem.gen_cost_c0[i];
        let c1 = problem.gen_cost_c1[i];
        let c2 = problem.gen_cost_c2[i];
        let cost_coeffs = vec![c0, c1, c2];
        let cost_model = CostModel::Polynomial(cost_coeffs.clone());

        generators.push(GenData {
            name: format!("Gen{}", problem.gen_id[i]),
            bus_id: BusId::new(problem.gen_bus_id[i] as usize),
            pmin_mw: problem.gen_p_min[i],
            pmax_mw: problem.gen_p_max[i],
            qmin_mvar: problem.gen_q_min[i],
            qmax_mvar: problem.gen_q_max[i],
            cost_coeffs,
            cost_model,
            capability_curve: Vec::new(),
        });
    }

    if generators.is_empty() {
        anyhow::bail!("No online generators in problem");
    }

    // Build generator-to-bus index mapping
    let gen_bus_idx: Vec<usize> = generators
        .iter()
        .map(|g| *bus_map.get(&(g.bus_id.value() as i64)).unwrap_or(&0))
        .collect();

    // ========================================================================
    // BUILD BRANCH DATA FOR FLOW CALCULATIONS
    // ========================================================================
    let mut branches = Vec::with_capacity(problem.branch_id.len());
    for i in 0..problem.branch_id.len() {
        if problem.branch_status[i] == 0 {
            continue;
        }

        let from_idx = *bus_map.get(&problem.branch_from[i]).unwrap();
        let to_idx = *bus_map.get(&problem.branch_to[i]).unwrap();

        branches.push(BranchData {
            name: format!("Branch{}", problem.branch_id[i]),
            from_idx,
            to_idx,
            r: problem.branch_r[i],
            x: problem.branch_x[i],
            b_charging: problem.branch_b[i],
            tap: problem.branch_tap[i],
            shift: problem.branch_shift[i],
            rate_mva: problem.branch_rate[i],
            angle_diff_max: 0.0,
        });
    }

    let n_branch = branches.len();

    // ========================================================================
    // CONSTRUCT AC-OPF PROBLEM
    // ========================================================================
    let n_gen = generators.len();
    let n_var = 2 * n_bus + 2 * n_gen;
    let ac_problem = AcOpfProblem {
        ybus,
        buses,
        generators,
        ref_bus: 0,
        base_mva: problem.base_mva,
        n_bus,
        n_gen,
        n_var,
        v_offset: 0,
        theta_offset: n_bus,
        pg_offset: 2 * n_bus,
        qg_offset: 2 * n_bus + n_gen,
        gen_bus_idx,
        branches,
        n_branch,
    };

    info!(
        "AC-OPF problem: {} buses, {} generators, {} branches, {} variables",
        ac_problem.n_bus, ac_problem.n_gen, ac_problem.n_branch, ac_problem.n_var
    );

    // ========================================================================
    // SOLVE WITH IPOPT
    // ========================================================================
    info!("Calling IPOPT solver...");
    let max_iter = Some(problem.max_iterations as usize);
    let tol = Some(problem.tolerance);

    let opf_solution = match ipopt_solve(&ac_problem, max_iter, tol) {
        Ok(sol) => sol,
        Err(e) => {
            error!("IPOPT solver failed: {:?}", e);
            return Ok(SolutionBatch {
                status: SolutionStatus::Error,
                objective: f64::NAN,
                iterations: 0,
                solve_time_ms: start_time.elapsed().as_millis() as i64,
                error_message: Some(format!("IPOPT solver error: {}", e)),
                bus_id: problem.bus_id.clone(),
                bus_v_mag: vec![f64::NAN; problem.bus_id.len()],
                bus_v_ang: vec![f64::NAN; problem.bus_id.len()],
                bus_lmp: vec![f64::NAN; problem.bus_id.len()],
                gen_id: problem.gen_id.clone(),
                gen_p: vec![f64::NAN; problem.gen_id.len()],
                gen_q: vec![f64::NAN; problem.gen_id.len()],
                branch_id: problem.branch_id.clone(),
                branch_p_from: vec![f64::NAN; problem.branch_id.len()],
                branch_q_from: vec![f64::NAN; problem.branch_id.len()],
                branch_p_to: vec![f64::NAN; problem.branch_id.len()],
                branch_q_to: vec![f64::NAN; problem.branch_id.len()],
            });
        }
    };

    let solve_time_ms = start_time.elapsed().as_millis() as i64;

    info!(
        "IPOPT converged={}, objective={:.2} $/hr",
        opf_solution.converged, opf_solution.objective_value
    );

    // ========================================================================
    // CONVERT SOLUTION BACK TO SOLUTIONBATCH
    // ========================================================================

    // Extract bus voltages (convert angles from degrees to radians for consistency)
    let mut bus_v_mag = vec![f64::NAN; problem.bus_id.len()];
    let mut bus_v_ang = vec![f64::NAN; problem.bus_id.len()];
    let mut bus_lmp = vec![0.0; problem.bus_id.len()];

    for (i, bus) in ac_problem.buses.iter().enumerate() {
        if let Some(&v_mag) = opf_solution.bus_voltage_mag.get(&bus.name) {
            bus_v_mag[i] = v_mag;
        }
        if let Some(&v_ang_deg) = opf_solution.bus_voltage_ang.get(&bus.name) {
            bus_v_ang[i] = v_ang_deg; // Already in degrees from OpfSolution
        }
        if let Some(&lmp) = opf_solution.bus_lmp.get(&bus.name) {
            bus_lmp[i] = lmp;
        }
    }

    // Extract generator dispatch
    let mut gen_p = vec![f64::NAN; problem.gen_id.len()];
    let mut gen_q = vec![f64::NAN; problem.gen_id.len()];

    for (i, gen) in ac_problem.generators.iter().enumerate() {
        if let Some(&p_mw) = opf_solution.generator_p.get(&gen.name) {
            gen_p[i] = p_mw;
        }
        if let Some(&q_mvar) = opf_solution.generator_q.get(&gen.name) {
            gen_q[i] = q_mvar;
        }
    }

    // Compute branch flows from voltage solution
    let v: Vec<f64> = bus_v_mag.clone();
    let theta: Vec<f64> = bus_v_ang.iter().map(|&ang| ang.to_radians()).collect();
    let branch_flows = compute_branch_flows(&ac_problem, &v, &theta);

    let mut branch_p_from = vec![0.0; problem.branch_id.len()];
    let mut branch_q_from = vec![0.0; problem.branch_id.len()];
    let mut branch_p_to = vec![0.0; problem.branch_id.len()];
    let mut branch_q_to = vec![0.0; problem.branch_id.len()];

    for (i, (pf, qf, pt, qt)) in branch_flows.iter().enumerate() {
        if i < problem.branch_id.len() {
            branch_p_from[i] = *pf;
            branch_q_from[i] = *qf;
            branch_p_to[i] = *pt;
            branch_q_to[i] = *qt;
        }
    }

    Ok(SolutionBatch {
        status: if opf_solution.converged {
            SolutionStatus::Optimal
        } else {
            SolutionStatus::Error
        },
        objective: opf_solution.objective_value,
        iterations: opf_solution.iterations as i32,
        solve_time_ms,
        error_message: None,
        bus_id: problem.bus_id.clone(),
        bus_v_mag,
        bus_v_ang,
        bus_lmp,
        gen_id: problem.gen_id.clone(),
        gen_p,
        gen_q,
        branch_id: problem.branch_id.clone(),
        branch_p_from,
        branch_q_from,
        branch_p_to,
        branch_q_to,
    })
}

/// Stub implementation when IPOPT is not available.
#[cfg(not(feature = "ipopt-sys"))]
fn solve_with_ipopt(problem: &ProblemBatch) -> Result<SolutionBatch> {
    error!("IPOPT is not available - this binary was built without ipopt-sys feature");
    error!("Rebuild with: cargo build -p gat-ipopt --features ipopt-sys --release");

    Ok(SolutionBatch {
        status: SolutionStatus::Error,
        objective: f64::NAN,
        iterations: 0,
        solve_time_ms: 0,
        error_message: Some("IPOPT not available - rebuild with ipopt-sys feature".to_string()),
        bus_id: problem.bus_id.clone(),
        bus_v_mag: vec![f64::NAN; problem.bus_id.len()],
        bus_v_ang: vec![f64::NAN; problem.bus_id.len()],
        bus_lmp: vec![f64::NAN; problem.bus_id.len()],
        gen_id: problem.gen_id.clone(),
        gen_p: vec![f64::NAN; problem.gen_id.len()],
        gen_q: vec![f64::NAN; problem.gen_id.len()],
        branch_id: problem.branch_id.clone(),
        branch_p_from: vec![f64::NAN; problem.branch_id.len()],
        branch_q_from: vec![f64::NAN; problem.branch_id.len()],
        branch_p_to: vec![f64::NAN; problem.branch_id.len()],
        branch_q_to: vec![f64::NAN; problem.branch_id.len()],
    })
}
