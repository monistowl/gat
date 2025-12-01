use std::path::Path;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};
use anyhow::Result;
use gat_algo::power_flow::{self, AcPowerFlowSolver};
use gat_cli::cli::PowerFlowCommands;
use gat_core::solver::SolverKind;
use gat_io::importers;

pub fn handle(command: &PowerFlowCommands) -> Result<()> {
    match command {
        PowerFlowCommands::Dc {
            grid_file,
            out,
            threads,
            solver,
            lp_solver: _, // unused in DC power flow
            out_partitions,
        } => {
            let start = Instant::now();
            configure_threads(threads);
            let solver_kind = solver.parse::<SolverKind>()?;
            let solver_impl = solver_kind.build_solver();
            let partitions = parse_partitions(out_partitions.as_ref());
            let out_path = Path::new(out);

            let network = importers::load_grid_from_arrow(grid_file.as_str())?;
            let res = power_flow::dc_power_flow(
                &network,
                solver_impl.as_ref(),
                out_path,
                &partitions,
            );

            record_run_timed(
                out,
                "pf dc",
                &[
                    ("grid_file", grid_file),
                    ("out", out),
                    ("threads", threads),
                    ("solver", solver_kind.as_str()),
                    ("out_partitions", out_partitions.as_deref().unwrap_or("")),
                ],
                start,
                &res,
            );
            res
        }
        PowerFlowCommands::Ac {
            grid_file,
            out,
            tol,
            max_iter,
            threads,
            solver,
            lp_solver: _, // unused in AC power flow
            out_partitions,
            q_limits,
        } => {
            let start = Instant::now();
            configure_threads(threads);
            let solver_kind = solver.parse::<SolverKind>()?;
            let solver_impl = solver_kind.build_solver();
            let partitions = parse_partitions(out_partitions.as_ref());
            let out_path = Path::new(out);

            let network = importers::load_grid_from_arrow(grid_file.as_str())?;

            let res = if *q_limits {
                // Use new Newton-Raphson solver with Q-limit enforcement
                let pf_solver = AcPowerFlowSolver::new()
                    .with_tolerance(*tol)
                    .with_max_iterations(*max_iter as usize)
                    .with_q_limit_enforcement(true);

                let solution = pf_solver.solve(&network)?;

                // Write results to output file
                power_flow::write_ac_pf_solution(&network, &solution, out_path, &partitions)?;

                if solution.converged {
                    tracing::info!(
                        "AC power flow converged in {} iterations (max mismatch: {:.2e})",
                        solution.iterations,
                        solution.max_mismatch
                    );
                }
                Ok(())
            } else {
                // Use legacy solver without Q-limit enforcement
                power_flow::ac_power_flow(
                    &network,
                    solver_impl.as_ref(),
                    *tol,
                    *max_iter,
                    out_path,
                    &partitions,
                )
            };

            let q_limits_str = if *q_limits { "true" } else { "false" };
            record_run_timed(
                out,
                "pf ac",
                &[
                    ("grid_file", grid_file),
                    ("threads", threads),
                    ("out", out),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                    ("solver", solver_kind.as_str()),
                    ("out_partitions", out_partitions.as_deref().unwrap_or("")),
                    ("q_limits", q_limits_str),
                ],
                start,
                &res,
            );
            res
        }
    }
}
