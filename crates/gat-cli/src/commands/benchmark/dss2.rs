//! DSS² State Estimation Benchmark
//!
//! Benchmarks WLS state estimation against the CIGRE Medium-Voltage test network
//! with synthetic measurements. Reproduces the WLS baseline from the DSS² paper:
//! "Deep Statistical Solver for Distribution System State Estimation" (arXiv:2301.01835)
//!
//! The benchmark:
//! 1. Builds the CIGRE 14-bus MV network
//! 2. Runs DC power flow to get the true state (bus angles)
//! 3. Generates noisy measurements from the true state
//! 4. Runs WLS state estimation
//! 5. Compares estimated angles to true angles
//!
//! Key metrics:
//! - Mean Absolute Error (MAE) in degrees
//! - Root Mean Square Error (RMSE) in degrees
//! - Maximum error in degrees
//! - Solve time in milliseconds

use anyhow::{anyhow, Context, Result};
use csv::Writer;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::time::Instant;
use tempfile::NamedTempFile;

use gat_algo::power_flow;
use gat_core::solver::SolverKind;
use gat_core::{Edge, Network, Node};
use gat_io::sources::cigre::{
    build_cigre_mv_network, generate_measurements, write_measurements_csv, CigreMvConfig,
    MeasurementGeneratorConfig,
};

/// Result for a single trial
#[derive(Debug, Clone, Serialize)]
pub struct Dss2TrialResult {
    /// Trial index (0-based)
    pub trial: usize,
    /// Random seed used for this trial
    pub seed: u64,
    /// Number of buses in the network
    pub num_buses: usize,
    /// Number of branches in the network
    pub num_branches: usize,
    /// Number of measurements used
    pub num_measurements: usize,
    /// Noise standard deviation
    pub noise_std: f64,
    /// Load scale factor
    pub load_scale: f64,
    /// Time to run DC power flow (ms)
    pub pf_time_ms: f64,
    /// Time to generate measurements (ms)
    pub meas_gen_time_ms: f64,
    /// Time to run WLS state estimation (ms)
    pub se_time_ms: f64,
    /// Total solve time (ms)
    pub total_time_ms: f64,
    /// Mean absolute error in bus angles (degrees)
    pub mae_deg: f64,
    /// Root mean square error in bus angles (degrees)
    pub rmse_deg: f64,
    /// Maximum absolute error in bus angles (degrees)
    pub max_error_deg: f64,
    /// Whether SE converged successfully
    pub converged: bool,
}

/// Aggregate statistics across all trials
#[derive(Debug, Clone, Serialize)]
pub struct Dss2Summary {
    /// Total number of trials
    pub total_trials: usize,
    /// Number of converged trials
    pub converged_trials: usize,
    /// Convergence rate (0.0 to 1.0)
    pub convergence_rate: f64,
    /// Mean MAE across trials (degrees)
    pub mean_mae_deg: f64,
    /// Standard deviation of MAE (degrees)
    pub std_mae_deg: f64,
    /// Mean RMSE across trials (degrees)
    pub mean_rmse_deg: f64,
    /// Mean max error across trials (degrees)
    pub mean_max_error_deg: f64,
    /// Median SE solve time (ms)
    pub median_se_time_ms: f64,
    /// Mean SE solve time (ms)
    pub mean_se_time_ms: f64,
    /// 95th percentile SE solve time (ms)
    pub p95_se_time_ms: f64,
}

/// Configuration for DSS² benchmark
#[derive(Debug, Clone)]
pub struct Dss2Config {
    pub trials: usize,
    pub noise_std: f64,
    pub load_scale: f64,
    pub num_flow: usize,
    pub num_injection: usize,
    pub base_seed: Option<u64>,
}

#[allow(clippy::too_many_arguments)]
pub fn handle(
    out: &str,
    trials: usize,
    noise_std: f64,
    load_scale: f64,
    num_flow: usize,
    num_injection: usize,
    seed: Option<u64>,
    threads: &str,
) -> Result<()> {
    // Configure thread pool
    if threads != "auto" {
        if let Ok(n) = threads.parse::<usize>() {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build_global()
                .ok();
        }
    }

    let config = Dss2Config {
        trials,
        noise_std,
        load_scale,
        num_flow,
        num_injection,
        base_seed: seed,
    };

    eprintln!("DSS² State Estimation Benchmark");
    eprintln!("================================");
    eprintln!("Network: CIGRE 14-bus MV");
    eprintln!("Trials: {}", config.trials);
    eprintln!("Noise σ: {:.1}%", config.noise_std * 100.0);
    eprintln!("Load scale: {:.2}", config.load_scale);
    eprintln!(
        "Measurements: {} flow + {} injection",
        config.num_flow, config.num_injection
    );
    eprintln!();

    // Run benchmark
    let results = run_benchmark(&config)?;

    // Compute summary statistics
    let summary = compute_summary(&results);

    // Write results to CSV
    write_results(out, &results, &summary)?;

    // Print summary
    print_summary(&summary);

    Ok(())
}

