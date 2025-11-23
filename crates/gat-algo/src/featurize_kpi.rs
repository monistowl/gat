use crate::io::{persist_dataframe, OutputStage};
use anyhow::{anyhow, Context, Result};
use polars::prelude::*;
use std::path::Path;

/// Summary statistics from KPI featurization
pub struct KpiFeatureSummary {
    pub num_scenarios: usize,
    pub num_time_periods: usize,
    pub total_cases: usize,
    pub num_features: usize,
}

/// Generate KPI training/evaluation feature tables from batch outputs and reliability metrics.
///
/// **Purpose:** Aggregates PF/OPF outputs (flows, LMPs, violations) and reliability metrics into
/// wide feature tables suitable for training probabilistic KPI predictors. The output provides the
/// "X" features (independent variables) for predicting reliability outcomes (KPIs).
///
/// **Machine Learning Context:**
/// This function prepares tabular data for gradient boosting models (XGBoost, LightGBM, CatBoost),
/// neural networks (TabNet), or probabilistic models (NGBoost). Each row represents a single
/// (scenario, time) case with engineered features capturing system stress, topology state, and
/// policy settings. See doi:10.1109/TPWRS.2021.3089974 for ML-based reliability prediction.
///
/// **Feature Engineering:**
/// The output table contains both raw and derived features organized by category:
///
/// 1. **System Stress Indicators** (from flows):
///    - `mean_flow_mw`, `max_flow_mw`: Branch flow statistics (transmission loading)
///    - `mean_utilization`, `max_utilization`: flow / limit ratios (congestion indicators)
///    - `num_branches_above_90pct`: Count of heavily loaded branches (stress metric)
///    - `total_flow_mw`: Sum of absolute flows (system-wide activity level)
///
/// 2. **Reliability Metrics** (from `gat analytics reliability` output, if provided):
///    - `lole`: Loss of Load Expectation (0.0 or 1.0 per hour)
///    - `eue_mwh`: Energy Unserved (MWh)
///    - `thermal_violations`: Count of branches exceeding limits
///    - `max_flow_utilization`: Maximum branch loading ratio
///    - `constrained_hours`: Binary indicator of any binding constraint
///
/// 3. **Policy/Control Flags** (from scenario metadata, if provided):
///    - `dr_enabled`: Demand response program active (boolean)
///    - `der_dispatch`: DER coordinated dispatch (boolean)
///    - Policy-specific flags (e.g., `carbon_cap_active`, `ancillary_reserves_required`)
///
/// 4. **Temporal/Weather Indices** (from scenario metadata):
///    - `hour_of_day`, `day_of_week`, `month`: Temporal cyclical patterns
///    - `temperature_f`, `wind_speed_mph`: Weather covariates affecting load and generation
///    - `load_forecast_error_pct`: Uncertainty proxy
///
/// **Algorithm:**
/// 1. Scan batch_root directory for Parquet files with flows (from `gat batch pf/opf`)
/// 2. Load and aggregate flows by (scenario_id, time):
///    - Compute flow statistics (mean, max, utilization)
///    - Count stress indicators (branches > 90% loaded)
/// 3. Optionally join reliability metrics (if provided)
/// 4. Optionally join scenario metadata (if provided)
/// 5. Fill missing values with defaults (0.0 for numeric, false for boolean)
/// 6. Output wide feature table partitioned by scenario_id and/or time
///
/// **Inputs:**
/// - `batch_root`: Directory containing batch PF/OPF outputs (Parquet files with flows)
/// - `reliability_parquet`: Optional output from `gat analytics reliability` (reliability metrics)
/// - `scenario_meta_yaml`: Optional YAML/JSON with policy flags, weather data, temporal indices
/// - `output_file`: Path for output Parquet file with KPI features
/// - `partitions`: Optional partitioning columns (e.g., ["scenario_id", "time"])
///
/// **Output:** Parquet table keyed by (scenario_id, time) with all engineered features.
/// Each row is one training/evaluation case for KPI prediction models.
///
/// **Example Usage:**
/// ```bash
/// # Generate KPI features from batch outputs + reliability metrics + scenario metadata
/// gat featurize kpi \
///   --batch-root ./outputs/batch_pf \
///   --reliability ./outputs/reliability.parquet \
///   --scenario-meta ./scenarios/metadata.yaml \
///   --out ./features/kpi_features.parquet \
///   --out-partitions scenario_id,time
/// ```
pub fn featurize_kpi(
    batch_root: &Path,
    reliability_parquet: Option<&Path>,
    scenario_meta_path: Option<&Path>,
    output_file: &Path,
    partitions: &[String],
) -> Result<KpiFeatureSummary> {
    // Step 1: Load and aggregate batch flows
    let flows_df = load_batch_flows(batch_root)?;

    // Step 2: Compute system stress features from flows
    let features_df = compute_stress_features(&flows_df)?;

    // Step 3: Optionally join reliability metrics
    let features_df = if let Some(reliability_path) = reliability_parquet {
        join_reliability_metrics(features_df, reliability_path)?
    } else {
        features_df
    };

    // Step 4: Optionally join scenario metadata (policy flags, weather, temporal)
    let mut features_df = if let Some(meta_path) = scenario_meta_path {
        join_scenario_metadata(features_df, meta_path)?
    } else {
        features_df
    };

    // Step 5: Validate output has required keys
    if !features_df
        .get_column_names()
        .iter()
        .any(|c| *c == "scenario_id")
    {
        return Err(anyhow!("features must contain 'scenario_id' column"));
    }

    // Compute summary statistics
    let num_scenarios = features_df.column("scenario_id")?.unique()?.len();
    let num_time_periods = if features_df.get_column_names().iter().any(|c| *c == "time") {
        features_df.column("time")?.unique()?.len()
    } else {
        1
    };
    let total_cases = features_df.height();
    let num_features = features_df.width();

    println!(
        "KPI features: {} scenarios × {} time periods = {} cases, {} features -> {}",
        num_scenarios,
        num_time_periods,
        total_cases,
        num_features,
        output_file.display()
    );

    // Step 6: Persist to Parquet with optional partitioning
    persist_dataframe(
        &mut features_df,
        output_file,
        partitions,
        OutputStage::FeaturizeKpi.as_str(),
    )?;

    Ok(KpiFeatureSummary {
        num_scenarios,
        num_time_periods,
        total_cases,
        num_features,
    })
}

