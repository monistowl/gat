use std::io;
use std::path::Path;

use anyhow::Result;
use gat_cli::cli::RunFormat;
use gat_cli::manifest::ManifestEntry;
use serde_json;

use crate::runs::resolve_manifest;

pub fn handle(root: &Path, target: &str, format: RunFormat) -> Result<()> {
    let record = resolve_manifest(root, target)?;
    match format {
        RunFormat::Plain => describe_manifest(&record.manifest),
        RunFormat::Json => {
            serde_json::to_writer_pretty(io::stdout(), &record.manifest)
                .map_err(|err| anyhow::anyhow!("serializing manifest: {err}"))?;
            println!();
        }
    }
    Ok(())
}

pub fn describe_manifest(manifest: &ManifestEntry) {
    println!(
        "Manifest {} (cmd: `{}` @ v{} from {})",
        manifest.run_id, manifest.command, manifest.version, manifest.timestamp
    );
    if let Some(seed) = &manifest.seed {
        println!("Seed: {seed}");
    }
    if !manifest.params.is_empty() {
        println!("Parameters:");
        for param in &manifest.params {
            println!("  {} = {}", param.name, param.value);
        }
    }
    if !manifest.inputs.is_empty() {
        println!("Inputs:");
        for input in &manifest.inputs {
            let hash = input.hash.as_deref().unwrap_or("unknown");
            println!("  {} ({})", input.path, hash);
        }
    }
    if !manifest.outputs.is_empty() {
        println!("Outputs:");
        for output in &manifest.outputs {
            println!("  {output}");
        }
    }
    if manifest.chunk_map.is_empty() {
        println!("Chunk map entries: 0");
    } else {
        println!("Chunk map:");
        for chunk in &manifest.chunk_map {
            let when = chunk.completed_at.as_deref().unwrap_or("pending");
            println!("  {} -> {} ({})", chunk.id, chunk.status, when);
        }
    }
}
