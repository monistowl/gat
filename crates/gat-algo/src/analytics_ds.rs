use crate::io::{persist_dataframe, OutputStage};
use crate::power_flow::branch_flow_dataframe;
use anyhow::{anyhow, Context, Result};
use csv::ReaderBuilder;
use gat_core::{solver::SolverBackend, Network, Node};
use polars::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Summary statistics from deliverability score computation.
/// Returned to callers so they can log how many buses and stress cases were analyzed.
pub struct DeliverabilitySummary {
    pub num_buses: usize,
    pub num_cases: usize,
}

/// CSV record for bus-level capacity limits (nameplate capacity).
#[derive(Deserialize)]
struct PmaxRecord {
    bus_id: usize,
    pmax: f64,
}

/// CSV record for branch thermal limits (maximum flow capacity).
#[derive(Deserialize)]
struct FlowLimitRecord {
    branch_id: i64,
    flow_limit: f64,
}

/// Key identifying a unique stress case (scenario + time combination).
/// Used to group branch flows by case when computing per-case DS.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CaseKey {
    scenario: Option<String>,
    time: Option<String>,
}

struct CaseData {
    key: CaseKey,
    flows: HashMap<i64, f64>,
}

struct DsRecord {
    bus_id: usize,
    scenario: Option<String>,
    time: Option<String>,
    ds_case: f64,
    pmax: f64,
}

