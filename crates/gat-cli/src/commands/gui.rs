#![cfg(feature = "gui")]
use std::path::PathBuf;

use anyhow::Result;
use tracing::info;

use gat_cli::cli::GuiCommands;

pub fn handle(command: &GuiCommands) -> Result<()> {
    match command {
        GuiCommands::Run { grid_file, output } => {
            // Use the output path as workspace if provided, otherwise use the grid file's directory
            let workspace = match output {
                Some(path) => PathBuf::from(path),
                None => PathBuf::from(grid_file)
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(".")),
            };

            let options = gat_notebook::NotebookOptions::with_workspace(&workspace);
            let app = gat_gui::GuiApp::Notebook(options);
            let report = gat_gui::launch(app)?;

            info!("gat-gui launched for {grid_file}: {report}");
            Ok(())
        }
    }
}
