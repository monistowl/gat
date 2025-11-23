use std::path::Path;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use anyhow::{anyhow, Result};
use gat_scenarios::apply::ScenarioApplyOptions;
use gat_scenarios::{load_spec_from_path, materialize_scenarios, resolve_scenarios};

pub fn handle(
    spec: &str,
    grid_file: Option<&str>,
    out_dir: &str,
    drop_outaged: bool,
) -> Result<()> {
    let start = Instant::now();
    let mut grid_used = String::new();
    let mut scenario_count = 0;
    let res = (|| -> Result<()> {
        let set = load_spec_from_path(Path::new(spec))?;
        let grid_path = grid_file.or(set.grid_file.as_deref()).ok_or_else(|| {
            anyhow!("base grid file must be provided either in the spec or via --grid-file")
        })?;
        grid_used = grid_path.to_string();
        let resolved = resolve_scenarios(&set)?;
        let options = ScenarioApplyOptions {
            drop_outaged_elements: drop_outaged,
        };
        let artifacts = materialize_scenarios(
            Path::new(&grid_used),
            Path::new(out_dir),
            &resolved,
            &options,
        )?;
        scenario_count = artifacts.len();
        println!(
            "Materialized {} scenario grids into {}",
            scenario_count, out_dir
        );
        Ok(())
    })();
    record_run_timed(
        out_dir,
        "scenarios materialize",
        &[
            ("spec", spec),
            ("grid_file", grid_used.as_str()),
            ("out_dir", out_dir),
            ("drop_outaged", &drop_outaged.to_string()),
            ("num_scenarios", &scenario_count.to_string()),
        ],
        start,
        &res,
    );
    res
}
