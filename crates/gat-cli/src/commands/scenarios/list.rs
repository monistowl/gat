use std::io::{stdout, Write};
use std::path::Path;

use anyhow::Result;
use gat_scenarios::{load_spec_from_path, resolve_scenarios};
use serde_json;

pub fn handle(spec: &str, format: &str) -> Result<()> {
    let set = load_spec_from_path(Path::new(spec))?;
    let resolved = resolve_scenarios(&set)?;
    if format.eq_ignore_ascii_case("json") {
        let mut out = stdout();
        serde_json::to_writer_pretty(&mut out, &resolved)?;
        out.write_all(b"\n")?;
    } else {
        println!(
            "{:<30} {:<14} {:<8}",
            "scenario_id", "time_slices", "weight"
        );
        for scenario in resolved {
            println!(
                "{:<30} {:<14} {:<8.3}",
                scenario.scenario_id,
                scenario.time_slices.len(),
                scenario.weight
            );
        }
    }
    Ok(())
}
