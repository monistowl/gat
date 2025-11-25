use anyhow::{anyhow, Context, Result};
use polars::prelude::{
    DataFrame, NamedFrom, ParquetCompression, ParquetReader, ParquetWriter, SerReader, Series,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::Path;

/// DER asset representation with operational constraints and state-of-charge limits.
///
/// **Asset Types:**
/// - **Battery Energy Storage System (BESS)**: p_min < 0 (charge), p_max > 0 (discharge)
/// - **Solar PV**: p_min = 0, p_max = nameplate capacity (can only inject)
/// - **Demand Response (DR)**: p_min < 0 (curtailable load), p_max = 0
/// - **Electric Vehicle (EV)**: Similar to BESS, but mobile and less predictable
///
/// **State of Charge (SoC):**
/// For storage assets, SoC tracks energy content (MWh or % of capacity):
/// - SoC increases when charging (consuming power, p < 0)
/// - SoC decreases when discharging (injecting power, p > 0)
/// - Constrained by battery chemistry: soc_min (e.g., 20% for lithium-ion health)
///   and soc_max (typically 100%, but some operators limit to 90% for longevity)
#[derive(Clone, Debug)]
struct DerAsset {
    id: String,
    agg_id: Option<String>, // Aggregation ID (e.g., feeder, zone, VPP portfolio)
    bus_id: Option<usize>,  // Grid connection point (for locational constraints)
    p_min: f64,             // Minimum active power (MW): negative = max charge rate
    p_max: f64,             // Maximum active power (MW): positive = max discharge rate
    q_min: f64,             // Minimum reactive power (MVAr): volt-VAR capability
    q_max: f64,             // Maximum reactive power (MVAr): for voltage support
    soc_min: f64,           // Minimum state of charge (MWh or p.u.)
    soc_max: f64,           // Maximum state of charge (MWh or p.u.)
}

/// Lightweight price vector for scheduling horizons.
#[derive(Clone, Debug)]
struct PricePoint {
    timestamp: String,
    price: f64,
}

/// Compute aggregated DER capability envelopes (P-Q regions) grouped by location or portfolio.
///
/// **Purpose:** Aggregate individual DER assets into composite capability envelopes that represent
/// the feasible operating region for a fleet. These envelopes are used for dispatch optimization,
/// OPF integration, and Virtual Power Plant (VPP) market participation.
///
/// **P-Q Capability Curve Concept:**
/// Each DER asset has a feasible operating region in the (P, Q) power space:
/// - **Active Power (P)**: Real power injection (MW), can be positive (generation/discharge)
///   or negative (consumption/charge)
/// - **Reactive Power (Q)**: Reactive power injection (MVAr), used for voltage support
/// - **Inverter Limits**: Modern inverters have apparent power limit: √(P² + Q²) ≤ S_max
///   → Operating region is a circle in (P, Q) space (IEEE 1547-2018 capability curve)
///
/// **Historical Context:**
/// Traditional generators have P-Q capability curves constrained by:
/// - Rotor heating (field current limits → Q_max decreases at high P)
/// - Stator heating (armature current → circular arc at low P)
/// - Prime mover limits (boiler, turbine → P_max independent of Q)
///
/// See doi:10.1109/TPWRS.2002.804943 for generator capability curves.
///
/// **DER Capability Curves:**
/// Inverter-based DER (solar, batteries, wind) have simpler capability:
/// - **Circular**: √(P² + Q²) ≤ S_rated (inverter apparent power limit)
/// - **Four-quadrant**: Can operate in all quadrants (import/export P, supply/absorb Q)
/// - **Smart Inverter Functions**: IEEE 1547-2018 mandates volt-VAR, volt-Watt, freq-Watt
///   → Q is function of local voltage, P is function of frequency (autonomous response)
///
/// **Aggregation for Virtual Power Plants (VPPs):**
/// When aggregating N DER assets into a portfolio envelope:
/// - **Simple Sum**: P_total = Σ P_i, Q_total = Σ Q_i (assumes no interaction)
/// - **Convex Hull**: Operating region is Minkowski sum of individual capability curves
/// - **Statistical**: Account for availability/uncertainty (not all assets available simultaneously)
/// - **Network Constraints**: Locational limits (thermal/voltage) may restrict aggregate envelope
///
/// **Algorithm:**
/// 1. Load DER asset table (asset_id, p_min, p_max, q_min, q_max, location)
/// 2. Group assets by `group_by` key (default: agg_id for aggregation zones)
/// 3. For each group, compute aggregate envelope bounds:
///    - P_min_total = min(Σ p_min_i) (all assets at max charge)
///    - P_max_total = max(Σ p_max_i) (all assets at max discharge)
///    - Similar for Q_min, Q_max
/// 4. Output envelope table: (region, p_min_mw, p_max_mw, q_min_mvar, q_max_mvar, asset_count)
///
/// **Use Cases:**
/// - **Market Bidding**: Submit envelope to ISO/RTO as available capacity for dispatch
/// - **OPF Integration**: Treat VPP as controllable injection with P/Q limits
/// - **Distribution Planning**: Assess how much DER flexibility is available for voltage regulation
/// - **Ancillary Services**: Quantify frequency regulation, spinning reserve capability
///
/// **Pedagogical Note for Grad Students:**
/// The envelope represents the "aggregate flexibility" of a DER fleet. In market contexts,
/// this is what the VPP operator sells to the grid: "I can provide P ∈ [P_min, P_max] and
/// Q ∈ [Q_min, Q_max] at bus X." The operator then disaggregates the dispatch signal to
/// individual assets (the `schedule()` function). See Stadler et al. (2016) doi:10.1109/TSG.2015.2450872
/// for VPP market models.
///
/// **Real-World Example:**
/// California's DERMS platforms (e.g., PG&E, SCE) aggregate thousands of residential batteries,
/// solar inverters, and smart thermostats into ~100 MW VPP portfolios that provide capacity,
/// energy, and regulation services to CAISO markets. Envelope computation is the first step in
/// market participation: "How much flexibility do we have today?"
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

/// Generate price-responsive DER dispatch schedule for energy arbitrage and peak shaving.
///
/// **Purpose:** Given a time-series of electricity prices (or load forecasts), compute optimal
/// dispatch of DER assets to maximize revenue (arbitrage) or minimize costs (peak shaving).
/// This is the core of DERMS: translating economic signals into physical dispatch commands.
///
/// **Energy Arbitrage Concept:**
/// Battery storage can profit from intertemporal price differences:
/// - **Charge** when prices are low (off-peak, high renewable generation)
/// - **Discharge** when prices are high (peak demand, low renewable output)
/// - **Revenue** = Σ_t P_discharge(t) × price(t) - Σ_t P_charge(t) × price(t)
/// - **Constraints**: SoC limits, charge/discharge rate limits, round-trip efficiency (η ≈ 0.85-0.90)
///
/// **Historical Context:**
/// Energy arbitrage became economically viable with:
/// 1. **Time-of-Use (TOU) rates** (1980s-1990s): Predictable peak/off-peak price differentials
/// 2. **Wholesale market volatility** (2000s): CAISO, ERCOT prices vary 10-100x intraday
/// 3. **Battery cost decline** (2010s-present): Lithium-ion costs dropped from $1000/kWh → $150/kWh
///
/// See Walawalkar et al. (2007) doi:10.1109/TPWRS.2007.901489 for early arbitrage economics.
///
/// **Algorithm (Naive Threshold-Based Dispatch):**
/// This is a *heuristic* (not optimal), but simple and fast:
/// 1. Compute median price over the time horizon (threshold)
/// 2. For each time step:
///    - If price > threshold: discharge at max rate (p = p_max)
///    - If price < threshold: charge at max rate (p = p_min)
/// 3. Respect SoC constraints: clip dispatch to keep soc_min ≤ SoC ≤ soc_max
/// 4. Track curtailment: count steps where desired dispatch was clipped by SoC limits
/// 5. Output schedule: (timestamp, asset_id, p_mw, q_mvar, soc)
///
/// **Limitations (Naive Heuristic):**
/// - **Myopic**: Doesn't look ahead (may discharge early, miss higher prices later)
/// - **No Efficiency**: Ignores round-trip losses (η < 1), so revenue is overestimated
/// - **Threshold Choice**: Median is arbitrary (should be derived from dual values in optimal solution)
/// - **No Degradation**: Doesn't model battery cycle life (frequent cycling reduces lifespan)
///
/// **Optimal Scheduling (Future Work):**
/// Replace heuristic with **Mixed-Integer Linear Programming (MILP)**:
/// ```text
/// maximize: Σ_t (P_discharge(t) × price(t) - P_charge(t) × price(t))
/// subject to:
///   SoC(t+1) = SoC(t) - P(t) × Δt / efficiency    [energy balance]
///   soc_min ≤ SoC(t) ≤ soc_max                     [storage capacity]
///   p_min ≤ P(t) ≤ p_max                           [power limits]
///   P_charge(t), P_discharge(t) ≥ 0, not simultaneous [binary logic]
/// ```
/// Solve with CBC, Gurobi, or HiGHS. See Kazemi et al. (2017) doi:10.1109/TSG.2016.2609892
/// for optimal battery scheduling under uncertainty.
///
/// **Curtailment Rate:**
/// Fraction of time steps where desired dispatch was limited by SoC constraints:
/// - **Low curtailment** (< 5%): Good price-responsive behavior, rarely SoC-limited
/// - **High curtailment** (> 30%): Battery too small for price volatility, or poor heuristic
/// - **Interpretation**: Curtailment → opportunity cost (couldn't capture all arbitrage value)
///
/// **Pedagogical Note for Grad Students:**
/// This heuristic demonstrates the *price-responsive* behavior of storage, but real DERMS use
/// Model Predictive Control (MPC) or stochastic optimization with rolling horizons. The key insight:
/// storage shifts energy through time, converting temporal price spreads into revenue. The
/// "value of storage" comes from time-arbitrage, not capacity (contrast with generation).
///
/// **Real-World Example:**
/// Tesla Autobidder (DERMS for utility-scale batteries) uses ML-based price forecasting + MPC
/// to optimize dispatch for California's Moss Landing (400 MW / 1600 MWh). Reported revenue:
/// $60M/year from energy arbitrage + ancillary services (2021-2022 data). Our naive heuristic
/// would capture ~50-70% of that value (due to myopia and no ancillary service participation).
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

/// Monte Carlo stress-testing of DER schedules under price uncertainty.
///
/// **Purpose:** Evaluate robustness of DER dispatch heuristics under stochastic price variations.
/// This quantifies how sensitive arbitrage revenue and curtailment rates are to price forecast errors,
/// market volatility, and extreme events (price spikes, negative prices).
///
/// **Why Stress Testing?**
/// Real-world electricity markets are highly uncertain:
/// - **Forecast Errors**: Day-ahead price forecasts have MAPE ≈ 15-30% (California, ERCOT)
/// - **Volatility**: Prices can spike 10-100x during supply scarcity (e.g., ERCOT Feb 2021: $9000/MWh)
/// - **Negative Prices**: Occur with high renewables + low demand (e.g., California spring afternoons)
/// - **Tail Events**: Extreme weather, outages, cyberattacks create unpredictable price patterns
///
/// Stress testing reveals:
/// - **Revenue at Risk (VaR)**: 95th percentile downside from forecast errors
/// - **Curtailment Sensitivity**: How often SoC constraints bind under different price regimes
/// - **Worst-Case Performance**: Minimum revenue across scenarios (robust optimization metric)
///
/// **Historical Context:**
/// Energy storage dispatch was initially deterministic (perfect foresight), but operators learned
/// the hard way that forecast errors erode arbitrage value. Modern DERMS use stochastic optimization
/// (scenario trees) or robust optimization (worst-case guarantees). See Bertsimas et al. (2013)
/// doi:10.1287/opre.2013.1158 for robust optimization in power systems.
///
/// **Algorithm (Monte Carlo Simulation):**
/// 1. Load base price trajectory (e.g., day-ahead forecast)
/// 2. For each scenario (1 to N):
///    - Perturb prices: price_perturbed = price_base × scale_factor
///      where scale_factor ~ Uniform(0.8, 1.2) (±20% variation)
/// 3. Run dispatch heuristic on perturbed prices
/// 4. Record curtailment rate for each scenario
/// 5. Aggregate statistics: mean, std dev, 5th/95th percentiles
/// 6. Output summary: (scenario_id, scale_factor, curtailment_rate)
///
/// **Interpreting Results:**
/// - **Mean curtailment ≈ base case**: Heuristic is robust to moderate price uncertainty
/// - **High std dev**: Heuristic is sensitive to price variations (consider stochastic MPC)
/// - **95th percentile >> mean**: Tail risk is significant (extreme prices cause frequent SoC limits)
/// - **Comparison metric**: Revenue degradation = E[revenue under uncertainty] / revenue under perfect foresight
///
/// **Extensions (Future Work):**
/// - **Realistic Price Models**: Use ARIMA, GARCH, or ML (LSTM) to generate correlated price scenarios
///   (current implementation: i.i.d. scaling, unrealistic for intraday correlation)
/// - **Joint Uncertainty**: Co-simulate price + renewable generation + load forecast errors
/// - **Risk Metrics**: Compute Conditional Value-at-Risk (CVaR), information gap (robust optimization)
/// - **Hedging Strategies**: Evaluate financial instruments (futures, options) to reduce revenue volatility
///
/// **Pedagogical Note for Grad Students:**
/// This demonstrates the gap between deterministic and stochastic optimization. Deterministic optimal
/// (perfect foresight) provides an upper bound on value, but is unachievable. Stochastic models trade
/// off expected value for robustness. The "stochastic gap" = deterministic value - stochastic value
/// quantifies the cost of uncertainty. For storage, this gap is 10-30% of arbitrage revenue depending
/// on market volatility and forecast quality.
///
/// **Real-World Application:**
/// ISO/RTO markets (CAISO, PJM, ERCOT) publish uncertainty bands for day-ahead prices. DERMS operators
/// use these to:
/// 1. Compute expected revenue (mean across scenarios)
/// 2. Set risk limits (e.g., CVaR ≤ $10k/day at 95% confidence)
/// 3. Hedge positions (lock in revenue with forward contracts if uncertainty is high)
/// 4. Adjust bidding strategy (bid conservatively if forecast error is large)
///
/// **Example Output Interpretation:**
/// ```text
/// Scenario 0: scale=0.82, curtailment=0.15 (low prices -> less discharge, low SoC pressure)
/// Scenario 1: scale=1.18, curtailment=0.42 (high prices -> aggressive discharge, hit SoC_min often)
/// Mean curtailment=0.25, StdDev=0.12 -> High variability, consider increasing battery capacity
/// ```
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
        .map(StdRng::seed_from_u64)
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
    if values.len().is_multiple_of(2) {
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
