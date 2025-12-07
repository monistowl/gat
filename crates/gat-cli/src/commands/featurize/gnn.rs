use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::FeaturizeCommands;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;
use gat_cli::common::GnnOutputFormat;
use gat_algo::featurize_gnn::{
    featurize_gnn_with_format, FeaturizeGnnConfig,
    GnnOutputFormat as AlgoGnnOutputFormat,
};

/// Handle `gat featurize gnn` command: export grid topology and flows as GNN-ready features.
///
/// **Purpose:** Converts power grid data into graph-structured features for Graph Neural Networks.
/// This enables ML models (e.g., Power-GNN for state estimation) to consume grid data without
/// worrying about power flow details. See doi:10.1109/TPWRS.2020.3041234 for GNNs in power systems.
///
/// **Output formats:**
/// - `arrow` (default): GAT native Parquet tables with optional partitioning
/// - `neurips-json`: NeurIPS PowerGraph benchmark format (one JSON per graph)
/// - `pytorch-geometric`: PyTorch Geometric compatible format (one JSON per graph)
pub fn handle(command: &FeaturizeCommands) -> Result<()> {
    let FeaturizeCommands::Gnn {
        grid_file,
        flows,
        out,
        format,
        out_partitions,
        group_by_scenario,
        group_by_time,
    } = command
    else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();

    // Convert CLI format enum to algo format enum
    let algo_format = match format {
        GnnOutputFormat::Arrow => AlgoGnnOutputFormat::Arrow,
        GnnOutputFormat::NeuripsJson => AlgoGnnOutputFormat::NeuripsJson,
        GnnOutputFormat::PytorchGeometric => AlgoGnnOutputFormat::PytorchGeometric,
    };

    let res = (|| -> Result<()> {
        // Load base grid topology (buses, branches, generators, loads)
        let network = importers::load_grid_from_arrow(grid_file)?;

        // Configure grouping behavior: each unique (scenario_id, time) becomes a separate graph
        let cfg = FeaturizeGnnConfig {
            group_by_scenario: *group_by_scenario,
            group_by_time: *group_by_time,
            ..Default::default()
        };

        // Export GNN features in requested format
        featurize_gnn_with_format(
            &network,
            Path::new(flows),
            Path::new(out),
            &partitions,
            &cfg,
            algo_format,
        )?;

        Ok(())
    })();

    let format_str = match format {
        GnnOutputFormat::Arrow => "arrow",
        GnnOutputFormat::NeuripsJson => "neurips-json",
        GnnOutputFormat::PytorchGeometric => "pytorch-geometric",
    };

    let params = [
        ("grid_file".to_string(), grid_file.to_string()),
        ("flows".to_string(), flows.to_string()),
        ("out".to_string(), out.to_string()),
        ("format".to_string(), format_str.to_string()),
        (
            "group_by_scenario".to_string(),
            group_by_scenario.to_string(),
        ),
        ("group_by_time".to_string(), group_by_time.to_string()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
    ];
    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "featurize gnn", &param_refs, start, &res);
    res
}
