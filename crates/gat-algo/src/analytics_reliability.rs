use crate::io::{persist_dataframe, OutputStage};
use anyhow::{anyhow, Context, Result};
use polars::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// Summary statistics from reliability metrics computation.
/// Returned to callers so they can log how many scenarios/cases were analyzed.
pub struct ReliabilitySummary {
    pub num_scenarios: usize,
    pub num_time_periods: usize,
    pub total_cases: usize,
    pub lole_hours: f64,
    pub eue_mwh: f64,
    pub thermal_violations: usize,
}

/// Reliability metrics record for a single scenario/time combination.
struct ReliabilityRecord {
    scenario_id: Option<String>,
    time: Option<String>,
    lole: f64,               // Loss of Load Expectation: 1.0 if unserved load > 0, else 0.0
    eue_mwh: f64,            // Energy Unserved/Not Served (MWh)
    thermal_violations: i64, // Count of branches exceeding thermal limits
    max_flow_utilization: f64, // Maximum branch flow / limit ratio
    constrained_hours: f64,  // 1.0 if any constraint binding, else 0.0
}

/// Compute reliability metrics (LOLE, EUE, thermal violations) from batch PF/OPF outputs.
///
/// **Reliability Metrics:**
/// - **LOLE (Loss of Load Expectation)**: Expected number of hours with unserved load.
///   For each scenario/time, LOLE = 1.0 if unserved load > threshold, else 0.0.
///   Weighted sum across scenarios gives expected hours. See doi:10.1109/TPWRS.2012.2187686.
///
/// - **EUE/ENS (Energy Unserved/Not Served)**: Total energy (MWh) that could not be served
///   due to capacity constraints. Computed from load shedding or unserved load flags in OPF.
///
/// - **Thermal Violations**: Count of branches where flow exceeds thermal limit.
///   Indicates transmission congestion and potential reliability issues.
///
/// **Algorithm:**
/// 1. Load batch outputs (Parquet files with flows, optionally partitioned by scenario_id/time)
/// 2. For each scenario/time case:
///    - Detect unserved load (from OPF unserved_energy column or load_shedding flags)
///    - Count thermal violations (flows exceeding branch limits)
///    - Compute max flow utilization (max(flow / limit) across branches)
/// 3. Aggregate metrics: weighted LOLE, EUE, violation counts
/// 4. Output Parquet table keyed by (scenario_id, time, zone) for RA/KPI work
///
/// **Inputs:**
/// - `batch_manifest`: JSON manifest from `gat batch` listing all job outputs
/// - `flows_parquet`: Optional single Parquet file with all flows (alternative to manifest)
/// - `branch_limits_csv`: CSV with branch thermal limits (branch_id, flow_limit)
/// - `scenario_weights_csv`: Optional CSV with scenario probabilities/weights
/// - `unserved_threshold_mw`: Minimum unserved load to count as LOLE event (default 0.1 MW)
///
/// **Output:** Parquet table with columns: scenario_id, time, lole, eue_mwh, thermal_violations,
/// max_flow_utilization, constrained_hours, weight
pub fn reliability_metrics(
    batch_manifest: Option<&Path>,
    flows_parquet: Option<&Path>,
    branch_limits_csv: Option<&str>,
    scenario_weights_csv: Option<&str>,
    output_file: &Path,
    partitions: &[String],
    unserved_threshold_mw: f64,
) -> Result<ReliabilitySummary> {
    // Load branch thermal limits if provided (for violation detection)
    let branch_limits = if let Some(csv_path) = branch_limits_csv {
        load_branch_limits(csv_path)?
    } else {
        HashMap::new()
    };

    // Load scenario weights if provided (for weighted aggregation)
    let scenario_weights = if let Some(csv_path) = scenario_weights_csv {
        load_scenario_weights(csv_path)?
    } else {
        HashMap::new()
    };

    // Load flows data: either from batch manifest or single Parquet file
    let flows_df = if let Some(manifest_path) = batch_manifest {
        load_flows_from_manifest(manifest_path)?
    } else if let Some(parquet_path) = flows_parquet {
        LazyFrame::scan_parquet(parquet_path.to_str().unwrap(), Default::default())?
            .collect()
            .context("loading flows parquet for reliability metrics")?
    } else {
        return Err(anyhow!(
            "either batch_manifest or flows_parquet must be provided"
        ));
    };

    // Validate required columns
    if !flows_df
        .get_column_names()
        .iter()
        .any(|c| *c == "branch_id")
    {
        return Err(anyhow!("flows must contain 'branch_id' column"));
    }
    if !flows_df.get_column_names().iter().any(|c| *c == "flow_mw") {
        return Err(anyhow!("flows must contain 'flow_mw' column"));
    }

    // Group flows by scenario/time to compute per-case metrics
    let records = compute_reliability_records(
        &flows_df,
        &branch_limits,
        &scenario_weights,
        unserved_threshold_mw,
    )?;

    if records.is_empty() {
        return Err(anyhow!("no reliability records generated"));
    }

    // Build output DataFrame
    let mut scenario_col = Vec::with_capacity(records.len());
    let mut time_col = Vec::with_capacity(records.len());
    let mut lole_col = Vec::with_capacity(records.len());
    let mut eue_col = Vec::with_capacity(records.len());
    let mut violations_col = Vec::with_capacity(records.len());
    let mut utilization_col = Vec::with_capacity(records.len());
    let mut constrained_col = Vec::with_capacity(records.len());
    let mut weight_col = Vec::with_capacity(records.len());

    for record in &records {
        scenario_col.push(record.scenario_id.clone());
        time_col.push(record.time.clone());
        lole_col.push(record.lole);
        eue_col.push(record.eue_mwh);
        violations_col.push(record.thermal_violations);
        utilization_col.push(record.max_flow_utilization);
        constrained_col.push(record.constrained_hours);
        // Get weight from scenario_weights map, default to 1.0
        let weight = record
            .scenario_id
            .as_ref()
            .and_then(|s| scenario_weights.get(s))
            .copied()
            .unwrap_or(1.0);
        weight_col.push(weight);
    }

    let mut df = DataFrame::new(vec![
        Series::new("scenario_id", scenario_col),
        Series::new("time", time_col),
        Series::new("lole", lole_col),
        Series::new("eue_mwh", eue_col),
        Series::new("thermal_violations", violations_col),
        Series::new("max_flow_utilization", utilization_col),
        Series::new("constrained_hours", constrained_col),
        Series::new("weight", weight_col),
    ])?;

    // Persist to Parquet with optional partitioning
    persist_dataframe(
        &mut df,
        output_file,
        partitions,
        OutputStage::AnalyticsReliability.as_str(),
    )?;

    // Compute summary statistics
    let num_scenarios = records
        .iter()
        .filter_map(|r| r.scenario_id.as_ref())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let num_time_periods = records
        .iter()
        .filter_map(|r| r.time.as_ref())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let total_cases = records.len();
    let lole_hours: f64 = records.iter().map(|r| r.lole).sum();
    let eue_mwh: f64 = records.iter().map(|r| r.eue_mwh).sum();
    let thermal_violations: usize = records.iter().map(|r| r.thermal_violations as usize).sum();

    println!(
        "Reliability metrics: {} scenarios × {} time periods = {} cases, LOLE={:.2} hours, EUE={:.2} MWh, {} violations -> {}",
        num_scenarios,
        num_time_periods,
        total_cases,
        lole_hours,
        eue_mwh,
        thermal_violations,
        output_file.display()
    );

    Ok(ReliabilitySummary {
        num_scenarios,
        num_time_periods,
        total_cases,
        lole_hours,
        eue_mwh,
        thermal_violations,
    })
}

