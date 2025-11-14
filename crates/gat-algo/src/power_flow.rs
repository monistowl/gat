use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

use crate::{io::persist_dataframe, test_utils::read_stage_dataframe, OutputStage};
use anyhow::{anyhow, Context, Result};
use csv::ReaderBuilder;
use gat_core::solver::SolverBackend;
use gat_core::{Edge, Network, Node};
use good_lp::solvers::{
    clarabel::clarabel as clarabel_solver, coin_cbc::coin_cbc as coin_cbc_solver,
    highs::highs as highs_solver,
};
use good_lp::{
    constraint, variable, variables, Expression, ProblemVariables, Solution, SolverModel, Variable,
};
use num_complex::Complex64;
use polars::prelude::{DataFrame, NamedFrom, PolarsResult, Series};
use rayon::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct LimitRecord {
    bus_id: usize,
    pmin: f64,
    pmax: f64,
    demand: f64,
}

#[derive(Deserialize)]
struct CostRecord {
    bus_id: usize,
    marginal_cost: f64,
}

#[derive(Deserialize)]
struct BranchLimitRecord {
    branch_id: i64,
    flow_limit: f64,
}

#[derive(Deserialize, Debug)]
struct PiecewiseSegment {
    bus_id: usize,
    start: f64,
    end: f64,
    slope: f64,
}

#[derive(Deserialize, Debug)]
struct ContingencyRecord {
    branch_id: i64,
    label: Option<String>,
}

#[derive(Deserialize, Debug)]
struct MeasurementRecord {
    measurement_type: String,
    branch_id: Option<i64>,
    bus_id: Option<usize>,
    value: f64,
    #[serde(default = "default_weight")]
    weight: f64,
    label: Option<String>,
}

fn default_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy)]
pub enum LpSolverKind {
    Clarabel,
    CoinCbc,
    Highs,
}

impl Default for LpSolverKind {
    fn default() -> Self {
        LpSolverKind::Clarabel
    }
}

impl LpSolverKind {
    pub fn available() -> &'static [&'static str] {
        &["clarabel", "coin_cbc", "highs"]
    }

    pub fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "clarabel" => Ok(LpSolverKind::Clarabel),
            "coin_cbc" | "cbc" => Ok(LpSolverKind::CoinCbc),
            "highs" => Ok(LpSolverKind::Highs),
            other => Err(anyhow!(
                "unknown lp solver '{}'; supported values: clarabel, coin_cbc, highs",
                other
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LpSolverKind::Clarabel => "clarabel",
            LpSolverKind::CoinCbc => "coin_cbc",
            LpSolverKind::Highs => "highs",
        }
    }
}

pub fn dc_power_flow(
    network: &Network,
    solver: &dyn SolverBackend,
    output_file: &Path,
    partitions: &[String],
) -> Result<()> {
    let injections = default_pf_injections(network);
    let (mut df, max_flow, min_flow) = branch_flow_dataframe(network, &injections, None, solver)
        .context("building branch flow table for DC power flow")?;
    persist_dataframe(&mut df, output_file, partitions, OutputStage::PfDc.as_str())?;
    println!(
        "DC power flow summary: {} branch(es), flow range [{:.3}, {:.3}] MW, persisted to {}",
        df.height(),
        min_flow,
        max_flow,
        output_file.display()
    );
    Ok(())
}

pub fn ac_power_flow(network: &Network, tol: f64, max_iter: u32) -> Result<()> {
    let df =
        bus_result_dataframe(network).context("building bus result table for AC power flow")?;
    println!(
        "AC power flow summary: tol={} max_iter={} -> {} buses documented",
        tol,
        max_iter,
        df.height()
    );
    Ok(())
}

pub fn dc_optimal_power_flow(
    network: &Network,
    solver: &dyn SolverBackend,
    cost_csv: &str,
    limits_csv: &str,
    output_file: &Path,
    partitions: &[String],
    branch_limit_csv: Option<&str>,
    piecewise_csv: Option<&str>,
    lp_solver: &LpSolverKind,
) -> Result<()> {
    let costs = load_costs(cost_csv)?;
    let limits = load_limits(limits_csv)?;
    if limits.is_empty() {
        return Err(anyhow!("limits file must contain at least one generator"));
    }

    let branch_limits = match branch_limit_csv {
        Some(path) => load_branch_limits(path)?,
        None => HashMap::new(),
    };
    let piecewise = match piecewise_csv {
        Some(path) => load_piecewise_costs(path)?,
        None => HashMap::new(),
    };

    let total_demand: f64 = limits.iter().map(|item| item.demand).sum();

    let mut vars = variables!();
    let mut cost_expr = Expression::from(0.0);
    let mut sum_dispatch = Expression::from(0.0);
    let mut gen_vars = Vec::new();
    let mut piecewise_constraints: Vec<(Expression, Variable)> = Vec::new();

    for spec in limits {
        let var = vars.add(variable().min(spec.pmin).max(spec.pmax));
        if let Some(segments) = piecewise.get(&spec.bus_id) {
            let (segment_cost, segment_sum) =
                build_piecewise_cost_expression(spec.bus_id, &spec, segments, &mut vars)?;
            cost_expr = cost_expr + segment_cost;
            piecewise_constraints.push((segment_sum, var));
        } else {
            let base_cost = *costs.get(&spec.bus_id).unwrap_or(&1.0);
            cost_expr = cost_expr + base_cost * var;
        }
        sum_dispatch = sum_dispatch + var;
        gen_vars.push((spec.bus_id, var, spec.demand));
    }

    let unsolved = vars.minimise(cost_expr);
    let mut problem_builder = Some(unsolved);
    let solution: Box<dyn Solution> = match lp_solver {
        LpSolverKind::Clarabel => {
            let problem = problem_builder
                .take()
                .expect("building LP problem")
                .using(clarabel_solver);
            let problem = add_dispatch_constraints(
                problem,
                &sum_dispatch,
                &piecewise_constraints,
                total_demand,
            );
            Box::new(problem.solve()?)
        }
        LpSolverKind::CoinCbc => {
            let problem = problem_builder
                .take()
                .expect("building LP problem")
                .using(coin_cbc_solver);
            let problem = add_dispatch_constraints(
                problem,
                &sum_dispatch,
                &piecewise_constraints,
                total_demand,
            );
            Box::new(problem.solve()?)
        }
        LpSolverKind::Highs => {
            let problem = problem_builder
                .take()
                .expect("building LP problem")
                .using(highs_solver);
            let problem = add_dispatch_constraints(
                problem,
                &sum_dispatch,
                &piecewise_constraints,
                total_demand,
            );
            Box::new(problem.solve()?)
        }
    };

    let mut injections = HashMap::new();
    for (bus_id, var, demand) in gen_vars.iter() {
        let dispatch = solution.value(*var);
        injections.insert(*bus_id, dispatch - *demand);
    }

    let (mut df, max_flow, min_flow) = branch_flow_dataframe(network, &injections, None, solver)
        .context("building branch flow table for DC-OPF")?;
    enforce_branch_limits(&df, &branch_limits)?;
    persist_dataframe(
        &mut df,
        output_file,
        partitions,
        OutputStage::OpfDc.as_str(),
    )
    .context("writing DC-OPF Parquet output")?;
    println!(
        "DC-OPF summary: {} branch(es), flow range [{:.3}, {:.3}] MW, persisted to {}",
        df.height(),
        min_flow,
        max_flow,
        output_file.display()
    );
    Ok(())
}

