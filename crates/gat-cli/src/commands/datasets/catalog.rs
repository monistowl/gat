use anyhow::Result;
use gat_cli::cli::PublicDatasetCommands;

pub mod describe;
pub mod fetch;
pub mod list;

pub fn handle_public(command: &PublicDatasetCommands) -> Result<()> {
    match command {
        PublicDatasetCommands::List { tag, query } => list::handle(tag.as_ref(), query.as_ref()),
        PublicDatasetCommands::Describe { id } => describe::handle(id),
        PublicDatasetCommands::Fetch {
            id,
            out,
            force,
            extract,
        } => fetch::handle(id, out.as_ref(), *extract, *force),
    }
}
