use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use gat_algo::opf::ac_nlp::{solve_ac_opf, AcOpfProblem};
use gat_algo::opf::{
    solve_cascaded, CascadedConfig, CascadedResult, OpfMethod, OpfSolution, OpfSolver,
};
use gat_algo::validation::ObjectiveGap;

#[cfg(feature = "solver-ipopt")]
use gat_algo::opf::ac_nlp::solve_with_ipopt;
use gat_algo::power_flow;
use gat_algo::LpSolverKind;
use gat_cli::cli::OpfCommands;
use gat_core::{Network, Node};
use gat_core::solver::SolverKind;
use gat_io::importers;
use serde::Serialize;

use crate::commands::benchmark::baseline::{load_baseline_objectives, normalize_case_name};
use crate::commands::telemetry::record_run;
use crate::commands::util::{configure_threads, parse_partitions};

// ============================================================================
// JSON Output Types for `opf run`
// ============================================================================

/// Stage result for cascaded solving
#[derive(Debug, Clone, Serialize)]
struct StageResult {
    converged: bool,
    time_ms: f64,
    objective: f64,
    iterations: usize,
}

/// Status block for JSON output
#[derive(Debug, Clone, Serialize)]
struct SolutionStatus {
    converged: bool,
    iterations: usize,
    solve_time_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    stages: Option<HashMap<String, StageResult>>,
}

/// Network summary for JSON output
#[derive(Debug, Clone, Serialize)]
struct NetworkSummary {
    buses: usize,
    branches: usize,
    generators: usize,
}

/// Violation entry for JSON output
#[derive(Debug, Clone, Serialize)]
struct ViolationEntry {
    #[serde(rename = "type")]
    violation_type: String,
    element: String,
    value: f64,
    limit: f64,
}

/// Full JSON output for `opf run`
#[derive(Debug, Clone, Serialize)]
struct OpfRunOutput {
    case_name: String,
    method: String,
    solver: String,
    warm_start: String,

    status: SolutionStatus,

    objective_value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_objective: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    objective_gap_pct: Option<f64>,

    network: NetworkSummary,

    solution: OpfSolution,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    violations: Vec<ViolationEntry>,
}

