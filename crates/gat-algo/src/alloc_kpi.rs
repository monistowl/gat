use crate::io::{persist_dataframe, OutputStage};
use anyhow::{anyhow, Context, Result};
use polars::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// Summary statistics from KPI contribution analysis
pub struct KpiContributionSummary {
    pub num_scenarios: usize,
    pub num_controls: usize,
    pub num_kpis: usize,
}

/// Compute simple contribution analysis for KPI changes across scenarios.
///
/// **Purpose:** Approximates the contribution of control actions/portfolios to KPI improvements
/// using gradient-based sensitivity or linear approximations. This is a stepping stone towards
/// full SHAP (SHapley Additive exPlanations) explainability.
///
/// **Explainability Context:**
/// In electricity planning, we run scenarios with different policy/control settings (demand response
/// programs, DER dispatch coordination, carbon caps, etc.) and observe changes in reliability KPIs
/// (LOLE, EUE, thermal violations). We want to attribute these KPI changes to specific interventions
/// to answer: "Which controls contributed most to reliability improvement?"
///
/// **Algorithm (v0 - Finite Differences):**
/// 1. Load KPI results across scenarios (scenario_id, kpi_name, kpi_value)
/// 2. Load scenario metadata (scenario_id, control flags like dr_enabled, der_dispatch, etc.)
/// 3. Identify baseline scenario (all controls off, or user-specified reference)
/// 4. For each control and KPI:
///    - Compute ΔKPIcontrol = mean(KPI | control=on) - mean(KPI | control=off)
///    - Compute contribution percentage: |ΔKPIcontrol| / Σ|ΔKPIi|
/// 5. Rank controls by absolute contribution magnitude
/// 6. Output contribution table: kpi_name, control_name, delta, contribution_pct, rank
///
/// **Pedagogical Note for Grad Students:**
/// This is a **naive attribution method** that assumes controls are independent (which they rarely
/// are). It provides a quick first-order approximation but has limitations:
///
/// **Limitations:**
/// - **Interaction Effects**: If control A and B together create synergies (or antagonisms),
///   this method misattributes contributions since it treats them independently.
/// - **Correlation**: If controls are correlated in the scenario design (e.g., DR and DER always
///   deployed together), we can't disentangle their individual effects.
/// - **Non-linearity**: KPI response may be highly non-linear in control settings; finite differences
///   only capture local sensitivity.
///
/// **Better Methods (Future Work):**
/// - **SHAP (Shapley Additive exPlanations)**: Computes marginal contributions by considering all
///   possible coalitions of controls. Satisfies fairness axioms (symmetry, dummy, additivity).
///   See Lundberg & Lee (2017), doi:10.5555/3295222.3295230.
/// - **Partition SHAP**: Hierarchical SHAP for nested control structures (zone-level vs. resource-level).
/// - **Regression-based attribution**: Fit linear/GLM model KPI ~ control_1 + ... + control_n,
///   use coefficients as contributions. Handles correlations better than naive differences.
/// - **Counterfactual analysis**: Use causal inference to estimate "what if control X was off?"
///   from observational scenario data. See Pearl's causal calculus or potential outcomes framework.
///
/// **References:**
/// - SHAP theory: Lundberg & Lee (2017), "A Unified Approach to Interpreting Model Predictions"
///   doi:10.5555/3295222.3295230
/// - SHAP applications: Molnar et al. (2019), doi:10.1038/s42256-019-0138-9
/// - Causal inference: Pearl (2009), "Causality: Models, Reasoning and Inference"
///
/// **Inputs:**
/// - `kpi_results_parquet`: Parquet file with KPI results (scenario_id, kpi_name, kpi_value)
///   Example: scenario_id="baseline", kpi_name="lole", kpi_value=2.5
/// - `scenario_meta_parquet`: Parquet file with scenario metadata (scenario_id, control flags)
///   Example: scenario_id="baseline", dr_enabled=false, der_dispatch=false
///   Example: scenario_id="dr_only", dr_enabled=true, der_dispatch=false
/// - `baseline_scenario_id`: Optional baseline scenario ID (default: "baseline" or first with all controls off)
/// - `output_file`: Path for output Parquet table with contributions
/// - `partitions`: Optional partitioning columns (e.g., ["kpi_name"])
///
/// **Output:** Parquet table with columns:
/// - kpi_name: Which KPI (lole, eue_mwh, thermal_violations, etc.)
/// - control_name: Which control (dr_enabled, der_dispatch, carbon_cap, etc.)
/// - baseline_value: KPI value in baseline scenario
/// - control_on_mean: Mean KPI value when control is on
/// - control_off_mean: Mean KPI value when control is off
/// - delta: control_on_mean - baseline_value (negative = improvement for KPIs like LOLE)
/// - abs_delta: |delta| (for ranking)
/// - contribution_pct: abs_delta / sum(abs_delta) across all controls (% attribution)
/// - rank: 1 = largest absolute contribution
///
/// **Example Usage:**
/// ```bash
/// # 1. Run scenarios with different control settings
/// gat batch opf --scenarios ./scenarios/control_variants.yaml --out ./outputs/opf_results.parquet
///
/// # 2. Compute KPI metrics for each scenario
/// gat analytics reliability --flows ./outputs/opf_results.parquet --out ./outputs/kpi_metrics.parquet
///
/// # 3. Attribute KPI changes to control actions
/// gat alloc kpi \
///   --kpi-results ./outputs/kpi_metrics.parquet \
///   --scenario-meta ./scenarios/control_metadata.parquet \
///   --out ./outputs/kpi_contributions.parquet
/// ```
pub fn compute_kpi_contributions(
    kpi_results_parquet: &Path,
    scenario_meta_parquet: &Path,
    baseline_scenario_id: Option<&str>,
    output_file: &Path,
    partitions: &[String],
) -> Result<KpiContributionSummary> {
    // Load KPI results (scenario_id, kpi columns like lole, eue_mwh, thermal_violations)
    let kpi_df =
        LazyFrame::scan_parquet(kpi_results_parquet.to_str().unwrap(), Default::default())?
            .collect()
            .context("loading KPI results")?;

    // Load scenario metadata (scenario_id, control flags like dr_enabled, der_dispatch)
    let meta_df =
        LazyFrame::scan_parquet(scenario_meta_parquet.to_str().unwrap(), Default::default())?
            .collect()
            .context("loading scenario metadata")?;

    // Validate required columns
    if !kpi_df.get_column_names().contains(&"scenario_id") {
        return Err(anyhow!("KPI results must contain 'scenario_id' column"));
    }
    if !meta_df.get_column_names().contains(&"scenario_id") {
        return Err(anyhow!(
            "Scenario metadata must contain 'scenario_id' column"
        ));
    }

    // Identify control columns before joining (to avoid moving meta_df)
    let control_columns = identify_control_columns(&meta_df)?;
    if control_columns.is_empty() {
        return Err(anyhow!(
            "No control columns found in metadata (expected boolean columns like dr_enabled)"
        ));
    }

    // Join KPI results with scenario metadata on scenario_id
    let joined_df = kpi_df
        .lazy()
        .left_join(meta_df.lazy(), col("scenario_id"), col("scenario_id"))
        .collect()?;

    // Identify KPI columns (numeric, not scenario_id or control flags)
    let kpi_columns = identify_kpi_columns(&joined_df)?;
    if kpi_columns.is_empty() {
        return Err(anyhow!(
            "No KPI columns found in results (expected numeric columns like lole, eue_mwh)"
        ));
    }

    // Identify baseline scenario (all controls off, or user-specified)
    let baseline_id =
        identify_baseline_scenario(&joined_df, &control_columns, baseline_scenario_id)?;

    // Compute baseline KPI values
    let baseline_kpis = extract_baseline_kpis(&joined_df, &baseline_id, &kpi_columns)?;

    // Compute contributions for each (kpi, control) pair
    let contributions =
        compute_contributions(&joined_df, &kpi_columns, &control_columns, &baseline_kpis)?;

    // Convert contributions to DataFrame
    let mut contrib_df = contributions_to_dataframe(contributions)?;

    println!(
        "KPI contribution analysis: {} KPIs × {} controls = {} contributions -> {}",
        kpi_columns.len(),
        control_columns.len(),
        contrib_df.height(),
        output_file.display()
    );

    // Persist to Parquet with optional partitioning
    persist_dataframe(
        &mut contrib_df,
        output_file,
        partitions,
        OutputStage::AllocKpi.as_str(),
    )?;

    Ok(KpiContributionSummary {
        num_scenarios: joined_df.height(),
        num_controls: control_columns.len(),
        num_kpis: kpi_columns.len(),
    })
}

