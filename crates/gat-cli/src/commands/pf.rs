use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};
use anyhow::Result;
use gat_algo::power_flow::{self, AcPowerFlowSolver, CpfSolver, FastDecoupledSolver};
use gat_cli::cli::PowerFlowCommands;
use gat_cli::common::{write_json, write_jsonl, FileOutputFormat, OutputDest, OutputFormat};
use gat_core::solver::SolverKind;
use gat_core::BusId;
use gat_io::importers;

pub fn handle(command: &PowerFlowCommands) -> Result<()> {
    match command {
        PowerFlowCommands::Dc {
            grid_file,
            out,
            threads,
            solver,
            lp_solver: _, // unused in DC power flow
            out_partitions,
            stdout_format,
            output_format,
            slack_bus: _, // TODO: wire into solver
        } => {
            let start = Instant::now();
            configure_threads(threads);
            let solver_kind = solver.parse::<SolverKind>()?;
            let solver_impl = solver_kind.build_solver();
            let partitions = parse_partitions(out_partitions.as_ref());
            let output_dest = OutputDest::parse(out);

            let network = importers::load_grid_from_arrow(grid_file.as_str())?;

            let res = match output_dest {
                OutputDest::File(path) => {
                    // Get DataFrame from DC power flow
                    let (df, _max_flow, _min_flow) =
                        power_flow::dc_power_flow_dataframe(&network, solver_impl.as_ref())?;

                    // Write to file based on output format
                    write_dataframe_to_file(&df, &path, *output_format, &partitions)
                }
                OutputDest::Stdout => {
                    // Compute the DC power flow and get the DataFrame
                    let (df, max_flow, min_flow) =
                        power_flow::dc_power_flow_dataframe(&network, solver_impl.as_ref())?;

                    // Print summary to stderr so stdout remains clean for piping
                    eprintln!(
                        "DC power flow summary: {} branch(es), flow range [{:.3}, {:.3}] MW",
                        df.height(),
                        min_flow,
                        max_flow
                    );

                    // Convert DataFrame to JSON and write to stdout
                    dataframe_to_stdout(&df, *stdout_format)
                }
            };

            record_run_timed(
                out,
                "pf dc",
                &[
                    ("grid_file", grid_file),
                    ("out", out),
                    ("threads", threads),
                    ("solver", solver_kind.as_str()),
                    ("out_partitions", out_partitions.as_deref().unwrap_or("")),
                ],
                start,
                &res,
            );
            res
        }
        PowerFlowCommands::Ac {
            grid_file,
            out,
            tol,
            max_iter,
            threads,
            solver,
            lp_solver: _, // unused in AC power flow
            out_partitions,
            q_limits,
            slack_bus: _,       // TODO: wire into solver
            show_iterations: _, // TODO: wire into solver
        } => {
            let start = Instant::now();
            configure_threads(threads);
            let solver_kind = solver.parse::<SolverKind>()?;
            let solver_impl = solver_kind.build_solver();
            let partitions = parse_partitions(out_partitions.as_ref());
            let out_path = Path::new(out);

            let network = importers::load_grid_from_arrow(grid_file.as_str())?;

            let res = if *q_limits {
                // Use new Newton-Raphson solver with Q-limit enforcement
                let pf_solver = AcPowerFlowSolver::new()
                    .with_tolerance(*tol)
                    .with_max_iterations(*max_iter as usize)
                    .with_q_limit_enforcement(true);

                let solution = pf_solver.solve(&network)?;

                // Write results to output file
                power_flow::write_ac_pf_solution(&network, &solution, out_path, &partitions)?;

                if solution.converged {
                    tracing::info!(
                        "AC power flow converged in {} iterations (max mismatch: {:.2e})",
                        solution.iterations,
                        solution.max_mismatch
                    );
                }
                Ok(())
            } else {
                // Use legacy solver without Q-limit enforcement
                power_flow::ac_power_flow(
                    &network,
                    solver_impl.as_ref(),
                    *tol,
                    *max_iter,
                    out_path,
                    &partitions,
                )
            };

            let q_limits_str = if *q_limits { "true" } else { "false" };
            record_run_timed(
                out,
                "pf ac",
                &[
                    ("grid_file", grid_file),
                    ("threads", threads),
                    ("out", out),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                    ("solver", solver_kind.as_str()),
                    ("out_partitions", out_partitions.as_deref().unwrap_or("")),
                    ("q_limits", q_limits_str),
                ],
                start,
                &res,
            );
            res
        }
        PowerFlowCommands::Fdpf {
            grid_file,
            out,
            tol,
            max_iter,
            threads,
            out_partitions,
        } => {
            let start = Instant::now();
            configure_threads(threads);
            let partitions = parse_partitions(out_partitions.as_ref());
            let out_path = Path::new(out);

            let network = importers::load_grid_from_arrow(grid_file.as_str())?;

            // Configure and run Fast-Decoupled solver
            let solver = FastDecoupledSolver::new()
                .with_tolerance(*tol)
                .with_max_iterations(*max_iter as usize);

            let solution = solver.solve(&network)?;

            // Write results to output file
            let res = power_flow::write_fdpf_solution(&network, &solution, out_path, &partitions);

            if solution.converged {
                tracing::info!(
                    "Fast-Decoupled PF converged in {} iterations (max mismatch: {:.2e})",
                    solution.iterations,
                    solution.max_mismatch
                );
            } else {
                tracing::warn!(
                    "Fast-Decoupled PF did not converge after {} iterations (max mismatch: {:.2e})",
                    solution.iterations,
                    solution.max_mismatch
                );
            }

            record_run_timed(
                out,
                "pf fdpf",
                &[
                    ("grid_file", grid_file),
                    ("threads", threads),
                    ("out", out),
                    ("tol", &tol.to_string()),
                    ("max_iter", &max_iter.to_string()),
                    ("out_partitions", out_partitions.as_deref().unwrap_or("")),
                ],
                start,
                &res,
            );
            res
        }
        PowerFlowCommands::Cpf {
            grid_file,
            out,
            step_size,
            tol,
            max_steps,
            target_bus,
            threads,
        } => {
            let start = Instant::now();
            configure_threads(threads);

            let mut network = importers::load_grid_from_arrow(grid_file.as_str())?;

            // Configure CPF solver
            let mut solver = CpfSolver::new()
                .with_step_size(*step_size)
                .with_tolerance(*tol);

            // Set max_steps
            solver.max_steps = *max_steps;

            // Set target bus if specified
            if let Some(bus_id) = target_bus {
                solver = solver.with_target_bus(BusId::new(*bus_id as usize));
            }

            // Run CPF analysis
            let result = solver.solve(&mut network)?;

            // Write results to JSON
            let res = write_cpf_result(&result, out);

            if result.converged {
                tracing::info!(
                    "CPF converged: max loading = {:.3} (margin = {:.1}%)",
                    result.max_loading,
                    result.loading_margin * 100.0
                );
                if let Some(critical) = result.critical_bus {
                    tracing::info!("Critical bus: {}", critical.value());
                }
            } else {
                tracing::warn!("CPF did not converge after {} steps", result.steps);
            }

            record_run_timed(
                out,
                "pf cpf",
                &[
                    ("grid_file", grid_file),
                    ("threads", threads),
                    ("out", out),
                    ("step_size", &step_size.to_string()),
                    ("tol", &tol.to_string()),
                    ("max_steps", &max_steps.to_string()),
                    (
                        "target_bus",
                        &target_bus.map(|b| b.to_string()).unwrap_or_default(),
                    ),
                ],
                start,
                &res,
            );
            res
        }
    }
}