fn add_dispatch_constraints<M>(
    mut problem: M,
    sum_dispatch: &Expression,
    piecewise_constraints: &[(Expression, Variable)],
    total_demand: f64,
) -> M
where
    M: SolverModel,
{
    problem = problem.with(constraint!(sum_dispatch.clone() == total_demand));
    for (segment_sum, gen_var) in piecewise_constraints {
        problem = problem.with(constraint!(segment_sum.clone() == *gen_var));
    }
    problem
}

pub fn n_minus_one_dc(
    network: &Network,
    solver: Arc<dyn SolverBackend>,
    contingencies_csv: &str,
    output_file: &Path,
    partitions: &[String],
    branch_limit_csv: Option<&str>,
) -> Result<()> {
    let contingencies = load_contingencies(contingencies_csv)?;
    if contingencies.is_empty() {
        return Err(anyhow!(
            "contingency file must contain at least one branch outage record"
        ));
    }

    let branch_limits = match branch_limit_csv {
        Some(path) => load_branch_limits(path)?,
        None => HashMap::new(),
    };

    let existing_branches: HashSet<i64> = network
        .graph
        .edge_references()
        .filter_map(|edge| {
            if let Edge::Branch(branch) = edge.weight() {
                Some(branch.id.value() as i64)
            } else {
                None
            }
        })
        .collect();

    for contingency in &contingencies {
        if !existing_branches.contains(&contingency.branch_id) {
            return Err(anyhow!(
                "contingency branch {} not found in network",
                contingency.branch_id
            ));
        }
    }

    let injections = default_pf_injections(network);

    struct ContingencySummary {
        branch_id: i64,
        label: String,
        branch_count: i64,
        max_flow_branch_id: Option<i64>,
        max_abs_flow: f64,
        max_flow: f64,
        min_flow: f64,
        violated: bool,
        violation_branch_id: Option<i64>,
        violation_mw: Option<f64>,
        violation_limit_mw: Option<f64>,
    }

    let solver = Arc::clone(&solver);
    let results: Vec<ContingencySummary> = contingencies
        .par_iter()
        .map(|contingency| -> Result<ContingencySummary> {
            let solver = Arc::clone(&solver);
            let (df, max_flow, min_flow) = branch_flow_dataframe(
                network,
                &injections,
                Some(contingency.branch_id),
                solver.as_ref(),
            )
            .context("building contingency branch flow table")?;
            let branch_ids = df.column("branch_id")?.i64()?;
            let flows = df.column("flow_mw")?.f64()?;
            let mut max_abs_flow = 0.0;
            let mut max_branch_id = None;
            let mut current_violation = 0.0;
            let mut current_violation_branch = None;
            let mut current_violation_limit = None;

            for idx in 0..df.height() {
                if let (Some(branch_id), Some(flow)) = (branch_ids.get(idx), flows.get(idx)) {
                    let abs_flow = flow.abs();
                    if abs_flow > max_abs_flow {
                        max_abs_flow = abs_flow;
                        max_branch_id = Some(branch_id);
                    }
                    if let Some(limit) = branch_limits.get(&branch_id) {
                        if abs_flow > *limit {
                            let violation = abs_flow - limit;
                            if violation > current_violation {
                                current_violation = violation;
                                current_violation_branch = Some(branch_id);
                                current_violation_limit = Some(*limit);
                            }
                        }
                    }
                }
            }

            let rows = df.height() as i64;
            let violated = current_violation_branch.is_some();

            Ok(ContingencySummary {
                branch_id: contingency.branch_id,
                label: contingency.label.clone().unwrap_or_default(),
                branch_count: rows,
                max_flow_branch_id: max_branch_id,
                max_abs_flow,
                max_flow,
                min_flow,
                violated,
                violation_branch_id: current_violation_branch,
                violation_mw: if violated {
                    Some(current_violation)
                } else {
                    None
                },
                violation_limit_mw: current_violation_limit,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let violation_total = results.iter().filter(|summary| summary.violated).count();
    let total_outages = results.len();
    let mut contingency_branch_ids = Vec::with_capacity(total_outages);
    let mut contingency_labels = Vec::with_capacity(total_outages);
    let mut branch_counts = Vec::with_capacity(total_outages);
    let mut max_flow_branch_ids = Vec::with_capacity(total_outages);
    let mut max_abs_flows = Vec::with_capacity(total_outages);
    let mut max_flows = Vec::with_capacity(total_outages);
    let mut min_flows = Vec::with_capacity(total_outages);
    let mut violation_flags = Vec::with_capacity(total_outages);
    let mut violation_branch_ids = Vec::with_capacity(total_outages);
    let mut violation_mw = Vec::with_capacity(total_outages);
    let mut violation_limits = Vec::with_capacity(total_outages);

    for summary in results {
        contingency_branch_ids.push(summary.branch_id);
        contingency_labels.push(summary.label);
        branch_counts.push(summary.branch_count);
        max_flow_branch_ids.push(summary.max_flow_branch_id);
        max_abs_flows.push(summary.max_abs_flow);
        max_flows.push(summary.max_flow);
        min_flows.push(summary.min_flow);
        violation_flags.push(summary.violated);
        violation_branch_ids.push(summary.violation_branch_id);
        violation_mw.push(summary.violation_mw);
        violation_limits.push(summary.violation_limit_mw);
    }

    let mut df = DataFrame::new(vec![
        Series::new("contingency_branch_id", contingency_branch_ids),
        Series::new("contingency_label", contingency_labels),
        Series::new("branch_count", branch_counts),
        Series::new("max_flow_branch_id", max_flow_branch_ids),
        Series::new("max_abs_flow_mw", max_abs_flows),
        Series::new("max_flow_mw", max_flows),
        Series::new("min_flow_mw", min_flows),
        Series::new("violated", violation_flags),
        Series::new("violation_branch_id", violation_branch_ids),
        Series::new("max_violation_mw", violation_mw),
        Series::new("violation_limit_mw", violation_limits),
    ])?;

    persist_dataframe(
        &mut df,
        output_file,
        partitions,
        OutputStage::Nminus1Dc.as_str(),
    )
    .context("writing N-1 DC Parquet output")?;

    println!(
        "N-1 DC summary: {} contingency(ies), {} violation(s), persisted to {}",
        total_outages,
        violation_total,
        output_file.display()
    );
    Ok(())
}

pub fn ac_optimal_power_flow(
    network: &Network,
    solver: &dyn SolverBackend,
    tol: f64,
    max_iter: u32,
    output_file: &Path,
    partitions: &[String],
) -> Result<()> {
    let (bus_ids, _, susceptance) = build_bus_susceptance(network, None);
    let injections = default_pf_injections(network);
    let bus_count = bus_ids.len();
    if bus_count == 0 {
        return Err(anyhow!("AC-OPF requires at least one bus to proceed"));
    }

    let mut angle_vec = vec![0.0; bus_count];
    let mut iterations = 0;
    let mut last_mismatch = 0.0;
    let mut converged = bus_count <= 1;
    let reduced_matrix = if bus_count > 1 {
        let mut reduced = vec![vec![0.0; bus_count - 1]; bus_count - 1];
        for i in 1..bus_count {
            for j in 1..bus_count {
                reduced[i - 1][j - 1] = susceptance[i][j];
            }
        }
        Some(reduced)
    } else {
        None
    };

    if let Some(reduced) = &reduced_matrix {
        for iter in 0..max_iter {
            iterations = iter + 1;
            let mut mismatches = vec![0.0; bus_count - 1];
            let mut max_mismatch: f64 = 0.0;
            for i in 1..bus_count {
                let bus_id = bus_ids[i];
                let p_spec = *injections.get(&bus_id).unwrap_or(&0.0);
                let p_calc =
                    (0..bus_count).fold(0.0, |acc, j| acc + susceptance[i][j] * angle_vec[j]);
                let mismatch = p_spec - p_calc;
                mismatches[i - 1] = mismatch;
                max_mismatch = max_mismatch.max(mismatch.abs());
            }
            last_mismatch = max_mismatch;
            if max_mismatch < tol {
                converged = true;
                break;
            }
            let delta = solve_linear_system(reduced, &mismatches, solver)
                .context("solving AC Jacobian for angle updates")?;
            for i in 1..bus_count {
                angle_vec[i] += delta[i - 1];
            }
        }

        if !converged {
            return Err(anyhow!(
                "AC-OPF did not converge within {} iterations",
                max_iter
            ));
        }
    }

    let mut angle_map = HashMap::new();
    for (idx, bus_id) in bus_ids.iter().enumerate() {
        angle_map.insert(*bus_id, angle_vec[idx]);
    }

    let (mut df, max_flow, min_flow) = branch_flow_dataframe_with_angles(network, &angle_map, None)
        .context("building branch flow table for AC-OPF")?;
    persist_dataframe(
        &mut df,
        output_file,
        partitions,
        OutputStage::OpfAc.as_str(),
    )
    .context("writing AC-OPF Parquet output")?;

    println!(
        "AC-OPF summary: tol={} max_iter={} ({} iteration(s), max mismatch {:.6}) -> {} branch(es), flow range [{:.3}, {:.3}] MW, persisted to {}",
        tol,
        max_iter,
        iterations,
        last_mismatch,
        df.height(),
        min_flow,
        max_flow,
        output_file.display()
    );

    Ok(())
}

pub fn state_estimation_wls(
    network: &Network,
    solver: &dyn SolverBackend,
    measurements_csv: &str,
    output_file: &Path,
    partitions: &[String],
    state_out: Option<&Path>,
    slack_bus: Option<usize>,
) -> Result<()> {
    let measurements = load_measurements(measurements_csv)?;
    let measurement_count = measurements.len();
    if measurement_count == 0 {
        return Err(anyhow!("measurements file must contain at least one entry"));
    }

    let (bus_ids, id_to_index, susceptance) = build_bus_susceptance(network, None);
    if bus_ids.len() < 2 {
        return Err(anyhow!(
            "network must contain at least two buses for WLS state estimation"
        ));
    }

    let default_slack = *bus_ids
        .first()
        .ok_or_else(|| anyhow!("network must contain at least one bus for WLS"))?;
    let slack_bus = if let Some(bus) = slack_bus {
        if !id_to_index.contains_key(&bus) {
            return Err(anyhow!("slack bus {} not found in network", bus));
        }
        bus
    } else {
        default_slack
    };
    let unknown_buses: Vec<usize> = bus_ids.iter().skip(1).cloned().collect();
    let mut unknown_idx = HashMap::new();
    for (idx, bus_id) in unknown_buses.iter().enumerate() {
        unknown_idx.insert(*bus_id, idx);
    }

    let measurement_rows = build_measurement_rows(
        &measurements,
        &susceptance,
        &id_to_index,
        &unknown_buses,
        &unknown_idx,
        slack_bus,
        &network,
    )?;

    let n_vars = unknown_buses.len();
    let mut normal = vec![vec![0.0; n_vars]; n_vars];
    let mut rhs = vec![0.0; n_vars];
    for row in &measurement_rows {
        let y_tilde = row.value - row.offset;
        for i in 0..n_vars {
            for j in 0..n_vars {
                normal[i][j] += row.h[i] * row.weight * row.h[j];
            }
            rhs[i] += row.h[i] * row.weight * y_tilde;
        }
    }

    let solution = solve_linear_system(&normal, &rhs, solver)?;
    let mut angle_map = HashMap::new();
    angle_map.insert(slack_bus, 0.0);
    for (idx, bus_id) in unknown_buses.iter().enumerate() {
        angle_map.insert(*bus_id, solution[idx]);
    }

    let mut indexes = Vec::new();
    let mut types = Vec::new();
    let mut targets = Vec::new();
    let mut values = Vec::new();
    let mut estimates = Vec::new();
    let mut residuals = Vec::new();
    let mut normalized_residuals = Vec::new();
    let mut weights = Vec::new();
    let mut chi2 = 0.0;

    for (idx, row) in measurement_rows.iter().enumerate() {
        let predicted = row
            .h
            .iter()
            .enumerate()
            .map(|(j, coeff)| coeff * solution[j])
            .sum::<f64>()
            + row.offset;
        let residual = predicted - row.value;
        let normalized = residual * row.weight.sqrt();
        chi2 += row.weight * residual * residual;
        indexes.push(idx as i64);
        types.push(row.kind.clone());
        targets.push(row.target.clone());
        values.push(row.value);
        estimates.push(predicted);
        residuals.push(residual);
        normalized_residuals.push(normalized);
        weights.push(row.weight);
    }

    let mut measurement_df = DataFrame::new(vec![
        Series::new("measurement_index", indexes),
        Series::new("measurement_type", types),
        Series::new("target", targets),
        Series::new("value", values),
        Series::new("estimate", estimates),
        Series::new("residual", residuals),
        Series::new("normalized_residual", normalized_residuals),
        Series::new("weight", weights),
    ])?;

    persist_dataframe(
        &mut measurement_df,
        output_file,
        partitions,
        OutputStage::SeWls.as_str(),
    )
    .context("writing state estimation measurements")?;

    if let Some(state_path) = state_out {
        let mut bus_ids_vec = Vec::new();
        let mut angle_vec = Vec::new();
        for bus_id in &bus_ids {
            bus_ids_vec.push(*bus_id as i64);
            angle_vec.push(*angle_map.get(bus_id).unwrap_or(&0.0));
        }
        let mut state_df = DataFrame::new(vec![
            Series::new("bus_id", bus_ids_vec),
            Series::new("angle_rad", angle_vec),
        ])?;
        persist_dataframe(&mut state_df, state_path, &[], OutputStage::SeWls.as_str())
            .context("writing state estimation angles")?;
        println!(
            "State angles persisted to {} ({} buses)",
            state_path.display(),
            bus_ids.len()
        );
    }

    println!(
        "State estimation (WLS): {} measurements, {} state (angles) solved, chi2 {:.3}, persisted to {}",
        measurement_rows.len(),
        n_vars,
        chi2,
        output_file.display()
    );
    Ok(())
}

fn branch_flow_dataframe(
    network: &Network,
    injections: &HashMap<usize, f64>,
    skip_branch: Option<i64>,
    solver: &dyn SolverBackend,
) -> Result<(DataFrame, f64, f64)> {
    let angles = compute_dc_angles(network, injections, skip_branch, solver)?;
    branch_flow_dataframe_with_angles(network, &angles, skip_branch).map_err(|err| anyhow!(err))
}

fn branch_flow_dataframe_with_angles(
    network: &Network,
    angles: &HashMap<usize, f64>,
    skip_branch: Option<i64>,
) -> PolarsResult<(DataFrame, f64, f64)> {
    let mut ids = Vec::new();
    let mut from_bus = Vec::new();
    let mut to_bus = Vec::new();
    let mut flows = Vec::new();

    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            let branch_id = branch.id.value() as i64;
            if skip_branch == Some(branch_id) {
                continue;
            }
            let reactance = branch.reactance.abs().max(1e-6);
            let theta_from = *angles.get(&branch.from_bus.value()).unwrap_or(&0.0);
            let theta_to = *angles.get(&branch.to_bus.value()).unwrap_or(&0.0);
            let flow = (theta_from - theta_to) / reactance;
            ids.push(branch.id.value() as i64);
            from_bus.push(branch.from_bus.value() as i64);
            to_bus.push(branch.to_bus.value() as i64);
            flows.push(flow);
        }
    }

    let df = DataFrame::new(vec![
        Series::new("branch_id", ids),
        Series::new("from_bus", from_bus),
        Series::new("to_bus", to_bus),
        Series::new("flow_mw", flows),
    ])?;

    let flow_vals: Vec<f64> = df
        .column("flow_mw")?
        .f64()?
        .into_iter()
        .filter_map(|opt| opt)
        .collect();
    let (max_flow, min_flow) = if flow_vals.is_empty() {
        (f64::NAN, f64::NAN)
    } else {
        (
            flow_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            flow_vals.iter().cloned().fold(f64::INFINITY, f64::min),
        )
    };
    Ok((df, max_flow, min_flow))
}

fn bus_result_dataframe(network: &Network) -> PolarsResult<DataFrame> {
    let mut ids = Vec::new();
    let mut names = Vec::new();
    let mut voltages = Vec::new();
    let mut angles = Vec::new();

    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            ids.push(bus.id.value() as i64);
            names.push(bus.name.clone());
            voltages.push(bus.voltage_kv);
            angles.push((bus.id.value() % 360) as f64);
        }
    }

    DataFrame::new(vec![
        Series::new("bus_id", ids),
        Series::new("name", names),
        Series::new("voltage_kv", voltages),
        Series::new("angle", angles),
    ])
}