/// Identify KPI columns (numeric, not metadata or identifiers)
fn identify_kpi_columns(df: &DataFrame) -> Result<Vec<String>> {
    let mut kpi_cols = Vec::new();
    for col_name in df.get_column_names() {
        // Skip scenario_id and control flags
        if col_name == "scenario_id"
            || col_name.ends_with("_enabled")
            || col_name.ends_with("_dispatch")
        {
            continue;
        }
        // Include numeric columns
        if let Ok(series) = df.column(col_name) {
            if series.dtype().is_numeric() {
                kpi_cols.push(col_name.to_string());
            }
        }
    }
    Ok(kpi_cols)
}

/// Identify control columns (boolean flags in metadata)
fn identify_control_columns(df: &DataFrame) -> Result<Vec<String>> {
    let mut control_cols = Vec::new();
    for col_name in df.get_column_names() {
        if col_name == "scenario_id" {
            continue;
        }
        // Include boolean columns (controls are typically boolean flags)
        if let Ok(series) = df.column(col_name) {
            if matches!(series.dtype(), DataType::Boolean) {
                control_cols.push(col_name.to_string());
            }
        }
    }
    Ok(control_cols)
}

/// Identify baseline scenario (all controls off, or user-specified)
fn identify_baseline_scenario(
    df: &DataFrame,
    control_columns: &[String],
    baseline_scenario_id: Option<&str>,
) -> Result<String> {
    if let Some(baseline_id) = baseline_scenario_id {
        return Ok(baseline_id.to_string());
    }

    // Find first scenario where all controls are false
    let scenario_col = df.column("scenario_id")?;
    for idx in 0..df.height() {
        let mut all_off = true;
        for control_col in control_columns {
            if let Ok(control_series) = df.column(control_col) {
                if let Ok(control_bool) = control_series.bool() {
                    if control_bool.get(idx) == Some(true) {
                        all_off = false;
                        break;
                    }
                }
            }
        }
        if all_off {
            // Get scenario_id as string
            let scenario_id = scenario_col.get(idx)?.to_string();
            // Remove quotes if present (AnyValue::to_string() may add quotes)
            let scenario_id = scenario_id.trim_matches('"').to_string();
            return Ok(scenario_id);
        }
    }

    Err(anyhow!("No baseline scenario found (expected scenario with all controls off, or specify --baseline-scenario)"))
}

