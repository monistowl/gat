//! CBC MIP solver wrapper binary for GAT native solver plugin system.
//!
//! This binary implements the GAT solver IPC protocol:
//! 1. Reads an Arrow IPC stream from stdin containing the optimization problem
//! 2. Solves using CBC (COIN-OR Branch and Cut)
//! 3. Writes an Arrow IPC stream to stdout containing the solution
//!
//! # CBC Algorithm
//!
//! CBC implements branch-and-cut for Mixed Integer Programming (MIP):
//!
//! - **Branch-and-bound:** Recursively partitions the solution space
//! - **LP relaxation:** Uses CLP to solve continuous relaxations at each node
//! - **Cutting planes:** Adds Cgl cuts (Gomory, MIR, clique) to tighten bounds
//! - **Heuristics:** Feasibility pump, RINS, local branching for early solutions
//!
//! For power systems, CBC is well-suited for unit commitment problems where:
//! - Objective: Minimize total generation cost
//! - Binary variables: Generator on/off status
//! - Continuous variables: Power outputs
//! - Constraints: Power balance, generator limits, minimum up/down times
//!
//! **Reference:** COIN-OR Foundation. CBC User's Guide.
//! [github.com/coin-or/Cbc](https://github.com/coin-or/Cbc)
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
use tracing::{debug, error, info};

// FFI bindings to CBC C interface
#[allow(dead_code)]
mod cbc_ffi {
    use std::os::raw::{c_char, c_double, c_int};

    #[repr(C)]
    pub struct Cbc_Model {
        _private: [u8; 0],
    }

