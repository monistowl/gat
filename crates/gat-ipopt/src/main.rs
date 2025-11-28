//! IPOPT solver wrapper binary for GAT native solver plugin system.
//!
//! This binary implements the GAT solver IPC protocol:
//! 1. Reads an Arrow IPC stream from stdin containing the optimization problem
//! 2. Solves using IPOPT (Interior Point OPTimizer)
//! 3. Writes an Arrow IPC stream to stdout containing the solution
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

    info!("gat-ipopt v{} (protocol v{})", env!("CARGO_PKG_VERSION"), PROTOCOL_VERSION);

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

    info!("Solution written: status={:?}, objective={:.6}",
          solution.status, solution.objective);

    Ok(())
}

/// Solve the optimization problem using IPOPT.
///
/// This is a placeholder implementation. A full implementation would:
/// 1. Convert ProblemBatch to IPOPT's NLP format
/// 2. Set up power flow equations as nonlinear constraints
/// 3. Configure IPOPT options for AC-OPF
/// 4. Parse solution back to SolutionBatch format
#[cfg(feature = "ipopt-sys")]
fn solve_with_ipopt(problem: &ProblemBatch) -> Result<SolutionBatch> {
    // TODO: Implement full IPOPT integration
    // This requires:
    // - Formulating the AC-OPF as an NLP
    // - Implementing the required IPOPT callbacks
    // - Mapping solution back to GAT's data structures

    info!("IPOPT integration not fully implemented - returning placeholder");

    Ok(SolutionBatch {
        status: SolutionStatus::Error,
        objective: f64::NAN,
        iterations: 0,
        solve_time_ms: 0,
        error_message: Some("IPOPT integration not yet complete".to_string()),
        bus_id: problem.bus_id.clone(),
        bus_v_mag: vec![1.0; problem.bus_id.len()],
        bus_v_ang: vec![0.0; problem.bus_id.len()],
        bus_lmp: vec![0.0; problem.bus_id.len()],
        gen_id: problem.gen_id.clone(),
        gen_p: vec![0.0; problem.gen_id.len()],
        gen_q: vec![0.0; problem.gen_id.len()],
        branch_id: problem.branch_id.clone(),
        branch_p_from: vec![0.0; problem.branch_id.len()],
        branch_q_from: vec![0.0; problem.branch_id.len()],
        branch_p_to: vec![0.0; problem.branch_id.len()],
        branch_q_to: vec![0.0; problem.branch_id.len()],
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
