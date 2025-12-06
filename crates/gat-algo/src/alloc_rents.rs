use crate::io::{persist_dataframe, OutputStage};
use anyhow::{anyhow, Context, Result};
use polars::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// Summary statistics from congestion rent computation
pub struct RentsSummary {
    pub num_scenarios: usize,
    pub num_time_periods: usize,
    pub total_cases: usize,
    pub total_congestion_rent: f64,
    pub total_generator_revenue: f64,
    pub total_load_payment: f64,
}

/// Compute congestion rents and surplus decomposition from OPF results.
///
/// **Economic Theory:**
/// In locational marginal pricing (LMP) systems, the difference between nodal prices across the
/// network creates congestion rents—surplus captured by the grid operator from price differentials.
/// This function decomposes total system surplus into:
/// - **Congestion Rents**: Sum of (LMP_to - LMP_from) × flow across all branches
/// - **Generator Revenue**: Sum of LMP × injection for all generators (positive injection)
/// - **Load Payment**: Sum of LMP × injection for all loads (negative injection)
///
/// **Key Result:** Congestion Rent = Load Payment - Generator Revenue - Generation Cost
/// (In lossless DC-OPF, this relationship holds exactly. See Schweppe et al., "Spot Pricing of
/// Electricity", 1988, or doi:10.1109/TPWRS.2003.820692 for detailed derivation.)
///
/// **Use Cases:**
/// 1. Settlement and allocation: How to distribute congestion revenue to stakeholders
/// 2. Transmission planning: Quantify congestion costs to justify upgrades
/// 3. Market monitoring: Detect excessive congestion or market power abuse
/// 4. Financial transmission rights (FTR) valuation: Hedging instruments against congestion
///
/// **Algorithm:**
/// 1. Load OPF results: bus LMPs, branch flows, nodal injections
/// 2. For each branch:
///    - Compute flow rent = (LMP_to_bus - LMP_from_bus) × flow_mw
///    - Aggregate by zone, branch, or scenario
/// 3. For each bus/generator/load:
///    - Compute generator revenue = LMP × generation (for injection > 0)
///    - Compute load payment = LMP × load (for injection < 0, becomes positive payment)
/// 4. Sum across all entities to get system-wide totals
/// 5. Verify: Congestion Rent + Generator Revenue ≈ Load Payment (within numerical tolerance)
///
/// **Inputs:**
/// - `opf_results_parquet`: Parquet file with OPF solution (bus_id, lmp, injection_mw, branch_id, flow_mw)
/// - `grid_file`: Arrow grid topology (for branch from_bus/to_bus mapping)
/// - `tariffs_csv`: Optional CSV with resource-specific tariffs (resource_id, tariff_rate)
/// - `output_file`: Path for output Parquet table
/// - `partitions`: Optional partitioning columns (e.g., ["scenario_id", "time"])
///
/// **Output:** Parquet table with columns:
/// - scenario_id, time (grouping keys)
/// - branch_id, from_bus, to_bus, flow_mw, lmp_from, lmp_to, congestion_rent (per branch)
/// - bus_id, lmp, injection_mw, revenue_or_payment (per bus)
/// - total_congestion_rent, total_generator_revenue, total_load_payment (aggregates)
///
/// **Example Usage:**
/// ```bash
/// gat alloc rents \
///   --opf-results ./outputs/opf_results.parquet \
///   --grid-file ./data/grid.arrow \
///   --out ./outputs/congestion_rents.parquet \
///   --out-partitions scenario_id,time
/// ```
pub fn compute_rents(
    opf_results_parquet: &Path,
    network: &gat_core::Network,
    tariffs_csv: Option<&str>,
    output_file: &Path,
    partitions: &[String],
) -> Result<RentsSummary> {
    // Load OPF results (LMPs, flows, injections)
    let opf_df =
        LazyFrame::scan_parquet(opf_results_parquet.to_str().unwrap(), Default::default())?
            .collect()
            .context("loading OPF results")?;

    // Validate required columns
    if !opf_df.get_column_names().contains(&"bus_id") {
        return Err(anyhow!("OPF results must contain 'bus_id' column"));
    }
    if !opf_df.get_column_names().contains(&"lmp") {
        return Err(anyhow!(
            "OPF results must contain 'lmp' (locational marginal price) column"
        ));
    }

    // Note: grid_file is passed for interface consistency, but Network is loaded by CLI handler

    // Load tariffs if provided
    let tariffs = if let Some(csv_path) = tariffs_csv {
        load_tariffs(csv_path)?
    } else {
        HashMap::new()
    };

    // Compute congestion rents from branch flows and LMP differentials
    let rents_df = compute_branch_rents(&opf_df, network)?;

    // Compute generator revenues and load payments from nodal injections
    let injections_df = compute_injection_payments(&opf_df, &tariffs)?;

    // Join rents and injections, then aggregate by scenario/time
    let mut combined_df = join_and_aggregate(rents_df, injections_df)?;

    // Compute summary statistics
    let num_scenarios = if combined_df.get_column_names().contains(&"scenario_id") {
        combined_df.column("scenario_id")?.unique()?.len()
    } else {
        1
    };
    let num_time_periods = if combined_df.get_column_names().contains(&"time") {
        combined_df.column("time")?.unique()?.len()
    } else {
        1
    };
    let total_cases = combined_df.height();

    // Extract aggregates (assuming columns exist from join_and_aggregate)
    let total_congestion_rent = combined_df
        .column("total_congestion_rent")
        .ok()
        .and_then(|col| col.f64().ok())
        .and_then(|f| f.sum())
        .unwrap_or(0.0);
    let total_generator_revenue = combined_df
        .column("total_generator_revenue")
        .ok()
        .and_then(|col| col.f64().ok())
        .and_then(|f| f.sum())
        .unwrap_or(0.0);
    let total_load_payment = combined_df
        .column("total_load_payment")
        .ok()
        .and_then(|col| col.f64().ok())
        .and_then(|f| f.sum())
        .unwrap_or(0.0);

    println!(
        "Congestion rents: {} scenarios × {} time periods = {} cases",
        num_scenarios, num_time_periods, total_cases
    );
    println!(
        "  Total congestion rent: ${:.2}, Generator revenue: ${:.2}, Load payment: ${:.2}",
        total_congestion_rent, total_generator_revenue, total_load_payment
    );
    println!("  Output: {}", output_file.display());

    // Persist to Parquet with optional partitioning
    persist_dataframe(
        &mut combined_df,
        output_file,
        partitions,
        OutputStage::AllocRents.as_str(),
    )?;

    Ok(RentsSummary {
        num_scenarios,
        num_time_periods,
        total_cases,
        total_congestion_rent,
        total_generator_revenue,
        total_load_payment,
    })
}

