use anyhow::Result;
use gat_cli::cli::GeoCommands;

pub mod featurize;
pub mod join;

pub fn handle(command: &GeoCommands) -> Result<()> {
    match command {
        GeoCommands::Join { .. } => join::handle(command),
        GeoCommands::Featurize { .. } => featurize::handle(command),
    }
}
