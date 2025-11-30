//! CLP LP solver wrapper binary for GAT native solver plugin system.
//!
//! This binary implements the GAT solver IPC v2 protocol:
//! 1. Reads length-prefixed Arrow IPC streams from stdin containing the optimization problem
//! 2. Solves using CLP (COIN-OR Linear Programming)
//! 3. Writes length-prefixed Arrow IPC streams to stdout containing the solution
//!
//! # CLP Algorithm
//!
//! CLP implements the dual revised simplex method, which is efficient for
//! large sparse LP problems. Key features:
//!
//! - **Dual simplex:** Maintains dual feasibility, iterates toward primal feasibility
//! - **Sparse linear algebra:** Uses efficient sparse matrix factorization
//! - **Presolve:** Reduces problem size before solving
//! - **Multiple pricing:** Partial pricing for very large problems
//!
//! For power systems, CLP is well-suited for DC-OPF problems where:
//! - Objective: Minimize generation cost
//! - Variables: Generator outputs, bus angles, branch flows
//! - Constraints: Power balance, line limits, generator limits
//!
//! **Reference:** Forrest, J., & Lougee-Heimer, R. (2005). CBC User's Guide.
//! In *Emerging Theory, Methods, and Applications* (pp. 257-277). INFORMS.
//! **DOI:** [10.1287/educ.1053.0020](https://doi.org/10.1287/educ.1053.0020)
//!
//! # Protocol
//!
//! The binary expects:
//! - Input: Arrow IPC stream with problem data (buses, generators, branches)
//! - Output: Arrow IPC stream with solution (voltages, power outputs, flows)
//!
//! Exit codes are defined in `gat_solver_common::ExitCode`.

use anyhow::{Context, Result};
use gat_solver_common::{ExitCode, ProblemBatch, SolutionBatch, SolutionStatus, PROTOCOL_VERSION};
use std::io::{self, Read, Write};
use tracing::{debug, error, info, warn};

// FFI bindings to CLP C interface
mod clp_ffi {
    use std::os::raw::{c_char, c_double, c_int};

    #[repr(C)]
    pub struct Clp_Simplex {
        _private: [u8; 0],
    }

    extern "C" {
        // Model creation/destruction
        pub fn Clp_newModel() -> *mut Clp_Simplex;
        pub fn Clp_deleteModel(model: *mut Clp_Simplex);

        // Problem loading
        pub fn Clp_loadProblem(
            model: *mut Clp_Simplex,
            num_cols: c_int,
            num_rows: c_int,
            start: *const c_int,      // Column starts (CCS format)
            index: *const c_int,      // Row indices
            value: *const c_double,   // Non-zero values
            col_lb: *const c_double,  // Column lower bounds
            col_ub: *const c_double,  // Column upper bounds
            obj: *const c_double,     // Objective coefficients
            row_lb: *const c_double,  // Row lower bounds
            row_ub: *const c_double,  // Row upper bounds
        );

        // Solving
        pub fn Clp_initialSolve(model: *mut Clp_Simplex) -> c_int;
        pub fn Clp_dual(model: *mut Clp_Simplex) -> c_int;
        pub fn Clp_primal(model: *mut Clp_Simplex, ifValuesPass: c_int) -> c_int;

        // Solution access
        pub fn Clp_status(model: *mut Clp_Simplex) -> c_int;
        pub fn Clp_objectiveValue(model: *mut Clp_Simplex) -> c_double;
        pub fn Clp_primalColumnSolution(model: *mut Clp_Simplex) -> *const c_double;
        pub fn Clp_dualColumnSolution(model: *mut Clp_Simplex) -> *const c_double;
        pub fn Clp_dualRowSolution(model: *mut Clp_Simplex) -> *const c_double;
        pub fn Clp_numberIterations(model: *mut Clp_Simplex) -> c_int;

        // Model info
        pub fn Clp_numberRows(model: *mut Clp_Simplex) -> c_int;
        pub fn Clp_numberColumns(model: *mut Clp_Simplex) -> c_int;

        // Options
        pub fn Clp_setLogLevel(model: *mut Clp_Simplex, level: c_int);
        pub fn Clp_setOptimizationDirection(model: *mut Clp_Simplex, direction: c_double);
        pub fn Clp_scaling(model: *mut Clp_Simplex, mode: c_int);
    }
}

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
        "gat-clp v{} (protocol v{})",
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

    // Parse the Arrow IPC problem (v2 multi-batch protocol)
    let problem = gat_solver_common::ipc::read_problem_v2(input.as_slice())
        .context("Failed to parse Arrow IPC v2 problem")?;

    info!(
        "Problem: {} buses, {} generators, {} branches",
        problem.bus_id.len(),
        problem.gen_id.len(),
        problem.branch_id.len()
    );

    // Solve the problem
    let solution = solve_with_clp(&problem)?;

    // Write solution to stdout (v2 multi-batch protocol)
    debug!("Writing solution to stdout...");
    let mut output = Vec::new();
    gat_solver_common::ipc::write_solution_v2(&solution, &mut output)
        .context("Failed to serialize solution v2")?;

    io::stdout()
        .write_all(&output)
        .context("Failed to write solution to stdout")?;

    info!(
        "Solution written: status={:?}, objective={:.6}",
        solution.status, solution.objective
    );

    Ok(())
}

