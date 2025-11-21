use anyhow::{anyhow, Context, Result};
use polars::prelude::{
    DataFrame, NamedFrom, ParquetCompression, ParquetReader, ParquetWriter, SerReader, Series,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::Path;

/// Simple representation of a DER asset extracted from the Parquet asset table.
#[derive(Clone, Debug)]
struct DerAsset {
    id: String,
    agg_id: Option<String>,
    bus_id: Option<usize>,
    p_min: f64,
    p_max: f64,
    q_min: f64,
    q_max: f64,
    soc_min: f64,
    soc_max: f64,
}

/// Lightweight price vector for scheduling horizons.
#[derive(Clone, Debug)]
struct PricePoint {
    timestamp: String,
    price: f64,
}

/// Emit aggregated DER envelopes grouped by `group_by` (defaults to `agg_id`).
pub fn envelope(
    grid_file: &Path,
    asset_file: &Path,
    output_file: &Path,
    group_by: Option<&str>,
) -> Result<()> {
    println!(
        "Using grid {:?} to provide topology context (not yet consumed).",
        grid_file
    );
    let df = read_parquet(asset_file)?;
    let assets = parse_assets(&df)?;
    let key = group_by.unwrap_or("agg_id");
    let groups = group_assets(&assets, key);

    let mut region = Vec::new();
    let mut p_min = Vec::new();
    let mut p_max = Vec::new();
    let mut q_min = Vec::new();
    let mut q_max = Vec::new();
    let mut asset_counts = Vec::new();

    for (name, members) in groups {
        region.push(name.clone());
        asset_counts.push(members.len() as i64);
        p_min.push(
            members
                .iter()
                .map(|asset| asset.p_min)
                .fold(f64::INFINITY, f64::min),
        );
        p_max.push(
            members
                .iter()
                .map(|asset| asset.p_max)
                .fold(f64::NEG_INFINITY, f64::max),
        );
        q_min.push(
            members
                .iter()
                .map(|asset| asset.q_min)
                .fold(f64::INFINITY, f64::min),
        );
        q_max.push(
            members
                .iter()
                .map(|asset| asset.q_max)
                .fold(f64::NEG_INFINITY, f64::max),
        );
    }

    let mut summary = DataFrame::new(vec![
        Series::new("region", region),
        Series::new("asset_count", asset_counts),
        Series::new("p_min_mw", p_min),
        Series::new("p_max_mw", p_max),
        Series::new("q_min_mvar", q_min),
        Series::new("q_max_mvar", q_max),
    ])?;
    persist_dataframe(output_file, &mut summary)?;
    println!(
        "DERMS envelope persisted {} regions to {} (grouped by {})",
        summary.height(),
        output_file.display(),
        key
    );
    Ok(())
}

/// Build a naive DER schedule driven by price signals.
pub fn schedule(
    asset_file: &Path,
    price_file: &Path,
    output_file: &Path,
    objective: &str,
) -> Result<f64> {
    let assets = parse_assets(&read_parquet(asset_file)?)?;
    let prices = parse_prices(&read_parquet(price_file)?)?;
    if prices.is_empty() {
        return Err(anyhow!("price series must contain at least one row"));
    }

    let median = compute_median_price(&prices);
    let (mut schedule_df, curtailment) = build_schedule(&assets, &prices, median)?;
    persist_dataframe(output_file, &mut schedule_df)?;

    println!(
        "DERMS schedule ({}) wrote {} rows to {}; curtailment {:.3}",
        objective,
        schedule_df.height(),
        output_file.display(),
        curtailment
    );
    Ok(curtailment)
}

/// Run randomized price perturbations to stress-test the DER schedule heuristics.
pub fn stress_test(
    asset_file: &Path,
    price_file: &Path,
    output_dir: &Path,
    scenarios: usize,
    seed: Option<u64>,
) -> Result<()> {
    if scenarios == 0 {
        return Err(anyhow!("scenarios must be >= 1"));
    }
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create stress-test directory '{}'",
            output_dir.display()
        )
    })?;

    let assets = parse_assets(&read_parquet(asset_file)?)?;
    let prices = parse_prices(&read_parquet(price_file)?)?;
    let median = compute_median_price(&prices);

    let mut rng = seed
        .map(|value| StdRng::seed_from_u64(value))
        .unwrap_or_else(StdRng::from_entropy);

    let mut scenario_ids = Vec::new();
    let mut scale_factors = Vec::new();
    let mut curtail_rates = Vec::new();

    for scenario in 0..scenarios {
        let scale = rng.gen_range(0.8..=1.2);
        let adjusted: Vec<PricePoint> = prices
            .iter()
            .map(|point| PricePoint {
                timestamp: point.timestamp.clone(),
                price: point.price * scale,
            })
            .collect();
        let (_, curtailment) = build_schedule(&assets, &adjusted, median)?;
        scenario_ids.push(scenario as i64);
        scale_factors.push(scale);
        curtail_rates.push(curtailment);
    }

    let mut summary = DataFrame::new(vec![
        Series::new("scenario", scenario_ids),
        Series::new("scale_factor", scale_factors),
        Series::new("curtailment_rate", curtail_rates),
    ])?;

    let summary_path = output_dir.join("derms_stress_summary.parquet");
    persist_dataframe(&summary_path, &mut summary)?;
    println!(
        "DERMS stress-test recorded {} scenarios to {}",
        scenarios,
        summary_path.display()
    );
    Ok(())
}

