#![cfg(feature = "tui")]
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use dirs::config_dir;
use tracing::info;

use gat_cli::cli::TuiCommands;

const TUI_CONFIG_TEMPLATE: &str = "\
poll_secs=1
solver=gauss
verbose=false
command=cargo run -p gat-cli -- --help
";

fn default_tui_config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("gat-tui").join("config.toml"))
}

fn write_tui_config(out: Option<&str>) -> Result<PathBuf> {
    let target = out
        .map(PathBuf::from)
        .or_else(default_tui_config_path)
        .ok_or_else(|| anyhow!("unable to determine gat-tui config path"))?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, TUI_CONFIG_TEMPLATE)?;
    Ok(target)
}

pub fn handle(command: &TuiCommands) -> Result<()> {
    match command {
        TuiCommands::Config { out } => {
            let path = write_tui_config(out.as_deref())?;
            info!("gat-tui config written to {}", path.display());
            Ok(())
        }
    }
}
