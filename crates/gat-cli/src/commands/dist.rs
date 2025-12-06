use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use tracing::info;

use crate::commands::telemetry::record_run_timed;
use gat_cli::cli::DistCommands;
use gat_core::solver::SolverKind;
use gat_dist::{hostcap_sweep, import_matpower_case, run_optimal_power_flow, run_power_flow};

pub fn handle(command: &DistCommands) -> Result<()> {
    match command {
        DistCommands::Import {
            m,
            out_dir,
            feeder_id,
        } => {
            info!("Importing MATPOWER {} into {}", m, out_dir);
            let start = Instant::now();
            let res = import_matpower_case(m, Path::new(out_dir), feeder_id.as_deref());
            record_run_timed(
                out_dir,
                "dist import matpower",
                &[
                    ("matpower", m),
                    ("out_dir", out_dir),
                    ("feeder_id", feeder_id.as_deref().unwrap_or("default")),
                ],
                start,
                &res,
            );
            res
        }
        DistCommands::Pf {
            grid_file,
            out,
            solver,
            tol,
            max_iter,
        } => {
            info!(
                "Running dist pf {} -> {} ({})",
                grid_file,
                out,
                solver.as_str()
            );
            let start = Instant::now();
            let res = run_power_flow(
                Path::new(grid_file),
                Path::new(out),
                solver.parse::<SolverKind>()?,
                *tol,
                *max_iter,
            );
            record_run_timed(
                out,
                "dist pf",
                &[
                    ("grid_file", grid_file),
                    ("solver", solver.as_str()),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                ],
                start,
                &res,
            );
            res
        }
        DistCommands::Opf {
            grid_file,
            out,
            objective,
            solver,
            tol,
            max_iter,
        } => {
            info!(
                "Running dist opf {} -> {} (objective {})",
                grid_file, out, objective
            );
            let start = Instant::now();
            let res = run_optimal_power_flow(
                Path::new(grid_file),
                Path::new(out),
                solver.parse::<SolverKind>()?,
                *tol,
                *max_iter,
                objective.as_str(),
            );
            record_run_timed(
                out,
                "dist opf",
                &[
                    ("grid_file", grid_file),
                    ("objective", objective),
                    ("solver", solver.as_str()),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                ],
                start,
                &res,
            );
            res
        }
        DistCommands::Hostcap {
            grid_file,
            out_dir,
            bus,
            max_injection,
            steps,
            solver,
        } => {
            info!("Running hostcap sweep on {} -> {}", grid_file, out_dir);
            let start = Instant::now();
            let res = hostcap_sweep(
                Path::new(grid_file),
                bus,
                *max_injection,
                *steps,
                Path::new(out_dir),
                solver.parse::<SolverKind>()?,
            );
            let max_injection_str = max_injection.to_string();
            let steps_str = steps.to_string();
            let buses = bus
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            record_run_timed(
                out_dir,
                "dist hostcap",
                &[
                    ("grid_file", grid_file),
                    ("buses", buses.as_str()),
                    ("max_injection", max_injection_str.as_str()),
                    ("steps", steps_str.as_str()),
                    ("solver", solver.as_str()),
                ],
                start,
                &res,
            );
            res
        }
    }
}