fn read_parquet(input: &Path) -> Result<DataFrame> {
    let file = File::open(input)
        .with_context(|| format!("opening parquet dataset '{}'", input.display()))?;
    let reader = ParquetReader::new(file);
    reader
        .finish()
        .with_context(|| format!("reading parquet dataset '{}'", input.display()))
}

fn persist_dataframe(path: &Path, df: &mut DataFrame) -> Result<()> {
    let mut file = File::create(path)
        .with_context(|| format!("creating Parquet output '{}'", path.display()))?;
    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(df)
        .with_context(|| format!("writing Parquet {}", path.display()))?;
    Ok(())
}

fn parse_assets(df: &DataFrame) -> Result<Vec<DerAsset>> {
    let height = df.height();
    let ids = column_utf8(df, "asset_id")?;
    let aggs = column_utf8(df, "agg_id")?;
    let buses = column_i64(df, "bus_id")?;
    let p_min = column_f64(df, "p_min", 0.0)?;
    let p_max = column_f64(df, "p_max", 0.0)?;
    let q_min = column_f64(df, "q_min", 0.0)?;
    let q_max = column_f64(df, "q_max", 0.0)?;
    let soc_min = column_f64(df, "soc_min", 0.0)?;
    let soc_max = column_f64(df, "soc_max", 1.0)?;

    let mut assets = Vec::with_capacity(height);
    for idx in 0..height {
        let id = ids[idx].clone().unwrap_or_else(|| format!("asset_{idx}"));
        let agg_id = aggs[idx].clone();
        let bus_id = buses[idx];
        assets.push(DerAsset {
            id,
            agg_id,
            bus_id,
            p_min: p_min[idx],
            p_max: p_max[idx],
            q_min: q_min[idx],
            q_max: q_max[idx],
            soc_min: soc_min[idx],
            soc_max: soc_max[idx],
        });
    }
    Ok(assets)
}

fn column_utf8(df: &DataFrame, column: &str) -> Result<Vec<Option<String>>> {
    if let Ok(series) = df.column(column) {
        let chunked = series
            .utf8()
            .with_context(|| format!("column '{}' must be utf8", column))?;
        Ok(chunked
            .into_iter()
            .map(|opt| opt.map(|value| value.to_string()))
            .collect())
    } else {
        let height = df.height();
        Ok(vec![None; height])
    }
}

