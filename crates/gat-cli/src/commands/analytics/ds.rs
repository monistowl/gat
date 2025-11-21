use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::AnalyticsCommands;
use gat_core::solver::SolverKind;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::{configure_threads, parse_partitions};

pub fn handle(command: &AnalyticsCommands) -> Result<()> {
    let AnalyticsCommands::Ds {
        grid_file,
        limits,
        branch_limits,
        flows,
        out,
        solver,
        sink_bus,
        threads,
        out_partitions,
    } = command else {
        unreachable!();
    };
    configure_threads(threads);
    let solver_kind = solver.parse::<SolverKind>()?;
    let solver_impl = solver_kind.build_solver();
    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();
    let mut summary = None;
    let res = (|| -> Result<()> {
        let network = importers::load_grid_from_arrow(grid_file)?;
        let result = gat_algo::deliverability_scores_dc(
            &network,
            solver_impl.as_ref(),
            limits,
            branch_limits,
            Path::new(flows),
            Path::new(out),
            &partitions,
            *sink_bus,
        )?;
        summary = Some(result);
        Ok(())
    })();
    
    // Build params list, including summary stats if available
    let mut params = vec![
        ("grid_file".to_string(), grid_file.to_string()),
        ("limits".to_string(), limits.to_string()),
        ("branch_limits".to_string(), branch_limits.to_string()),
        ("flows".to_string(), flows.to_string()),
        ("solver".to_string(), solver_kind.as_str().to_string()),
        ("sink_bus".to_string(), sink_bus.to_string()),
        ("threads".to_string(), threads.to_string()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
    ];
    if let Some(ref s) = summary {
        // Print summary and add to params (using ref to avoid move)
        println!(
            "DS results: {} buses × {} cases written to {}",
            s.num_buses,
            s.num_cases,
            out
        );
        params.push(("num_buses".to_string(), s.num_buses.to_string()));
        params.push(("num_cases".to_string(), s.num_cases.to_string()));
    }
    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "analytics ds", &param_refs, start, &res);
    res
}
