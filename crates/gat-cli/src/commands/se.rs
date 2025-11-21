use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::SeCommands;
use gat_dist::power_flow;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;
use crate::parse_partitions;

pub fn handle(command: &SeCommands) -> Result<()> {
    match command {
        SeCommands::Wls {
            grid_file,
            measurements,
            out,
            state_out,
            threads,
            solver,
            out_partitions,
            slack_bus,
        } => {
            let start = Instant::now();
            let solver_kind = solver.parse()?;
            let solver_impl = solver_kind.build_solver();
            let partitions = parse_partitions(out_partitions.as_ref());
            let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
            let out_path = Path::new(out);
            let state_path = state_out.as_deref().map(Path::new);
            let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                Ok(network) => power_flow::state_estimation_wls(
                    &network,
                    solver_impl.as_ref(),
                    measurements,
                    out_path,
                    &partitions,
                    state_path,
                    *slack_bus,
                ),
                Err(e) => Err(e),
            };
            let slack_spec = slack_bus.map(|id| id.to_string());
            record_run_timed(
                out,
                "se wls",
                &[
                    ("grid_file", grid_file),
                    ("measurements", measurements),
                    ("threads", threads),
                    ("solver", solver_kind.as_str()),
                    ("out_partitions", partition_spec.as_str()),
                    ("slack_bus", slack_spec.as_deref().unwrap_or("auto")),
                ],
                start,
                &res,
            );
            res
        }
    }
}
