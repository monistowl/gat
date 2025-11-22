use anyhow::Result;
use gat_cli::cli::AnalyticsCommands;

pub mod ds;
pub mod ptdf;
pub mod reliability;
pub mod elcc; // NEW

pub fn handle(command: &AnalyticsCommands) -> Result<()> {
    match command {
        AnalyticsCommands::Ptdf { .. } => ptdf::handle(command),
        AnalyticsCommands::Ds { .. } => ds::handle(command),
        AnalyticsCommands::Reliability { .. } => reliability::handle(command),
        AnalyticsCommands::Elcc { .. } => elcc::handle(command), // NEW
    }
}