/// Compute reliability records for each scenario/time combination.
///
/// **Algorithm:**
/// 1. Group flows by (scenario_id, time) to create distinct cases
/// 2. For each case:
///    - Check for unserved_energy column (from OPF) to compute EUE
///    - Compare flows to branch limits to count violations
///    - Compute max flow utilization
///    - Set LOLE = 1.0 if EUE > threshold, else 0.0
fn compute_reliability_records(
    flows_df: &DataFrame,
    branch_limits: &HashMap<i64, f64>,
    _scenario_weights: &HashMap<String, f64>,
    unserved_threshold_mw: f64,
) -> Result<Vec<ReliabilityRecord>> {
    let mut records = Vec::new();

    // Determine grouping columns (scenario_id, time)
    let has_scenario = flows_df
        .get_column_names()
        .iter()
        .any(|c| *c == "scenario_id");
    let has_time = flows_df.get_column_names().iter().any(|c| *c == "time");
    let has_unserved = flows_df
        .get_column_names()
        .iter()
        .any(|c| *c == "unserved_energy_mw");

    // Group flows by case (scenario_id, time)
    let group_cols: Vec<String> = [
        has_scenario.then_some("scenario_id"),
        has_time.then_some("time"),
    ]
    .into_iter()
    .flatten()
    .map(|s| s.to_string())
    .collect();

    if group_cols.is_empty() {
        // Single case: treat entire DataFrame as one scenario/time
        let record = compute_case_metrics(
            flows_df,
            branch_limits,
            None,
            None,
            has_unserved,
            unserved_threshold_mw,
        )?;
        records.push(record);
    } else {
        // Multiple cases: group by scenario_id and/or time
        let group_by = flows_df.group_by(&group_cols)?;
        let groups = group_by.get_groups();

        for group in groups.iter() {
            // Extract first row index and case DataFrame
            let (first_row_idx, case_df) = match group {
                GroupsIndicator::Idx((first, idx_vec)) => {
                    let first_idx = first as usize;
                    let idx_ca = IdxCa::new("row_idx", idx_vec.as_slice());
                    let df = flows_df.take(&idx_ca)?;
                    (first_idx, df)
                }
                GroupsIndicator::Slice([first, len]) => {
                    let first_idx = first as usize;
                    let df = flows_df.slice(first as i64, len as usize);
                    (first_idx, df)
                }
            };

            // Extract scenario_id and time from first row
            let scenario_id = if has_scenario {
                extract_string_value(flows_df, "scenario_id", first_row_idx)?
            } else {
                None
            };
            let time = if has_time {
                extract_string_value(flows_df, "time", first_row_idx)?
            } else {
                None
            };

            let record = compute_case_metrics(
                &case_df,
                branch_limits,
                scenario_id,
                time,
                has_unserved,
                unserved_threshold_mw,
            )?;
            records.push(record);
        }
    }

    Ok(records)
}