/// Compute DC-approximate Deliverability Scores (DS) for each bus across stress cases.
///
/// **Deliverability Score** measures the fraction of a bus's nameplate capacity that can be
/// delivered to the system before branch thermal limits are violated. This is a key metric
/// for resource adequacy (RA) accreditation, where DS × ELCC determines effective capacity.
///
/// **Algorithm** (DC approximation):
/// 1. For each bus, compute PTDF (Power Transfer Distribution Factor) row via DC power flow
///    (see doi:10.1109/TPWRS.2007.899019 for DC flow formulation).
/// 2. For each stress case (scenario/time), find the maximum additional injection ΔP at the bus
///    such that |f_ℓ + PTDF_ℓ × ΔP| ≤ F_ℓ for all branches ℓ, where f_ℓ is current flow and F_ℓ
///    is the thermal limit.
/// 3. DS_case = min(1.0, ΔP_max / P_max) where P_max is nameplate capacity.
/// 4. Aggregate DS_mean across all cases (simple mean in v0).
///
/// **Inputs:**
/// - `network`: Grid topology (buses, branches, reactances)
/// - `solver`: Linear solver backend for DC flow (B'θ = P)
/// - `limits_csv`: Bus capacity limits (bus_id, pmax)
/// - `branch_limits_csv`: Branch thermal limits (branch_id, flow_limit)
/// - `flows_parquet`: Branch flows from DC PF/OPF runs, optionally partitioned by scenario_id/time
/// - `slack_bus`: Reference bus for PTDF computation (injection at source, withdrawal at slack)
///
/// **Output:** Parquet table with columns: bus_id, scenario_id, time, ds_case, pmax_mw, ds_mean
pub fn deliverability_scores_dc(
    network: &Network,
    solver: &dyn SolverBackend,
    limits_csv: &str,
    branch_limits_csv: &str,
    flows_parquet: &Path,
    output_file: &Path,
    partitions: &[String],
    slack_bus: usize,
) -> Result<DeliverabilitySummary> {
    if flows_parquet.as_os_str().is_empty() {
        return Err(anyhow!("flows Parquet path must be provided"));
    }

    // Load bus capacity limits (nameplate P_max for each bus)
    let limits = load_limits(limits_csv)?;
    // Load branch thermal limits (maximum flow F_ℓ for each branch)
    let branch_limits = load_branch_limits(branch_limits_csv)?;

    // Load branch flows from previous DC PF/OPF runs.
    // Expected schema: branch_id, flow_mw, optionally scenario_id, time
    let flows_df = LazyFrame::scan_parquet(flows_parquet.to_str().unwrap(), Default::default())?
        .collect()
        .context("reading flows parquet for DS")?;

    // Group flows by case (scenario_id, time) to handle multiple stress scenarios
    let cases = collapse_cases(&flows_df)?;
    if cases.is_empty() {
        return Err(anyhow!("flows parquet contains no branch data"));
    }

    // Collect all bus IDs from the network topology
    let bus_ids = collect_bus_ids(network);
    if bus_ids.is_empty() {
        return Err(anyhow!("network contains no buses"));
    }

    // Validate slack bus selection: use provided slack if it exists, otherwise default to first bus
    let slack_bus = if bus_ids.contains(&slack_bus) {
        slack_bus
    } else {
        bus_ids[0]
    };

    // Pre-compute PTDF rows for all buses to avoid redundant DC flow solves.
    // PTDF_ℓ,i = flow on branch ℓ per 1 MW injection at bus i (withdrawal at slack).
    // This is computed once per bus and reused across all stress cases.
    let mut ptdf_cache: HashMap<usize, HashMap<i64, f64>> = HashMap::new();
    for bus_id in &bus_ids {
        let row = compute_ptdf_row(network, solver, *bus_id, slack_bus)
            .with_context(|| format!("building PTDF row for bus {}", bus_id))?;
        ptdf_cache.insert(*bus_id, row);
    }

    // Compute DS for each (bus, case) combination
    let mut records = Vec::new();
    let mut ds_summary: HashMap<usize, (f64, usize)> = HashMap::new();

    for bus_id in &bus_ids {
        // Get nameplate capacity for this bus (default to 1.0 MW if not specified)
        let pmax = *limits.get(bus_id).unwrap_or(&1.0);
        let ptdf = ptdf_cache.get(bus_id).expect("PTDF row missing");

        // For each stress case, compute how much additional injection is feasible
        for case in &cases {
            let ds_case = compute_ds(pmax, ptdf, &branch_limits, &case.flows);

            // Track running sum for mean aggregation
            ds_summary
                .entry(*bus_id)
                .and_modify(|(sum, count)| {
                    *sum += ds_case;
                    *count += 1;
                })
                .or_insert((ds_case, 1));

            records.push(DsRecord {
                bus_id: *bus_id,
                scenario: case.key.scenario.clone(),
                time: case.key.time.clone(),
                ds_case,
                pmax,
            });
        }
    }

    // Compute mean DS across all cases for each bus (simple unweighted mean in v0)
    let ds_means: HashMap<usize, f64> = ds_summary
        .iter()
        .map(|(bus_id, (sum, count))| (*bus_id, sum / (*count as f64)))
        .collect();

    let record_count = records.len();
    if record_count == 0 {
        return Err(anyhow!("no DS rows generated"));
    }

    // Build output DataFrame columns
    let mut bus_col = Vec::with_capacity(record_count);
    let mut scenario_col = Vec::with_capacity(record_count);
    let mut time_col = Vec::with_capacity(record_count);
    let mut ds_case_col = Vec::with_capacity(record_count);
    let mut pmax_col = Vec::with_capacity(record_count);
    let mut ds_mean_col = Vec::with_capacity(record_count);

    for record in &records {
        bus_col.push(record.bus_id as i64);
        scenario_col.push(record.scenario.clone());
        time_col.push(record.time.clone());
        ds_case_col.push(record.ds_case);
        pmax_col.push(record.pmax);
        // Include aggregated mean DS in each row (repeated for all cases of same bus)
        let mean = ds_means
            .get(&record.bus_id)
            .copied()
            .unwrap_or(record.ds_case);
        ds_mean_col.push(mean);
    }

    // Construct output DataFrame with per-case and aggregated DS metrics
    let mut df = DataFrame::new(vec![
        Series::new("bus_id", bus_col),
        Series::new("scenario_id", scenario_col),
        Series::new("time", time_col),
        Series::new("ds_case", ds_case_col),
        Series::new("pmax_mw", pmax_col),
        Series::new("ds_mean", ds_mean_col),
    ])?;

    // Persist to Parquet with optional partitioning (e.g., by bus_id, scenario_id)
    persist_dataframe(
        &mut df,
        output_file,
        partitions,
        OutputStage::AnalyticsDs.as_str(),
    )?;

    println!(
        "Deliverability Score table: {} buses × {} cases -> {}",
        bus_ids.len(),
        cases.len(),
        output_file.display()
    );

    Ok(DeliverabilitySummary {
        num_buses: bus_ids.len(),
        num_cases: cases.len(),
    })
}

/// Compute PTDF row for a single bus: branch flows per 1 MW injection at source (withdrawal at sink).
///
/// **PTDF (Power Transfer Distribution Factor)** measures how branch flows change when power is
/// transferred from one bus to another. For DS computation, we inject 1 MW at the source bus
/// and withdraw 1 MW at the slack/sink bus, then solve DC power flow to get branch flow changes.
/// These flow changes are the PTDF coefficients (see doi:10.1109/TPWRS.2008.916398).
///
/// **Returns:** Map from branch_id to PTDF value (flow on that branch per 1 MW transfer)
fn compute_ptdf_row(
    network: &Network,
    solver: &dyn SolverBackend,
    source: usize,
    sink: usize,
) -> Result<HashMap<i64, f64>> {
    // Create unit injection pattern: +1 MW at source, -1 MW at sink (slack/reference)
    let mut injections = HashMap::new();
    injections.insert(source, 1.0);
    injections.insert(sink, -1.0);

    // Solve DC power flow: B'θ = P where P is our injection pattern
    // The resulting branch flows are the PTDF values (since we used 1 MW injection)
    let (df, _, _) = branch_flow_dataframe(network, &injections, None, solver)?;

    // Extract branch_id -> flow_mw mapping (these flows are the PTDF coefficients)
    let branch_col = df.column("branch_id")?.i64()?;
    let flow_col = df.column("flow_mw")?.f64()?;
    let mut row_map = HashMap::new();
    for idx in 0..df.height() {
        if let (Some(branch_id), Some(flow)) = (branch_col.get(idx), flow_col.get(idx)) {
            row_map.insert(branch_id, flow);
        }
    }
    Ok(row_map)
}

