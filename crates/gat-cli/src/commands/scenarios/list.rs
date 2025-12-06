use std::io::stdout;
use std::path::Path;

use anyhow::Result;
use gat_scenarios::{load_spec_from_path, resolve_scenarios};
use serde_json;

use gat_cli::common::{write_csv_from_json, write_json, write_jsonl, OutputFormat};

pub fn handle(spec: &str, format: &OutputFormat) -> Result<()> {
    let set = load_spec_from_path(Path::new(spec))?;
    let resolved = resolve_scenarios(&set)?;

    match format {
        OutputFormat::Table => {
            println!(
                "{:<30} {:<14} {:<8}",
                "scenario_id", "time_slices", "weight"
            );
            for scenario in &resolved {
                println!(
                    "{:<30} {:<14} {:<8.3}",
                    scenario.scenario_id,
                    scenario.time_slices.len(),
                    scenario.weight
                );
            }
        }
        OutputFormat::Json => {
            write_json(&resolved, &mut stdout(), true)?;
        }
        OutputFormat::Jsonl => {
            write_jsonl(&resolved, &mut stdout())?;
        }
        OutputFormat::Csv => {
            // Convert resolved scenarios to JSON values for CSV output
            let json_data: Vec<serde_json::Value> = resolved
                .iter()
                .map(|scenario| {
                    serde_json::json!({
                        "scenario_id": scenario.scenario_id,
                        "time_slices": scenario.time_slices.len(),
                        "weight": scenario.weight,
                    })
                })
                .collect();
            write_csv_from_json(&json_data, &mut stdout())?;
        }
    }
    Ok(())
}