/// Compute reliability metrics for a single case (scenario/time combination).
///
/// **Metrics computed:**
/// - EUE: Sum of unserved_energy_mw if present, else 0.0
/// - Thermal violations: Count of branches where |flow| > limit
/// - Max flow utilization: max(|flow| / limit) across all branches
/// - LOLE: 1.0 if EUE > threshold, else 0.0
/// - Constrained hours: 1.0 if any violation or high utilization, else 0.0
fn compute_case_metrics(
    case_df: &DataFrame,
    branch_limits: &HashMap<i64, f64>,
    scenario_id: Option<String>,
    time: Option<String>,
    has_unserved: bool,
    unserved_threshold_mw: f64,
) -> Result<ReliabilityRecord> {
    // Compute EUE (Energy Unserved) from unserved_energy column if present
    let eue_mwh = if has_unserved {
        let unserved_col = case_df.column("unserved_energy_mw")?.f64()?;
        unserved_col
            .into_iter()
            .map(|v| v.unwrap_or(0.0))
            .sum::<f64>()
    } else {
        0.0
    };

    // Count thermal violations and compute max utilization
    let branch_col = case_df.column("branch_id")?.i64()?;
    let flow_col = case_df.column("flow_mw")?.f64()?;
    let mut thermal_violations = 0i64;
    let mut max_utilization: f64 = 0.0;

    for idx in 0..case_df.height() {
        if let (Some(branch_id), Some(flow)) = (branch_col.get(idx), flow_col.get(idx)) {
            let flow_abs = flow.abs();
            if let Some(&limit) = branch_limits.get(&branch_id) {
                if limit > 0.0 {
                    let utilization = flow_abs / limit;
                    max_utilization = max_utilization.max(utilization);
                    if utilization > 1.0 {
                        thermal_violations += 1;
                    }
                }
            }
        }
    }

    // LOLE = 1.0 if unserved load exceeds threshold, else 0.0
    let lole = if eue_mwh > unserved_threshold_mw {
        1.0
    } else {
        0.0
    };

    // Constrained hours: 1.0 if any violation or high utilization (>0.95), else 0.0
    let constrained_hours = if thermal_violations > 0 || max_utilization > 0.95 {
        1.0
    } else {
        0.0
    };

    Ok(ReliabilityRecord {
        scenario_id,
        time,
        lole,
        eue_mwh,
        thermal_violations,
        max_flow_utilization: max_utilization,
        constrained_hours,
    })
}

