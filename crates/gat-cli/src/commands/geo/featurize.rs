use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::GeoCommands;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;
use gat_algo::featurize_geo::featurize_spatial_timeseries;

/// Handle `gat geo featurize` command: produce time-series feature tables keyed by (polygon_id, time).
///
/// **Purpose:** Aggregates time-series grid metrics (load, voltage, violations, etc.) from bus-level
/// to polygon-level (tracts, zip codes, neighborhoods) using the bus-to-polygon mapping from `gat geo join`.
/// Computes temporal features (lags, rolling statistics, seasonal indicators) suitable for spatial forecasting
/// and planning models.
///
/// **Spatial-Temporal Feature Engineering:**
/// Combines two dimensions of analytics:
/// 1. **Spatial Aggregation**: Sum/mean/max grid metrics across all buses within each polygon
/// 2. **Temporal Features**: Lags, rolling windows, seasonal patterns over time
///
/// **Feature Categories:**
///
/// **A. Lagged Features:**
/// - **Purpose**: Capture time dependencies and autocorrelation in load/generation patterns
/// - **Example**: load_lag_24h (yesterday same hour), load_lag_168h (last week same hour)
/// - **Use Case**: Load forecasting (AR terms), reliability prediction (recent stress indicators)
/// - **Implementation**: Shift time series by lag period, join back to original timestamps
///
/// **B. Rolling Window Statistics:**
/// - **Purpose**: Smooth noise and capture trends over recent history
/// - **Example**: load_mean_7d (7-day average), voltage_min_24h (24-hour minimum)
/// - **Use Case**: Anomaly detection (compare current to rolling baseline), trend analysis
/// - **Implementation**: Sliding window aggregations (mean, std, min, max, quantiles)
///
/// **C. Seasonal Features:**
/// - **Purpose**: Capture systematic patterns by time-of-day, day-of-week, season
/// - **Example**: hour_of_day (0-23), is_weekend (boolean), month_of_year (1-12)
/// - **Use Case**: Forecast models, operations planning (weekday vs. weekend staffing)
/// - **Implementation**: Extract from timestamp, optionally one-hot encode
///
/// **D. Event Flags:**
/// - **Purpose**: Mark special events that disrupt normal patterns
/// - **Example**: is_holiday, is_extreme_weather, is_outage
/// - **Use Case**: Conditional forecasting, resilience analysis
/// - **Implementation**: Join with external calendar/weather/outage datasets
///
/// **Time-Series Forecasting Context:**
/// This feature fabric is the "X matrix" for supervised learning models predicting spatial loads,
/// reliability metrics, or DER adoption. Common model architectures:
/// - **Classical**: ARIMA, SARIMAX with spatial covariates (census demographics)
/// - **ML**: Gradient boosting (XGBoost, LightGBM), random forests with lag features
/// - **Deep Learning**: LSTMs with spatial embeddings, Temporal Graph Neural Networks (TGNNs)
/// - **Spatial Econometrics**: Spatial autoregressive (SAR) models with geographic weights matrix
///
/// **References:**
/// - Spatial-temporal load forecasting: doi:10.1016/j.energy.2020.117515
/// - Rolling statistics in time series: Hyndman & Athanasopoulos, "Forecasting: Principles and Practice" (2021)
/// - Feature engineering for ML: Kuhn & Johnson, "Feature Engineering and Selection" (2019)
///
/// **Workflow Example:**
/// ```bash
/// # 1. Perform spatial join (from previous step)
/// gat geo join \
///   --grid-file ./data/grid.arrow \
///   --polygons ./data/tracts.parquet \
///   --out ./outputs/bus_to_tract.parquet
///
/// # 2. Run time-series PF to generate bus-level loads over time
/// gat ts solve \
///   --grid-file ./data/grid.arrow \
///   --timeseries ./data/load_profiles.parquet \
///   --out ./outputs/ts_results.parquet
///
/// # 3. Featurize: aggregate to tract-level, compute lags and rolling stats
/// gat geo featurize \
///   --mapping ./outputs/bus_to_tract.parquet \
///   --timeseries ./outputs/ts_results.parquet \
///   --lags 1,24,168 \
///   --windows 24,168 \
///   --seasonal true \
///   --out ./outputs/tract_features.parquet
///
/// # 4. Join with demographic data, train forecast model
/// # e.g., Python: df = pd.read_parquet("tract_features.parquet")
/// #              X = df[feature_cols], y = df["load_mw"]
/// #              model = xgb.XGBRegressor(); model.fit(X, y)
/// ```
///
/// **Output Schema:**
/// - polygon_id: GIS polygon identifier (from mapping table)
/// - time: Timestamp (ISO 8601 format)
/// - load_mw: Aggregated load across buses in polygon (sum)
/// - voltage_pu_mean: Mean voltage across buses (average)
/// - voltage_pu_min: Minimum voltage (worst-case)
/// - violations_count: Count of voltage/thermal violations
/// - load_lag_1h, load_lag_24h, ...: Lagged load values
/// - load_mean_24h, load_std_168h, ...: Rolling window statistics
/// - hour_of_day, day_of_week, month_of_year: Seasonal features
/// - is_weekend, is_peak_hour: Derived boolean flags
///
/// **Real-World Applications:**
///
/// **1. Spatial Load Forecasting (California IOUs):**
/// - PG&E, SCE, SDG&E use census-tract-level forecasts for distribution planning
/// - Join tract features with demographic projections (population growth, EV adoption, building electrification)
/// - 5-10 year distribution investment plans ($10B+ annually) depend on spatial load forecasts
///
/// **2. Equity-Focused Reliability Planning:**
/// - Compute tract-level SAIDI, join with CalEnviroScreen disadvantaged community (DAC) indicators
/// - California CPUC requires utilities to report reliability by DAC status and improve equity
/// - E.g., if DAC tracts have 50% higher SAIDI, prioritize grid hardening investments there
///
/// **3. DER Adoption Forecasting:**
/// - Historical rooftop solar adoption by zip code as target variable
/// - Features: income, housing stock, solar irradiance, net metering policy lags
/// - Duke Energy uses zip-code solar forecasts for hosting capacity and voltage planning
///
/// **4. Extreme Weather Resilience:**
/// - Aggregate historical outage counts by neighborhood, join with flood maps and tree density
/// - Build resilience score model: predict outage risk given weather forecast
/// - ConEd uses this for pre-positioning crews before hurricanes in NYC
///
/// **Pedagogical Note for Grad Students:**
/// This implements a "feature store" pattern common in ML production systems. Instead of recomputing
/// lags/rolling stats for every model training run, we precompute and materialize them as a versioned
/// Parquet dataset. This ensures consistency across models (training, validation, production inference)
/// and enables rapid experimentation. For streaming/online learning, replace rolling windows with
/// exponential moving averages (EMA) that can be updated incrementally. Modern ML platforms (Tecton,
/// Feast, Databricks Feature Store) provide similar abstractions with time-travel queries and
/// point-in-time correctness guarantees.
pub fn handle(command: &GeoCommands) -> Result<()> {
    let GeoCommands::Featurize {
        mapping,
        timeseries,
        lags,
        windows,
        seasonal,
        out,
        out_partitions,
    } = command
    else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();

    let res = (|| -> Result<()> {
        // Parse lag periods (comma-separated hours)
        let lag_periods: Vec<usize> = lags
            .as_ref()
            .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
            .unwrap_or_default();

        // Parse rolling window sizes (comma-separated hours)
        let window_sizes: Vec<usize> = windows
            .as_ref()
            .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
            .unwrap_or_default();

        // Perform spatial-temporal featurization
        let summary = featurize_spatial_timeseries(
            Path::new(mapping),
            Path::new(timeseries),
            &lag_periods,
            &window_sizes,
            *seasonal,
            Path::new(out),
            &partitions,
        )?;

        // Print summary statistics
        println!(
            "Spatial-temporal featurization: {} polygons × {} time periods = {} feature rows",
            summary.num_polygons, summary.num_time_periods, summary.total_rows
        );
        println!(
            "  Features: {} base + {} lags + {} rolling stats + {} seasonal = {} total",
            summary.num_base_features,
            lag_periods.len(),
            window_sizes.len(),
            if *seasonal { 5 } else { 0 },
            summary.num_total_features
        );
        println!("  Output: {}", out);

        Ok(())
    })();

    // Record run telemetry
    let params = [
        ("mapping".to_string(), mapping.to_string()),
        ("timeseries".to_string(), timeseries.to_string()),
        (
            "lags".to_string(),
            lags.as_deref().unwrap_or("").to_string(),
        ),
        (
            "windows".to_string(),
            windows.as_deref().unwrap_or("").to_string(),
        ),
        ("seasonal".to_string(), seasonal.to_string()),
        ("out".to_string(), out.to_string()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
    ];

    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "geo featurize", &param_refs, start, &res);
    res
}