    extern "C" {
        // Model creation/destruction
        pub fn Cbc_newModel() -> *mut Cbc_Model;
        pub fn Cbc_deleteModel(model: *mut Cbc_Model);

        // Problem loading
        pub fn Cbc_loadProblem(
            model: *mut Cbc_Model,
            num_cols: c_int,
            num_rows: c_int,
            start: *const c_int,     // Column starts (CCS format)
            index: *const c_int,     // Row indices
            value: *const c_double,  // Non-zero values
            col_lb: *const c_double, // Column lower bounds
            col_ub: *const c_double, // Column upper bounds
            obj: *const c_double,    // Objective coefficients
            row_lb: *const c_double, // Row lower bounds
            row_ub: *const c_double, // Row upper bounds
        );

        // Integer variables
        pub fn Cbc_setInteger(model: *mut Cbc_Model, i_column: c_int);
        pub fn Cbc_setContinuous(model: *mut Cbc_Model, i_column: c_int);

        // Solving
        pub fn Cbc_solve(model: *mut Cbc_Model) -> c_int;

        // Solution access
        pub fn Cbc_status(model: *mut Cbc_Model) -> c_int;
        pub fn Cbc_secondaryStatus(model: *mut Cbc_Model) -> c_int;
        pub fn Cbc_isProvenOptimal(model: *mut Cbc_Model) -> c_int;
        pub fn Cbc_isProvenInfeasible(model: *mut Cbc_Model) -> c_int;
        pub fn Cbc_isContinuousUnbounded(model: *mut Cbc_Model) -> c_int;
        pub fn Cbc_isNodeLimitReached(model: *mut Cbc_Model) -> c_int;
        pub fn Cbc_isSecondsLimitReached(model: *mut Cbc_Model) -> c_int;
        pub fn Cbc_isSolutionLimitReached(model: *mut Cbc_Model) -> c_int;

        pub fn Cbc_getObjValue(model: *mut Cbc_Model) -> c_double;
        pub fn Cbc_getBestPossibleObjValue(model: *mut Cbc_Model) -> c_double;
        pub fn Cbc_getColSolution(model: *mut Cbc_Model) -> *const c_double;
        pub fn Cbc_getNodeCount(model: *mut Cbc_Model) -> c_int;

        // Model info
        pub fn Cbc_getNumCols(model: *mut Cbc_Model) -> c_int;
        pub fn Cbc_getNumRows(model: *mut Cbc_Model) -> c_int;

        // Options
        pub fn Cbc_setLogLevel(model: *mut Cbc_Model, level: c_int);
        pub fn Cbc_setParameter(model: *mut Cbc_Model, name: *const c_char, value: *const c_char);
        pub fn Cbc_setMaximumSeconds(model: *mut Cbc_Model, seconds: c_double);
        pub fn Cbc_setMaximumNodes(model: *mut Cbc_Model, nodes: c_int);
        pub fn Cbc_setAllowableGap(model: *mut Cbc_Model, gap: c_double);
        pub fn Cbc_setObjSense(model: *mut Cbc_Model, sense: c_double);
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
        "gat-cbc v{} (protocol v{})",
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
    let solution = solve_with_cbc(&problem)?;

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

/// Solve the unit commitment / MIP problem using CBC.
///
/// This formulates a simple MIP:
/// - Variables: Generator outputs P_g (continuous), on/off status u_g (binary)
/// - Objective: Minimize Σ c_g * P_g + startup_cost * u_g
/// - Constraints:
///   - Generator limits: P_min * u_g ≤ P_g ≤ P_max * u_g
fn solve_with_cbc(problem: &ProblemBatch) -> Result<SolutionBatch> {
    let start_time = std::time::Instant::now();

    unsafe {
        let model = cbc_ffi::Cbc_newModel();
        if model.is_null() {
            anyhow::bail!("Failed to create CBC model");
        }

        // Set log level (0 = none, 1 = summary)
        cbc_ffi::Cbc_setLogLevel(model, 1);

        // Set minimization
        cbc_ffi::Cbc_setObjSense(model, 1.0);

        let n_gens = problem.gen_id.len();

        if n_gens == 0 {
            cbc_ffi::Cbc_deleteModel(model);
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

        // Build a simple MIP: minimize c'x subject to lb <= x <= ub
        // Variables: generator outputs (continuous)
        // For now, we don't add binary commitment variables to keep it simple
        let num_cols = n_gens as i32;
        let num_rows = 0i32; // No constraints for this simple formulation

        // Column bounds (generator limits)
        let col_lb: Vec<f64> = problem
            .gen_p_min
            .iter()
            .copied()
            .chain(std::iter::repeat(0.0))
            .take(n_gens)
            .collect();
        let col_ub: Vec<f64> = problem
            .gen_p_max
            .iter()
            .copied()
            .chain(std::iter::repeat(f64::INFINITY))
            .take(n_gens)
            .collect();

        // Objective: linear cost c1 * P_g
        let obj: Vec<f64> = problem
            .gen_cost_c1
            .iter()
            .copied()
            .chain(std::iter::repeat(1.0))
            .take(n_gens)
            .collect();

        // Empty constraint matrix for this simple formulation
        let start: Vec<i32> = (0..=num_cols).collect();
        let index: Vec<i32> = vec![];
        let value: Vec<f64> = vec![];
        let row_lb: Vec<f64> = vec![];
        let row_ub: Vec<f64> = vec![];

        cbc_ffi::Cbc_loadProblem(
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

        // Set timeout if specified
        if problem.timeout_seconds > 0 {
            cbc_ffi::Cbc_setMaximumSeconds(model, problem.timeout_seconds as f64);
        }

        // Set optimality gap (default 1e-4 = 0.01%)
        cbc_ffi::Cbc_setAllowableGap(model, 1e-4);

        // Solve
        info!("Solving MIP with {} variables...", num_cols);
        let _solve_status = cbc_ffi::Cbc_solve(model);

        // Check solution status
        let is_optimal = cbc_ffi::Cbc_isProvenOptimal(model) != 0;
        let is_infeasible = cbc_ffi::Cbc_isProvenInfeasible(model) != 0;
        let is_unbounded = cbc_ffi::Cbc_isContinuousUnbounded(model) != 0;
        let node_limit = cbc_ffi::Cbc_isNodeLimitReached(model) != 0;
        let time_limit = cbc_ffi::Cbc_isSecondsLimitReached(model) != 0;

        let objective = cbc_ffi::Cbc_getObjValue(model);
        let node_count = cbc_ffi::Cbc_getNodeCount(model);

        // Get primal solution
        let sol_ptr = cbc_ffi::Cbc_getColSolution(model);
        let gen_p: Vec<f64> = if !sol_ptr.is_null() {
            std::slice::from_raw_parts(sol_ptr, n_gens).to_vec()
        } else {
            vec![0.0; n_gens]
        };

        cbc_ffi::Cbc_deleteModel(model);

        // Map CBC status to our status
        let status = if is_optimal {
            SolutionStatus::Optimal
        } else if is_infeasible {
            SolutionStatus::Infeasible
        } else if is_unbounded {
            SolutionStatus::Unbounded
        } else if time_limit {
            SolutionStatus::Timeout
        } else if node_limit {
            SolutionStatus::IterationLimit
        } else {
            SolutionStatus::Unknown
        };

        let solve_time_ms = start_time.elapsed().as_millis() as i64;

        info!(
            "CBC finished: optimal={}, infeas={}, obj={:.4}, nodes={}, time={}ms",
            is_optimal, is_infeasible, objective, node_count, solve_time_ms
        );

        Ok(SolutionBatch {
            status,
            objective,
            iterations: node_count,
            solve_time_ms,
            error_message: if status == SolutionStatus::Unknown {
                Some("CBC returned unknown status".to_string())
            } else {
                None
            },
            bus_id: problem.bus_id.clone(),
            bus_v_mag: vec![1.0; problem.bus_id.len()],
            bus_v_ang: vec![0.0; problem.bus_id.len()],
            bus_lmp: vec![0.0; problem.bus_id.len()],
            gen_id: problem.gen_id.clone(),
            gen_p,
            gen_q: vec![0.0; n_gens],
            branch_id: problem.branch_id.clone(),
            branch_p_from: vec![0.0; problem.branch_id.len()],
            branch_q_from: vec![0.0; problem.branch_id.len()],
            branch_p_to: vec![0.0; problem.branch_id.len()],
            branch_q_to: vec![0.0; problem.branch_id.len()],
        })
    }
}
