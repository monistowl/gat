use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
struct ManifestEntry {
    run_id: String,
    command: String,
    version: &'static str,
    timestamp: String,
    outputs: Vec<String>,
    params: Vec<Param>,
}

#[derive(Serialize)]
struct Param {
    name: String,
    value: String,
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
        version: env!("CARGO_PKG_VERSION"),
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
