#![cfg(feature = "gui")]
use anyhow::Result;
use tracing::info;

use gat_cli::cli::GuiCommands;

pub fn handle(command: &GuiCommands) -> Result<()> {
    match command {
        GuiCommands::Run { grid_file, output } => {
            let summary = gat_gui::launch(output.as_deref())?;
            info!("gat-gui reported summary for {grid_file}: {summary}");
            Ok(())
        }
    }
}
