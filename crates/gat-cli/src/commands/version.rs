use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};
use gat_cli::cli::VersionCommands;
use serde_json::json;

fn canonical_version() -> Result<String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("cargo metadata failed"));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    if let Some(version) = metadata
        .get("metadata")
        .and_then(|m| m.get("release"))
        .and_then(|r| r.get("version"))
        .and_then(|v| v.as_str())
    {
        Ok(version.to_string())
    } else {
        Err(anyhow!("workspace_metadata.release.version is not set"))
    }
}

fn release_tag_to_version(tag: &str) -> String {
    if tag.starts_with('v') {
        tag.trim_start_matches('v').to_string()
    } else {
        tag.to_string()
    }
}

pub fn handle(command: &VersionCommands) -> Result<()> {
    let VersionCommands::Sync { tag, manifest } = command;
    let version = canonical_version()?;
    println!("{version}");

    if let Some(tag_value) = tag.as_ref() {
        let expected = release_tag_to_version(tag_value);
        if expected != version {
            return Err(anyhow!(
                "tag {} resolves to {}, but canonical version is {}",
                tag_value,
                expected,
                version
            ));
        }
    }

    if let Some(path) = manifest {
        let json = json!({
            "version": version,
            "tag": tag,
        });
        fs::write(Path::new(path), serde_json::to_string_pretty(&json)?)?;
    }

    Ok(())
}