/// Write a Polars DataFrame to a file in the specified format
fn write_dataframe_to_file(
    df: &polars::prelude::DataFrame,
    path: &std::path::PathBuf,
    format: FileOutputFormat,
    partitions: &[String],
) -> Result<()> {
    use gat_algo::io::persist_dataframe;
    use gat_algo::OutputStage;
    use polars::prelude::*;

    match format {
        FileOutputFormat::Parquet => {
            // Write to Parquet using existing infrastructure
            let mut df_mut = df.clone();
            persist_dataframe(&mut df_mut, path, partitions, OutputStage::PfDc.as_str())?;
            println!(
                "DC power flow summary: {} branch(es), persisted to {}",
                df.height(),
                path.display()
            );
            Ok(())
        }
        FileOutputFormat::Json => {
            // Convert DataFrame to JSON
            let column_names: Vec<_> = df.get_column_names();
            let mut json_objects = Vec::new();

            for row_idx in 0..df.height() {
                let mut obj = serde_json::Map::new();
                for (col_idx, &col_name) in column_names.iter().enumerate() {
                    let series = &df.get_columns()[col_idx];
                    let value = match series.dtype() {
                        DataType::Int64 => {
                            let val = series.i64()?.get(row_idx);
                            val.map(|v| serde_json::Value::Number(v.into()))
                                .unwrap_or(serde_json::Value::Null)
                        }
                        DataType::Float64 => {
                            let val = series.f64()?.get(row_idx);
                            val.and_then(serde_json::Number::from_f64)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        }
                        DataType::Utf8 => {
                            let val = series.utf8()?.get(row_idx);
                            val.map(|s| serde_json::Value::String(s.to_string()))
                                .unwrap_or(serde_json::Value::Null)
                        }
                        _ => serde_json::Value::Null,
                    };
                    obj.insert(col_name.to_string(), value);
                }
                json_objects.push(serde_json::Value::Object(obj));
            }

            // Write JSON to file
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, &json_objects)?;
            println!(
                "DC power flow summary: {} branch(es), persisted to {}",
                df.height(),
                path.display()
            );
            Ok(())
        }
        FileOutputFormat::Csv => {
            // Write CSV to file
            let file = File::create(path)?;
            let mut writer = CsvWriter::new(file);
            writer
                .finish(&mut df.clone())
                .map_err(|e| anyhow::anyhow!("Failed to write CSV: {}", e))?;
            println!(
                "DC power flow summary: {} branch(es), persisted to {}",
                df.height(),
                path.display()
            );
            Ok(())
        }
    }
}