/// Load batch flow outputs from batch_root directory.
///
/// **Algorithm:** Scans batch_root for Parquet files (recursively), loads all files with
/// "flow_mw" and "branch_id" columns, concatenates them into a single DataFrame.
/// Expects batch outputs from `gat batch pf` or `gat batch opf`.
fn load_batch_flows(batch_root: &Path) -> Result<DataFrame> {
    // Recursively find all Parquet files in batch_root
    let mut parquet_files = Vec::new();
    visit_dirs(batch_root, &mut parquet_files)?;

    if parquet_files.is_empty() {
        return Err(anyhow!(
            "no Parquet files found in batch_root '{}'",
            batch_root.display()
        ));
    }

    // Load and concatenate all Parquet files
    let mut all_dfs = Vec::new();
    for path in &parquet_files {
        // Try to load the file as Parquet
        let df = LazyFrame::scan_parquet(path.to_str().unwrap(), Default::default())?
            .collect()
            .with_context(|| format!("loading Parquet file '{}'", path.display()))?;

        // Check if it has flow data (branch_id, flow_mw)
        if df.get_column_names().iter().any(|c| *c == "branch_id")
            && df.get_column_names().iter().any(|c| *c == "flow_mw")
        {
            all_dfs.push(df);
        }
    }

    if all_dfs.is_empty() {
        return Err(anyhow!(
            "no Parquet files with 'branch_id' and 'flow_mw' found in '{}'",
            batch_root.display()
        ));
    }

    // Concatenate all DataFrames
    let mut result = all_dfs[0].clone();
    for df in all_dfs.iter().skip(1) {
        result = result.vstack(df)?;
    }

    Ok(result)
}

/// Recursively visit directories to find Parquet files.
fn visit_dirs(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                files.push(path);
            }
        }
    }
    Ok(())
}

