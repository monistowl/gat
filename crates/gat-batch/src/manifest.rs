use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::job::BatchJobRecord;

/// Aggregated statistics for a batch run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStats {
    /// Total execution time in milliseconds
    pub total_time_ms: f64,
    /// Minimum job execution time in milliseconds
    pub min_time_ms: f64,
    /// Maximum job execution time in milliseconds
    pub max_time_ms: f64,
    /// Mean job execution time in milliseconds
    pub mean_time_ms: f64,
    /// Median job execution time in milliseconds
    pub median_time_ms: f64,
    /// 95th percentile execution time in milliseconds
    pub p95_time_ms: f64,
    /// For AC solvers: average iterations to convergence
    pub avg_iterations: Option<f64>,
    /// For AC solvers: convergence rate (converged / total AC jobs)
    pub convergence_rate: Option<f64>,
}

impl BatchStats {
    /// Compute statistics from a list of job records.
    pub fn from_jobs(jobs: &[BatchJobRecord], total_time_ms: f64) -> Self {
        let mut times: Vec<f64> = jobs.iter().filter_map(|j| j.duration_ms).collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let (min_time_ms, max_time_ms, mean_time_ms, median_time_ms, p95_time_ms) =
            if times.is_empty() {
                (0.0, 0.0, 0.0, 0.0, 0.0)
            } else {
                let sum: f64 = times.iter().sum();
                let mean = sum / times.len() as f64;
                let median = if times.len() % 2 == 0 {
                    (times[times.len() / 2 - 1] + times[times.len() / 2]) / 2.0
                } else {
                    times[times.len() / 2]
                };
                let p95_idx = ((times.len() as f64) * 0.95).ceil() as usize;
                let p95 = times[p95_idx.min(times.len() - 1)];
                (times[0], times[times.len() - 1], mean, median, p95)
            };

        // Compute AC solver statistics if available
        let ac_jobs: Vec<_> = jobs.iter().filter(|j| j.iterations.is_some()).collect();
        let (avg_iterations, convergence_rate) = if ac_jobs.is_empty() {
            (None, None)
        } else {
            let total_iters: u32 = ac_jobs.iter().filter_map(|j| j.iterations).sum();
            let avg = total_iters as f64 / ac_jobs.len() as f64;
            let converged_count = ac_jobs.iter().filter(|j| j.converged == Some(true)).count();
            let rate = converged_count as f64 / ac_jobs.len() as f64;
            (Some(avg), Some(rate))
        };

        BatchStats {
            total_time_ms,
            min_time_ms,
            max_time_ms,
            mean_time_ms,
            median_time_ms,
            p95_time_ms,
            avg_iterations,
            convergence_rate,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchManifest {
    pub created_at: DateTime<Utc>,
    pub task: String,
    pub num_jobs: usize,
    pub success: usize,
    pub failure: usize,
    pub jobs: Vec<BatchJobRecord>,
    /// Aggregated statistics (populated after all jobs complete)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<BatchStats>,
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
    use serde_json;
    use std::fs::File;
    let file =
        File::open(path).with_context(|| format!("opening batch manifest '{}'", path.display()))?;
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
            duration_ms: Some(42.5),
            iterations: None,
            converged: None,
        };
        let manifest = BatchManifest {
            created_at: Utc::now(),
            task: "pf-dc".into(),
            num_jobs: 1,
            success: 1,
            failure: 0,
            jobs: vec![record.clone()],
            stats: None,
        };
        let tmp = NamedTempFile::new().unwrap();
        write_batch_manifest(tmp.path(), &manifest).unwrap();
        let text = fs::read_to_string(tmp.path()).unwrap();
        let parsed: BatchManifest = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.task, "pf-dc");
        assert_eq!(parsed.jobs.first().unwrap().job_id, record.job_id);
        assert_eq!(parsed.jobs.first().unwrap().duration_ms, Some(42.5));
    }
}
