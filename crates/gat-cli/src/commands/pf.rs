use std::path::Path;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};
use anyhow::Result;
use gat_algo::power_flow::{self, AcPowerFlowSolver};
use gat_cli::cli::PowerFlowCommands;
use gat_cli::common::{write_json, write_jsonl, OutputDest, OutputFormat};
use gat_core::solver::SolverKind;
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
                    // Existing Parquet write logic
                    power_flow::dc_power_flow(&network, solver_impl.as_ref(), &path, &partitions)
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
            slack_bus: _, // TODO: wire into solver
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
                            val.and_then(|v| serde_json::Number::from_f64(v))
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
