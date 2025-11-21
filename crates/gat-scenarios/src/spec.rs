use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSet {
    pub version: Option<u32>,
    pub grid_file: Option<String>,
    #[serde(default)]
    pub defaults: ScenarioDefaults,
    #[serde(default)]
    pub scenarios: Vec<ScenarioSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefaults {
    #[serde(default = "default_scale")]
    pub load_scale: f64,
    #[serde(default = "default_scale")]
    pub renewable_scale: f64,
    #[serde(default)]
    pub time_slices: Vec<String>,
    #[serde(default = "default_weight")]
    pub weight: f64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_scale() -> f64 {
    1.0
}

fn default_weight() -> f64 {
    1.0
}

impl Default for ScenarioDefaults {
    fn default() -> Self {
        Self {
            load_scale: default_scale(),
            renewable_scale: default_scale(),
            time_slices: Vec::new(),
            weight: default_weight(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub scenario_id: String,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub outages: Vec<OutageSpec>,
    #[serde(default)]
    pub dispatch_overrides: Option<Vec<DispatchOverrideSpec>>,
    pub load_scale: Option<f64>,
    pub renewable_scale: Option<f64>,
    #[serde(default)]
    pub time_slices: Option<Vec<String>>,
    pub weight: Option<f64>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutageSpec {
    Branch { id: String },
    Gen { id: String },
    Bus { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchOverrideSpec {
    pub resource_id: String,
    pub p_max_mw: Option<f64>,
    pub p_min_mw: Option<f64>,
    pub must_run: Option<bool>,
    pub cost_multiplier: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedScenario {
    pub scenario_id: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub outages: Vec<OutageSpec>,
    pub dispatch_overrides: Vec<DispatchOverrideSpec>,
    pub load_scale: f64,
    pub renewable_scale: f64,
    pub time_slices: Vec<DateTime<Utc>>,
    pub weight: f64,
    pub metadata: HashMap<String, String>,
}

pub fn load_spec_from_path(path: &Path) -> Result<ScenarioSet> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("reading scenario spec '{}'", path.display()))?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") => {
            serde_yaml::from_str(&data).context("parsing scenario spec yaml")
        }
        Some(ext) if ext.eq_ignore_ascii_case("json") => {
            serde_json::from_str(&data).context("parsing scenario spec json")
        }
        _ => serde_yaml::from_str(&data)
            .or_else(|_| serde_json::from_str(&data))
            .context("parsing scenario spec"),
    }
}

pub fn resolve_scenarios(set: &ScenarioSet) -> Result<Vec<ResolvedScenario>> {
    if set.scenarios.is_empty() {
        return Err(anyhow!("scenario set contains no scenarios"));
    }
    let defaults = set.defaults.clone();
    let mut seen = HashSet::new();
    let mut resolved = Vec::with_capacity(set.scenarios.len());
    for scenario in &set.scenarios {
        if scenario.scenario_id.trim().is_empty() {
            return Err(anyhow!("scenario_id cannot be empty"));
        }
        if !seen.insert(scenario.scenario_id.clone()) {
            return Err(anyhow!(
                "duplicate scenario_id '{}' in spec",
                scenario.scenario_id
            ));
        }
        let time_strings = scenario
            .time_slices
            .as_ref()
            .map(|t| t.as_slice())
            .unwrap_or_else(|| defaults.time_slices.as_slice());
        if time_strings.is_empty() {
            return Err(anyhow!(
                "scenario '{}' must declare at least one time slice",
                scenario.scenario_id
            ));
        }
        let time_slices = parse_time_slices(time_strings).with_context(|| {
            format!(
                "parsing time slices for scenario '{}'",
                scenario.scenario_id
            )
        })?;
        let tags = scenario
            .tags
            .as_ref()
            .cloned()
            .unwrap_or_else(|| defaults.tags.clone());
        let metadata = scenario
            .metadata
            .as_ref()
            .cloned()
            .unwrap_or_else(|| defaults.metadata.clone());
        let dispatch_overrides = scenario
            .dispatch_overrides
            .as_ref()
            .cloned()
            .unwrap_or_default();
        resolved.push(ResolvedScenario {
            scenario_id: scenario.scenario_id.clone(),
            description: scenario.description.clone(),
            tags,
            outages: scenario.outages.clone(),
            dispatch_overrides,
            load_scale: scenario.load_scale.unwrap_or(defaults.load_scale),
            renewable_scale: scenario.renewable_scale.unwrap_or(defaults.renewable_scale),
            time_slices,
            weight: scenario.weight.unwrap_or(defaults.weight),
            metadata,
        });
    }
    Ok(resolved)
}

pub fn validate(set: &ScenarioSet) -> Result<()> {
    resolve_scenarios(set).map(|_| ())
}

fn parse_time_slices(values: &[String]) -> Result<Vec<DateTime<Utc>>> {
    let mut slices = Vec::with_capacity(values.len());
    for value in values {
        let parsed = DateTime::parse_from_rfc3339(value)
            .or_else(|_| DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
            .with_context(|| format!("parsing time slice '{}'; use RFC3339", value))?
            .with_timezone(&Utc);
        slices.push(parsed);
    }
    Ok(slices)
}