fn default_pf_injections(network: &Network) -> HashMap<usize, f64> {
    let mut injections = HashMap::new();
    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Gen(gen) => {
                *injections.entry(gen.bus.value()).or_insert(0.0) += gen.active_power_mw;
            }
            Node::Load(load) => {
                *injections.entry(load.bus.value()).or_insert(0.0) -= load.active_power_mw;
            }
            _ => {}
        }
    }

    if injections.is_empty() {
        let mut bus_ids: Vec<usize> = network
            .graph
            .node_indices()
            .filter_map(|idx| match &network.graph[idx] {
                Node::Bus(bus) => Some(bus.id.value()),
                _ => None,
            })
            .collect();
        bus_ids.sort_unstable();
        if bus_ids.len() >= 2 {
            injections.insert(bus_ids[0], 1.0);
            injections.insert(bus_ids[1], -1.0);
        }
    }
    injections
}

fn build_bus_susceptance(
    network: &Network,
    skip_branch: Option<i64>,
) -> (Vec<usize>, HashMap<usize, usize>, Vec<Vec<f64>>) {
    let mut bus_ids: Vec<usize> = network
        .graph
        .node_indices()
        .filter_map(|idx| match &network.graph[idx] {
            Node::Bus(bus) => Some(bus.id.value()),
            _ => None,
        })
        .collect();
    bus_ids.sort_unstable();

    let mut id_to_index = HashMap::new();
    for (idx, bus_id) in bus_ids.iter().enumerate() {
        id_to_index.insert(*bus_id, idx);
    }

    let mut susceptance = vec![vec![0.0; bus_ids.len()]; bus_ids.len()];
    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            let branch_id = branch.id.value() as i64;
            if skip_branch == Some(branch_id) {
                continue;
            }
            let from = branch.from_bus.value();
            let to = branch.to_bus.value();
            if let (Some(&i), Some(&j)) = (id_to_index.get(&from), id_to_index.get(&to)) {
                let reactance = branch.reactance.abs().max(1e-6);
                let b = 1.0 / reactance;
                susceptance[i][j] -= b;
                susceptance[j][i] -= b;
                susceptance[i][i] += b;
                susceptance[j][j] += b;
            }
        }
    }
    (bus_ids, id_to_index, susceptance)
}

