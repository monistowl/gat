use std::fs::File;
use std::path::Path;

use anyhow::Result;
use gat_scenarios::{load_spec_from_path, resolve_scenarios};
use serde_json;

pub fn handle(spec: &str, grid_file: Option<&str>, out: &str) -> Result<()> {
    let mut scenario_set = load_spec_from_path(Path::new(spec))?;
    if let Some(grid) = grid_file {
        scenario_set.grid_file = Some(grid.to_string());
    }
    let resolved = resolve_scenarios(&scenario_set)?;
    let file = File::create(out)?;
    serde_json::to_writer_pretty(file, &resolved)?;
    Ok(())
}
