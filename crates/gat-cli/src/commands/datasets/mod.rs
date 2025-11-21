use std::path::Path;

use anyhow::Result;
use gat_cli::cli::DatasetCommands;

pub mod archives;
pub mod catalog;
pub mod formats;

pub fn handle(command: &DatasetCommands) -> Result<()> {
    match command {
        DatasetCommands::RtsGmlc { command } => archives::handle_rts_gmlc(command),
        DatasetCommands::Hiren { command } => archives::handle_hiren(command),
        DatasetCommands::Dsgrid { out } => formats::handle_dsgrid(Path::new(out)),
        DatasetCommands::Sup3rcc { command } => archives::handle_sup3rcc(command),
        DatasetCommands::Public { command } => catalog::handle_public(command),
        DatasetCommands::Pras { path, out } => {
            formats::handle_pras(Path::new(path), Path::new(out))
        }
    }
}