/// Extract baseline KPI values
fn extract_baseline_kpis(
    df: &DataFrame,
    baseline_id: &str,
    kpi_columns: &[String],
) -> Result<HashMap<String, f64>> {
    let scenario_col = df.column("scenario_id")?;
    let mut baseline_kpis = HashMap::new();

    // Find baseline row
    for idx in 0..df.height() {
        let scenario_id = scenario_col
            .get(idx)
            .ok()
            .map(|v| v.to_string().trim_matches('"').to_string());
        if let Some(scenario_id) = scenario_id {
            if scenario_id.as_str() == baseline_id {
                // Extract KPI values
                for kpi_col in kpi_columns {
                    if let Ok(kpi_series) = df.column(kpi_col) {
                        if let Ok(kpi_f64) = kpi_series.f64() {
                            if let Some(value) = kpi_f64.get(idx) {
                                baseline_kpis.insert(kpi_col.to_string(), value);
                            }
                        }
                    }
                }
                break;
            }
        }
    }

    if baseline_kpis.is_empty() {
        return Err(anyhow!(
            "Baseline scenario '{}' not found or has no KPI values",
            baseline_id
        ));
    }

    Ok(baseline_kpis)
}

/// Contribution record for a single (kpi, control) pair
struct Contribution {
    kpi_name: String,
    control_name: String,
    baseline_value: f64,
    control_on_mean: f64,
    control_off_mean: f64,
    delta: f64,
    abs_delta: f64,
}

