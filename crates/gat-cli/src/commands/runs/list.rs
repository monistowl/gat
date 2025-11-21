use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use gat_cli::cli::RunFormat;
use tabwriter::TabWriter;

use crate::runs::{discover_runs, summaries, RunRecord};

pub fn handle(root: &Path, format: RunFormat) -> Result<()> {
    let records = discover_runs(root)?;
    match format {
        RunFormat::Plain => print_run_table(&records),
        RunFormat::Json => print_run_json(&records),
    }
}

fn print_run_table(records: &[RunRecord]) -> Result<()> {
    let mut writer = TabWriter::new(io::stdout());
    writeln!(writer, "RUN ID\tCOMMAND\tTIMESTAMP\tVERSION\tMANIFEST")?;
    for record in records {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}",
            record.manifest.run_id,
            record.manifest.command,
            record.manifest.timestamp,
            record.manifest.version,
            record.path.display(),
        )?;
    }
    writer.flush()?;
    Ok(())
}

fn print_run_json(records: &[RunRecord]) -> Result<()> {
    let runs = summaries(records);
    serde_json::to_writer_pretty(io::stdout(), &runs)
        .map_err(|err| anyhow::anyhow!("serializing run list to JSON: {err}"))?;
    println!();
    Ok(())
}