pub fn handle(command: &OpfCommands) -> Result<()> {
    match command {
        OpfCommands::Dc {
            grid_file,
            cost,
            limits,
            out,
            branch_limits,
            piecewise,
            threads,
            solver,
            lp_solver,
            out_partitions,
        } => {
            configure_threads(threads);
            let solver_kind = solver.parse::<SolverKind>()?;
            let solver_impl = solver_kind.build_solver();
            let lp_solver_kind = lp_solver.parse::<LpSolverKind>()?;
            let partitions = parse_partitions(out_partitions.as_ref());
            let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
            let out_path = Path::new(out);

            let network = importers::load_grid_from_arrow(grid_file.as_str())?;
            power_flow::dc_optimal_power_flow(
                &network,
                solver_impl.as_ref(),
                cost.as_str(),
                limits.as_str(),
                out_path,
                &partitions,
                branch_limits.as_deref(),
                piecewise.as_deref(),
                &lp_solver_kind,
            )?;

            record_run(
                out,
                "opf dc",
                &[
                    ("grid_file", grid_file),
                    ("threads", threads),
                    ("solver", solver_kind.as_str()),
                    ("lp_solver", lp_solver_kind.as_str()),
                    ("out_partitions", partition_spec.as_str()),
                ],
            );
            Ok(())
        }
        OpfCommands::Ac {
            grid_file,
            out,
            tol,
            max_iter,
            threads,
            solver,
            out_partitions,
        } => {
            configure_threads(threads);
            let solver_kind = solver.parse::<SolverKind>()?;
            let solver_impl = solver_kind.build_solver();
            let partitions = parse_partitions(out_partitions.as_ref());
            let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
            let out_path = Path::new(out);

            let network = importers::load_grid_from_arrow(grid_file.as_str())?;
            power_flow::ac_optimal_power_flow(
                &network,
                solver_impl.as_ref(),
                *tol,
                *max_iter,
                out_path,
                &partitions,
            )?;

            record_run(
                out,
                "opf ac",
                &[
                    ("grid_file", grid_file),
                    ("threads", threads),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                    ("solver", solver_kind.as_str()),
                    ("out_partitions", partition_spec.as_str()),
                ],
            );
            Ok(())
        }
        OpfCommands::AcNlp {
            grid_file,
            out,
            tol,
            max_iter,
            warm_start,
            threads,
            solver,
        } => {
            configure_threads(threads);

            // Load network
            let network =
                importers::load_grid_from_arrow(grid_file.as_str()).context("loading grid file")?;

            // Build AC-OPF problem
            let problem = AcOpfProblem::from_network(&network)
                .context("building AC-OPF problem from network")?;

            // Solve using selected solver
            let solution = match solver.as_str() {
                "lbfgs" => solve_ac_opf(&problem, *max_iter as usize, *tol)
                    .context("solving AC-OPF with L-BFGS")?,
                #[cfg(feature = "solver-ipopt")]
                "ipopt" => solve_with_ipopt(&problem, Some(*max_iter as usize), Some(*tol))
                    .context("solving AC-OPF with IPOPT")?,
                #[cfg(not(feature = "solver-ipopt"))]
                "ipopt" => {
                    bail!("IPOPT solver requested but gat-cli was not compiled with solver-ipopt feature. \
                           Rebuild with: cargo build --features solver-ipopt");
                }
                other => {
                    bail!("Unknown solver '{}'. Available: lbfgs, ipopt", other);
                }
            };

            // Output results
            if solution.converged {
                println!(
                    "AC-OPF converged in {} iterations (objective: ${:.2}/hr)",
                    solution.iterations, solution.objective_value
                );

                // Print generator dispatch summary
                println!("\nGenerator Dispatch:");
                for (gen, mw) in &solution.generator_p {
                    let mvar = solution.generator_q.get(gen).unwrap_or(&0.0);
                    println!("  {}: {:.1} MW, {:.1} MVAr", gen, mw, mvar);
                }

                // Print voltage summary
                let v_min = solution
                    .bus_voltage_mag
                    .values()
                    .copied()
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0);
                let v_max = solution
                    .bus_voltage_mag
                    .values()
                    .copied()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0);
                println!("\nVoltage range: {:.4} - {:.4} p.u.", v_min, v_max);

                // Write JSON output
                let json = serde_json::to_string_pretty(&solution)
                    .context("serializing solution to JSON")?;
                let mut file = File::create(out).context("creating output file")?;
                file.write_all(json.as_bytes())
                    .context("writing JSON output")?;

                println!("\nResults written to {}", out);

                record_run(
                    out,
                    "opf ac-nlp",
                    &[
                        ("grid_file", grid_file),
                        ("threads", threads),
                        ("tol", &tol.to_string()),
                        ("max_iter", &max_iter.to_string()),
                        ("warm_start", warm_start),
                        ("solver", solver),
                    ],
                );
            } else {
                println!(
                    "AC-OPF did not converge after {} iterations",
                    solution.iterations
                );
            }

            Ok(())
        }
        OpfCommands::Run {
            input,
            method,
            solver,
            warm_start,
            enhanced,
            tol,
            max_iter,
            timeout,
            baseline,
            output_violations,
            out,
            threads,
        } => {
            configure_threads(threads);
            handle_run(
                input,
                method,
                solver,
                warm_start,
                *enhanced,
                *tol,
                *max_iter,
                *timeout,
                baseline.as_deref(),
                *output_violations,
                out.as_deref(),
            )
        }
    }
}

// ============================================================================
// `opf run` Implementation
// ============================================================================

/// Warm-start strategy for AC-OPF
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WarmStartStrategy {
    Flat,
    Dc,
    Socp,
    Cascaded,
}

impl WarmStartStrategy {
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "flat" => Ok(WarmStartStrategy::Flat),
            "dc" => Ok(WarmStartStrategy::Dc),
            "socp" => Ok(WarmStartStrategy::Socp),
            "cascaded" => Ok(WarmStartStrategy::Cascaded),
            other => bail!(
                "Unknown warm-start strategy '{}'. Options: flat, dc, socp, cascaded",
                other
            ),
        }
    }
}

/// Load MATPOWER network from file or directory
fn load_matpower_network(input: &str) -> Result<(Network, String)> {
    let path = Path::new(input);

    if path.is_dir() {
        // Find .m file in directory
        let m_file = std::fs::read_dir(path)
            .with_context(|| format!("reading directory: {}", input))?
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().map(|ext| ext == "m").unwrap_or(false))
            .ok_or_else(|| anyhow::anyhow!("No .m file found in directory: {}", input))?;

        let case_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let network = importers::load_matpower_network(&m_file.path())
            .with_context(|| format!("loading MATPOWER file: {}", m_file.path().display()))?;

        Ok((network, case_name))
    } else {
        // Direct .m file
        let case_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let network = importers::load_matpower_network(path)
            .with_context(|| format!("loading MATPOWER file: {}", input))?;

        Ok((network, case_name))
    }
}

