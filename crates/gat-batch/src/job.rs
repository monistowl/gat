use chrono::{DateTime, Utc};
use gat_scenarios::manifest::ScenarioArtifact;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Batch task categories mirror the PF/OPF stages so downstream analytics keep the DOI:10.1109/TPWRS.2007.899019 naming conventions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskKind {
    PfDc,
    PfAc,
    OpfDc,
    OpfAc,
}

impl TaskKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskKind::PfDc => "pf-dc",
            TaskKind::PfAc => "pf-ac",
            TaskKind::OpfDc => "opf-dc",
            TaskKind::OpfAc => "opf-ac",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchJob {
    pub job_id: String,
    pub scenario_id: String,
    pub time: Option<DateTime<Utc>>,
    pub grid_file: PathBuf,
    pub tags: Vec<String>,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJobRecord {
    pub job_id: String,
    pub scenario_id: String,
    pub time: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub output: String,
}

pub fn jobs_from_artifacts(artifacts: &[ScenarioArtifact], task: TaskKind) -> Vec<BatchJob> {
    artifacts
        .iter()
        .map(|artifact| BatchJob {
            job_id: format!("{}:{}", task.as_str(), artifact.scenario_id),
            scenario_id: artifact.scenario_id.clone(),
            time: artifact.time_slices.first().cloned(),
            grid_file: PathBuf::from(&artifact.grid_file),
            tags: artifact.tags.clone(),
            weight: artifact.weight,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn make_artifact() -> ScenarioArtifact {
        ScenarioArtifact {
            scenario_id: "s1".into(),
            description: None,
            grid_file: "grid.arrow".into(),
            time_slices: vec!["2025-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()],
            load_scale: 1.0,
            renewable_scale: 1.0,
            weight: 1.0,
            tags: vec!["tag".into()],
            metadata: Default::default(),
        }
    }

    #[test]
    fn jobs_from_artifacts_builds_identifiers() {
        let artifacts = vec![make_artifact()];
        let jobs = jobs_from_artifacts(&artifacts, TaskKind::PfDc);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_id, "pf-dc:s1");
    }
}