/// Compute Deliverability Score for a single bus in a single stress case.
///
/// **Algorithm:** For each branch, find the maximum additional injection ΔP such that the branch
/// flow constraint |f_ℓ + PTDF_ℓ × ΔP| ≤ F_ℓ is satisfied. The limiting branch gives the tightest
/// bound, and DS = min(1.0, ΔP_max / P_max).
///
/// **Mathematical formulation:**
/// - Current flow on branch ℓ: f_ℓ
/// - Branch thermal limit: F_ℓ
/// - PTDF coefficient: PTDF_ℓ (flow change per 1 MW injection)
/// - Constraint: |f_ℓ + PTDF_ℓ × ΔP| ≤ F_ℓ
/// - Solve for ΔP_max = min over all branches of feasible ΔP
/// - DS = min(1.0, ΔP_max / P_max)
///
/// See DC power flow formulation in doi:10.1109/TPWRS.2007.899019.
///
/// **Returns:** DS value in [0, 1] where 1.0 means full nameplate is deliverable, 0.0 means none.
fn compute_ds(
    pmax: f64,
    ptdf: &HashMap<i64, f64>,
    branch_limits: &HashMap<i64, f64>,
    flows: &HashMap<i64, f64>,
) -> f64 {
    // Edge case: zero or negative capacity means no deliverability
    if pmax <= 0.0 {
        return 0.0;
    }

    // Find the minimum ΔP across all branches (the tightest constraint)
    let mut delta_p = f64::INFINITY;

    for (&branch_id, &ptdf_value) in ptdf {
        // Skip branches with zero PTDF (they don't constrain this bus)
        if ptdf_value.abs() < 1e-9 {
            continue;
        }

        // Get current flow and thermal limit for this branch
        let flow = *flows.get(&branch_id).unwrap_or(&0.0);
        let limit = *branch_limits.get(&branch_id).unwrap_or(&1e6); // Default to very high limit if missing

        if limit <= 0.0 {
            continue; // Invalid limit, skip
        }

        // Compute the maximum ΔP that keeps this branch within limits
        // Constraint: |flow + ptdf × ΔP| ≤ limit
        // This gives bounds on ΔP; we take the minimum positive bound across all branches
        if let Some(bound) = branch_bound(limit, flow, ptdf_value) {
            delta_p = delta_p.min(bound);
        }
    }

    // If no finite bound found, no capacity is deliverable
    if !delta_p.is_finite() {
        delta_p = 0.0;
    }

    // DS is the fraction of nameplate that's deliverable, clamped to [0, 1]
    (delta_p / pmax).clamp(0.0, 1.0)
}

/// Compute the maximum ΔP that keeps a branch within thermal limits.
///
/// **Constraint:** |flow + ptdf × ΔP| ≤ limit
///
/// This yields two bounds:
/// - Upper: flow + ptdf × ΔP ≤ limit  →  ΔP ≤ (limit - flow) / ptdf  (if ptdf > 0)
/// - Lower: flow + ptdf × ΔP ≥ -limit  →  ΔP ≥ (-limit - flow) / ptdf  (if ptdf > 0)
///
/// For positive injection (ΔP ≥ 0), we need both constraints satisfied, so we take the minimum
/// of the two upper bounds (handling sign of ptdf correctly).
///
/// **Returns:** Some(ΔP_max) if a positive bound exists, None otherwise
fn branch_bound(limit: f64, flow: f64, ptdf: f64) -> Option<f64> {
    let mut candidate = f64::INFINITY;

    // Helper to check if a computed bound is valid (positive and finite)
    let check = |value: f64, candidate: &mut f64| {
        if value > 0.0 && value.is_finite() {
            *candidate = candidate.min(value);
        }
    };

    // Check upper bound: flow + ptdf × ΔP ≤ limit
    check((limit - flow) / ptdf, &mut candidate);
    // Check lower bound: flow + ptdf × ΔP ≥ -limit
    check((-limit - flow) / ptdf, &mut candidate);

    if candidate.is_infinite() {
        None // No valid positive bound found
    } else {
        Some(candidate)
    }
}