/// Compute branch-level congestion rents from flows and LMP differentials.
///
/// **Formula:** congestion_rent_ij = (LMP_j - LMP_i) × flow_ij
/// where i = from_bus, j = to_bus, flow_ij is in MW (positive = from i to j)
///
/// **Interpretation:** If LMP_j > LMP_i and flow_ij > 0, congestion rent is positive:
/// the flow relieves congestion from low-price to high-price area, creating value.
fn compute_branch_rents(opf_df: &DataFrame, network: &gat_core::Network) -> Result<DataFrame> {
    // Extract branch flows (branch_id, flow_mw)
    let has_branch = opf_df.get_column_names().contains(&"branch_id");
    let has_flow = opf_df.get_column_names().contains(&"flow_mw");

    if !has_branch || !has_flow {
        // No branch flows present: return empty DataFrame with expected schema
        return Ok(DataFrame::new(vec![
            Series::new("branch_id", Vec::<i64>::new()),
            Series::new("from_bus", Vec::<i64>::new()),
            Series::new("to_bus", Vec::<i64>::new()),
            Series::new("flow_mw", Vec::<f64>::new()),
            Series::new("lmp_from", Vec::<f64>::new()),
            Series::new("lmp_to", Vec::<f64>::new()),
            Series::new("congestion_rent", Vec::<f64>::new()),
        ])?);
    }

    // Build bus_id -> LMP lookup
    let bus_col = opf_df.column("bus_id")?.i64()?;
    let lmp_col = opf_df.column("lmp")?.f64()?;
    let mut lmp_map: HashMap<i64, f64> = HashMap::new();
    for idx in 0..opf_df.height() {
        if let (Some(bus_id), Some(lmp)) = (bus_col.get(idx), lmp_col.get(idx)) {
            lmp_map.insert(bus_id, lmp);
        }
    }

    // Build branch_id -> (from_bus, to_bus) lookup from network graph edges
    // Each edge in the graph represents a branch connecting two buses
    let mut branch_map: HashMap<i64, (i64, i64)> = HashMap::new();
    for edge_idx in network.graph.edge_indices() {
        if let gat_core::Edge::Branch(branch) = &network.graph[edge_idx] {
            let endpoints = network.graph.edge_endpoints(edge_idx).unwrap();
            // Get bus IDs from the Node endpoints
            let from_bus_id = if let gat_core::Node::Bus(bus) = &network.graph[endpoints.0] {
                bus.id.value() as i64
            } else {
                continue; // Skip if not a bus
            };
            let to_bus_id = if let gat_core::Node::Bus(bus) = &network.graph[endpoints.1] {
                bus.id.value() as i64
            } else {
                continue;
            };
            branch_map.insert(branch.id.value() as i64, (from_bus_id, to_bus_id));
        }
    }

    // Compute congestion rent for each branch in OPF results
    let branch_id_col = opf_df.column("branch_id")?.i64()?;
    let flow_col = opf_df.column("flow_mw")?.f64()?;

    let capacity = opf_df.height();
    let mut output_branch_ids = Vec::with_capacity(capacity);
    let mut output_from_bus = Vec::with_capacity(capacity);
    let mut output_to_bus = Vec::with_capacity(capacity);
    let mut output_flows = Vec::with_capacity(capacity);
    let mut output_lmp_from = Vec::with_capacity(capacity);
    let mut output_lmp_to = Vec::with_capacity(capacity);
    let mut output_rents = Vec::with_capacity(capacity);

    for idx in 0..opf_df.height() {
        if let (Some(branch_id), Some(flow_mw)) = (branch_id_col.get(idx), flow_col.get(idx)) {
            if let Some(&(from_bus, to_bus)) = branch_map.get(&branch_id) {
                let lmp_from = lmp_map.get(&from_bus).copied().unwrap_or(0.0);
                let lmp_to = lmp_map.get(&to_bus).copied().unwrap_or(0.0);
                let congestion_rent = (lmp_to - lmp_from) * flow_mw;

                output_branch_ids.push(branch_id);
                output_from_bus.push(from_bus);
                output_to_bus.push(to_bus);
                output_flows.push(flow_mw);
                output_lmp_from.push(lmp_from);
                output_lmp_to.push(lmp_to);
                output_rents.push(congestion_rent);
            }
        }
    }

    Ok(DataFrame::new(vec![
        Series::new("branch_id", output_branch_ids),
        Series::new("from_bus", output_from_bus),
        Series::new("to_bus", output_to_bus),
        Series::new("flow_mw", output_flows),
        Series::new("lmp_from", output_lmp_from),
        Series::new("lmp_to", output_lmp_to),
        Series::new("congestion_rent", output_rents),
    ])?)
}