fn column_f64(df: &DataFrame, column: &str, default: f64) -> Result<Vec<f64>> {
    if let Ok(series) = df.column(column) {
        let chunked = series
            .f64()
            .with_context(|| format!("column '{}' must be float", column))?;
        Ok(chunked
            .into_iter()
            .map(|opt| opt.unwrap_or(default))
            .collect())
    } else {
        Ok(vec![default; df.height()])
    }
}

fn column_i64(df: &DataFrame, column: &str) -> Result<Vec<Option<usize>>> {
    if let Ok(series) = df.column(column) {
        let chunked = series
            .i64()
            .with_context(|| format!("column '{}' must be integer", column))?;
        Ok(chunked
            .into_iter()
            .map(|opt| opt.map(|value| value as usize))
            .collect())
    } else {
        Ok(vec![None; df.height()])
    }
}

fn group_assets<'a>(assets: &'a [DerAsset], key: &str) -> HashMap<String, Vec<&'a DerAsset>> {
    let mut map: HashMap<String, Vec<&'a DerAsset>> = HashMap::new();
    for asset in assets {
        let region = match key {
            "bus" => asset
                .bus_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| asset.id.clone()),
            _ => asset.agg_id.clone().unwrap_or_else(|| asset.id.clone()),
        };
        map.entry(region).or_default().push(asset);
    }
    map
}

fn parse_prices(df: &DataFrame) -> Result<Vec<PricePoint>> {
    let timestamps = column_utf8(df, "timestamp")?;
    let values = column_f64(df, "value", 0.0)?;
    let mut result = Vec::new();
    for idx in 0..df.height() {
        result.push(PricePoint {
            timestamp: timestamps[idx]
                .clone()
                .unwrap_or_else(|| format!("t{}", idx)),
            price: values[idx],
        });
    }
    Ok(result)
}

fn compute_median_price(prices: &[PricePoint]) -> f64 {
    let mut values: Vec<f64> = prices.iter().map(|point| point.price).collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

fn build_schedule(
    assets: &[DerAsset],
    prices: &[PricePoint],
    threshold: f64,
) -> Result<(DataFrame, f64)> {
    #[derive(Clone)]
    struct AssetState {
        asset: DerAsset,
        soc: f64,
    }

    let mut states: Vec<AssetState> = assets
        .iter()
        .map(|asset| AssetState {
            asset: asset.clone(),
            soc: (asset.soc_min + asset.soc_max) / 2.0,
        })
        .collect();

    let mut timestamps = Vec::new();
    let mut asset_ids = Vec::new();
    let mut p_mw = Vec::new();
    let mut q_mvar = Vec::new();
    let mut soc = Vec::new();

    let mut curtailment_count = 0usize;
    let mut total_steps = 0usize;

    for point in prices {
        for state in states.iter_mut() {
            let desired = if point.price >= threshold {
                state.asset.p_min
            } else {
                state.asset.p_max
            };
            let delta = -desired;
            let next_soc = (state.soc + delta)
                .max(state.asset.soc_min)
                .min(state.asset.soc_max);
            let actual_delta = next_soc - state.soc;
            let actual_p = -actual_delta;
            if (actual_p - desired).abs() > 1e-6 {
                curtailment_count += 1;
            }
            state.soc = next_soc;

            timestamps.push(point.timestamp.clone());
            asset_ids.push(state.asset.id.clone());
            p_mw.push(actual_p);
            q_mvar.push(0.0);
            soc.push(state.soc);
            total_steps += 1;
        }
    }

    let rate = if total_steps == 0 {
        0.0
    } else {
        curtailment_count as f64 / total_steps as f64
    };

    let df = DataFrame::new(vec![
        Series::new("timestamp", timestamps),
        Series::new("asset_id", asset_ids),
        Series::new("p_mw", p_mw),
        Series::new("q_mvar", q_mvar),
        Series::new("soc", soc),
    ])?;

    Ok((df, rate))
}