/// Load flows from batch manifest by reading all job output Parquet files.
///
/// **Algorithm:** Reads batch_manifest.json, loads each job's output Parquet file,
/// and concatenates them into a single DataFrame with scenario_id/time columns.
fn load_flows_from_manifest(manifest_path: &Path) -> Result<DataFrame> {
    use chrono::{DateTime, Utc};
    use polars::prelude::ParquetReader;
    use serde::{Deserialize, Serialize};
    use serde_json;
    use std::fs::File;

    // Define BatchManifest locally to avoid dependency on gat-batch
    #[derive(Debug, Serialize, Deserialize)]
    struct BatchManifest {
        created_at: DateTime<Utc>,
        task: String,
        num_jobs: usize,
        success: usize,
        failure: usize,
        jobs: Vec<BatchJobRecord>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct BatchJobRecord {
        job_id: String,
        scenario_id: String,
        time: Option<String>,
        status: String,
        error: Option<String>,
        output: String,
    }

    // Load batch manifest JSON
    let file = File::open(manifest_path)
        .with_context(|| format!("opening batch manifest '{}'", manifest_path.display()))?;
    let manifest: BatchManifest = serde_json::from_reader(file)
        .with_context(|| format!("parsing batch manifest '{}'", manifest_path.display()))?;
    let mut all_dfs = Vec::new();

    for job in &manifest.jobs {
        if job.status == "ok" {
            let file = File::open(&job.output)
                .with_context(|| format!("opening job output '{}'", job.output))?;
            let reader = ParquetReader::new(file);
            let mut current_df = reader
                .finish()
                .with_context(|| format!("reading job output '{}'", job.output))?;

            // Add scenario_id column if not present
            if !current_df
                .get_column_names()
                .iter()
                .any(|c| *c == "scenario_id")
            {
                current_df.with_column(Series::new(
                    "scenario_id",
                    vec![job.scenario_id.clone(); current_df.height()],
                ))?;
            }

            // Add time column if not present
            if !current_df.get_column_names().iter().any(|c| *c == "time") {
                let time_str = job.time.as_deref().map(|s| s.to_string());
                current_df.with_column(Series::new("time", vec![time_str; current_df.height()]))?;
            }

            all_dfs.push(current_df);
        }
    }

    if all_dfs.is_empty() {
        return Err(anyhow!("no successful jobs found in batch manifest"));
    }

    // Concatenate all DataFrames
    let mut result = all_dfs[0].clone();
    for df in all_dfs.iter().skip(1) {
        result = result.vstack(df)?;
    }

    Ok(result)
}

/// Load branch thermal limits from CSV file.
///
/// **Expected CSV format:** branch_id, flow_limit (maximum flow in MW)
fn load_branch_limits(path: &str) -> Result<HashMap<i64, f64>> {
    use csv::ReaderBuilder;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct LimitRecord {
        branch_id: i64,
        flow_limit: f64,
    }

    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening branch limits CSV")?;
    let mut map = HashMap::new();
    for result in rdr.deserialize() {
        let record: LimitRecord = result.context("parsing branch limit record")?;
        map.insert(record.branch_id, record.flow_limit.abs());
    }
    Ok(map)
}

/// Load scenario weights/probabilities from CSV file.
///
/// **Expected CSV format:** scenario_id, weight (probability or importance weight)
fn load_scenario_weights(path: &str) -> Result<HashMap<String, f64>> {
    use csv::ReaderBuilder;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct WeightRecord {
        scenario_id: String,
        weight: f64,
    }

    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening scenario weights CSV")?;
    let mut map = HashMap::new();
    for result in rdr.deserialize() {
        let record: WeightRecord = result.context("parsing scenario weight record")?;
        map.insert(record.scenario_id, record.weight.abs());
    }
    Ok(map)
}

/// Extract string value from DataFrame column at given row index.
fn extract_string_value(df: &DataFrame, column: &str, idx: usize) -> Result<Option<String>> {
    let series = df.column(column)?;
    match series.get(idx) {
        Ok(value) => {
            let s = value.to_string();
            if s == "null" {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
        Err(_) => Ok(None),
    }
}
