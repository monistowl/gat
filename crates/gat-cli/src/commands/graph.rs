use std::fs;
use std::time::Instant;

use anyhow::{bail, Result};
use gat_cli::cli::GraphCommands;
use gat_core::graph_utils;
use gat_io::helpers::{validate_network, ImportDiagnostics, Severity, ValidationConfig};
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;

#[cfg(feature = "viz")]
use gat_viz::layout::layout_network;
#[cfg(feature = "viz")]
use serde_json;

pub fn handle(command: &GraphCommands) -> Result<()> {
    match command {
        GraphCommands::Stats { grid_file } => {
            let start = Instant::now();
            let result = (|| -> Result<()> {
                let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                let stats = graph_utils::graph_stats(&network)?;
                println!("Graph statistics for {grid_file}:");
                println!("  Nodes         : {}", stats.node_count);
                println!("  Edges         : {}", stats.edge_count);
                println!("  Components    : {}", stats.connected_components);
                println!(
                    "  Degree [min/avg/max]: {}/{:.2}/{}",
                    stats.min_degree, stats.avg_degree, stats.max_degree
                );
                println!("  Density       : {:.4}", stats.density);
                Ok(())
            })();
            record_run_timed(
                grid_file,
                "graph stats",
                &[("grid_file", grid_file)],
                start,
                &result,
            );
            result
        }
        GraphCommands::Islands { grid_file, emit } => {
            let start = Instant::now();
            let result = (|| -> Result<()> {
                let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                let analysis = graph_utils::find_islands(&network)?;
                for summary in &analysis.islands {
                    println!(
                        "Island {}: {} node(s)",
                        summary.island_id, summary.node_count
                    );
                }
                if *emit {
                    println!("\nNode → Island assignments:");
                    for assignment in &analysis.assignments {
                        println!(
                            "  idx {:>3}: {:<20} -> island {}",
                            assignment.node_index, assignment.label, assignment.island_id
                        );
                    }
                }
                Ok(())
            })();
            record_run_timed(
                grid_file,
                "graph islands",
                &[("grid_file", grid_file), ("emit", &emit.to_string())],
                start,
                &result,
            );
            result
        }
        GraphCommands::Validate {
            grid_file,
            strict,
            skip_topology,
            verbose,
        } => {
            let start = Instant::now();
            let result = (|| -> Result<()> {
                let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                let stats = network.stats();

                let config = ValidationConfig {
                    strict: *strict,
                    skip_topology: *skip_topology,
                    ..Default::default()
                };

                // Run validation, collecting issues into diagnostics
                let mut diag = ImportDiagnostics::new();
                validate_network(&network, &mut diag, &config);

                // Print summary
                println!("Validation results for {grid_file}:");
                println!(
                    "  Network: {} buses, {} branches, {} generators, {} loads",
                    stats.num_buses, stats.num_branches, stats.num_gens, stats.num_loads
                );
                println!();

                // Print issues
                let warning_count = diag.warning_count();
                let error_count = diag.error_count();

                if warning_count == 0 && error_count == 0 {
                    println!("✓ No issues found");
                } else {
                    if *verbose > 0 {
                        // Show all issues with details
                        for issue in &diag.issues {
                            let marker = if issue.severity == Severity::Error {
                                "✗"
                            } else {
                                "⚠"
                            };
                            match &issue.entity {
                                Some(entity) => {
                                    println!("  {} {}: {}", marker, entity, issue.message);
                                }
                                None => {
                                    println!("  {} {}: {}", marker, issue.category, issue.message);
                                }
                            }
                        }
                        println!();
                    }

                    println!(
                        "Summary: {} warning(s), {} error(s){}",
                        warning_count,
                        error_count,
                        if *verbose == 0 { " (use -v for details)" } else { "" }
                    );

                    if *strict && (warning_count > 0 || error_count > 0) {
                        bail!(
                            "Validation failed in strict mode ({} warning(s), {} error(s))",
                            warning_count,
                            error_count
                        );
                    }
                }

                Ok(())
            })();
            record_run_timed(
                grid_file,
                "graph validate",
                &[
                    ("grid_file", grid_file),
                    ("strict", &strict.to_string()),
                    ("skip_topology", &skip_topology.to_string()),
                ],
                start,
                &result,
            );
            result
        }
        GraphCommands::Export {
            grid_file,
            format,
            out,
        } => {
            let start = Instant::now();
            let result = (|| -> Result<()> {
                let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                let dot = graph_utils::export_graph(&network, format)?;
                if let Some(path) = out {
                    fs::write(path, &dot)?;
                    println!("Graph exported to {path}");
                } else {
                    println!("{dot}");
                }
                Ok(())
            })();
            record_run_timed(
                grid_file,
                "graph export",
                &[("grid_file", grid_file), ("format", format)],
                start,
                &result,
            );
            result
        }
        #[cfg(feature = "viz")]
        GraphCommands::Visualize {
            grid_file,
            iterations,
            out,
        } => {
            let start = Instant::now();
            let result = (|| -> Result<()> {
                let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                let layout = layout_network(&network, *iterations);
                let payload =
                    serde_json::to_string_pretty(&layout).map_err(|err| anyhow::anyhow!(err))?;
                if let Some(path) = out {
                    fs::write(path, &payload)?;
                    println!("Layout written to {path}");
                } else {
                    println!("{payload}");
                }
                Ok(())
            })();
            record_run_timed(
                grid_file,
                "graph visualize",
                &[
                    ("grid_file", grid_file),
                    ("iterations", &iterations.to_string()),
                ],
                start,
                &result,
            );
            result
        }
    }
}
