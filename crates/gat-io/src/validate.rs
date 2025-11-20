use std::{fs::File, path::Path};

use anyhow::{anyhow, bail, Context, Result};
#[cfg(feature = "parquet")]
use polars::prelude::ParquetReader;
use polars::prelude::{CsvReader, DataFrame, SerReader};
use serde::Deserialize;
use serde_json;

#[derive(Deserialize)]
struct DatasetSpec {
    dataset: String,
    columns: Vec<ColumnSpec>,
}

#[derive(Deserialize)]
struct ColumnSpec {
    name: String,
    dtype: Option<String>,
}

pub fn validate_dataset(spec_file: &str) -> Result<()> {
    let spec_path = Path::new(spec_file);
    let text = std::fs::read_to_string(spec_path)
        .with_context(|| format!("reading spec file '{}'", spec_file))?;
    let spec: DatasetSpec =
        serde_json::from_str(&text).context("parsing dataset spec as JSON schema")?;
    let dataset_path = spec_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(&spec.dataset);

    if !dataset_path.exists() {
        bail!("dataset file '{}' does not exist", dataset_path.display());
    }

    let df = read_dataframe(&dataset_path)?;
    for column in spec.columns {
        let series = df
            .column(&column.name)
            .with_context(|| format!("column '{}' missing from dataset", column.name))?;
        if let Some(expected) = &column.dtype {
            let actual = series.dtype().to_string();
            if !eq_dtype(expected, &actual) {
                bail!(
                    "column '{}' dtype mismatch: expected {}, found {}",
                    column.name,
                    expected,
                    actual
                );
            }
        }
    }

    println!(
        "Dataset '{}' conforms to spec '{}'",
        dataset_path.display(),
        spec_file
    );
    Ok(())
}

fn read_dataframe(path: &Path) -> Result<DataFrame> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let mut file =
        File::open(path).with_context(|| format!("opening dataset '{}'", path.display()))?;
    match extension.as_str() {
        #[cfg(feature = "parquet")]
        "parquet" => {
            let reader = ParquetReader::new(&mut file);
            reader.finish().context("reading Parquet dataset")
        }
        #[cfg(not(feature = "parquet"))]
        "parquet" => Err(anyhow!(
            "parquet support is disabled; rebuild with the 'parquet' feature"
        )),
        "csv" => {
            let reader = CsvReader::new(&mut file);
            reader.finish().context("reading CSV dataset")
        }
        other => Err(anyhow!(
            "unsupported dataset extension '{}' (use .csv or .parquet)",
            other
        )),
    }
}

fn eq_dtype(expected: &str, actual: &str) -> bool {
    expected.eq_ignore_ascii_case(actual)
        || expected.eq_ignore_ascii_case(strip_mod(expected))
        || strip_mod(expected).eq_ignore_ascii_case(strip_mod(actual))
}

fn strip_mod(dtype: &str) -> &str {
    dtype.split('_').next().unwrap_or(dtype)
}

#[cfg(all(test, feature = "parquet"))]
mod tests {
    use super::*;
    use polars::prelude::*;
    use tempfile::tempdir;

    fn write_parquet(df: &mut DataFrame, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        ParquetWriter::new(&mut file)
            .finish(df)
            .context("writing Parquet fixture")?;
        Ok(())
    }

    #[test]
    fn validate_dataset_success() {
        let mut df = DataFrame::new(vec![
            Series::new("sensor", ["A", "B"]),
            Series::new("value", [1.0, 2.0]),
        ])
        .unwrap();
        let dir = tempdir().unwrap();
        let data_path = dir.path().join("data.parquet");
        write_parquet(&mut df, &data_path).unwrap();

        let spec = serde_json::json!({
            "dataset": "data.parquet",
            "columns": [
                { "name": "sensor", "dtype": "Utf8" },
                { "name": "value", "dtype": "Float64" }
            ]
        });
        let spec_path = dir.path().join("spec.json");
        std::fs::write(&spec_path, serde_json::to_string_pretty(&spec).unwrap()).unwrap();

        assert!(validate_dataset(spec_path.to_str().unwrap()).is_ok());
    }

    #[test]
    fn validate_dataset_missing_column() {
        let mut df = DataFrame::new(vec![Series::new("sensor", ["A", "B"])].clone()).unwrap();
        let dir = tempdir().unwrap();
        let data_path = dir.path().join("data.parquet");
        write_parquet(&mut df, &data_path).unwrap();

        let spec = serde_json::json!({
            "dataset": "data.parquet",
            "columns": [
                { "name": "sensor", "dtype": "Utf8" },
                { "name": "value", "dtype": "Float64" }
            ]
        });
        let spec_path = dir.path().join("spec.json");
        std::fs::write(&spec_path, serde_json::to_string_pretty(&spec).unwrap()).unwrap();

        assert!(validate_dataset(spec_path.to_str().unwrap()).is_err());
    }
}
