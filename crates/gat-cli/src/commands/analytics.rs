use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_algo::power_flow;
use gat_cli::cli::AnalyticsCommands;
use gat_core::solver::SolverKind;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};

pub fn handle(command: &AnalyticsCommands) -> Result<()> {
    match command {
        AnalyticsCommands::Ptdf {
            grid_file,
            source,
            sink,
            transfer,
            out,
            out_partitions,
            threads,
            solver,
        } => {
            configure_threads(threads);
            let start = Instant::now();
            let solver_kind = solver.parse::<SolverKind>()?;
            let solver_impl = solver_kind.build_solver();
            let partitions = parse_partitions(out_partitions.as_ref());
            let partition_spec = out_partitions.as_deref().unwrap_or("");
            let res = (|| -> Result<()> {
                let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                power_flow::ptdf_analysis(
                    &network,
                    solver_impl.as_ref(),
                    *source,
                    *sink,
                    *transfer,
                    Path::new(out),
                    &partitions,
                )?;
                Ok(())
            })();
            record_run_timed(
                out,
                "analytics ptdf",
                &[
                    ("grid_file", grid_file),
                    ("source", &source.to_string()),
                    ("sink", &sink.to_string()),
                    ("transfer", &transfer.to_string()),
                    ("solver", solver_kind.as_str()),
                    ("out_partitions", partition_spec),
                ],
                start,
                &res,
            );
            res
        }
    }
}
