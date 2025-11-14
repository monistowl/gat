use crate::io::{staged_output_path, OutputStage};
use anyhow::{Context, Result};
use polars::prelude::{DataFrame, ParquetReader, SerReader};
use std::{fs::File, path::Path};

pub fn read_stage_dataframe(base: &Path, stage: OutputStage) -> Result<DataFrame> {
    let path = staged_output_path(base, stage.as_str());
    let file = File::open(&path).with_context(|| format!("opening {}", path.display()))?;
    ParquetReader::new(file)
        .finish()
        .context("reading stage parquet output")
}