fn run_benchmark(config: &Dss2Config) -> Result<Vec<Dss2TrialResult>> {
    // Build the CIGRE MV network once
    let cigre_config = CigreMvConfig {
        base_kv: 20.0,
        base_mva: 100.0,
        load_scale: config.load_scale,
    };
    let network = build_cigre_mv_network(&cigre_config);

    let num_buses = network
        .graph
        .node_weights()
        .filter(|n| matches!(n, Node::Bus(_)))
        .count();
    let num_branches = network
        .graph
        .edge_weights()
        .filter(|e| matches!(e, Edge::Branch(_)))
        .count();

    eprintln!("Network built: {} buses, {} branches", num_buses, num_branches);

    // Run DC power flow to get true state
    let pf_start = Instant::now();
    let true_angles = run_dc_power_flow(&network)?;
    let pf_time_base = pf_start.elapsed().as_secs_f64() * 1000.0;
    eprintln!("Base PF time: {:.2} ms", pf_time_base);

    // Generate true injection values from the network
    let true_injections = compute_true_injections(&network);

    // Compute branch flows from DC power flow (simplified: use P = B * (θ_i - θ_j))
    let branch_flows = compute_branch_flows(&network, &true_angles);

    // Run trials in parallel
    let base_seed = config.base_seed.unwrap_or(42);
    let results: Vec<Dss2TrialResult> = (0..config.trials)
        .into_par_iter()
        .map(|trial| {
            run_single_trial(
                trial,
                &network,
                &true_angles,
                &true_injections,
                &branch_flows,
                config,
                base_seed + trial as u64,
                num_buses,
                num_branches,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(results)
}

fn run_dc_power_flow(network: &Network) -> Result<HashMap<usize, f64>> {
    // Use the new dc_power_flow_angles function that properly solves B'θ = P
    // and returns the actual bus angles from the DC power flow solution.
    power_flow::dc_power_flow_angles(network)
}

fn compute_true_injections(network: &Network) -> HashMap<usize, f64> {
    let mut injections: HashMap<usize, f64> = HashMap::new();

    // Sum up generation and load at each bus
    for node in network.graph.node_weights() {
        match node {
            Node::Gen(g) => {
                let bus_id = g.bus.value();
                *injections.entry(bus_id).or_insert(0.0) += g.active_power.value();
            }
            Node::Load(l) => {
                let bus_id = l.bus.value();
                *injections.entry(bus_id).or_insert(0.0) -= l.active_power.value();
            }
            _ => {}
        }
    }

    // Normalize to per-unit
    for val in injections.values_mut() {
        *val /= 100.0; // base_mva
    }

    injections
}

/// Compute branch power flows from bus angles using DC power flow equations
/// P_ij = (θ_i - θ_j) / x_ij  (in per-unit)
///
/// Returns flows in per-unit, which is what the WLS state estimation expects.
fn compute_branch_flows(network: &Network, bus_angles: &HashMap<usize, f64>) -> HashMap<i64, f64> {
    let mut flows: HashMap<i64, f64> = HashMap::new();

    for edge in network.graph.edge_weights() {
        if let Edge::Branch(b) = edge {
            let from_bus = b.from_bus.value();
            let to_bus = b.to_bus.value();

            // Get angles (default to 0.0 if not found)
            let theta_from = bus_angles.get(&from_bus).copied().unwrap_or(0.0);
            let theta_to = bus_angles.get(&to_bus).copied().unwrap_or(0.0);

            // DC power flow: P = (θ_i - θ_j) / x_ij
            // Note: reactance is in per-unit, so flow is in per-unit
            let x = b.reactance;
            if x.abs() > 1e-10 {
                let flow_pu = (theta_from - theta_to) / x;
                // WLS SE expects per-unit values (not MW!)
                flows.insert(b.id.value() as i64, flow_pu);
            }
        }
    }

    flows
}

#[allow(clippy::too_many_arguments)]
fn run_single_trial(
    trial: usize,
    network: &Network,
    true_angles: &HashMap<usize, f64>,
    true_injections: &HashMap<usize, f64>,
    branch_flows: &HashMap<i64, f64>,
    config: &Dss2Config,
    seed: u64,
    num_buses: usize,
    num_branches: usize,
) -> Result<Dss2TrialResult> {
    let total_start = Instant::now();

    // Generate measurements with noise
    let meas_start = Instant::now();
    let meas_config = MeasurementGeneratorConfig {
        num_flow_measurements: config.num_flow,
        num_injection_measurements: config.num_injection,
        num_voltage_measurements: 0, // DC SE doesn't use voltage measurements
        noise_std_dev: config.noise_std,
        base_weight: 1.0,
        seed: Some(seed),
    };

    // generate_measurements takes (bus_angles, branch_flows, bus_injections, config)
    let measurements = generate_measurements(true_angles, branch_flows, true_injections, &meas_config);
    let meas_gen_time = meas_start.elapsed().as_secs_f64() * 1000.0;

    // Write measurements to temporary CSV
    let temp_csv = NamedTempFile::new()?;
    {
        let mut csv_file = std::fs::File::create(temp_csv.path())?;
        write_measurements_csv(&measurements, &mut csv_file)?;
    }

    // Run WLS state estimation
    let se_start = Instant::now();
    let solver_kind: SolverKind = "gauss".parse()?;
    let solver = solver_kind.build_solver();

    let temp_out = NamedTempFile::new()?;
    let temp_state = NamedTempFile::new()?;

    let se_result = power_flow::state_estimation_wls(
        network,
        solver.as_ref(),
        temp_csv.path().to_str().unwrap(),
        temp_out.path(),
        &[],
        Some(temp_state.path()),
        None, // auto slack bus
    );

    let se_time = se_start.elapsed().as_secs_f64() * 1000.0;
    let total_time = total_start.elapsed().as_secs_f64() * 1000.0;

    let converged = match &se_result {
        Ok(()) => true,
        Err(e) => {
            eprintln!("Trial {} SE error: {}", trial, e);
            false
        }
    };

    // Compute error metrics by reading estimated angles from state output
    let (mae_deg, rmse_deg, max_error_deg) = if converged {
        compute_error_metrics_from_file(true_angles, temp_state.path())
            .unwrap_or((f64::NAN, f64::NAN, f64::NAN))
    } else {
        (f64::NAN, f64::NAN, f64::NAN)
    };

    Ok(Dss2TrialResult {
        trial,
        seed,
        num_buses,
        num_branches,
        num_measurements: measurements.len(),
        noise_std: config.noise_std,
        load_scale: config.load_scale,
        pf_time_ms: 0.0, // PF only run once at start
        meas_gen_time_ms: meas_gen_time,
        se_time_ms: se_time,
        total_time_ms: total_time,
        mae_deg,
        rmse_deg,
        max_error_deg,
        converged,
    })
}

/// Compute error metrics by reading the estimated state from the parquet file
/// and comparing with the true angles from DC power flow.
fn compute_error_metrics_from_file(
    true_angles: &HashMap<usize, f64>,
    state_file: &Path,
) -> Result<(f64, f64, f64)> {
    use polars::prelude::*;

    // Read the parquet file with estimated angles
    let df = LazyFrame::scan_parquet(state_file, Default::default())?
        .collect()?;

    let bus_ids = df.column("bus_id")?.i64()?;
    let angles = df.column("angle_rad")?.f64()?;

    let mut errors = Vec::new();

    // Compare each estimated angle with the true angle
    for i in 0..bus_ids.len() {
        if let (Some(bus_id), Some(est_angle)) = (bus_ids.get(i), angles.get(i)) {
            let bus_id = bus_id as usize;
            if let Some(&true_angle) = true_angles.get(&bus_id) {
                // Convert error from radians to degrees
                let error_rad = (est_angle - true_angle).abs();
                let error_deg = error_rad.to_degrees();
                errors.push(error_deg);
            }
        }
    }

    if errors.is_empty() {
        return Err(anyhow!("No matching bus angles found"));
    }

    // Calculate MAE (Mean Absolute Error)
    let mae = errors.iter().sum::<f64>() / errors.len() as f64;

    // Calculate RMSE (Root Mean Square Error)
    let mse = errors.iter().map(|e| e * e).sum::<f64>() / errors.len() as f64;
    let rmse = mse.sqrt();

    // Calculate Max Error
    let max_error = errors.iter().cloned().fold(0.0_f64, f64::max);

    Ok((mae, rmse, max_error))
}

fn compute_summary(results: &[Dss2TrialResult]) -> Dss2Summary {
    let total = results.len();
    let converged: Vec<_> = results.iter().filter(|r| r.converged).collect();
    let num_converged = converged.len();

    if converged.is_empty() {
        return Dss2Summary {
            total_trials: total,
            converged_trials: 0,
            convergence_rate: 0.0,
            mean_mae_deg: f64::NAN,
            std_mae_deg: f64::NAN,
            mean_rmse_deg: f64::NAN,
            mean_max_error_deg: f64::NAN,
            median_se_time_ms: f64::NAN,
            mean_se_time_ms: f64::NAN,
            p95_se_time_ms: f64::NAN,
        };
    }

    // MAE statistics
    let maes: Vec<f64> = converged.iter().map(|r| r.mae_deg).collect();
    let mean_mae = maes.iter().sum::<f64>() / maes.len() as f64;
    let variance = maes.iter().map(|m| (m - mean_mae).powi(2)).sum::<f64>() / maes.len() as f64;
    let std_mae = variance.sqrt();

    // RMSE
    let mean_rmse = converged.iter().map(|r| r.rmse_deg).sum::<f64>() / num_converged as f64;

    // Max error
    let mean_max_error = converged.iter().map(|r| r.max_error_deg).sum::<f64>() / num_converged as f64;

    // Timing statistics
    let mut se_times: Vec<f64> = converged.iter().map(|r| r.se_time_ms).collect();
    se_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median_idx = se_times.len() / 2;
    let median_se_time = se_times[median_idx];
    let mean_se_time = se_times.iter().sum::<f64>() / se_times.len() as f64;
    let p95_idx = (se_times.len() as f64 * 0.95) as usize;
    let p95_se_time = se_times[p95_idx.min(se_times.len() - 1)];

    Dss2Summary {
        total_trials: total,
        converged_trials: num_converged,
        convergence_rate: num_converged as f64 / total as f64,
        mean_mae_deg: mean_mae,
        std_mae_deg: std_mae,
        mean_rmse_deg: mean_rmse,
        mean_max_error_deg: mean_max_error,
        median_se_time_ms: median_se_time,
        mean_se_time_ms: mean_se_time,
        p95_se_time_ms: p95_se_time,
    }
}

fn write_results(out: &str, results: &[Dss2TrialResult], summary: &Dss2Summary) -> Result<()> {
    let out_path = Path::new(out);

    // Write trial results
    let file = File::create(out_path).context("Failed to create output file")?;
    let mut writer = Writer::from_writer(BufWriter::new(file));

    for result in results {
        writer.serialize(result)?;
    }
    writer.flush()?;

    // Write summary to companion file
    let summary_path = out_path.with_extension("summary.json");
    let summary_file = File::create(&summary_path).context("Failed to create summary file")?;
    serde_json::to_writer_pretty(BufWriter::new(summary_file), summary)?;

    eprintln!("Results written to: {}", out);
    eprintln!("Summary written to: {}", summary_path.display());

    Ok(())
}

fn print_summary(summary: &Dss2Summary) {
    eprintln!();
    eprintln!("=== DSS² Benchmark Summary ===");
    eprintln!();
    eprintln!("Convergence: {}/{} ({:.1}%)",
        summary.converged_trials,
        summary.total_trials,
        summary.convergence_rate * 100.0
    );
    eprintln!();
    eprintln!("Accuracy (converged trials):");
    eprintln!("  MAE:       {:.4}° ± {:.4}°", summary.mean_mae_deg, summary.std_mae_deg);
    eprintln!("  RMSE:      {:.4}°", summary.mean_rmse_deg);
    eprintln!("  Max Error: {:.4}°", summary.mean_max_error_deg);
    eprintln!();
    eprintln!("Performance:");
    eprintln!("  Median SE time: {:.2} ms", summary.median_se_time_ms);
    eprintln!("  Mean SE time:   {:.2} ms", summary.mean_se_time_ms);
    eprintln!("  P95 SE time:    {:.2} ms", summary.p95_se_time_ms);
    eprintln!();
    eprintln!("Reference: DSS² paper WLS baseline typically achieves:");
    eprintln!("  - MAE < 0.5° with 2% measurement noise");
    eprintln!("  - 100% convergence on feasible cases");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dss2_benchmark_smoke() {
        let config = Dss2Config {
            trials: 5,
            noise_std: 0.02,
            load_scale: 1.0,
            num_flow: 5,
            num_injection: 3,
            base_seed: Some(42),
        };

        let results = run_benchmark(&config).unwrap();
        assert_eq!(results.len(), 5);

        // Verify all trials ran (converged or not)
        // Note: WLS SE may not converge on all synthetic measurement configurations
        // The benchmark infrastructure is working if we get results for all trials
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.trial, i);
            assert!(result.se_time_ms >= 0.0);
            assert!(result.num_measurements > 0);
        }

        // Summary should compute even if no trials converged
        let summary = compute_summary(&results);
        assert_eq!(summary.total_trials, 5);
    }
}
