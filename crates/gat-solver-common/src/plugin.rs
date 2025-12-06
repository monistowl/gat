//! Plugin harness for solver binaries.
//!
//! Provides common infrastructure for solver plugin binaries, eliminating
//! boilerplate for tracing setup, IPC handling, and error management.
//!
//! # Usage
//!
//! ```rust,ignore
//! use gat_solver_common::plugin::{run_solver_plugin, SolverPlugin};
//! use gat_solver_common::{ProblemBatch, SolutionBatch};
//! use anyhow::Result;
//!
//! struct ClpSolver;
//!
//! impl SolverPlugin for ClpSolver {
//!     fn name(&self) -> &'static str { "gat-clp" }
//!     fn solve(&self, problem: &ProblemBatch) -> Result<SolutionBatch> {
//!         // Solver implementation
//!     }
//! }
//!
//! fn main() {
//!     run_solver_plugin(ClpSolver);
//! }
//! ```

use crate::error::ExitCode;
use crate::ipc;
use crate::problem::ProblemBatch;
use crate::solution::SolutionBatch;
use crate::PROTOCOL_VERSION;
use anyhow::{Context, Result};
use std::io::{self, Read, Write};
use tracing::{debug, error, info};

/// Trait for implementing a solver plugin.
///
/// Implement this trait to create a solver plugin binary. The harness
/// handles all IPC, logging, and error handling.
pub trait SolverPlugin {
    /// The solver name (e.g., "gat-clp").
    fn name(&self) -> &'static str;

    /// Solve the given problem batch.
    ///
    /// The implementation should:
    /// 1. Extract problem data from `ProblemBatch`
    /// 2. Call the underlying solver (via FFI or native Rust)
    /// 3. Return the solution or an error
    fn solve(&self, problem: &ProblemBatch) -> Result<SolutionBatch>;

    /// Whether to use v2 (multi-batch) IPC protocol.
    ///
    /// Override to return `false` for legacy single-batch protocol.
    /// Default is `true` (v2 protocol).
    fn use_v2_protocol(&self) -> bool {
        true
    }

    /// Additional initialization before solving.
    ///
    /// Called after tracing is initialized but before reading the problem.
    /// Override to perform solver-specific initialization (e.g., license checks).
    fn init(&self) -> Result<()> {
        Ok(())
    }
}

/// Run a solver plugin with standard harness.
///
/// This function:
/// 1. Initializes tracing (respects `RUST_LOG` environment variable)
/// 2. Logs version and protocol information
/// 3. Reads the problem from stdin (Arrow IPC format)
/// 4. Calls `plugin.solve()` with the problem
/// 5. Writes the solution to stdout (Arrow IPC format)
/// 6. Exits with appropriate exit code
///
/// # Exit Codes
///
/// - `0`: Success
/// - `1`: Solver error (logged to stderr)
/// - `2`: IPC error (protocol/serialization issues)
/// - `3`: Initialization error
pub fn run_solver_plugin<P: SolverPlugin>(plugin: P) -> ! {
    // Initialize tracing to stderr (respects RUST_LOG env var)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(io::stderr)
        .init();

    info!(
        "{} v{} (protocol v{})",
        plugin.name(),
        env!("CARGO_PKG_VERSION"),
        PROTOCOL_VERSION
    );

    let exit_code = match run_plugin_inner(&plugin) {
        Ok(()) => ExitCode::Success,
        Err(e) => {
            error!("Solver error: {:?}", e);
            ExitCode::SolverError
        }
    };

    std::process::exit(exit_code as i32);
}

/// Inner implementation that can return errors.
fn run_plugin_inner<P: SolverPlugin>(plugin: &P) -> Result<()> {
    // Plugin-specific initialization
    plugin.init().context("Solver initialization failed")?;

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
    let problem = if plugin.use_v2_protocol() {
        ipc::read_problem_v2(input.as_slice()).context("Failed to parse Arrow IPC v2 problem")?
    } else {
        ipc::read_problem(input.as_slice()).context("Failed to parse Arrow IPC problem")?
    };

    info!(
        "Problem: {} buses, {} generators, {} branches",
        problem.bus_id.len(),
        problem.gen_id.len(),
        problem.branch_id.len()
    );

    // Solve the problem
    let solution = plugin.solve(&problem)?;

    // Write solution to stdout
    debug!("Writing solution to stdout...");
    let mut output = Vec::new();
    if plugin.use_v2_protocol() {
        ipc::write_solution_v2(&solution, &mut output).context("Failed to serialize solution v2")?;
    } else {
        ipc::write_solution(&solution, &mut output).context("Failed to serialize solution")?;
    }

    io::stdout()
        .write_all(&output)
        .context("Failed to write solution to stdout")?;

    info!(
        "Solution written: status={:?}, objective={:.6}",
        solution.status, solution.objective
    );

    Ok(())
}