/// Count network elements
fn count_network_elements(network: &Network) -> NetworkSummary {
    let mut buses = 0;
    let mut generators = 0;

    for node in network.graph.node_weights() {
        match node {
            Node::Bus(_) => buses += 1,
            Node::Gen(_) => generators += 1,
            _ => {}
        }
    }

    NetworkSummary {
        buses,
        branches: network.graph.edge_count(),
        generators,
    }
}

/// Collect detailed violations from network and solution
fn collect_violations(network: &Network, solution: &OpfSolution) -> Vec<ViolationEntry> {
    let mut violations = Vec::new();

    // Check generator P violations
    for node in network.graph.node_weights() {
        if let Node::Gen(gen) = node {
            if !gen.status {
                continue;
            }
            if let Some(&pg) = solution.generator_p.get(&gen.name) {
                if gen.pmax_mw.is_finite() && pg > gen.pmax_mw + 0.01 {
                    violations.push(ViolationEntry {
                        violation_type: "generator_pmax".to_string(),
                        element: gen.name.clone(),
                        value: pg,
                        limit: gen.pmax_mw,
                    });
                }
                if pg < gen.pmin_mw - 0.01 {
                    violations.push(ViolationEntry {
                        violation_type: "generator_pmin".to_string(),
                        element: gen.name.clone(),
                        value: pg,
                        limit: gen.pmin_mw,
                    });
                }
            }
        }
    }

    // Check voltage violations
    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            if let Some(&vm) = solution.bus_voltage_mag.get(&bus.name) {
                if let Some(vmax) = bus.vmax_pu {
                    if vm > vmax + 0.001 {
                        violations.push(ViolationEntry {
                            violation_type: "voltage_max".to_string(),
                            element: bus.name.clone(),
                            value: vm,
                            limit: vmax,
                        });
                    }
                }
                if let Some(vmin) = bus.vmin_pu {
                    if vm < vmin - 0.001 {
                        violations.push(ViolationEntry {
                            violation_type: "voltage_min".to_string(),
                            element: bus.name.clone(),
                            value: vm,
                            limit: vmin,
                        });
                    }
                }
            }
        }
    }

    // Check branch flow violations
    for edge in network.graph.edge_weights() {
        use gat_core::Edge;
        if let Edge::Branch(branch) = edge {
            let p_flow = solution
                .branch_p_flow
                .get(&branch.name)
                .copied()
                .unwrap_or(0.0);
            let q_flow = solution
                .branch_q_flow
                .get(&branch.name)
                .copied()
                .unwrap_or(0.0);
            let s_flow = (p_flow.powi(2) + q_flow.powi(2)).sqrt();

            let s_limit = branch.s_max_mva.or(branch.rating_a_mva);
            if let Some(s_max) = s_limit {
                if s_max > 0.0 && s_flow > s_max + 0.1 {
                    violations.push(ViolationEntry {
                        violation_type: "branch_flow".to_string(),
                        element: branch.name.clone(),
                        value: s_flow,
                        limit: s_max,
                    });
                }
            }
        }
    }

    violations
}

