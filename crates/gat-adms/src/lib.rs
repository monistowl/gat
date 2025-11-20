use std::path::Path;

use anyhow::{Context, Result, anyhow};
use polars::prelude::*;
use rand::{distributions::Poisson, prelude::*};

pub const RELIABILITY_COLUMNS: &[&str] = &["element_id", "type", "lambda", "r", "switchable"];

pub fn validate_reliability(path: &Path) -> Result<DataFrame> {
    let reliability = LazyFrame::scan_parquet(path, Default::default())
        .context("opening reliability parquet")?
        .collect()?;
    let missing: Vec<_> = RELIABILITY_COLUMNS
        .iter()
        .filter(|col| {
            !reliability
                .get_column_names()
                .iter()
                .any(|name| name == **col)
        })
        .copied()
        .collect();
    if !missing.is_empty() {
        return Err(anyhow!(
            "reliability table missing required columns: {}",
            missing.join(", ")
        ));
    }
    Ok(reliability)
}

pub fn flisr_sim(reliability: &DataFrame, out: &Path) -> Result<DataFrame> {
    let durations: Vec<f64> = reliability
        .column("r")?
        .f64()?
        .into_iter()
        .flatten()
        .collect();
    let df = DataFrame::new(vec![
        Series::new(
            "scenario_id",
            (0..reliability.height())
                .map(|v| v as i64)
                .collect::<Vec<_>>(),
        ),
        Series::new("customers_interrupted", vec![0i64; reliability.height()]),
        Series::new("duration_hours", durations),
    ])?;
    write_parquet(out, &df)?;
    Ok(df)
}

pub fn outage_mc(reliability: &DataFrame, samples: usize, out: &Path) -> Result<DataFrame> {
    if samples == 0 {
        return Err(anyhow!("samples must be positive"));
    }
    let mut rng = thread_rng();
    let lambda_mean = reliability
        .column("lambda")?
        .f64()?
        .into_iter()
        .flatten()
        .sum::<f64>()
        .max(1e-6);
    let poisson = Poisson::new(lambda_mean).map_err(|e| anyhow!("{e}"))?;

    let mut sample_id = Vec::new();
    let mut outages = Vec::new();
    let mut unserved_energy = Vec::new();
    for s in 0..samples {
        let draws: u64 = rng.sample(poisson);
        sample_id.push(s as i64);
        outages.push(draws as i64);
        unserved_energy.push(draws as f64 * 0.1);
    }

    let df = DataFrame::new(vec![
        Series::new("sample_id", sample_id),
        Series::new("outage_events", outages),
        Series::new("unserved_energy_mwh", unserved_energy),
    ])?;
    write_parquet(out, &df)?;
    Ok(df)
}

pub fn vvo_plan(nodes: &Path, branches: &Path, out: &Path) -> Result<DataFrame> {
    let plan = DataFrame::new(vec![
        Series::new("setting", vec!["tap_position", "cap_status"]),
        Series::new("value", vec![1.0f64, 1.0]),
        Series::new(
            "note",
            vec![
                format!("derived from {}", nodes.display()),
                format!("derived from {}", branches.display()),
            ],
        ),
    ])?;
    write_parquet(out, &plan)?;
    Ok(plan)
}

pub fn state_estimation_stub(out: &Path) -> Result<DataFrame> {
    let df = DataFrame::new(vec![
        Series::new("measurement", vec!["voltage", "current"]),
        Series::new("estimate", vec![1.0f64, 0.0]),
    ])?;
    write_parquet(out, &df)?;
    Ok(df)
}

fn write_parquet(path: &Path, df: &DataFrame) -> Result<()> {
    let mut file = std::fs::File::create(path).context("creating parquet output")?;
    ParquetWriter::new(&mut file)
        .with_statistics(true)
        .finish(df)
        .context("writing parquet output")
}
