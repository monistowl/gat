use std::path::Path;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};
use anyhow::Result;
use gat_algo::power_flow;
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
            lp_solver,
            out_partitions,
        } => {
            let start = Instant::now();
            let res = (|| -> Result<()> {
                configure_threads(threads);
                let solver_kind = solver.parse::<SolverKind>()?;
                let solver_impl = solver_kind.build_solver();
                let _ = lp_solver;
                let partitions = parse_partitions(out_partitions.as_ref());
                let out_path = Path::new(out);
                match importers::load_grid_from_arrow(grid_file.as_str()) {
                    Ok(network) => power_flow::dc_power_flow(
                        &network,
                        solver_impl.as_ref(),
                        out_path,
                        &partitions,
                    ),
                    Err(e) => Err(e),
                }
            })();
            let solver_name = solver.parse::<SolverKind>()?.as_str();
            record_run_timed(
                out,
                "pf dc",
                &[
                    ("grid_file", grid_file),
                    ("out", out),
                    ("threads", threads),
                    ("solver", solver_name),
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
            lp_solver,
            out_partitions,
        } => {
            let start = Instant::now();
            let res = (|| -> Result<()> {
                configure_threads(threads);
                let solver_kind = solver.parse::<SolverKind>()?;
                let solver_impl = solver_kind.build_solver();
                let _ = lp_solver;
                let partitions = parse_partitions(out_partitions.as_ref());
                let out_path = Path::new(out);
                match importers::load_grid_from_arrow(grid_file.as_str()) {
                    Ok(network) => power_flow::ac_power_flow(
                        &network,
                        solver_impl.as_ref(),
                        *tol,
                        *max_iter,
                        out_path,
                        &partitions,
                    ),
                    Err(e) => Err(e),
                }
            })();
            let solver_name = solver.parse::<SolverKind>()?.as_str();
            record_run_timed(
                out,
                "pf ac",
                &[
                    ("grid_file", grid_file),
                    ("threads", threads),
                    ("out", out),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                    ("solver", solver_name),
                    ("out_partitions", out_partitions.as_deref().unwrap_or("")),
                ],
                start,
                &res,
            );
            res
        }
    }
}
