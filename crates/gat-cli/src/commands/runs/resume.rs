use std::env;
use std::path::Path;
use std::process::Command;

use crate::runs::resolve_manifest;
use anyhow::Result;
use gat_cli::manifest::ManifestEntry;

use super::describe::describe_manifest;

pub fn handle(root: &Path, manifest: &str, execute: bool) -> Result<()> {
    let record = resolve_manifest(root, manifest)?;
    describe_manifest(&record.manifest);
    if execute {
        resume_manifest(&record.manifest)?;
        println!("Manifest {} resumed", record.manifest.run_id);
    } else {
        println!("Manifest {} ready (not executed)", record.manifest.run_id);
    }
    Ok(())
}

fn resume_manifest(manifest: &ManifestEntry) -> Result<()> {
    let mut args: Vec<String> = manifest
        .command
        .split_whitespace()
        .map(String::from)
        .collect();
    for param in &manifest.params {
        match param.name.as_str() {
            "grid_file" => args.push(param.value.clone()),
            _ => {
                args.push(format!("--{}", param.name));
                args.push(param.value.clone());
            }
        }
    }
    let exe = env::current_exe()?;
    let status = Command::new(exe).args(&args).status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("resumed run failed with {status}"));
    }
    Ok(())
}