fn compute_dc_angles(
    network: &Network,
    injections: &HashMap<usize, f64>,
    skip_branch: Option<i64>,
    solver: &dyn SolverBackend,
) -> Result<HashMap<usize, f64>> {
    let (bus_ids, _, susceptance) = build_bus_susceptance(network, skip_branch);
    let node_count = bus_ids.len();
    if node_count == 0 {
        return Ok(HashMap::new());
    }
    if node_count == 1 {
        let mut angles = HashMap::new();
        angles.insert(bus_ids[0], 0.0);
        return Ok(angles);
    }

    let mut rhs = vec![0.0; node_count];
    for (idx, bus_id) in bus_ids.iter().enumerate() {
        rhs[idx] = *injections.get(bus_id).unwrap_or(&0.0);
    }

    let mut reduced = vec![vec![0.0; node_count - 1]; node_count - 1];
    let mut reduced_rhs = vec![0.0; node_count - 1];
    for i in 1..node_count {
        for j in 1..node_count {
            reduced[i - 1][j - 1] = susceptance[i][j];
        }
        reduced_rhs[i - 1] = rhs[i];
    }

    let solution = solve_linear_system(&reduced, &reduced_rhs, solver)?;
    let mut angles = HashMap::new();
    angles.insert(bus_ids[0], 0.0);
    for (i, bus_id) in bus_ids.iter().enumerate().skip(1) {
        angles.insert(*bus_id, solution[i - 1]);
    }
    Ok(angles)
}

