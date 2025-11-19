use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

use crate::manifest::{self, ManifestEntry};

#[derive(Clone, Debug)]
pub struct RunRecord {
    pub manifest: ManifestEntry,
    pub path: PathBuf,
}

#[derive(Serialize)]
pub struct RunSummary {
    pub run_id: String,
    pub command: String,
    pub timestamp: String,
    pub version: String,
    pub manifest_path: String,
    pub outputs: Vec<String>,
}

impl RunSummary {
    pub fn from_record(record: &RunRecord) -> Self {
        Self {
            run_id: record.manifest.run_id.clone(),
            command: record.manifest.command.clone(),
            timestamp: record.manifest.timestamp.clone(),
            version: record.manifest.version.clone(),
            manifest_path: record.path.display().to_string(),
            outputs: record.manifest.outputs.clone(),
        }
    }
}

pub fn discover_runs(root: &Path) -> Result<Vec<RunRecord>> {
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut runs = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| should_enter(entry))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if is_manifest_file(&entry) {
            let path = entry.path().to_path_buf();
            let manifest = manifest::read_manifest(&path)
                .with_context(|| format!("reading manifest {}", path.display()))?;
            runs.push(RunRecord { manifest, path });
        }
    }
    Ok(runs)
}

pub fn resolve_manifest(root: &Path, target: &str) -> Result<RunRecord> {
    if target.is_empty() {
        return Err(anyhow!("manifest target cannot be empty"));
    }

    let candidate = PathBuf::from(target);
    if candidate.exists() && candidate.is_file() {
        return load_manifest(&candidate);
    }

    let root_candidate = root.join(target);
    if root_candidate.exists() && root_candidate.is_file() {
        return load_manifest(&root_candidate);
    }

    let runs = discover_runs(root)?;
    if let Some(record) = runs.iter().find(|record| record.manifest.run_id == target) {
        return Ok(record.clone());
    }

    if let Some(record) = runs
        .iter()
        .find(|record| record.path.file_stem().map_or(false, |stem| stem == target))
    {
        return Ok(record.clone());
    }

    Err(anyhow!("run manifest not found for '{}'", target))
}

pub fn summaries(records: &[RunRecord]) -> Vec<RunSummary> {
    records.iter().map(RunSummary::from_record).collect()
}

fn load_manifest(path: &Path) -> Result<RunRecord> {
    let manifest = manifest::read_manifest(path)
        .with_context(|| format!("reading manifest {}", path.display()))?;
    Ok(RunRecord {
        manifest,
        path: path.to_path_buf(),
    })
}

fn is_manifest_file(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    name == "run.json" || (name.starts_with("run-") && name.ends_with(".json"))
}

fn should_enter(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    if entry.file_type().is_dir() {
        match entry.file_name().to_str() {
            Some(".git") | Some("target") | Some("node_modules") => false,
            Some(name) if name.starts_with('.') => false,
            _ => true,
        }
    } else {
        true
    }
}