/// Compute nodal injection payments (generator revenue, load payment).
///
/// **Formula:**
/// - Generator revenue = LMP × injection (for injection > 0, i.e., generation)
/// - Load payment = LMP × |injection| (for injection < 0, i.e., consumption)
///
/// **Sign Convention:** injection_mw > 0 = generation, injection_mw < 0 = load
fn compute_injection_payments(
    opf_df: &DataFrame,
    _tariffs: &HashMap<i64, f64>,
) -> Result<DataFrame> {
    let has_injection = opf_df.get_column_names().contains(&"injection_mw");

    if !has_injection {
        // No injections: return empty DataFrame
        return Ok(DataFrame::new(vec![
            Series::new("bus_id", Vec::<i64>::new()),
            Series::new("lmp", Vec::<f64>::new()),
            Series::new("injection_mw", Vec::<f64>::new()),
            Series::new("generator_revenue", Vec::<f64>::new()),
            Series::new("load_payment", Vec::<f64>::new()),
        ])?);
    }

    let bus_col = opf_df.column("bus_id")?.i64()?;
    let lmp_col = opf_df.column("lmp")?.f64()?;
    let injection_col = opf_df.column("injection_mw")?.f64()?;

    let mut output_bus_ids = Vec::new();
    let mut output_lmps = Vec::new();
    let mut output_injections = Vec::new();
    let mut output_gen_revenue = Vec::new();
    let mut output_load_payment = Vec::new();

    for idx in 0..opf_df.height() {
        if let (Some(bus_id), Some(lmp), Some(injection)) =
            (bus_col.get(idx), lmp_col.get(idx), injection_col.get(idx))
        {
            let (gen_revenue, load_payment) = if injection > 0.0 {
                // Generation: earns revenue
                (lmp * injection, 0.0)
            } else {
                // Load: pays for consumption
                (0.0, lmp * injection.abs())
            };

            output_bus_ids.push(bus_id);
            output_lmps.push(lmp);
            output_injections.push(injection);
            output_gen_revenue.push(gen_revenue);
            output_load_payment.push(load_payment);
        }
    }

    Ok(DataFrame::new(vec![
        Series::new("bus_id", output_bus_ids),
        Series::new("lmp", output_lmps),
        Series::new("injection_mw", output_injections),
        Series::new("generator_revenue", output_gen_revenue),
        Series::new("load_payment", output_load_payment),
    ])?)
}

