use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use polars::datatypes::IdxSize;
use polars::frame::group_by::GroupsIndicator;
use polars::prelude::*;
#[cfg(feature = "parquet")]
use polars::prelude::{ParquetReader, ParquetWriter};

pub fn resample_timeseries(
    input_path: &str,
    timestamp_column: &str,
    value_column: &str,
    rule: &str,
    output_path: &str,
    partitions: &[String],
) -> Result<()> {
    let df = read_frame(input_path)?;
    let timestamp_series = df
        .column(timestamp_column)?
        .cast(&DataType::Int64)
        .context("casting timestamp column to Int64")?;
    let value_series = df
        .column(value_column)?
        .cast(&DataType::Float64)
        .context("casting value column to Float64")?;

    let timestamps = timestamp_series.i64()?;
    let values = value_series.f64()?;
    let period = parse_rule(rule)?;

    if period <= 0 {
        return Err(anyhow!("resample rule must be positive"));
    }

    let mut buckets: BTreeMap<i64, BucketStats> = BTreeMap::new();
    for (ts_opt, val_opt) in timestamps.into_iter().zip(values.into_iter()) {
        if let (Some(ts), Some(value)) = (ts_opt, val_opt) {
            let bucket = floor_bucket(ts, period);
            let entry = buckets.entry(bucket).or_default();
            entry.count += 1;
            entry.sum += value;
            entry.min = entry.min.min(value);
            entry.max = entry.max.max(value);
        }
    }

    let mut bucket_start = Vec::with_capacity(buckets.len());
    let mut means = Vec::with_capacity(buckets.len());
    let mut counts = Vec::with_capacity(buckets.len());
    let mut mins = Vec::with_capacity(buckets.len());
    let mut maxs = Vec::with_capacity(buckets.len());

    for (bucket, stats) in buckets {
        bucket_start.push(bucket);
        counts.push(stats.count as i64);
        means.push(stats.sum / stats.count as f64);
        mins.push(stats.min);
        maxs.push(stats.max);
    }

    let mut out = DataFrame::new(vec![
        Series::new("bucket_start", bucket_start),
        Series::new("count", counts),
        Series::new("mean_value", means),
        Series::new("min_value", mins),
        Series::new("max_value", maxs),
    ])?;

    write_frame_staged(&mut out, output_path, "ts-resample", partitions)?;
    Ok(())
}

pub fn join_timeseries(
    left_path: &str,
    right_path: &str,
    on: &str,
    output_path: &str,
    partitions: &[String],
) -> Result<()> {
    let left_df = read_frame(left_path)?;
    let right_df = read_frame(right_path)?;
    let joined = left_df
        .outer_join(&right_df, &[on], &[on])
        .context("joining time series")?;

    let mut joined = joined;
    write_frame_staged(&mut joined, output_path, "ts-join", partitions)?;
    Ok(())
}

pub fn aggregate_timeseries(
    input_path: &str,
    group_col: &str,
    value_column: &str,
    agg: &str,
    output_path: &str,
    partitions: &[String],
) -> Result<()> {
    let df = read_frame(input_path)?;
    let (suffix, expr) = match agg {
        "sum" => ("_sum", col(value_column).sum()),
        "mean" => ("_mean", col(value_column).mean()),
        "min" => ("_min", col(value_column).min()),
        "max" => ("_max", col(value_column).max()),
        "count" => ("_count", col(value_column).count()),
        other => {
            return Err(anyhow!(
                "unsupported aggregation '{}'; use sum, mean, min, max, or count",
                other
            ));
        }
    };

    let mut agg_df = df
        .lazy()
        .group_by([col(group_col)])
        .agg([expr])
        .collect()
        .context("running groupby aggregation")?;

    let alias_name = format!("{value_column}{suffix}");
    agg_df
        .rename(value_column, alias_name.as_str())
        .context("renaming aggregated column")?;

    let mut out = agg_df.clone();
    write_frame_staged(&mut out, output_path, "ts-agg", partitions)?;
    Ok(())
}

fn read_frame(path: &str) -> Result<DataFrame> {
    let path = Path::new(path);
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;

    match extension.as_str() {
        #[cfg(feature = "parquet")]
        "parquet" => {
            let reader = ParquetReader::new(&mut file);
            reader.finish().context("reading Parquet file")
        }
        #[cfg(not(feature = "parquet"))]
        "parquet" => Err(anyhow!(
            "parquet support is disabled; rebuild with the 'parquet' feature"
        )),
        "csv" => {
            let reader = CsvReader::new(&mut file);
            reader.has_header(true).finish().context("reading CSV file")
        }
        _ => Err(anyhow!(
            "unsupported file extension '{}'; use .csv or .parquet",
            extension
        )),
    }
}

