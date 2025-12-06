use std::io;
use std::path::Path;

use anyhow::Result;
use gat_cli::common::OutputFormat;
use gat_cli::manifest::ManifestEntry;
use serde_json;

use crate::runs::resolve_manifest;

pub fn handle(root: &Path, target: &str, format: OutputFormat) -> Result<()> {
    let record = resolve_manifest(root, target)?;
    match format {
        OutputFormat::Table => describe_manifest(&record.manifest),
        OutputFormat::Json => {
            serde_json::to_writer_pretty(io::stdout(), &record.manifest)
                .map_err(|err| anyhow::anyhow!("serializing manifest: {err}"))?;
            println!();
        }
        OutputFormat::Jsonl => {
            serde_json::to_writer(io::stdout(), &record.manifest)
                .map_err(|err| anyhow::anyhow!("serializing manifest: {err}"))?;
            println!();
        }
        OutputFormat::Csv => {
            // For CSV, we'll output manifest as a single-row CSV with key columns
            println!("run_id,command,timestamp,version");
            println!(
                "{},{},{},{}",
                record.manifest.run_id,
                csv_escape(&record.manifest.command),
                record.manifest.timestamp,
                record.manifest.version
            );
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

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
