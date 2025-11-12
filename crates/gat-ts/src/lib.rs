use std::{collections::BTreeMap, fs::File, path::Path};

use anyhow::{anyhow, Context, Result};
use polars::prelude::*;

pub fn resample_timeseries(
    input_path: &str,
    timestamp_column: &str,
    value_column: &str,
    rule: &str,
    output_path: &str,
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

    write_frame(&mut out, output_path)?;
    Ok(())
}

pub fn join_timeseries(
    left_path: &str,
    right_path: &str,
    on: &str,
    output_path: &str,
) -> Result<()> {
    let left_df = read_frame(left_path)?;
    let right_df = read_frame(right_path)?;
    let joined = left_df
        .outer_join(&right_df, &[on], &[on])
        .context("joining time series")?;

    let mut joined = joined;
    write_frame(&mut joined, output_path)?;
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
        "parquet" => {
            let reader = ParquetReader::new(&mut file);
            reader.finish().context("reading Parquet file")
        }
        "csv" => {
            let reader = CsvReader::new(&mut file);
            reader.finish().context("reading CSV file")
        }
        _ => Err(anyhow!(
            "unsupported file extension '{}'; use .csv or .parquet",
            extension
        )),
    }
}

fn write_frame(df: &mut DataFrame, path: &str) -> Result<()> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
    {
        Some(ext) if ext == "parquet" => ParquetWriter::new(&mut file)
            .finish(df)
            .map(|_| ())
            .context("writing Parquet file"),
        Some(ext) if ext == "csv" => CsvWriter::new(&mut file)
            .finish(df)
            .context("writing CSV file"),
        _ => Err(anyhow!(
            "unsupported output extension for {}; use .csv or .parquet",
            path.display()
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::ParquetWriter;
    use tempfile::tempdir;

    fn write_parquet(df: &mut DataFrame, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        ParquetWriter::new(&mut file).finish(df)?;
        Ok(())
    }

    #[test]
    fn resample_buckets_into_periods() {
        let mut df = DataFrame::new(vec![
            Series::new("timestamp", vec![0i64, 1, 3, 7, 11]),
            Series::new("value", vec![10.0, 20.0, 15.0, 40.0, 50.0]),
        ])
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
        )
        .unwrap();

        let result = read_frame(output.to_str().unwrap()).unwrap();
        assert_eq!(result.height(), 3);
        let counts = result.column("count").unwrap().i64().unwrap();
        assert_eq!(counts.get(0), Some(3));
        assert_eq!(counts.get(1), Some(1));
        assert_eq!(counts.get(2), Some(1));
    }

    #[test]
    fn join_timeseries_reads_both_files() {
        let mut left = DataFrame::new(vec![
            Series::new("timestamp", vec![1i64, 2]),
            Series::new("value_l", vec![10.0, 20.0]),
        ])
        .unwrap();
        let mut right = DataFrame::new(vec![
            Series::new("timestamp", vec![2i64, 3]),
            Series::new("value_r", vec![30.0, 40.0]),
        ])
        .unwrap();

        let dir = tempdir().unwrap();
        let left_path = dir.path().join("left.parquet");
        let right_path = dir.path().join("right.parquet");
        write_parquet(&mut left, &left_path).unwrap();
        write_parquet(&mut right, &right_path).unwrap();
        let out = dir.path().join("joined.parquet");

        join_timeseries(
            left_path.to_str().unwrap(),
            right_path.to_str().unwrap(),
            "timestamp",
            out.to_str().unwrap(),
        )
        .unwrap();

        let joined = read_frame(out.to_str().unwrap()).unwrap();
        assert_eq!(joined.height(), 3);
        let timestamp = joined.column("timestamp").unwrap().i64().unwrap();
        let mut values = timestamp
            .into_iter()
            .filter_map(|opt| opt)
            .collect::<Vec<_>>();
        values.sort();
        assert_eq!(values.as_slice(), [1, 2, 3]);
    }
}