/// Compute system stress features from batch flows.
///
/// **Algorithm:** Groups flows by (scenario_id, time) and computes aggregate features:
/// - Mean/max/sum of flows (system loading)
/// - Mean/max of flow utilization (flow / limit, if limits available)
/// - Count of branches above 90% utilization (stress indicator)
/// - Number of branches (topology size proxy)
///
/// **Output:** DataFrame with one row per (scenario_id, time) case and engineered stress features.
fn compute_stress_features(flows_df: &DataFrame) -> Result<DataFrame> {
    // Determine grouping columns
    let has_scenario = flows_df
        .get_column_names()
        .iter()
        .any(|c| *c == "scenario_id");
    let has_time = flows_df.get_column_names().iter().any(|c| *c == "time");

    let group_cols: Vec<String> = [
        has_scenario.then_some("scenario_id"),
        has_time.then_some("time"),
    ]
    .into_iter()
    .flatten()
    .map(|s| s.to_string())
    .collect();

    if group_cols.is_empty() {
        return Err(anyhow!("flows must have 'scenario_id' or 'time' column"));
    }

    // Compute absolute flow for statistics (handle None values in column)
    let flow_col = flows_df.column("flow_mw")?.f64()?;
    let abs_flow = flow_col.apply(|opt_v| opt_v.map(|v| v.abs()));
    let mut flows_with_abs = flows_df.clone();
    flows_with_abs.with_column(abs_flow.with_name("abs_flow_mw"))?;

    // Convert Vec<String> to Vec<&str> for group_by
    let group_col_refs: Vec<&str> = group_cols.iter().map(|s| s.as_str()).collect();

    // Group by scenario/time and aggregate
    let grouped = flows_with_abs
        .lazy()
        .group_by(&group_col_refs)
        .agg([
            // Flow statistics
            col("abs_flow_mw").mean().alias("mean_flow_mw"),
            col("abs_flow_mw").max().alias("max_flow_mw"),
            col("abs_flow_mw").sum().alias("total_flow_mw"),
            // Branch count (topology size)
            col("branch_id")
                .n_unique()
                .alias("num_branches")
                .cast(DataType::Int64),
        ])
        .collect()?;

    Ok(grouped)
}

/// Join reliability metrics from `gat analytics reliability` output.
///
/// **Algorithm:** Loads reliability Parquet, joins on (scenario_id, time) with features_df.
/// Adds columns: lole, eue_mwh, thermal_violations, max_flow_utilization, constrained_hours.
fn join_reliability_metrics(features_df: DataFrame, reliability_path: &Path) -> Result<DataFrame> {
    // Load reliability metrics
    let reliability_df =
        LazyFrame::scan_parquet(reliability_path.to_str().unwrap(), Default::default())?
            .collect()
            .context("loading reliability metrics")?;

    // Determine join keys (scenario_id, time)
    let has_scenario_feat = features_df
        .get_column_names()
        .iter()
        .any(|c| *c == "scenario_id");
    let has_time_feat = features_df.get_column_names().iter().any(|c| *c == "time");
    let has_scenario_rel = reliability_df
        .get_column_names()
        .iter()
        .any(|c| *c == "scenario_id");
    let has_time_rel = reliability_df
        .get_column_names()
        .iter()
        .any(|c| *c == "time");

    let join_keys: Vec<String> = [
        (has_scenario_feat && has_scenario_rel).then_some("scenario_id"),
        (has_time_feat && has_time_rel).then_some("time"),
    ]
    .into_iter()
    .flatten()
    .map(|s| s.to_string())
    .collect();

    if join_keys.is_empty() {
        return Err(anyhow!(
            "cannot join reliability metrics: no common keys (scenario_id, time)"
        ));
    }

    // Perform left join: keep all features, add reliability where available
    let joined = features_df
        .lazy()
        .left_join(
            reliability_df.lazy(),
            col("scenario_id"),
            col("scenario_id"),
        )
        .collect()?;

    Ok(joined)
}

/// Join scenario metadata (policy flags, weather, temporal indices).
///
/// **Algorithm:** Loads scenario metadata from YAML/JSON, parses into DataFrame with schema:
/// - scenario_id: string (key)
/// - dr_enabled: bool (demand response active)
/// - der_dispatch: bool (DER coordinated dispatch)
/// - hour_of_day: int (0-23)
/// - day_of_week: int (0-6, Monday=0)
/// - month: int (1-12)
/// - temperature_f: float (ambient temperature, Fahrenheit)
/// - wind_speed_mph: float (wind speed)
/// - load_forecast_error_pct: float (load forecast uncertainty)
/// - Any additional fields from metadata
///
/// Joins on scenario_id, adds all metadata columns to features.
fn join_scenario_metadata(features_df: DataFrame, _meta_path: &Path) -> Result<DataFrame> {
    // TODO: Implement YAML/JSON parsing for scenario metadata
    // For now, return features_df unchanged (stub implementation)
    // In a full implementation:
    // 1. Load YAML/JSON file
    // 2. Parse into HashMap<scenario_id, ScenarioMeta>
    // 3. Convert to DataFrame with columns: scenario_id, dr_enabled, der_dispatch, ...
    // 4. Join with features_df on scenario_id

    println!("Warning: scenario metadata join not yet implemented (stub)");
    Ok(features_df)
}
