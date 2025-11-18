use anyhow::{Context, Result};
use polars::frame::group_by::GroupsIndicator;
use polars::prelude::{DataFrame, IdxCa, IdxSize, NamedFrom, ParquetWriter};
use std::{
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy)]
pub enum OutputStage {
    PfDc,
    OpfDc,
    OpfAc,
    Nminus1Dc,
    SeWls,
}

impl OutputStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputStage::PfDc => "pf-dc",
            OutputStage::OpfDc => "opf-dc",
            OutputStage::OpfAc => "opf-ac",
            OutputStage::Nminus1Dc => "nminus1-dc",
            OutputStage::SeWls => "se-wls",
        }
    }
}

pub fn staged_output_path(output: &Path, stage: &str) -> PathBuf {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let file_name = output.file_name().unwrap_or_else(|| OsStr::new("output"));
    parent.join(stage).join(file_name)
}

pub fn persist_dataframe(
    df: &mut DataFrame,
    output: &Path,
    partitions: &[String],
    stage: &str,
) -> Result<()> {
    let staged = staged_output_path(output, stage);
    if partitions.is_empty() {
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating output directory '{}'", parent.display()))?;
        }
        let mut file = File::create(&staged)
            .with_context(|| format!("creating Parquet output '{}'", staged.display()))?;
        ParquetWriter::new(&mut file)
            .finish(df)
            .context("writing Parquet output")?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating output directory '{}'", parent.display()))?;
        }
        fs::copy(&staged, output).with_context(|| {
            format!("copying {} to {}", staged.display(), output.display())
        })?;
    } else {
        write_partitions(df, &staged, partitions)?;
    }
    Ok(())
}

fn write_partitions(df: &DataFrame, output: &Path, partitions: &[String]) -> Result<()> {
    let group_by = df.group_by(partitions)?;
    let groups = group_by.get_groups();
    for (i, group) in groups.iter().enumerate() {
        let (mut partition_df, first) = match group {
            GroupsIndicator::Idx((first, indices)) => {
                let idx_ca = IdxCa::new("row_idx", indices.as_slice());
                (df.take(&idx_ca)?, first)
            }
            GroupsIndicator::Slice([first, len]) => (df.slice(first as i64, len as usize), first),
        };
        let dir = partition_dir(output, partitions, df, first)?;
        write_partition_file(&mut partition_df, &dir, i)?;
    }
    Ok(())
}

fn write_partition_file(df: &mut DataFrame, dir: &Path, index: usize) -> Result<()> {
    fs::create_dir_all(dir)
        .with_context(|| format!("creating partition directory '{}'", dir.display()))?;
    let file_path = dir.join(format!("part-{index:04}.parquet"));
    let mut file = File::create(&file_path)
        .with_context(|| format!("creating partition file '{}'", file_path.display()))?;
    ParquetWriter::new(&mut file)
        .finish(df)
        .with_context(|| format!("writing partition file '{}'", file_path.display()))?;
    Ok(())
}

fn partition_dir(
    output: &Path,
    partitions: &[String],
    df: &DataFrame,
    row_idx: IdxSize,
) -> Result<PathBuf> {
    let mut path = output.to_path_buf();
    for key in partitions {
        let series = df.column(key)?;
        let idx = row_idx as usize;
        let value = series.get(idx)?;
        let value = sanitize_partition_value(&value.to_string());
        path.push(format!("{key}={value}"));
    }
    Ok(path)
}

fn sanitize_partition_value(value: &str) -> String {
    value.replace(std::path::MAIN_SEPARATOR, "_")
}
