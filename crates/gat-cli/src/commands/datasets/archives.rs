use std::path::Path;

use anyhow::Result;
use gat_cli::cli::{HirenCommands, RtsGmlcCommands, Sup3rccCommands};

use crate::dataset::{fetch_hiren, fetch_rts_gmlc, fetch_sup3rcc, list_hiren, sample_sup3rcc_grid};

pub fn handle_rts_gmlc(command: &RtsGmlcCommands) -> Result<()> {
    match command {
        RtsGmlcCommands::Fetch { out, tag } => fetch_rts_gmlc(Path::new(out), tag.as_deref()),
    }
}

pub fn handle_hiren(command: &HirenCommands) -> Result<()> {
    match command {
        HirenCommands::List => {
            let cases = list_hiren().unwrap_or_default();
            for case in cases {
                println!("{case}");
            }
            Ok(())
        }
        HirenCommands::Fetch { case, out } => fetch_hiren(case, Path::new(out)),
    }
}

pub fn handle_sup3rcc(command: &Sup3rccCommands) -> Result<()> {
    match command {
        Sup3rccCommands::Fetch { out } => fetch_sup3rcc(Path::new(out)),
        Sup3rccCommands::Sample { grid, out } => {
            sample_sup3rcc_grid(Path::new(grid), Path::new(out))
        }
    }
}
