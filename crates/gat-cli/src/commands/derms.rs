use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use tracing::info;

use crate::commands::telemetry::record_run_timed;
use gat_cli::cli::DermsCommands;
use gat_derms::{envelope, schedule, stress_test};

pub fn handle(command: &DermsCommands) -> Result<()> {
    match command {
        DermsCommands::Envelope {
            grid_file,
            assets,
            out,
            group_by,
        } => {
            info!("Building DERMS envelope {} -> {}", assets, out);
            let start = Instant::now();
            let res = envelope(
                Path::new(grid_file),
                Path::new(assets),
                Path::new(out),
                group_by.as_deref(),
            );
            record_run_timed(
                out,
                "derms envelope",
                &[
                    ("grid_file", grid_file),
                    ("assets", assets),
                    ("out", out),
                    ("group_by", group_by.as_deref().unwrap_or("agg_id")),
                ],
                start,
                &res,
            );
            res
        }
        DermsCommands::Schedule {
            assets,
            price_series,
            out,
            objective,
        } => {
            let start = Instant::now();
            let res = (|| -> Result<()> {
                let curtailment = schedule(
                    Path::new(assets),
                    Path::new(price_series),
                    Path::new(out),
                    objective.as_str(),
                )?;
                info!(
                    "DERMS schedule wrote {} with curtailment {:.3}",
                    out, curtailment
                );
                Ok(())
            })();
            record_run_timed(
                out,
                "derms schedule",
                &[
                    ("assets", assets),
                    ("price_series", price_series),
                    ("out", out),
                    ("objective", objective),
                ],
                start,
                &res,
            );
            res
        }
        DermsCommands::StressTest {
            assets,
            price_series,
            out_dir,
            scenarios,
            seed,
        } => {
            info!(
                "Running DERMS stress-test ({scenarios} scenarios) -> {}",
                out_dir
            );
            let start = Instant::now();
            let res = stress_test(
                Path::new(assets),
                Path::new(price_series),
                Path::new(out_dir),
                *scenarios,
                *seed,
            );
            let seed_str = seed.map(|v| v.to_string());
            record_run_timed(
                out_dir,
                "derms stress-test",
                &[
                    ("assets", assets),
                    ("price_series", price_series),
                    ("out_dir", out_dir),
                    ("scenarios", &scenarios.to_string()),
                    ("seed", seed_str.as_deref().unwrap_or("none")),
                ],
                start,
                &res,
            );
            res
        }
    }
}