fn write_frame_staged(
    df: &mut DataFrame,
    path: &str,
    stage: &str,
    partitions: &[String],
) -> Result<()> {
    let output = Path::new(path);
    let staged = staged_output_path(output, stage);
    if !partitions.is_empty() && !cfg!(feature = "parquet") {
        bail!("partitioned output requires parquet support; rebuild with the 'parquet' feature");
    }

    if partitions.is_empty() {
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file =
            File::create(&staged).with_context(|| format!("creating {}", staged.display()))?;
        let write_result = match staged
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
        {
            #[cfg(feature = "parquet")]
            Some(ext) if ext == "parquet" => ParquetWriter::new(&mut file)
                .finish(df)
                .map(|_| ())
                .context("writing Parquet file"),
            #[cfg(not(feature = "parquet"))]
            Some(ext) if ext == "parquet" => Err(anyhow!(
                "parquet support is disabled; rebuild with the 'parquet' feature"
            )),
            Some(ext) if ext == "csv" => CsvWriter::new(&mut file)
                .finish(df)
                .context("writing CSV file"),
            _ => Err(anyhow!(
                "unsupported output extension for {}; use .csv or .parquet",
                staged.display()
            )),
        };
        write_result?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&staged, output)
            .with_context(|| format!("copying {} to {}", staged.display(), output.display()))?;
        Ok(())
    } else {
        write_partitions(df, &staged, partitions)
    }
}

fn staged_output_path(output: &Path, stage: &str) -> PathBuf {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let file_name = output.file_name().unwrap_or_else(|| OsStr::new("output"));
    parent.join(stage).join(file_name)
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

#[cfg(feature = "parquet")]
fn write_partition_file(df: &mut DataFrame, dir: &Path, index: usize) -> Result<()> {
    fs::create_dir_all(dir)?;
    let file_path = dir.join(format!("part-{index:04}.parquet"));
    let mut file = File::create(&file_path)?;
    ParquetWriter::new(&mut file)
        .finish(df)
        .map(|_| ())
        .context("writing partition file")
}

#[cfg(not(feature = "parquet"))]
fn write_partition_file(_df: &mut DataFrame, _dir: &Path, _index: usize) -> Result<()> {
    bail!("parquet support is disabled; rebuild with the 'parquet' feature to write partitions")
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

fn parse_rule(rule: &str) -> Result<i64> {
    let trimmed = rule.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("rule cannot be empty"));
    }
    let (value_str, unit) = match trimmed.chars().last() {
        Some(ch) if ch.is_ascii_alphabetic() => (&trimmed[..trimmed.len() - 1], Some(ch)),
        _ => (trimmed, None),
    };

    let value = value_str.parse::<i64>().context("parsing rule duration")?;
    let multiplier = match unit.unwrap_or('s') {
        's' => 1,
        'm' => 60,
        'h' => 3600,
        other => {
            return Err(anyhow!("unsupported time unit '{}'; expected s/m/h", other));
        }
    };
    Ok(value * multiplier)
}

fn floor_bucket(ts: i64, period: i64) -> i64 {
    ts - ts.rem_euclid(period)
}

struct BucketStats {
    count: usize,
    sum: f64,
    min: f64,
    max: f64,
}

impl Default for BucketStats {
    fn default() -> Self {
        BucketStats {
            count: 0,
            sum: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }
}

#[cfg(all(test, feature = "parquet"))]
mod tests {
    use super::*;
    use polars::prelude::{CsvWriter, ParquetWriter};
    use std::{ffi::OsStr, fs, fs::File, path::Path};
    use tempfile::tempdir;

    fn staged_path(base: &Path, stage: &str) -> PathBuf {
        let parent = base.parent().unwrap_or_else(|| Path::new("."));
        let file_name = base.file_name().unwrap_or_else(|| OsStr::new("output"));
        parent.join(stage).join(file_name)
    }

    fn write_parquet(df: &mut DataFrame, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        ParquetWriter::new(&mut file).finish(df)?;
        Ok(())
    }

