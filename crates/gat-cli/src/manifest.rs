use std::{env, fs, io::Read, path::Path};

use anyhow::{Context, Result};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ManifestEntry {
    pub run_id: String,
    pub command: String,
    pub version: String,
    pub timestamp: String,
    pub seed: Option<String>,
    #[serde(default)]
    pub inputs: Vec<InputArtifact>,
    pub outputs: Vec<String>,
    pub params: Vec<Param>,
    #[serde(default)]
    pub chunk_map: Vec<ChunkState>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct InputArtifact {
    pub path: String,
    pub hash: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ChunkState {
    pub id: String,
    pub status: String,
    pub completed_at: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
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
    let params_vec: Vec<Param> = params
        .iter()
        .map(|(k, v)| Param {
            name: k.to_string(),
            value: v.to_string(),
        })
        .collect();
    let inputs = gather_inputs(&params_vec);
    let seed = env::var("GAT_RUN_SEED")
        .or_else(|_| env::var("GAT_SEED"))
        .ok();
    let manifest = ManifestEntry {
        run_id: run_id.clone(),
        command: command.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now().to_rfc3339(),
        seed,
        inputs,
        outputs: vec![output.display().to_string()],
        params: params_vec,
        chunk_map: Vec::new(),
    };
    let json = serde_json::to_string_pretty(&manifest)?;
    let path = dir.join(format!("run-{}.json", run_id));
    fs::write(&path, &json).with_context(|| format!("writing {}", path.display()))?;
    let canonical = dir.join("run.json");
    fs::write(&canonical, &json).with_context(|| format!("writing {}", canonical.display()))?;
    println!("Recorded run manifest {}", canonical.display());
    Ok(())
}

pub fn read_manifest(path: &Path) -> Result<ManifestEntry> {
    let json = fs::read_to_string(path)?;
    let manifest = serde_json::from_str(&json)?;
    Ok(manifest)
}

pub fn manifest_json_schema() -> serde_json::Value {
    let schema = schemars::schema_for!(ManifestEntry);
    serde_json::to_value(&schema).expect("failed to serialize manifest schema")
}

fn gather_inputs(params: &[Param]) -> Vec<InputArtifact> {
    params
        .iter()
        .filter_map(|param| {
            let path = Path::new(&param.value);
            if path.exists() && path.is_file() {
                Some(InputArtifact {
                    path: path.display().to_string(),
                    hash: hash_file(path).ok(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
