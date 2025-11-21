use anyhow::Result;
use gat_cli::cli::BatchCommands;

pub mod opf;
pub mod pf;

pub fn handle(command: &BatchCommands) -> Result<()> {
    match command {
        BatchCommands::Pf { .. } => pf::handle(command),
        BatchCommands::Opf { .. } => opf::handle(command),
    }
}