/// Print rich console output
fn print_console_output(
    case_name: &str,
    network_summary: &NetworkSummary,
    method: &str,
    solver: &str,
    warm_start: &str,
    cascaded_result: Option<&CascadedResult>,
    solution: &OpfSolution,
    baseline_gap: Option<&ObjectiveGap>,
    violations: &[ViolationEntry],
    output_violations: bool,
) {
    // Header box
    eprintln!();
    eprintln!("╭─ {} ─────────────────────────────────────────╮", case_name);
    eprintln!(
        "│  Network:  {} buses, {} branches, {} generators",
        network_summary.buses, network_summary.branches, network_summary.generators
    );
    let method_display = match method {
        "ac" => format!("AC-OPF ({}) with {} warm-start", solver.to_uppercase(), warm_start),
        "socp" => "SOCP Relaxation".to_string(),
        "dc" => "DC-OPF".to_string(),
        "economic" => "Economic Dispatch".to_string(),
        _ => method.to_string(),
    };
    eprintln!("│  Method:   {}", method_display);
    eprintln!("╰──────────────────────────────────────────────────────────────╯");
    eprintln!();

    // Stage results (if cascaded)
    if let Some(result) = cascaded_result {
        if let Some(dc) = &result.dc_solution {
            let status = if dc.converged { "✓ converged" } else { "✗ failed" };
            eprintln!(
                "  DC stage:    {:12}  {:>7.1} ms     obj: ${:.2}/hr",
                status, dc.solve_time_ms as f64, dc.objective_value
            );
        }
        if let Some(socp) = &result.socp_solution {
            let status = if socp.converged { "✓ converged" } else { "✗ failed" };
            eprintln!(
                "  SOCP stage:  {:12}  {:>7.1} ms     obj: ${:.2}/hr",
                status, socp.solve_time_ms as f64, socp.objective_value
            );
        }
        if let Some(ac) = &result.ac_solution {
            let status = if ac.converged { "✓ converged" } else { "✗ failed" };
            eprintln!(
                "  AC stage:    {:12}  {:>7.1} ms     obj: ${:.2}/hr ({} iters)",
                status, ac.solve_time_ms as f64, ac.objective_value, ac.iterations
            );
        }
        eprintln!();
    } else {
        // Single-stage result
        let status = if solution.converged { "✓ converged" } else { "✗ failed" };
        eprintln!(
            "  Result: {} in {:.1} ms ({} iterations)",
            status, solution.solve_time_ms as f64, solution.iterations
        );
        eprintln!("  Objective: ${:.2}/hr", solution.objective_value);
        eprintln!();
    }

    // Baseline comparison
    if let Some(gap) = baseline_gap {
        let sign = if gap.gap_rel >= 0.0 { "+" } else { "" };
        eprintln!(
            "  Baseline comparison: ${:.2} → gap: {}{:.2}%",
            gap.ref_objective,
            sign,
            gap.gap_rel * 100.0
        );
        eprintln!();
    }

    // Voltage range
    if !solution.bus_voltage_mag.is_empty() {
        let v_min = solution
            .bus_voltage_mag
            .values()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        let v_max = solution
            .bus_voltage_mag
            .values()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        eprintln!("  Voltage range:    {:.4} – {:.4} p.u.", v_min, v_max);
    }

    // Branch loading
    if !solution.branch_p_flow.is_empty() {
        // Find max branch loading (would need limits from network for %)
        let max_flow = solution
            .branch_p_flow
            .values()
            .map(|v| v.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        eprintln!("  Max branch flow:  {:.1} MW", max_flow);
    }

    eprintln!();

    // Violations
    if output_violations && !violations.is_empty() {
        eprintln!("  Violations ({}):", violations.len());
        for v in violations.iter().take(10) {
            let pct = if v.limit.abs() > 0.001 {
                ((v.value - v.limit) / v.limit * 100.0).abs()
            } else {
                0.0
            };
            eprintln!(
                "    • {}: {} {:.2} vs limit {:.2} (+{:.1}%)",
                v.element, v.violation_type, v.value, v.limit, pct
            );
        }
        if violations.len() > 10 {
            eprintln!("    ... and {} more", violations.len() - 10);
        }
        eprintln!();
    }

    eprintln!(
        "  Total time: {:.1} ms",
        cascaded_result
            .map(|r| r.total_time_ms as f64)
            .unwrap_or(solution.solve_time_ms as f64)
    );
    eprintln!();
}

#[allow(clippy::too_many_arguments)]
fn handle_run(
    input: &str,
    method: &str,
    solver: &str,
    warm_start: &str,
    enhanced: bool,
    tol: f64,
    max_iter: u32,
    timeout: u64,
    baseline: Option<&str>,
    output_violations: bool,
    out: Option<&str>,
) -> Result<()> {
    // Parse method
    let opf_method = method
        .parse::<OpfMethod>()
        .map_err(|e| anyhow::anyhow!("Invalid OPF method '{}': {}", method, e))?;

    // Parse warm-start
    let warm_start_strategy = WarmStartStrategy::from_str(warm_start)?;

    // Warn if warm-start specified for non-AC method
    if opf_method != OpfMethod::AcOpf && warm_start_strategy != WarmStartStrategy::Flat {
        eprintln!(
            "Warning: --warm-start '{}' is ignored for method '{}' (only applies to AC-OPF)",
            warm_start, method
        );
    }

    // Load network
    let (network, case_name) = load_matpower_network(input)?;
    let network_summary = count_network_elements(&network);

    // Load baseline if provided
    let baseline_map: HashMap<String, f64> = if let Some(baseline_path) = baseline {
        load_baseline_objectives(Path::new(baseline_path))?
    } else {
        HashMap::new()
    };

    // Solve based on method and warm-start
    let start = Instant::now();
    let (solution, cascaded_result): (OpfSolution, Option<CascadedResult>) =
        if opf_method == OpfMethod::AcOpf && warm_start_strategy == WarmStartStrategy::Cascaded {
            // Use cascaded solver
            let config = CascadedConfig {
                max_iterations: max_iter as usize,
                tolerance: tol,
                use_loss_factors: true,
                prefer_native: solver == "ipopt",
                use_enhanced_socp: enhanced,
                timeout_seconds: timeout,
            };
            let result = solve_cascaded(&network, OpfMethod::AcOpf, &config)
                .context("cascaded OPF solve failed")?;
            let sol = result.final_solution.clone();
            (sol, Some(result))
        } else {
            // Use standard solver
            let mut opf_solver = OpfSolver::new()
                .with_method(opf_method)
                .with_max_iterations(max_iter as usize)
                .with_tolerance(tol)
                .with_timeout(timeout)
                .enhanced_socp(enhanced);

            // Configure native solver preference for AC
            if opf_method == OpfMethod::AcOpf {
                opf_solver = opf_solver.prefer_native(solver == "ipopt");
            }

            let sol = opf_solver.solve(&network).context("OPF solve failed")?;
            (sol, None)
        };
    let _solve_time = start.elapsed();

    // Compute violations
    let violations = if output_violations || out.is_some() {
        collect_violations(&network, &solution)
    } else {
        Vec::new()
    };

    // Compute baseline gap
    let normalized_name = normalize_case_name(&case_name);
    let baseline_objective = baseline_map
        .get(&normalized_name)
        .or_else(|| baseline_map.get(&case_name))
        .copied();
    let baseline_gap = baseline_objective.map(|ref_obj| ObjectiveGap::new(solution.objective_value, ref_obj));

    // Print console output
    print_console_output(
        &case_name,
        &network_summary,
        method,
        solver,
        warm_start,
        cascaded_result.as_ref(),
        &solution,
        baseline_gap.as_ref(),
        &violations,
        output_violations,
    );

    // Write JSON output if requested
    if let Some(out_path) = out {
        let mut stages = None;
        if let Some(ref result) = cascaded_result {
            let mut stage_map = HashMap::new();
            if let Some(dc) = &result.dc_solution {
                stage_map.insert(
                    "dc".to_string(),
                    StageResult {
                        converged: dc.converged,
                        time_ms: dc.solve_time_ms as f64,
                        objective: dc.objective_value,
                        iterations: dc.iterations,
                    },
                );
            }
            if let Some(socp) = &result.socp_solution {
                stage_map.insert(
                    "socp".to_string(),
                    StageResult {
                        converged: socp.converged,
                        time_ms: socp.solve_time_ms as f64,
                        objective: socp.objective_value,
                        iterations: socp.iterations,
                    },
                );
            }
            if let Some(ac) = &result.ac_solution {
                stage_map.insert(
                    "ac".to_string(),
                    StageResult {
                        converged: ac.converged,
                        time_ms: ac.solve_time_ms as f64,
                        objective: ac.objective_value,
                        iterations: ac.iterations,
                    },
                );
            }
            if !stage_map.is_empty() {
                stages = Some(stage_map);
            }
        }

        let output = OpfRunOutput {
            case_name: case_name.clone(),
            method: method.to_string(),
            solver: solver.to_string(),
            warm_start: warm_start.to_string(),
            status: SolutionStatus {
                converged: solution.converged,
                iterations: solution.iterations,
                solve_time_ms: cascaded_result
                    .as_ref()
                    .map(|r| r.total_time_ms as f64)
                    .unwrap_or(solution.solve_time_ms as f64),
                stages,
            },
            objective_value: solution.objective_value,
            baseline_objective,
            objective_gap_pct: baseline_gap.as_ref().map(|g| g.gap_rel * 100.0),
            network: network_summary,
            solution,
            violations,
        };

        let json = serde_json::to_string_pretty(&output).context("serializing solution to JSON")?;
        let mut file = File::create(out_path).context("creating output file")?;
        file.write_all(json.as_bytes())
            .context("writing JSON output")?;
        eprintln!("  Results written to {}", out_path);
    }

    Ok(())
}