fn solve_linear_system(
    matrix: &[Vec<f64>],
    injections: &[f64],
    solver: &dyn SolverBackend,
) -> Result<Vec<f64>> {
    solver.solve(matrix, injections)
}

fn load_costs(path: &str) -> Result<HashMap<usize, f64>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening cost CSV")?;
    let mut map = HashMap::new();
    for result in rdr.deserialize() {
        let record: CostRecord = result.context("parsing cost CSV record")?;
        map.insert(record.bus_id, record.marginal_cost);
    }
    Ok(map)
}

fn load_limits(path: &str) -> Result<Vec<LimitRecord>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening limits CSV")?;
    let mut out = Vec::new();
    for result in rdr.deserialize() {
        let record: LimitRecord = result.context("parsing limits CSV record")?;
        out.push(record);
    }
    Ok(out)
}

fn load_branch_limits(path: &str) -> Result<HashMap<i64, f64>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening branch limits CSV")?;
    let mut map = HashMap::new();
    for result in rdr.deserialize() {
        let record: BranchLimitRecord = result.context("parsing branch limit record")?;
        map.insert(record.branch_id, record.flow_limit);
    }
    Ok(map)
}

fn load_contingencies(path: &str) -> Result<Vec<ContingencyRecord>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening contingencies CSV")?;
    let mut out = Vec::new();
    for result in rdr.deserialize() {
        let record: ContingencyRecord = result.context("parsing contingency record")?;
        out.push(record);
    }
    Ok(out)
}

fn load_piecewise_costs(path: &str) -> Result<HashMap<usize, Vec<PiecewiseSegment>>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening piecewise CSV")?;
    let mut map: HashMap<usize, Vec<PiecewiseSegment>> = HashMap::new();
    for result in rdr.deserialize() {
        let record: PiecewiseSegment = result.context("parsing piecewise segment")?;
        map.entry(record.bus_id).or_default().push(record);
    }
    for segs in map.values_mut() {
        segs.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    Ok(map)
}

fn load_measurements(path: &str) -> Result<Vec<MeasurementRecord>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("opening measurements CSV")?;
    let mut out = Vec::new();
    for result in rdr.deserialize() {
        let record: MeasurementRecord = result.context("parsing measurement record")?;
        if record.weight <= 0.0 {
            return Err(anyhow!("measurement weights must be positive"));
        }
        out.push(record);
    }
    Ok(out)
}

struct MeasurementRow {
    kind: String,
    target: String,
    h: Vec<f64>,
    offset: f64,
    value: f64,
    weight: f64,
}

struct BranchDescriptor {
    from_bus: usize,
    to_bus: usize,
    reactance: f64,
}

