use std::{
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ManifestEntry {
    pub run_id: String,
    pub command: String,
    pub version: String,
    pub timestamp: String,
    pub outputs: Vec<String>,
    pub params: Vec<Param>,
}

#[derive(Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub value: String,
}

pub fn record_manifest(output: &Path, command: &str, params: &[(&str, &str)]) -> Result<()> {
    let run_id = Uuid::new_v4().to_string();
    let dir = output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    fs::create_dir_all(&dir)?;
    let manifest = ManifestEntry {
        run_id: run_id.clone(),
        command: command.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now().to_rfc3339(),
        outputs: vec![output.display().to_string()],
        params: params
            .iter()
            .map(|(k, v)| Param {
                name: k.to_string(),
                value: v.to_string(),
            })
            .collect(),
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    let path = dir.join(format!("run-{}.json", run_id));
    fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    println!("Recorded run manifest {}", path.display());
    Ok(())
}

pub fn read_manifest(path: &Path) -> Result<ManifestEntry> {
    let json = fs::read_to_string(path)?;
    let manifest = serde_json::from_str(&json)?;
    Ok(manifest)
}
