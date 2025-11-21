use anyhow::Result;
use gat_cli::cli::FeaturizeCommands;

pub mod gnn;

pub fn handle(command: &FeaturizeCommands) -> Result<()> {
    match command {
        FeaturizeCommands::Gnn { .. } => gnn::handle(command),
    }
}
