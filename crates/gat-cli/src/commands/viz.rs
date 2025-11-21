#![cfg(feature = "viz")]
use std::fs;

use anyhow::Result;
use tracing::info;

use gat_cli::cli::VizCommands;
use gat_io::importers;
use gat_viz::layout::layout_network;
use serde_json;

pub fn handle(command: &VizCommands) -> Result<()> {
    match command {
        VizCommands::Plot { grid_file, output } => {
            let network = importers::load_grid_from_arrow(grid_file)?;
            let layout = layout_network(&network, 150);
            let payload = serde_json::to_string_pretty(&layout)?;
            if let Some(path) = output {
                fs::write(path, &payload)?;
                println!("Layout written to {path}");
            } else {
                println!("{payload}");
            }
            info!("Visualization produced for {grid_file}");
            Ok(())
        }
    }
}