    #[test]
    fn join_respects_partitions() {
        let temp_dir = tempdir().unwrap();
        let left = temp_dir.path().join("left.parquet");
        let right = temp_dir.path().join("right.parquet");
        let out = temp_dir.path().join("joined.parquet");
        let df = df![
            "timestamp" => &[0i64, 1],
            "value" => &[1.0f64, 2.0],
            "sensor" => &["A", "B"]
        ]
        .unwrap();
        let mut df_clone = df.clone();
        write_frame_staged(&mut df_clone, left.to_str().unwrap(), "ts-test", &[]).unwrap();
        let mut df_clone2 = df.clone();
        write_frame_staged(&mut df_clone2, right.to_str().unwrap(), "ts-test", &[]).unwrap();
        let partitions = vec!["sensor".to_string()];
        let left_stage = staged_path(&left, "ts-test");
        let right_stage = staged_path(&right, "ts-test");
        join_timeseries(
            left_stage.to_str().unwrap(),
            right_stage.to_str().unwrap(),
            "timestamp",
            out.to_str().unwrap(),
            &partitions,
        )
        .unwrap();
        let staged = staged_path(&out, "ts-join");
        assert!(staged.exists());
    }

    #[test]
    fn aggregate_shares_stage() {
        let temp_dir = tempdir().unwrap();
        let input = temp_dir.path().join("agg.csv");
        let out = temp_dir.path().join("agg.parquet");
        fs::write(&input, "sensor,value\nA,1\nA,2\nB,3\n").unwrap();
        let partitions = vec!["sensor".to_string()];
        aggregate_timeseries(
            input.to_str().unwrap(),
            "sensor",
            "value",
            "sum",
            out.to_str().unwrap(),
            &partitions,
        )
        .unwrap();
        let staged = staged_path(&out, "ts-agg");
        assert!(staged.exists());
    }

    #[test]
    fn resample_buckets_into_periods() {
        let mut df = df![
            "timestamp" => &[0i64, 1, 3, 7, 11],
            "value" => &[10.0, 20.0, 15.0, 40.0, 50.0],
        ]
        .unwrap();
        let dir = tempdir().unwrap();
        let input = dir.path().join("src.parquet");
        write_parquet(&mut df, &input).unwrap();
        let output = dir.path().join("resampled.parquet");
        resample_timeseries(
            input.to_str().unwrap(),
            "timestamp",
            "value",
            "5s",
            output.to_str().unwrap(),
            &[],
        )
        .unwrap();
        let staged = staged_path(&output, "ts-resample");
        let result = read_frame(staged.to_str().unwrap()).unwrap();
        assert_eq!(result.height(), 3);
        let counts = result.column("count").unwrap().i64().unwrap();
        assert_eq!(counts.get(0), Some(3));
        assert_eq!(counts.get(1), Some(1));
        assert_eq!(counts.get(2), Some(1));
    }

    #[test]
    fn join_timeseries_reads_both_files() {
        let mut left = df![
            "timestamp" => &[1i64, 2],
            "value_l" => &[10.0, 20.0],
        ]
        .unwrap();
        let mut right = df![
            "timestamp" => &[1i64, 3],
            "value_r" => &[15.0, 25.0],
        ]
        .unwrap();
        let dir = tempdir().unwrap();
        let left_path = dir.path().join("left.parquet");
        let right_path = dir.path().join("right.parquet");
        write_parquet(&mut left, &left_path).unwrap();
        write_parquet(&mut right, &right_path).unwrap();
        let output = dir.path().join("joined.parquet");
        join_timeseries(
            left_path.to_str().unwrap(),
            right_path.to_str().unwrap(),
            "timestamp",
            output.to_str().unwrap(),
            &[],
        )
        .unwrap();
        let staged = staged_path(&output, "ts-join");
        let result = read_frame(staged.to_str().unwrap()).unwrap();
        assert_eq!(result.height(), 3);
    }

    #[test]
    fn aggregate_timeseries_sums_by_group() {
        let mut df = df![ "sensor" => &["A", "A", "B"], "value" => &[1.0, 2.0, 3.0] ].unwrap();
        let dir = tempdir().unwrap();
        let input = dir.path().join("amounts.csv");
        let mut csv_file = File::create(&input).unwrap();
        CsvWriter::new(&mut csv_file).finish(&mut df).unwrap();
        let output = dir.path().join("agg.parquet");
        aggregate_timeseries(
            input.to_str().unwrap(),
            "sensor",
            "value",
            "sum",
            output.to_str().unwrap(),
            &[],
        )
        .unwrap();
        let staged = staged_path(&output, "ts-agg");
        let result = read_frame(staged.to_str().unwrap()).unwrap();
        assert_eq!(result.height(), 2);
    }
}
