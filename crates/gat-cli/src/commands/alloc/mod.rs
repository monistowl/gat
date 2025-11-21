use anyhow::Result;
use gat_cli::cli::AllocCommands;

pub mod rents;
pub mod kpi;

pub fn handle(command: &AllocCommands) -> Result<()> {
    match command {
        AllocCommands::Rents { .. } => rents::handle(command),
        AllocCommands::Kpi { .. } => kpi::handle(command),
    }
}
