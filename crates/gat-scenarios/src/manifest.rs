use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use gat_io::importers;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::Path;

use crate::apply::apply_scenario_to_network;
use crate::apply::ScenarioApplyOptions;
use crate::spec::ResolvedScenario;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioArtifact {
    pub scenario_id: String,
    pub description: Option<String>,
    pub grid_file: String,
    pub time_slices: Vec<DateTime<Utc>>,
    pub load_scale: f64,
    pub renewable_scale: f64,
    pub weight: f64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Materialize scenarios by applying them to a base grid and saving per-scenario artifacts.
///
/// **Algorithm:**
/// 1. For each resolved scenario, load the base grid.
/// 2. Apply scenario modifications (outages, scaling) to create a scenario-specific grid.
/// 3. Save the modified grid as `out_dir/<scenario_id>/grid.arrow`.
/// 4. Collect metadata into `ScenarioArtifact` records.
/// 5. Write a manifest JSON file listing all artifacts.
///
/// **Output structure:**
/// ```text
/// out_dir/
///   scenario_manifest.json          # Summary of all scenarios
///   <scenario_id>/
///     grid.arrow                    # Scenario-specific grid snapshot
/// ```
///
/// This manifest can be consumed by `gat batch` to run PF/OPF across all scenarios.
pub fn materialize_scenarios(
    grid_file: &Path,
    out_dir: &Path,
    scenarios: &[ResolvedScenario],
    options: &ScenarioApplyOptions,
) -> Result<Vec<ScenarioArtifact>> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("creating scenario output directory '{}'", out_dir.display()))?;
    let grid_input = grid_file.to_str().ok_or_else(|| {
        anyhow!(
            "grid file path '{}' is not valid unicode",
            grid_file.display()
        )
    })?;
    let mut artifacts = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let mut network = importers::load_grid_from_arrow(grid_input)?;
        apply_scenario_to_network(&mut network, scenario, options)?;
        let scenario_dir = out_dir.join(sanitize_name(&scenario.scenario_id));
        fs::create_dir_all(&scenario_dir)
            .with_context(|| format!("creating scenario directory '{}'", scenario_dir.display()))?;
        let grid_path = scenario_dir.join("grid.arrow");
        let grid_path_str = grid_path.display().to_string();
        importers::export_network_to_arrow(&network, &grid_path_str)?;
        artifacts.push(ScenarioArtifact {
            scenario_id: scenario.scenario_id.clone(),
            description: scenario.description.clone(),
            grid_file: grid_path_str,
            time_slices: scenario.time_slices.clone(),
            load_scale: scenario.load_scale,
            renewable_scale: scenario.renewable_scale,
            weight: scenario.weight,
            tags: scenario.tags.clone(),
            metadata: scenario.metadata.clone(),
        });
    }
    let manifest_path = out_dir.join("scenario_manifest.json");
    write_manifest(&manifest_path, &artifacts)?;
    Ok(artifacts)
}

pub fn write_manifest(path: &Path, artifacts: &[ScenarioArtifact]) -> Result<()> {
    let file = fs::File::create(path)
        .with_context(|| format!("creating scenario manifest '{}'", path.display()))?;
    serde_json::to_writer_pretty(file, artifacts)
        .with_context(|| format!("writing scenario manifest '{}'", path.display()))?;
    Ok(())
}

pub fn load_manifest(path: &Path) -> Result<Vec<ScenarioArtifact>> {
    let file = File::open(path)
        .with_context(|| format!("opening scenario manifest '{}'", path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("parsing scenario manifest '{}'", path.display()))
}

fn sanitize_name(value: &str) -> String {
    let filtered: String = value
        .chars()
        .map(|c| if matches!(c, '/' | '\\') { '_' } else { c })
        .collect();
    if filtered.is_empty() {
        "scenario".to_string()
    } else {
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn writes_and_reads_manifest() {
        let artifact = ScenarioArtifact {
            scenario_id: "test".into(),
            description: Some("desc".into()),
            grid_file: "grid.arrow".into(),
            time_slices: vec!["2025-01-01T00:00:00Z".parse().unwrap()],
            load_scale: 1.0,
            renewable_scale: 1.0,
            weight: 1.0,
            tags: vec!["foo".into()],
            metadata: HashMap::new(),
        };
        let tmp = NamedTempFile::new().unwrap();
        write_manifest(tmp.path(), &[artifact.clone()]).unwrap();
        let text = fs::read_to_string(tmp.path()).unwrap();
        let parsed: Vec<ScenarioArtifact> = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.first().unwrap().scenario_id, "test");
    }
}