/// Load bus capacity limits from CSV file.
///
/// **Expected CSV format:** bus_id, pmax (nameplate capacity in MW)
/// Uses absolute value to handle negative entries gracefully.
fn load_limits(path: &str) -> Result<HashMap<usize, f64>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening limits CSV")?;
    let mut map = HashMap::new();
    for result in rdr.deserialize() {
        let record: PmaxRecord = result.context("parsing limits record")?;
        map.insert(record.bus_id, record.pmax.abs());
    }
    Ok(map)
}

/// Load branch thermal limits from CSV file.
///
/// **Expected CSV format:** branch_id, flow_limit (maximum flow in MW)
/// Uses absolute value to handle negative entries gracefully.
fn load_branch_limits(path: &str) -> Result<HashMap<i64, f64>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening branch limits CSV")?;
    let mut map = HashMap::new();
    for result in rdr.deserialize() {
        let record: FlowLimitRecord = result.context("parsing branch limit record")?;
        map.insert(record.branch_id, record.flow_limit.abs());
    }
    Ok(map)
}

/// Group branch flows by stress case (scenario_id, time combination).
///
/// **Purpose:** The input flows DataFrame may contain flows for multiple scenarios and time periods.
/// This function groups them by (scenario_id, time) to create distinct stress cases for DS computation.
/// If scenario_id or time columns are missing, all flows are treated as a single case.
///
/// **Returns:** Vector of CaseData, each containing flows for one unique stress case.
fn collapse_cases(df: &DataFrame) -> Result<Vec<CaseData>> {
    let branch_col = df.column("branch_id")?.i64()?;
    let flow_col = df.column("flow_mw")?.f64()?;

    // Optional columns: if present, they partition flows into distinct cases
    let scenario_col = df.column("scenario_id").ok();
    let time_col = df.column("time").ok();

    // Group flows by case key (scenario_id, time)
    let mut map: HashMap<CaseKey, HashMap<i64, f64>> = HashMap::new();
    for idx in 0..df.height() {
        let branch = match branch_col.get(idx) {
            Some(value) => value,
            None => continue, // Skip rows with missing branch_id
        };
        let flow = match flow_col.get(idx) {
            Some(value) => value,
            None => continue, // Skip rows with missing flow_mw
        };

        // Create case key from optional scenario/time columns
        let key = CaseKey {
            scenario: column_to_string(scenario_col, idx)?,
            time: column_to_string(time_col, idx)?,
        };

        // Add this branch flow to the appropriate case
        map.entry(key)
            .or_insert_with(HashMap::new)
            .insert(branch, flow);
    }

    // Convert map entries into CaseData structs
    Ok(map
        .into_iter()
        .map(|(key, flows)| CaseData { key, flows })
        .collect())
}

/// Extract string value from optional Polars Series at given index.
///
/// **Returns:** Some(String) if column exists and value is non-null, None otherwise.
///
/// Note: Polars AnyValue represents null as a special variant, and to_string() on null
/// will produce a string representation. We check for null by comparing the string to "null".
fn column_to_string(series: Option<&Series>, idx: usize) -> Result<Option<String>> {
    if let Some(series) = series {
        match series.get(idx) {
            Ok(value) => {
                let s = value.to_string();
                // Polars null values stringify to "null"
                if s == "null" {
                    Ok(None)
                } else {
                    Ok(Some(s))
                }
            }
            Err(_) => Ok(None), // Index out of bounds or other error
        }
    } else {
        Ok(None)
    }
}

/// Extract all bus IDs from the network topology.
///
/// **Purpose:** Collects all bus nodes from the graph, filtering out non-bus nodes.
/// Used to determine which buses should have DS computed.
fn collect_bus_ids(network: &Network) -> Vec<usize> {
    network
        .graph
        .node_indices()
        .filter_map(|node_idx| match &network.graph[node_idx] {
            Node::Bus(bus) => Some(bus.id.value()),
            _ => None, // Skip non-bus nodes (e.g., substations, switches)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::Series;

    #[test]
    fn branch_bound_helpers_handle_limiting_flows() {
        assert_eq!(branch_bound(1.0, 0.0, 1.0).unwrap(), 1.0);
        assert!(branch_bound(1.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn column_to_string_handles_nulls() {
        let series = Series::new("scenario_id", &[Some("base"), None]);
        // Polars stringifies strings with quotes, so we expect "\"base\""
        assert_eq!(
            column_to_string(Some(&series), 0).unwrap(),
            Some("\"base\"".into())
        );
        assert!(column_to_string(Some(&series), 1).unwrap().is_none());
    }
}
