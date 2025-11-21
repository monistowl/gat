use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::job::BatchJobRecord;

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchManifest {
    pub created_at: DateTime<Utc>,
    pub task: String,
    pub num_jobs: usize,
    pub success: usize,
    pub failure: usize,
    pub jobs: Vec<BatchJobRecord>,
}

pub fn write_batch_manifest(path: &Path, manifest: &BatchManifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating manifest directory '{}'", parent.display()))?;
    }
    let json =
        serde_json::to_string_pretty(manifest).context("serializing batch manifest to JSON")?;
    fs::write(path, json)
        .with_context(|| format!("writing batch manifest '{}'", path.display()))?;
    Ok(())
}

pub fn load_batch_manifest(path: &Path) -> Result<BatchManifest> {
    use std::fs::File;
    use serde_json;
    let file = File::open(path)
        .with_context(|| format!("opening batch manifest '{}'", path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("parsing batch manifest '{}'", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn manifest_writes_and_reads_back() {
        let record = BatchJobRecord {
            job_id: "pf-dc:s1".into(),
            scenario_id: "s1".into(),
            time: None,
            status: "ok".into(),
            error: None,
            output: "out/result.parquet".into(),
        };
        let manifest = BatchManifest {
            created_at: Utc::now(),
            task: "pf-dc".into(),
            num_jobs: 1,
            success: 1,
            failure: 0,
            jobs: vec![record.clone()],
        };
        let tmp = NamedTempFile::new().unwrap();
        write_batch_manifest(tmp.path(), &manifest).unwrap();
        let text = fs::read_to_string(tmp.path()).unwrap();
        let parsed: BatchManifest = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.task, "pf-dc");
        assert_eq!(parsed.jobs.first().unwrap().job_id, record.job_id);
    }
}