/// Compute contributions for all (kpi, control) pairs
fn compute_contributions(
    df: &DataFrame,
    kpi_columns: &[String],
    control_columns: &[String],
    baseline_kpis: &HashMap<String, f64>,
) -> Result<Vec<Contribution>> {
    let mut contributions = Vec::with_capacity(kpi_columns.len() * control_columns.len());
    let row_count = df.height();

    for kpi_col in kpi_columns {
        let baseline_value = *baseline_kpis.get(kpi_col).unwrap_or(&0.0);

        for control_col in control_columns {
            // Compute mean KPI when control is on vs. off
            let control_series = df.column(control_col)?.bool()?;
            let kpi_series = df.column(kpi_col)?.f64()?;

            let mut control_on_values = Vec::with_capacity(row_count);
            let mut control_off_values = Vec::with_capacity(row_count);

            for idx in 0..df.height() {
                if let (Some(control_val), Some(kpi_val)) =
                    (control_series.get(idx), kpi_series.get(idx))
                {
                    if control_val {
                        control_on_values.push(kpi_val);
                    } else {
                        control_off_values.push(kpi_val);
                    }
                }
            }

            let control_on_mean = if !control_on_values.is_empty() {
                control_on_values.iter().sum::<f64>() / control_on_values.len() as f64
            } else {
                baseline_value
            };

            let control_off_mean = if !control_off_values.is_empty() {
                control_off_values.iter().sum::<f64>() / control_off_values.len() as f64
            } else {
                baseline_value
            };

            let delta = control_on_mean - baseline_value;
            let abs_delta = delta.abs();

            contributions.push(Contribution {
                kpi_name: kpi_col.to_string(),
                control_name: control_col.to_string(),
                baseline_value,
                control_on_mean,
                control_off_mean,
                delta,
                abs_delta,
            });
        }
    }

    Ok(contributions)
}

/// Convert contributions to DataFrame with ranking and contribution percentages
fn contributions_to_dataframe(mut contributions: Vec<Contribution>) -> Result<DataFrame> {
    // Sort by absolute delta (largest first)
    contributions.sort_by(|a, b| b.abs_delta.partial_cmp(&a.abs_delta).unwrap());

    // Compute total absolute delta for contribution percentage
    let total_abs_delta: f64 = contributions.iter().map(|c| c.abs_delta).sum();

    let capacity = contributions.len();
    let mut kpi_names = Vec::with_capacity(capacity);
    let mut control_names = Vec::with_capacity(capacity);
    let mut baseline_values = Vec::with_capacity(capacity);
    let mut control_on_means = Vec::with_capacity(capacity);
    let mut control_off_means = Vec::with_capacity(capacity);
    let mut deltas = Vec::with_capacity(capacity);
    let mut abs_deltas = Vec::with_capacity(capacity);
    let mut contribution_pcts = Vec::with_capacity(capacity);
    let mut ranks = Vec::with_capacity(capacity);

    for (rank, contrib) in contributions.iter().enumerate() {
        let contribution_pct = if total_abs_delta > 0.0 {
            contrib.abs_delta / total_abs_delta * 100.0
        } else {
            0.0
        };

        kpi_names.push(contrib.kpi_name.clone());
        control_names.push(contrib.control_name.clone());
        baseline_values.push(contrib.baseline_value);
        control_on_means.push(contrib.control_on_mean);
        control_off_means.push(contrib.control_off_mean);
        deltas.push(contrib.delta);
        abs_deltas.push(contrib.abs_delta);
        contribution_pcts.push(contribution_pct);
        ranks.push((rank + 1) as i64);
    }

    Ok(DataFrame::new(vec![
        Series::new("kpi_name", kpi_names),
        Series::new("control_name", control_names),
        Series::new("baseline_value", baseline_values),
        Series::new("control_on_mean", control_on_means),
        Series::new("control_off_mean", control_off_means),
        Series::new("delta", deltas),
        Series::new("abs_delta", abs_deltas),
        Series::new("contribution_pct", contribution_pcts),
        Series::new("rank", ranks),
    ])?)
}