fn build_measurement_rows(
    measurements: &[MeasurementRecord],
    susceptance: &[Vec<f64>],
    id_to_index: &HashMap<usize, usize>,
    unknown_buses: &[usize],
    unknown_idx: &HashMap<usize, usize>,
    slack_bus: usize,
    network: &Network,
) -> Result<Vec<MeasurementRow>> {
    let mut branch_map = HashMap::new();
    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            branch_map.insert(
                branch.id.value() as i64,
                BranchDescriptor {
                    from_bus: branch.from_bus.value(),
                    to_bus: branch.to_bus.value(),
                    reactance: branch.reactance,
                },
            );
        }
    }

    let mut rows = Vec::new();
    for record in measurements {
        let kind = record.measurement_type.to_lowercase();
        let mut h = vec![0.0; unknown_buses.len()];
        let mut offset = 0.0;
        let target = record.label.clone().unwrap_or_else(|| match kind.as_str() {
            "flow" => format!("branch {}", record.branch_id.unwrap_or(-1)),
            "injection" => format!("bus {}", record.bus_id.unwrap_or(0)),
            "angle" => format!("angle {}", record.bus_id.unwrap_or(0)),
            "voltage" => format!("voltage {}", record.bus_id.unwrap_or(0)),
            _ => "measurement".to_string(),
        });

        match kind.as_str() {
            "flow" => {
                let branch_id = record
                    .branch_id
                    .ok_or_else(|| anyhow!("flow measurement must include branch_id"))?;
                let branch = branch_map
                    .get(&branch_id)
                    .ok_or_else(|| anyhow!("branch {} not found for measurement", branch_id))?;
                let gain = 1.0 / branch.reactance.abs().max(1e-6);
                let mut add_bus = |bus_id: usize, sign: f64| {
                    if bus_id == slack_bus {
                        return;
                    }
                    if let Some(&col) = unknown_idx.get(&bus_id) {
                        h[col] += sign * gain;
                    }
                };
                add_bus(branch.from_bus, 1.0);
                add_bus(branch.to_bus, -1.0);
            }
            "injection" => {
                let bus_id = record
                    .bus_id
                    .ok_or_else(|| anyhow!("injection measurement must include bus_id"))?;
                let matrix_idx = *id_to_index.get(&bus_id).ok_or_else(|| {
                    anyhow!(
                        "bus {} not present in network for injection measurement",
                        bus_id
                    )
                })?;
                let row = &susceptance[matrix_idx];
                for (col, unknown_bus) in unknown_buses.iter().enumerate() {
                    let bus_idx = *id_to_index.get(unknown_bus).unwrap_or(&0);
                    h[col] = row[bus_idx];
                }
                let slack_idx = *id_to_index.get(&slack_bus).unwrap_or(&matrix_idx);
                offset = row[slack_idx] * 0.0;
            }
            "angle" | "voltage" => {
                let bus_id = record
                    .bus_id
                    .ok_or_else(|| anyhow!("{} measurement must include bus_id", kind))?;
                if let Some(&col) = unknown_idx.get(&bus_id) {
                    h[col] = 1.0;
                }
            }
            _ => {
                return Err(anyhow!(
                    "unsupported measurement type '{}'",
                    record.measurement_type
                ));
            }
        }

        rows.push(MeasurementRow {
            kind,
            target,
            h,
            offset,
            value: record.value,
            weight: record.weight,
        });
    }

    Ok(rows)
}

fn build_piecewise_cost_expression(
    bus_id: usize,
    spec: &LimitRecord,
    segments: &[PiecewiseSegment],
    vars: &mut ProblemVariables,
) -> Result<(Expression, Expression)> {
    if segments.is_empty() {
        return Err(anyhow!(
            "piecewise cost data for bus {} must include at least one segment",
            bus_id
        ));
    }

    const TOL: f64 = 1e-6;
    let first = segments.first().unwrap();
    if first.start > spec.pmin + TOL {
        return Err(anyhow!(
            "piecewise segments for bus {} must begin at or before pmin ({:.3})",
            bus_id,
            spec.pmin
        ));
    }
    let last = segments.last().unwrap();
    if last.end < spec.pmax - TOL {
        return Err(anyhow!(
            "piecewise segments for bus {} must extend to pmax ({:.3})",
            bus_id,
            spec.pmax
        ));
    }

    let mut cost_expr = Expression::from(0.0);
    let mut segment_sum = Expression::from(0.0);
    let mut prev_end = first.start;
    for segment in segments {
        if segment.end <= segment.start {
            return Err(anyhow!(
                "piecewise segment for bus {} has non-positive range: [{:.3}, {:.3}]",
                bus_id,
                segment.start,
                segment.end
            ));
        }
        if segment.start < prev_end - TOL {
            return Err(anyhow!(
                "piecewise segment for bus {} overlaps previous range at {:.3}",
                bus_id,
                segment.start
            ));
        }

        if segment.start > prev_end + TOL {
            return Err(anyhow!(
                "gap between piecewise segments for bus {}: [{:.3}, {:.3}] missing coverage starting at {:.3}",
                bus_id,
                prev_end,
                segment.start,
                segment.start
            ));
        }

        let width = segment.end - segment.start;
        let seg_var = vars.add(variable().min(0.0).max(width));
        segment_sum = segment_sum + seg_var;
        cost_expr = cost_expr + segment.slope * seg_var;
        prev_end = segment.end;
    }

    Ok((cost_expr, segment_sum))
}

fn enforce_branch_limits(df: &DataFrame, limits: &HashMap<i64, f64>) -> Result<()> {
    let branch_ids = df.column("branch_id")?.i64()?;
    let flows = df.column("flow_mw")?.f64()?;
    let mut violations = Vec::new();
    for idx in 0..df.height() {
        if let Some(branch_id) = branch_ids.get(idx) {
            if let Some(limit) = limits.get(&branch_id) {
                if let Some(flow) = flows.get(idx) {
                    if flow.abs() > *limit {
                        violations.push((branch_id, flow, *limit));
                    }
                }
            }
        }
    }
    if !violations.is_empty() {
        let details: Vec<String> = violations
            .iter()
            .map(|(branch_id, flow, limit)| {
                format!(
                    "branch {} |flow| {:.3} > limit {:.3}",
                    branch_id,
                    flow.abs(),
                    limit
                )
            })
            .collect();
        return Err(anyhow!(
            "branch limits violated by {} element(s): {}",
            violations.len(),
            details.join(", ")
        ));
    }
    Ok(())
}

#[allow(dead_code)]
fn build_y_bus(
    network: &Network,
) -> (
    HashMap<usize, HashMap<usize, Complex64>>,
    Vec<usize>,
    HashMap<usize, usize>,
) {
    let mut ybus: HashMap<usize, HashMap<usize, Complex64>> = HashMap::new();
    let mut bus_order = Vec::new();
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            bus_order.push(bus.id.value());
        }
    }
    let mut id_to_index = HashMap::new();
    for (idx, bus_id) in bus_order.iter().enumerate() {
        id_to_index.insert(*bus_id, idx);
    }

    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            let i = branch.from_bus.value();
            let j = branch.to_bus.value();
            let admittance = Complex64::new(0.0, -1.0 / branch.reactance.max(1e-6));
            ybus.entry(i)
                .or_default()
                .entry(j)
                .and_modify(|v| *v += admittance)
                .or_insert(admittance);
            ybus.entry(j)
                .or_default()
                .entry(i)
                .and_modify(|v| *v += admittance)
                .or_insert(admittance);
            ybus.entry(i)
                .or_default()
                .entry(i)
                .and_modify(|v| *v -= admittance)
                .or_insert(-admittance);
            ybus.entry(j)
                .or_default()
                .entry(j)
                .and_modify(|v| *v -= admittance)
                .or_insert(-admittance);
        }
    }
    (ybus, bus_order, id_to_index)
}

