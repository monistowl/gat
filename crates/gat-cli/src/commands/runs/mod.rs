use anyhow::Result;
use gat_cli::cli::RunsCommands;

pub mod describe;
pub mod list;
pub mod resume;

pub fn handle(command: &RunsCommands) -> Result<()> {
    match command {
        RunsCommands::List { root, format } => list::handle(root.as_path(), *format),
        RunsCommands::Describe {
            target,
            root,
            format,
        } => describe::handle(root.as_path(), target.as_str(), *format),
        RunsCommands::Resume {
            root,
            manifest,
            execute,
        } => resume::handle(root.as_path(), manifest.as_str(), *execute),
    }
}
