use anyhow::Result;
use gat_cli::cli::AllocCommands;

pub mod kpi;
pub mod rents;

pub fn handle(command: &AllocCommands) -> Result<()> {
    match command {
        AllocCommands::Rents { .. } => rents::handle(command),
        AllocCommands::Kpi { .. } => kpi::handle(command),
    }
}
