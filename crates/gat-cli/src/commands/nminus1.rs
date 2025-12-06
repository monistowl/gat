use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_algo::power_flow::n_minus_one_dc;
use gat_cli::cli::Nminus1Commands;
use gat_core::solver::SolverKind;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};

pub fn handle(command: &Nminus1Commands) -> Result<()> {
    match command {
        Nminus1Commands::Dc {
            grid_file,
            contingencies,
            out,
            branch_limits,
            threads,
            solver,
            out_partitions,
            rating_type: _,
        } => {
            let start = Instant::now();
            configure_threads(threads);
            let solver_kind = solver.parse::<SolverKind>()?;
            let solver_impl = solver_kind.build_solver();
            let partitions = parse_partitions(out_partitions.as_ref());
            let res = (|| -> Result<()> {
                let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                n_minus_one_dc(
                    &network,
                    solver_impl.clone(),
                    contingencies.as_str(),
                    Path::new(out),
                    &partitions,
                    branch_limits.as_deref(),
                )
            })();
            record_run_timed(
                out,
                "nminus1 dc",
                &[
                    ("grid_file", grid_file),
                    ("contingencies", contingencies),
                    ("out", out),
                    ("branch_limits", branch_limits.as_deref().unwrap_or("none")),
                    ("threads", threads),
                    ("solver", solver_kind.as_str()),
                    ("out_partitions", out_partitions.as_deref().unwrap_or("")),
                ],
                start,
                &res,
            );
            res
        }
    }
}
