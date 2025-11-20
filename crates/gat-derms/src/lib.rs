use std::path::Path;

use anyhow::{Context, Result, anyhow};
use polars::prelude::*;
use rand::prelude::*;

pub const DER_ASSET_COLUMNS: &[&str] = &[
    "asset_id",
    "bus_id",
    "phase",
    "asset_type",
    "p_min",
    "p_max",
    "q_min",
    "q_max",
    "ramp_up",
    "ramp_down",
    "energy_cap",
    "soc_min",
    "soc_max",
    "efficiency",
    "owner_id",
    "agg_id",
    "priority",
    "cost_curve_id",
];

pub fn validate_assets(path: &Path) -> Result<DataFrame> {
    let assets = LazyFrame::scan_parquet(path, Default::default())
        .context("opening der_assets parquet")?
        .collect()?;
    let missing: Vec<_> = DER_ASSET_COLUMNS
        .iter()
        .filter(|col| !assets.get_column_names().iter().any(|name| name == **col))
        .copied()
        .collect();
    if !missing.is_empty() {
        return Err(anyhow!(
            "der_assets missing required columns: {}",
            missing.join(", ")
        ));
    }
    Ok(assets)
}

pub fn envelope(
    vertices_out: &Path,
    assets: &DataFrame,
    group_col: Option<&str>,
) -> Result<DataFrame> {
    let grouping = group_col.unwrap_or("agg_id");
    let groups = assets
        .column(grouping)
        .context("group column missing")?
        .utf8()?;
    let p_min = assets.column("p_min")?.f64()?;
    let p_max = assets.column("p_max")?.f64()?;
    let q_min = assets.column("q_min")?.f64()?;
    let q_max = assets.column("q_max")?.f64()?;

    let mut agg: std::collections::HashMap<String, (f64, f64, f64, f64)> =
        std::collections::HashMap::new();
    for idx in 0..assets.height() {
        let key = groups.get(idx).unwrap_or("unassigned").to_string();
        let entry = agg.entry(key).or_insert((0.0, 0.0, 0.0, 0.0));
        entry.0 += p_min.get(idx).unwrap_or(0.0);
        entry.1 += p_max.get(idx).unwrap_or(0.0);
        entry.2 += q_min.get(idx).unwrap_or(0.0);
        entry.3 += q_max.get(idx).unwrap_or(0.0);
    }

    let mut region = Vec::new();
    let mut vertex_id = Vec::new();
    let mut p = Vec::new();
    let mut q = Vec::new();
    for (region_id, (p_lo, p_hi, q_lo, q_hi)) in agg {
        let vertices = vec![(p_lo, q_lo), (p_lo, q_hi), (p_hi, q_lo), (p_hi, q_hi)];
        for (idx, (p_v, q_v)) in vertices.into_iter().enumerate() {
            region.push(region_id.clone());
            vertex_id.push(idx as i64);
            p.push(p_v);
            q.push(q_v);
        }
    }

    let df = DataFrame::new(vec![
        Series::new("region", region.clone()),
        Series::new("vertex_id", vertex_id),
        Series::new("p_mw", p),
        Series::new("q_mvar", q),
    ])?;
    write_parquet(vertices_out, &df)?;
    Ok(df)
}

pub fn schedule(
    assets: &DataFrame,
    horizon: usize,
    timestep_mins: u32,
    out: &Path,
    summary_out: Option<&Path>,
) -> Result<(DataFrame, Option<DataFrame>)> {
    if horizon == 0 {
        return Err(anyhow!("horizon must be positive"));
    }
    let mut asset_ids: Vec<String> = Vec::new();
    let mut time_index = Vec::new();
    let mut p = Vec::new();
    let mut q = Vec::new();
    let mut soc = Vec::new();

    for row in 0..assets.height() {
        let asset = assets
            .column("asset_id")?
            .utf8()?
            .get(row)
            .unwrap_or("asset");
        let p_max = assets.column("p_max")?.f64()?.get(row).unwrap_or(0.0);
        let q_max = assets.column("q_max")?.f64()?.get(row).unwrap_or(0.0);
        for t in 0..horizon {
            asset_ids.push(asset.to_string());
            time_index.push(t as i64);
            p.push(p_max);
            q.push(q_max);
            soc.push(assets.column("soc_max")?.f64()?.get(row).unwrap_or(0.0));
        }
    }

    let schedule = DataFrame::new(vec![
        Series::new("asset_id", asset_ids),
        Series::new("timestep", time_index),
        Series::new("p_mw", p),
        Series::new("q_mvar", q),
        Series::new("soc", soc),
        Series::new(
            "timestep_minutes",
            vec![timestep_mins as i64; time_index.len()],
        ),
    ])?;
    write_parquet(out, &schedule)?;

    let summary = if let Some(path) = summary_out {
        let grouped = schedule
            .clone()
            .lazy()
            .group_by([col("asset_id")])
            .agg([
                col("p_mw").sum().alias("p_total"),
                col("q_mvar").sum().alias("q_total"),
            ])
            .collect()?;
        write_parquet(path, &grouped)?;
        Some(grouped)
    } else {
        None
    };

    Ok((schedule, summary))
}

pub fn stress_test(assets: &DataFrame, runs: usize, out: &Path) -> Result<DataFrame> {
    if runs == 0 {
        return Err(anyhow!("runs must be positive"));
    }
    let mut rng = thread_rng();
    let mut run_id = Vec::new();
    let mut curtailed = Vec::new();
    let mut voltage_violations = Vec::new();

    for r in 0..runs {
        run_id.push(r as i64);
        let curtail: f64 = rng.gen_range(0.0..1.0);
        curtailed.push(curtail);
        voltage_violations.push(rng.gen_range(0..3) as i64);
    }

    let df = DataFrame::new(vec![
        Series::new("run_id", run_id),
        Series::new("curtailment_fraction", curtailed),
        Series::new("voltage_violations", voltage_violations),
        Series::new("asset_count", vec![assets.height() as i64; curtailed.len()]),
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