#[allow(dead_code)]
fn compute_p(
    ybus: &HashMap<usize, HashMap<usize, Complex64>>,
    id_to_index: &HashMap<usize, usize>,
    angles: &[f64],
    bus_id: usize,
) -> f64 {
    let idx = *id_to_index.get(&bus_id).unwrap_or(&0);
    let theta_i = angles[idx];
    ybus.get(&bus_id)
        .map(|neighbors| {
            neighbors.iter().fold(0.0, |acc, (other, adm)| {
                let neighbor_idx = *id_to_index.get(other).unwrap_or(&idx);
                let theta_j = angles[neighbor_idx];
                acc + adm.im * (theta_i - theta_j)
            })
        })
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::solver::GaussSolver;
    use gat_core::{Branch, BranchId, Bus, BusId, Edge, Network, Node};
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn build_simple_network() -> Network {
        let mut network = Network::new();
        let b0 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(0),
            name: "Bus 0".to_string(),
            voltage_kv: 138.0,
        }));
        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
        }));
        network.graph.add_edge(
            b0,
            b1,
            Edge::Branch(Branch {
                id: BranchId::new(0),
                name: "Line 0-1".to_string(),
                from_bus: BusId::new(0),
                to_bus: BusId::new(1),
                resistance: 0.01,
                reactance: 0.1,
            }),
        );
        network
    }

    fn build_parallel_network() -> Network {
        let mut network = Network::new();
        let b0 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(0),
            name: "Bus 0".to_string(),
            voltage_kv: 138.0,
        }));
        let b1 = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
        }));
        network.graph.add_edge(
            b0,
            b1,
            Edge::Branch(Branch {
                id: BranchId::new(0),
                name: "Line 0-1a".to_string(),
                from_bus: BusId::new(0),
                to_bus: BusId::new(1),
                resistance: 0.01,
                reactance: 0.1,
            }),
        );
        network.graph.add_edge(
            b0,
            b1,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Line 0-1b".to_string(),
                from_bus: BusId::new(0),
                to_bus: BusId::new(1),
                resistance: 0.01,
                reactance: 0.1,
            }),
        );
        network
    }

    #[test]
    fn dc_power_flow_writes_parquet() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let out = temp_dir.path().join("dc.parquet");
        let solver = GaussSolver::default();
        dc_power_flow(&network, &solver, &out, &[]).unwrap();

        let df = read_stage_dataframe(&out, OutputStage::PfDc).unwrap();
        assert_eq!(df.height(), 1);
        let flow = df.column("flow_mw").unwrap().f64().unwrap().get(0).unwrap();
        assert!(!flow.is_nan());
    }

    #[test]
    fn dc_opf_respects_branch_limits() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let cost_path = temp_dir.path().join("costs.csv");
        let limits_path = temp_dir.path().join("limits.csv");
        let branch_limits_path = temp_dir.path().join("branch_limits.csv");
        fs::write(&cost_path, "bus_id,marginal_cost\n0,100\n1,10\n").unwrap();
        fs::write(&limits_path, "bus_id,pmin,pmax,demand\n0,0,5,1\n1,0,5,0\n").unwrap();
        fs::write(&branch_limits_path, "branch_id,flow_limit\n0,0.001\n").unwrap();

        let out = temp_dir.path().join("opf.parquet");
        let solver = GaussSolver::default();
        assert!(dc_optimal_power_flow(
            &network,
            &solver,
            cost_path.to_str().unwrap(),
            limits_path.to_str().unwrap(),
            &out,
            &[],
            Some(branch_limits_path.to_str().unwrap()),
            None,
            &LpSolverKind::Clarabel,
        )
        .is_err());
    }

    #[test]
    fn dc_opf_honors_costs() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let cost_path = temp_dir.path().join("costs.csv");
        let limits_path = temp_dir.path().join("limits.csv");
        fs::write(&cost_path, "bus_id,marginal_cost\n0,10\n1,20\n").unwrap();
        fs::write(&limits_path, "bus_id,pmin,pmax,demand\n0,0,5,1\n1,0,5,0\n").unwrap();

        let out = temp_dir.path().join("opf.parquet");
        let solver = GaussSolver::default();
        dc_optimal_power_flow(
            &network,
            &solver,
            cost_path.to_str().unwrap(),
            limits_path.to_str().unwrap(),
            &out,
            &[],
            None,
            None,
            &LpSolverKind::Clarabel,
        )
        .unwrap();

        let df = read_stage_dataframe(&out, OutputStage::OpfDc).unwrap();
        assert_eq!(df.height(), 1);
    }

    #[test]
    fn dc_opf_coin_cbc_available() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let cost_path = temp_dir.path().join("costs.csv");
        let limits_path = temp_dir.path().join("limits.csv");
        fs::write(&cost_path, "bus_id,marginal_cost\n0,10\n1,20\n").unwrap();
        fs::write(&limits_path, "bus_id,pmin,pmax,demand\n0,0,5,1\n1,0,5,0\n").unwrap();
        let out = temp_dir.path().join("coin.parquet");
        let solver = GaussSolver::default();
        dc_optimal_power_flow(
            &network,
            &solver,
            cost_path.to_str().unwrap(),
            limits_path.to_str().unwrap(),
            &out,
            &[],
            None,
            None,
            &LpSolverKind::CoinCbc,
        )
        .unwrap();
        let df = read_stage_dataframe(&out, OutputStage::OpfDc).unwrap();
        assert_eq!(df.height(), 1);
    }

    #[test]
    fn dc_opf_highs_available() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let cost_path = temp_dir.path().join("costs.csv");
        let limits_path = temp_dir.path().join("limits.csv");
        fs::write(&cost_path, "bus_id,marginal_cost\n0,10\n1,20\n").unwrap();
        fs::write(&limits_path, "bus_id,pmin,pmax,demand\n0,0,5,1\n1,0,5,0\n").unwrap();
        let out = temp_dir.path().join("highs.parquet");
        let solver = GaussSolver::default();
        dc_optimal_power_flow(
            &network,
            &solver,
            cost_path.to_str().unwrap(),
            limits_path.to_str().unwrap(),
            &out,
            &[],
            None,
            None,
            &LpSolverKind::Highs,
        )
        .unwrap();
        let df = read_stage_dataframe(&out, OutputStage::OpfDc).unwrap();
        assert_eq!(df.height(), 1);
    }

    #[test]
    fn dc_opf_piecewise_requires_full_coverage() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let cost_path = temp_dir.path().join("costs.csv");
        let limits_path = temp_dir.path().join("limits.csv");
        let piecewise_path = temp_dir.path().join("piecewise.csv");
        fs::write(&cost_path, "bus_id,marginal_cost\n0,10\n1,20\n").unwrap();
        fs::write(&limits_path, "bus_id,pmin,pmax,demand\n0,0,5,1\n1,0,5,0\n").unwrap();
        fs::write(&piecewise_path, "bus_id,start,end,slope\n0,0,2,10\n").unwrap();

        let out = temp_dir.path().join("opf.parquet");
        let solver = GaussSolver::default();
        let err = dc_optimal_power_flow(
            &network,
            &solver,
            cost_path.to_str().unwrap(),
            limits_path.to_str().unwrap(),
            &out,
            &[],
            None,
            Some(piecewise_path.to_str().unwrap()),
            &LpSolverKind::Clarabel,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("extend to pmax"),
            "unexpected error: {:?}",
            err
        );
    }

    #[test]
    fn dc_opf_piecewise_honors_segments() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let cost_path = temp_dir.path().join("costs.csv");
        let limits_path = temp_dir.path().join("limits.csv");
        let piecewise_path = temp_dir.path().join("piecewise.csv");
        fs::write(&cost_path, "bus_id,marginal_cost\n0,10\n1,20\n").unwrap();
        fs::write(&limits_path, "bus_id,pmin,pmax,demand\n0,0,5,1\n1,0,5,0\n").unwrap();
        fs::write(
            &piecewise_path,
            "bus_id,start,end,slope\n0,0,3,10\n0,3,5,15\n",
        )
        .unwrap();

        let out = temp_dir.path().join("opf.parquet");
        let solver = GaussSolver::default();
        dc_optimal_power_flow(
            &network,
            &solver,
            cost_path.to_str().unwrap(),
            limits_path.to_str().unwrap(),
            &out,
            &[],
            None,
            Some(piecewise_path.to_str().unwrap()),
            &LpSolverKind::Clarabel,
        )
        .unwrap();
    }

    #[test]
    fn n_minus_one_dc_detects_violation() {
        let network = build_parallel_network();
        let temp_dir = tempdir().unwrap();
        let contingencies_path = temp_dir.path().join("contingencies.csv");
        let branch_limits_path = temp_dir.path().join("branch_limits.csv");
        let out = temp_dir.path().join("nminus1.parquet");
        fs::write(&contingencies_path, "branch_id,label\n0,line0\n1,line1\n").unwrap();
        fs::write(&branch_limits_path, "branch_id,flow_limit\n0,5\n1,0.1\n").unwrap();

        let solver = Arc::new(GaussSolver::default());
        n_minus_one_dc(
            &network,
            solver,
            contingencies_path.to_str().unwrap(),
            &out,
            &[],
            Some(branch_limits_path.to_str().unwrap()),
        )
        .unwrap();

        let df = read_stage_dataframe(&out, OutputStage::Nminus1Dc).unwrap();
        assert_eq!(df.height(), 2);
        let contingency_ids = df.column("contingency_branch_id").unwrap().i64().unwrap();
        assert_eq!(
            contingency_ids.into_no_null_iter().collect::<Vec<_>>(),
            vec![0, 1]
        );
        let violated = df.column("violated").unwrap().bool().unwrap();
        assert!(violated.get(0).unwrap());
        assert!(!violated.get(1).unwrap());
        let violation_branch = df.column("violation_branch_id").unwrap().i64().unwrap();
        assert_eq!(violation_branch.get(0), Some(1));
        assert_eq!(violation_branch.get(1), None);
    }

    #[test]
    fn ac_opf_writes_parquet() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let out = temp_dir.path().join("ac.parquet");
        let solver = GaussSolver::default();
        ac_optimal_power_flow(&network, &solver, 1e-6, 10, &out, &[]).unwrap();

        let df = read_stage_dataframe(&out, OutputStage::OpfAc).unwrap();
        assert_eq!(df.height(), 1);
    }

    #[test]
    fn state_estimation_wls_outputs_residuals() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let meas_path = temp_dir.path().join("measurements.csv");
        let out = temp_dir.path().join("se.parquet");
        let state_out = temp_dir.path().join("states.parquet");
        fs::write(
            &meas_path,
            "measurement_type,branch_id,bus_id,value,weight,label\nflow,0,,1.0,1.0,line0-1\ninjection,,1,0.5,1.0,bus1\n",
        )
        .unwrap();

        let solver = GaussSolver::default();
        state_estimation_wls(
            &network,
            &solver,
            meas_path.to_str().unwrap(),
            &out,
            &[],
            Some(state_out.as_path()),
            None,
        )
        .unwrap();

        let meas_df = read_stage_dataframe(&out, OutputStage::SeWls).unwrap();
        assert_eq!(meas_df.height(), 2);

        let state_df = read_stage_dataframe(&state_out, OutputStage::SeWls).unwrap();
        assert_eq!(state_df.height(), 2);
    }

    #[test]
    fn state_estimation_angle_measurement_with_slack_flag() {
        let network = build_simple_network();
        let temp_dir = tempdir().unwrap();
        let meas_path = temp_dir.path().join("angle.csv");
        let out = temp_dir.path().join("se-angle.parquet");
        let state_out = temp_dir.path().join("state-angle.parquet");
        fs::write(
            &meas_path,
            "measurement_type,branch_id,bus_id,value,weight,label\nangle,,0,0.0,1.0,bus0-angle\ninjection,,1,0.0,1.0,bus1-inj\n",
        )
        .unwrap();

        let solver = GaussSolver::default();
        state_estimation_wls(
            &network,
            &solver,
            meas_path.to_str().unwrap(),
            &out,
            &[],
            Some(state_out.as_path()),
            Some(1),
        )
        .unwrap();

        let meas_df = read_stage_dataframe(&out, OutputStage::SeWls).unwrap();
        assert_eq!(meas_df.height(), 2);
        let state_df = read_stage_dataframe(&state_out, OutputStage::SeWls).unwrap();
        assert_eq!(state_df.height(), 2);
    }
}
