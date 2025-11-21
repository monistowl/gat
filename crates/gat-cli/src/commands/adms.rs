use std::path::Path;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use anyhow::Result;
use gat_adms::{flisr_sim, outage_mc, state_estimation, vvo_plan};
use gat_cli::cli::AdmsCommands;

pub fn handle(command: &AdmsCommands) -> Result<()> {
    match command {
        AdmsCommands::FlisrSim {
            grid_file,
            reliability,
            output_dir,
            scenarios,
            solver,
            tol,
            max_iter,
        } => {
            let start = Instant::now();
            let res = flisr_sim(
                Path::new(grid_file),
                Some(Path::new(reliability)),
                Path::new(output_dir),
                *scenarios,
                solver.parse()?,
                *tol,
                *max_iter,
            );
            record_run_timed(
                output_dir,
                "adms flisr-sim",
                &[
                    ("grid_file", grid_file),
                    ("reliability", reliability),
                    ("output_dir", output_dir),
                    ("scenarios", &scenarios.to_string()),
                    ("solver", solver.as_str()),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                ],
                start,
                &res,
            );
            res
        }
        AdmsCommands::VvoPlan {
            grid_file,
            output_dir,
            day_types,
            solver,
            tol,
            max_iter,
        } => {
            let start = Instant::now();
            let parsed_days = day_types
                .split(',')
                .map(|day| day.trim().to_string())
                .filter(|day| !day.is_empty())
                .collect::<Vec<_>>();
            let res = vvo_plan(
                Path::new(grid_file),
                Path::new(output_dir),
                &parsed_days,
                solver.parse()?,
                *tol,
                *max_iter,
            );
            record_run_timed(
                output_dir,
                "adms vvo-plan",
                &[
                    ("grid_file", grid_file),
                    ("output_dir", output_dir),
                    ("day_types", day_types),
                    ("solver", solver.as_str()),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                ],
                start,
                &res,
            );
            res
        }
        AdmsCommands::OutageMc {
            reliability,
            output_dir,
            samples,
            seed,
        } => {
            let start = Instant::now();
            let res = outage_mc(
                Path::new(reliability),
                Path::new(output_dir),
                *samples,
                *seed,
            );
            let seed_str = seed.map(|v| v.to_string());
            record_run_timed(
                output_dir,
                "adms outage-mc",
                &[
                    ("reliability", reliability),
                    ("output_dir", output_dir),
                    ("samples", &samples.to_string()),
                    ("seed", seed_str.as_deref().unwrap_or("none")),
                ],
                start,
                &res,
            );
            res
        }
        AdmsCommands::StateEstimation {
            grid_file,
            measurements,
            out,
            state_out,
            solver,
            slack_bus,
        } => {
            let start = Instant::now();
            let res = state_estimation(
                Path::new(grid_file),
                Path::new(measurements),
                Path::new(out),
                state_out.as_deref().map(Path::new),
                solver.parse()?,
                1e-6,
                20,
                *slack_bus,
            );
            record_run_timed(
                out,
                "adms state-estimation",
                &[
                    ("grid_file", grid_file),
                    ("measurements", measurements),
                    ("out", out),
                    ("state_out", state_out.as_deref().unwrap_or("not requested")),
                    ("solver", solver.as_str()),
                    (
                        "slack_bus",
                        slack_bus
                            .map(|id| id.to_string())
                            .as_deref()
                            .unwrap_or("auto"),
                    ),
                ],
                start,
                &res,
            );
            res
        }
    }
}
