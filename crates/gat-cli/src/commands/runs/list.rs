use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use gat_cli::common::OutputFormat;
use tabwriter::TabWriter;

use crate::runs::{discover_runs, summaries, RunRecord};

pub fn handle(root: &Path, format: OutputFormat) -> Result<()> {
    let records = discover_runs(root)?;
    match format {
        OutputFormat::Table => print_run_table(&records),
        OutputFormat::Json => print_run_json(&records),
        OutputFormat::Jsonl => print_run_jsonl(&records),
        OutputFormat::Csv => print_run_csv(&records),
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

fn print_run_jsonl(records: &[RunRecord]) -> Result<()> {
    let runs = summaries(records);
    for run in runs {
        serde_json::to_writer(io::stdout(), &run)
            .map_err(|err| anyhow::anyhow!("serializing run to JSONL: {err}"))?;
        println!();
    }
    Ok(())
}

fn print_run_csv(records: &[RunRecord]) -> Result<()> {
    let mut writer = io::stdout();
    writeln!(writer, "run_id,command,timestamp,version,manifest_path")?;
    for record in records {
        // Escape CSV fields if they contain commas or quotes
        let command = csv_escape(&record.manifest.command);
        let manifest_path = csv_escape(&record.path.display().to_string());
        writeln!(
            writer,
            "{},{},{},{},{}",
            record.manifest.run_id,
            command,
            record.manifest.timestamp,
            record.manifest.version,
            manifest_path,
        )?;
    }
    Ok(())
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