/// Convert a Polars DataFrame to JSON/JSONL/CSV and write to stdout
fn dataframe_to_stdout(df: &polars::prelude::DataFrame, format: OutputFormat) -> Result<()> {
    use polars::prelude::*;
    use std::io;

    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            // Convert DataFrame rows to JSON objects manually
            let column_names: Vec<_> = df.get_column_names();
            let mut json_objects = Vec::new();

            for row_idx in 0..df.height() {
                let mut obj = serde_json::Map::new();
                for (col_idx, &col_name) in column_names.iter().enumerate() {
                    let series = &df.get_columns()[col_idx];
                    let value = match series.dtype() {
                        DataType::Int64 => {
                            let val = series.i64()?.get(row_idx);
                            val.map(|v| serde_json::Value::Number(v.into()))
                                .unwrap_or(serde_json::Value::Null)
                        }
                        DataType::Float64 => {
                            let val = series.f64()?.get(row_idx);
                            val.and_then(serde_json::Number::from_f64)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        }
                        DataType::Utf8 => {
                            let val = series.utf8()?.get(row_idx);
                            val.map(|s| serde_json::Value::String(s.to_string()))
                                .unwrap_or(serde_json::Value::Null)
                        }
                        _ => serde_json::Value::Null,
                    };
                    obj.insert(col_name.to_string(), value);
                }
                json_objects.push(serde_json::Value::Object(obj));
            }

            if format == OutputFormat::Json {
                write_json(&json_objects, &mut io::stdout(), true)?;
            } else {
                write_jsonl(&json_objects, &mut io::stdout())?;
            }
            Ok(())
        }
        OutputFormat::Csv => {
            // Write CSV to stdout
            CsvWriter::new(io::stdout())
                .finish(&mut df.clone())
                .map_err(|e| anyhow::anyhow!("Failed to write CSV: {}", e))?;
            Ok(())
        }
        OutputFormat::Table => {
            // For table format, just print the DataFrame (Polars has nice built-in formatting)
            println!("{}", df);
            Ok(())
        }
    }
}

/// Write CPF result to JSON file
fn write_cpf_result(result: &gat_algo::power_flow::CpfResult, path: &str) -> Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct CpfOutputPoint {
        loading: f64,
        voltage: f64,
    }

    #[derive(Serialize)]
    struct CpfOutput {
        converged: bool,
        max_loading: f64,
        loading_margin: f64,
        loading_margin_percent: f64,
        critical_bus: Option<usize>,
        steps: usize,
        nose_curve: Vec<CpfOutputPoint>,
        voltages_at_max: std::collections::HashMap<usize, f64>,
    }

    let output = CpfOutput {
        converged: result.converged,
        max_loading: result.max_loading,
        loading_margin: result.loading_margin,
        loading_margin_percent: result.loading_margin * 100.0,
        critical_bus: result.critical_bus.map(|b| b.value()),
        steps: result.steps,
        nose_curve: result
            .nose_curve
            .iter()
            .map(|p| CpfOutputPoint {
                loading: p.loading,
                voltage: p.voltage,
            })
            .collect(),
        voltages_at_max: result
            .voltage_at_max
            .iter()
            .map(|(k, v)| (k.value(), *v))
            .collect(),
    };

    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &output)?;
    Ok(())
}
