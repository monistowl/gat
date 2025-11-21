use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::FeaturizeCommands;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;
use gat_algo::featurize_kpi::featurize_kpi;

/// Handle `gat featurize kpi` command: generate KPI training/evaluation feature tables.
///
/// **Purpose:** Aggregates batch PF/OPF outputs and reliability metrics into wide feature tables
/// suitable for training probabilistic KPI predictors. This prepares the "X" (independent variables)
/// for ML models that predict reliability outcomes, congestion events, or resource adequacy metrics.
///
/// **Machine Learning Context:**
/// The output is designed for gradient boosting models (XGBoost, LightGBM, CatBoost), neural
/// networks (TabNet), or probabilistic models (NGBoost) that predict KPIs from system state.
/// Each row is a (scenario, time) case with engineered features capturing system stress, topology
/// state, and policy settings. See doi:10.1109/TPWRS.2021.3089974 for ML-based reliability prediction.
///
/// **Feature Categories:**
/// 1. System stress indicators: flow statistics, utilization, congestion counts
/// 2. Reliability metrics: LOLE, EUE, thermal violations (if --reliability provided)
/// 3. Policy/control flags: DR, DER dispatch, carbon caps (if --scenario-meta provided)
/// 4. Temporal/weather: hour, day, month, temperature, wind (if --scenario-meta provided)
///
/// **Example Usage:**
/// ```bash
/// # Basic: aggregate batch flows only
/// gat featurize kpi --batch-root ./outputs/batch_pf --out ./features/kpi_features.parquet
///
/// # With reliability metrics (recommended for RA/KPI work)
/// gat featurize kpi \
///   --batch-root ./outputs/batch_pf \
///   --reliability ./outputs/reliability.parquet \
///   --out ./features/kpi_features.parquet
///
/// # Full feature set: flows + reliability + scenario metadata
/// gat featurize kpi \
///   --batch-root ./outputs/batch_pf \
///   --reliability ./outputs/reliability.parquet \
///   --scenario-meta ./scenarios/metadata.yaml \
///   --out ./features/kpi_features.parquet \
///   --out-partitions scenario_id,time
/// ```
pub fn handle(command: &FeaturizeCommands) -> Result<()> {
    let FeaturizeCommands::Kpi {
        batch_root,
        reliability,
        scenario_meta,
        out,
        out_partitions,
    } = command
    else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();

    let res = (|| -> Result<()> {
        // Generate KPI features from batch outputs, optionally joining reliability + metadata
        let summary = featurize_kpi(
            Path::new(batch_root),
            reliability.as_deref().map(Path::new),
            scenario_meta.as_deref().map(Path::new),
            Path::new(out),
            &partitions,
        )?;

        // Print summary statistics
        println!(
            "KPI featurization: {} scenarios × {} time periods = {} cases, {} features -> {}",
            summary.num_scenarios,
            summary.num_time_periods,
            summary.total_cases,
            summary.num_features,
            out
        );

        Ok(())
    })();

    // Record run telemetry
    let mut params = vec![
        ("batch_root".to_string(), batch_root.to_string()),
        ("out".to_string(), out.to_string()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
    ];
    if let Some(ref r) = reliability {
        params.push(("reliability".to_string(), r.to_string()));
    }
    if let Some(ref m) = scenario_meta {
        params.push(("scenario_meta".to_string(), m.to_string()));
    }

    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "featurize kpi", &param_refs, start, &res);
    res
}