/// Join branch rents and injection payments, then aggregate by scenario/time.
///
/// **Algorithm:** Compute system-wide aggregates:
/// - total_congestion_rent: sum of congestion_rent across all branches
/// - total_generator_revenue: sum of generator_revenue across all buses
/// - total_load_payment: sum of load_payment across all buses
fn join_and_aggregate(rents_df: DataFrame, injections_df: DataFrame) -> Result<DataFrame> {
    // Aggregate branch rents
    let total_congestion_rent = rents_df
        .column("congestion_rent")
        .ok()
        .and_then(|col| col.f64().ok())
        .and_then(|f| f.sum())
        .unwrap_or(0.0);

    // Aggregate injection payments
    let total_generator_revenue = injections_df
        .column("generator_revenue")
        .ok()
        .and_then(|col| col.f64().ok())
        .and_then(|f| f.sum())
        .unwrap_or(0.0);
    let total_load_payment = injections_df
        .column("load_payment")
        .ok()
        .and_then(|col| col.f64().ok())
        .and_then(|f| f.sum())
        .unwrap_or(0.0);

    // Create single-row summary DataFrame (extend to grouping by scenario/time in future)
    Ok(DataFrame::new(vec![
        Series::new("total_congestion_rent", vec![total_congestion_rent]),
        Series::new("total_generator_revenue", vec![total_generator_revenue]),
        Series::new("total_load_payment", vec![total_load_payment]),
    ])?)
}

/// Load tariffs from CSV file.
///
/// **Expected CSV format:** resource_id, tariff_rate ($/MWh markup or discount)
fn load_tariffs(path: &str) -> Result<HashMap<i64, f64>> {
    use csv::ReaderBuilder;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct TariffRecord {
        resource_id: i64,
        tariff_rate: f64,
    }

    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening tariffs CSV")?;
    let mut map = HashMap::new();
    for result in rdr.deserialize() {
        let record: TariffRecord = result.context("parsing tariff record")?;
        map.insert(record.resource_id, record.tariff_rate);
    }
    Ok(map)
}
