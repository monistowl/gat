use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::ImportCommands;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;

pub fn handle(command: &ImportCommands) -> Result<()> {
    match command {
        ImportCommands::Psse { raw, output } => {
            let start = Instant::now();
            let res = importers::import_psse_raw(raw, output).map(|_| ());
            record_run_timed(
                output,
                "import psse",
                &[("raw", raw), ("output", output)],
                start,
                &res,
            );
            res
        }
        ImportCommands::Matpower { m, output } => {
            let start = Instant::now();
            let res = importers::import_matpower_case(m, output).map(|_| ());
            record_run_timed(
                output,
                "import matpower",
                &[("case", m), ("output", output)],
                start,
                &res,
            );
            res
        }
        ImportCommands::Cim { rdf, output } => {
            let start = Instant::now();
            let res = importers::import_cim_rdf(rdf, output).map(|_| ());
            record_run_timed(
                output,
                "import cim",
                &[("rdf", rdf), ("output", output)],
                start,
                &res,
            );
            res
        }
    }
}
