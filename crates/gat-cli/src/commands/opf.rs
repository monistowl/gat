use std::path::Path;

use anyhow::Result;
use gat_algo::power_flow;
use gat_algo::LpSolverKind;
use gat_cli::cli::OpfCommands;
use gat_core::solver::SolverKind;
use gat_io::importers;

use crate::commands::telemetry::record_run;
use crate::commands::util::{configure_threads, parse_partitions};

pub fn handle(command: &OpfCommands) -> Result<()> {
    match command {
        OpfCommands::Dc {
            grid_file,
            cost,
            limits,
            out,
            branch_limits,
            piecewise,
            threads,
            solver,
            lp_solver,
            out_partitions,
        } => {
            configure_threads(threads);
            let result = (|| -> Result<()> {
                let solver_kind = solver.parse::<SolverKind>()?;
                let solver_impl = solver_kind.build_solver();
                let lp_solver_kind = lp_solver.parse::<LpSolverKind>()?;
                let partitions = parse_partitions(out_partitions.as_ref());
                let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                let out_path = Path::new(out);
                let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                    Ok(network) => power_flow::dc_optimal_power_flow(
                        &network,
                        solver_impl.as_ref(),
                        cost.as_str(),
                        limits.as_str(),
                        out_path,
                        &partitions,
                        branch_limits.as_deref(),
                        piecewise.as_deref(),
                        &lp_solver_kind,
                    ),
                    Err(e) => Err(e),
                };
                if res.is_ok() {
                    record_run(
                        out,
                        "opf dc",
                        &[
                            ("grid_file", grid_file),
                            ("threads", threads),
                            ("solver", solver_kind.as_str()),
                            ("lp_solver", lp_solver_kind.as_str()),
                            ("out_partitions", partition_spec.as_str()),
                        ],
                    );
                }
                res
            })();
            result
        }
        OpfCommands::Ac {
            grid_file,
            out,
            tol,
            max_iter,
            threads,
            solver,
            out_partitions,
        } => {
            configure_threads(threads);
            let result = (|| -> Result<()> {
                let solver_kind = solver.parse::<SolverKind>()?;
                let solver_impl = solver_kind.build_solver();
                let partitions = parse_partitions(out_partitions.as_ref());
                let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                let out_path = Path::new(out);
                let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                    Ok(network) => power_flow::ac_optimal_power_flow(
                        &network,
                        solver_impl.as_ref(),
                        *tol,
                        *max_iter,
                        out_path,
                        &partitions,
                    ),
                    Err(e) => Err(e),
                };
                if res.is_ok() {
                    record_run(
                        out,
                        "opf ac",
                        &[
                            ("grid_file", grid_file),
                            ("threads", threads),
                            ("tol", &tol.to_string()),
                            ("max_iter", &max_iter.to_string()),
                            ("solver", solver_kind.as_str()),
                            ("out_partitions", partition_spec.as_str()),
                        ],
                    );
                }
                res
            })();
            result
        }
    }
}