/// Solve the DC-OPF problem using CLP.
///
/// This formulates a DC-OPF as a linear program:
/// - Variables: Generator outputs P_g, bus angles θ
/// - Objective: Minimize Σ c_g * P_g (linear cost)
/// - Constraints:
///   - Power balance at each bus: Σ P_g(bus) - P_load(bus) = Σ B'[i,j] * θ[j]
///   - Generator limits: P_min ≤ P_g ≤ P_max
///   - Line flow limits: -S_max ≤ b_ij * (θ_i - θ_j) ≤ S_max
///
/// # Variable Layout (columns)
///
/// - Columns 0..n_gen: Generator power outputs P_g
/// - Columns n_gen..n_gen+(n_bus-1): Bus angles θ (excluding reference bus θ_ref=0)
///
/// # Constraint Layout (rows)
///
/// - Rows 0..n_bus: Power balance constraints (equality)
/// - Rows n_bus..n_bus+n_branch: Line flow upper limits
/// - Rows n_bus+n_branch..n_bus+2*n_branch: Line flow lower limits (as -flow <= S_max)
fn solve_with_clp(problem: &ProblemBatch) -> Result<SolutionBatch> {
    let start_time = std::time::Instant::now();

    let n_bus = problem.bus_id.len();
    let n_gen = problem.gen_id.len();
    let n_branch = problem.branch_id.len();

    // Handle empty problem
    if n_gen == 0 || n_bus == 0 {
        return Ok(SolutionBatch {
            status: SolutionStatus::Optimal,
            objective: 0.0,
            iterations: 0,
            solve_time_ms: start_time.elapsed().as_millis() as i64,
            error_message: None,
            bus_id: problem.bus_id.clone(),
            bus_v_mag: vec![1.0; n_bus],
            bus_v_ang: vec![0.0; n_bus],
            bus_lmp: vec![0.0; n_bus],
            gen_id: problem.gen_id.clone(),
            gen_p: vec![],
            gen_q: vec![],
            branch_id: problem.branch_id.clone(),
            branch_p_from: vec![0.0; n_branch],
            branch_q_from: vec![0.0; n_branch],
            branch_p_to: vec![0.0; n_branch],
            branch_q_to: vec![0.0; n_branch],
        });
    }

    // Build bus ID to index mapping
    let bus_id_to_idx: std::collections::HashMap<i64, usize> = problem
        .bus_id
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    // Reference bus is the first bus (index 0), its angle is fixed at 0
    let ref_bus_idx = 0usize;

    // === Build B' susceptance matrix entries ===
    // B'[i,j] = -b_ij (off-diagonal), B'[i,i] = Σ b_ik (diagonal)
    // b_ij = 1 / x_ij (susceptance)
    let mut b_prime: std::collections::HashMap<(usize, usize), f64> =
        std::collections::HashMap::new();

    for k in 0..n_branch {
        let from_bus = problem.branch_from[k];
        let to_bus = problem.branch_to[k];

        let i = *bus_id_to_idx.get(&from_bus).unwrap_or(&0);
        let j = *bus_id_to_idx.get(&to_bus).unwrap_or(&0);

        let x = problem.branch_x[k];
        if x.abs() < 1e-12 {
            warn!("Branch {} has near-zero reactance, skipping", problem.branch_id[k]);
            continue;
        }
        let b = 1.0 / x; // susceptance

        // Off-diagonal: B'[i,j] = B'[j,i] = -b
        *b_prime.entry((i, j)).or_insert(0.0) -= b;
        *b_prime.entry((j, i)).or_insert(0.0) -= b;

        // Diagonal: B'[i,i] += b, B'[j,j] += b
        *b_prime.entry((i, i)).or_insert(0.0) += b;
        *b_prime.entry((j, j)).or_insert(0.0) += b;
    }

    // Compute load at each bus
    let mut bus_load: Vec<f64> = vec![0.0; n_bus];
    for i in 0..n_bus {
        bus_load[i] = problem.bus_p_load[i];
    }

    // Build generator bus mapping
    let gen_bus_idx: Vec<usize> = problem
        .gen_bus_id
        .iter()
        .map(|&bus_id| *bus_id_to_idx.get(&bus_id).unwrap_or(&0))
        .collect();

    // === Build LP in CCS format ===
    //
    // Variables:
    //   [0..n_gen): P_g (generator outputs)
    //   [n_gen..n_gen+n_bus-1): θ (bus angles, excluding ref bus)
    //
    // Constraints:
    //   [0..n_bus): Power balance at each bus
    //   [n_bus..n_bus+n_branch): Line flow upper limit: b*(θ_i - θ_j) <= S_max
    //   [n_bus+n_branch..n_bus+2*n_branch): Line flow lower limit: -b*(θ_i - θ_j) <= S_max

    let n_theta = n_bus - 1; // All buses except reference
    let num_cols = n_gen + n_theta;
    let num_rows = n_bus + 2 * n_branch;

    // Theta variable index: bus_idx -> column index (None for ref bus)
    let theta_col = |bus_idx: usize| -> Option<usize> {
        if bus_idx == ref_bus_idx {
            None // Reference bus has no theta variable (fixed at 0)
        } else if bus_idx < ref_bus_idx {
            Some(n_gen + bus_idx)
        } else {
            Some(n_gen + bus_idx - 1)
        }
    };

    // Build sparse matrix in COO (coordinate) format first, then convert to CCS
    // Each entry: (row, col, value)
    let mut coo_entries: Vec<(usize, usize, f64)> = Vec::new();

    debug!(
        "DC-OPF setup: n_bus={}, n_gen={}, n_branch={}, n_theta={}, ref_bus={}",
        n_bus, n_gen, n_branch, n_theta, ref_bus_idx
    );
    debug!("Bus loads: {:?}", bus_load);
    debug!("Gen bus indices: {:?}", gen_bus_idx);

    // --- Power balance constraints ---
    // Row i: Σ P_g(bus i) - P_load(i) = Σ_j B'[i,j] * θ[j]
    // Rewritten: Σ P_g(bus i) - Σ_j B'[i,j] * θ[j] = P_load(i)

    // Generator contributions to power balance
    for g in 0..n_gen {
        let bus_idx = gen_bus_idx[g];
        coo_entries.push((bus_idx, g, 1.0)); // +1 * P_g in row bus_idx
    }

    // Theta contributions to power balance (from B' matrix)
    for (&(i, j), &b_ij) in &b_prime {
        if let Some(theta_j_col) = theta_col(j) {
            // Constraint i: -B'[i,j] * θ[j] (we subtract B'*θ from gen)
            coo_entries.push((i, theta_j_col, -b_ij));
        }
        // If j is reference bus, θ[j] = 0, no contribution
    }

    // --- Line flow constraints ---
    // For branch k from bus i to bus j with susceptance b:
    //   Flow = b * (θ_i - θ_j)
    //   Upper: b * θ_i - b * θ_j <= S_max  (row n_bus + k)
    //   Lower: -b * θ_i + b * θ_j <= S_max (row n_bus + n_branch + k)

    for k in 0..n_branch {
        let from_bus = problem.branch_from[k];
        let to_bus = problem.branch_to[k];

        let i = *bus_id_to_idx.get(&from_bus).unwrap_or(&0);
        let j = *bus_id_to_idx.get(&to_bus).unwrap_or(&0);

        let x = problem.branch_x[k];
        if x.abs() < 1e-12 {
            continue;
        }
        let b = 1.0 / x;

        let upper_row = n_bus + k;
        let lower_row = n_bus + n_branch + k;

        // Upper limit: b * θ_i - b * θ_j <= S_max
        if let Some(col_i) = theta_col(i) {
            coo_entries.push((upper_row, col_i, b));
            coo_entries.push((lower_row, col_i, -b));
        }
        if let Some(col_j) = theta_col(j) {
            coo_entries.push((upper_row, col_j, -b));
            coo_entries.push((lower_row, col_j, b));
        }
    }

    // Convert COO to CCS (Compressed Column Storage)
    // Sort by column, then by row
    coo_entries.sort_by_key(|&(row, col, _)| (col, row));

    // Merge duplicates (same row, col)
    let mut merged: Vec<(usize, usize, f64)> = Vec::new();
    for (row, col, val) in coo_entries {
        if let Some(last) = merged.last_mut() {
            if last.0 == row && last.1 == col {
                last.2 += val;
                continue;
            }
        }
        merged.push((row, col, val));
    }

    // Build CCS arrays
    let mut col_start: Vec<i32> = vec![0; num_cols + 1];
    let mut row_index: Vec<i32> = Vec::with_capacity(merged.len());
    let mut values: Vec<f64> = Vec::with_capacity(merged.len());

    let mut current_col = 0;
    for (row, col, val) in &merged {
        // Fill in column starts for empty columns
        while current_col < *col {
            col_start[current_col + 1] = row_index.len() as i32;
            current_col += 1;
        }
        row_index.push(*row as i32);
        values.push(*val);
    }
    // Fill remaining column starts
    while current_col < num_cols {
        col_start[current_col + 1] = row_index.len() as i32;
        current_col += 1;
    }

    debug!("CCS matrix: col_start={:?}", col_start);
    debug!("CCS matrix: row_index={:?}", row_index);
    debug!("CCS matrix: values={:?}", values);

    // === Variable bounds ===
    let mut col_lb: Vec<f64> = Vec::with_capacity(num_cols);
    let mut col_ub: Vec<f64> = Vec::with_capacity(num_cols);

    // Generator bounds
    for g in 0..n_gen {
        let pmin = if g < problem.gen_p_min.len() {
            problem.gen_p_min[g].max(0.0)
        } else {
            0.0
        };
        let pmax = if g < problem.gen_p_max.len() {
            let p = problem.gen_p_max[g];
            if p.is_finite() { p } else { 1e6 }
        } else {
            1e6
        };
        col_lb.push(pmin);
        col_ub.push(pmax);
    }

    // Theta bounds (angles can be any real number, use large bounds)
    for _ in 0..n_theta {
        col_lb.push(-1e6);
        col_ub.push(1e6);
    }

    // === Objective coefficients ===
    let mut obj: Vec<f64> = Vec::with_capacity(num_cols);

    // Generator costs (c1 * P_g)
    for g in 0..n_gen {
        let c1 = if g < problem.gen_cost_c1.len() {
            problem.gen_cost_c1[g]
        } else {
            1.0 // Default cost if not provided
        };
        obj.push(c1);
    }

    // Theta variables have zero cost
    for _ in 0..n_theta {
        obj.push(0.0);
    }

    // === Constraint bounds ===
    let mut row_lb: Vec<f64> = Vec::with_capacity(num_rows);
    let mut row_ub: Vec<f64> = Vec::with_capacity(num_rows);

    // Power balance constraints (equality): row = P_load
    for i in 0..n_bus {
        row_lb.push(bus_load[i]);
        row_ub.push(bus_load[i]);
    }

    // Line flow constraints
    for k in 0..n_branch {
        // Get thermal limit (S_max in MVA, which equals P_max for DC approximation)
        let s_max = if k < problem.branch_rate.len() {
            let rate = problem.branch_rate[k];
            if rate.is_finite() && rate > 0.0 {
                rate
            } else {
                1e6 // No limit
            }
        } else {
            1e6
        };

        // Upper limit row: flow <= S_max
        row_lb.push(-1e20); // No lower bound on this form
        row_ub.push(s_max);

        // Lower limit row: -flow <= S_max
        // (will be added in next loop)
    }

    for k in 0..n_branch {
        let s_max = if k < problem.branch_rate.len() {
            let rate = problem.branch_rate[k];
            if rate.is_finite() && rate > 0.0 {
                rate
            } else {
                1e6
            }
        } else {
            1e6
        };

        row_lb.push(-1e20);
        row_ub.push(s_max);
    }

    debug!("Variable bounds: col_lb={:?}, col_ub={:?}", col_lb, col_ub);
    debug!("Objective coeffs: {:?}", obj);
    debug!("Constraint bounds: row_lb={:?}, row_ub={:?}", row_lb, row_ub);

    // === Solve with CLP ===
    unsafe {
        let model = clp_ffi::Clp_newModel();
        if model.is_null() {
            anyhow::bail!("Failed to create CLP model");
        }

        // Set log level (0 = none, 1 = final, 2 = factorization, 3 = progress)
        clp_ffi::Clp_setLogLevel(model, 1);

        // Set minimization
        clp_ffi::Clp_setOptimizationDirection(model, 1.0);

        info!(
            "Loading LP: {} variables ({} gens + {} angles), {} constraints ({} balance + {} line limits)",
            num_cols, n_gen, n_theta, num_rows, n_bus, 2 * n_branch
        );

        clp_ffi::Clp_loadProblem(
            model,
            num_cols as i32,
            num_rows as i32,
            col_start.as_ptr(),
            row_index.as_ptr(),
            values.as_ptr(),
            col_lb.as_ptr(),
            col_ub.as_ptr(),
            obj.as_ptr(),
            row_lb.as_ptr(),
            row_ub.as_ptr(),
        );

        // Enable scaling for numerical stability
        clp_ffi::Clp_scaling(model, 3);

        // Solve
        debug!("Running CLP dual simplex...");
        let _solve_status = clp_ffi::Clp_initialSolve(model);

        let clp_status = clp_ffi::Clp_status(model);
        let objective = clp_ffi::Clp_objectiveValue(model);
        let iterations = clp_ffi::Clp_numberIterations(model);

        // Get primal solution
        let primal_ptr = clp_ffi::Clp_primalColumnSolution(model);
        let primal: Vec<f64> = if !primal_ptr.is_null() {
            std::slice::from_raw_parts(primal_ptr, num_cols).to_vec()
        } else {
            vec![0.0; num_cols]
        };

        // Extract generator outputs
        let gen_p: Vec<f64> = primal[0..n_gen].to_vec();
        debug!("Primal solution: {:?}", primal);
        debug!("Generator outputs: {:?}", gen_p);

        // Extract bus angles
        let mut bus_v_ang: Vec<f64> = vec![0.0; n_bus];
        for bus_idx in 0..n_bus {
            if let Some(col) = theta_col(bus_idx) {
                bus_v_ang[bus_idx] = primal[col];
            }
            // Reference bus stays at 0
        }

        // Get dual solution (shadow prices / LMPs)
        // The duals of the power balance constraints are the LMPs
        let dual_ptr = clp_ffi::Clp_dualRowSolution(model);
        let bus_lmp: Vec<f64> = if !dual_ptr.is_null() {
            // First n_bus rows are power balance constraints
            std::slice::from_raw_parts(dual_ptr, n_bus).to_vec()
        } else {
            vec![0.0; n_bus]
        };

        // Compute branch flows: P_ij = b_ij * (θ_i - θ_j)
        let mut branch_p_from: Vec<f64> = Vec::with_capacity(n_branch);
        for k in 0..n_branch {
            let from_bus = problem.branch_from[k];
            let to_bus = problem.branch_to[k];

            let i = *bus_id_to_idx.get(&from_bus).unwrap_or(&0);
            let j = *bus_id_to_idx.get(&to_bus).unwrap_or(&0);

            let x = problem.branch_x[k];
            let flow = if x.abs() > 1e-12 {
                let b = 1.0 / x;
                let theta_i = bus_v_ang[i];
                let theta_j = bus_v_ang[j];
                b * (theta_i - theta_j)
            } else {
                0.0
            };
            branch_p_from.push(flow);
        }

        // Branch flow at "to" end is negative of "from" (lossless DC)
        let branch_p_to: Vec<f64> = branch_p_from.iter().map(|&f| -f).collect();

        clp_ffi::Clp_deleteModel(model);

        // Map CLP status to our status
        // 0 = optimal, 1 = primal infeasible, 2 = dual infeasible, 3 = stopped, 4 = errors
        let status = match clp_status {
            0 => SolutionStatus::Optimal,
            1 => SolutionStatus::Infeasible,
            2 => SolutionStatus::Unbounded,
            3 => SolutionStatus::IterationLimit,
            _ => SolutionStatus::Error,
        };

        let solve_time_ms = start_time.elapsed().as_millis() as i64;

        info!(
            "CLP finished: status={}, obj={:.4}, iters={}, time={}ms",
            clp_status, objective, iterations, solve_time_ms
        );

        Ok(SolutionBatch {
            status,
            objective,
            iterations,
            solve_time_ms,
            error_message: if status == SolutionStatus::Error {
                Some(format!("CLP returned status {}", clp_status))
            } else {
                None
            },
            bus_id: problem.bus_id.clone(),
            bus_v_mag: vec![1.0; n_bus], // DC assumption: |V| = 1.0 p.u.
            bus_v_ang,
            bus_lmp,
            gen_id: problem.gen_id.clone(),
            gen_p,
            gen_q: vec![0.0; n_gen], // DC-OPF doesn't solve for Q
            branch_id: problem.branch_id.clone(),
            branch_p_from,
            branch_q_from: vec![0.0; n_branch], // No reactive power in DC
            branch_p_to,
            branch_q_to: vec![0.0; n_branch],
        })
    }
}
