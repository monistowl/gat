use anyhow::Result;
use gat_cli::cli::ScenariosCommands;

pub mod expand;
pub mod list;
pub mod materialize;
pub mod validate;

pub fn handle(command: &ScenariosCommands) -> Result<()> {
    match command {
        ScenariosCommands::Validate { spec } => validate::handle(spec),
        ScenariosCommands::List { spec, format } => list::handle(spec, format),
        ScenariosCommands::Expand {
            spec,
            grid_file,
            out,
        } => expand::handle(spec, grid_file.as_deref(), out),
        ScenariosCommands::Materialize {
            spec,
            grid_file,
            out_dir,
            drop_outaged,
        } => materialize::handle(spec, grid_file.as_deref(), out_dir, *drop_outaged),
    }
}
