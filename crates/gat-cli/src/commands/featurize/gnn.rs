use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::FeaturizeCommands;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;
use gat_algo::featurize_gnn::{featurize_gnn_dc, FeaturizeGnnConfig};

/// Handle `gat featurize gnn` command: export grid topology and flows as GNN-ready features.
///
/// **Purpose:** Converts power grid data into graph-structured features for Graph Neural Networks.
/// This enables ML models (e.g., Power-GNN for state estimation) to consume grid data without
/// worrying about power flow details. See doi:10.1109/TPWRS.2020.3041234 for GNNs in power systems.
pub fn handle(command: &FeaturizeCommands) -> Result<()> {
    let FeaturizeCommands::Gnn {
        grid_file,
        flows,
        out,
        out_partitions,
        group_by_scenario,
        group_by_time,
    } = command else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();

    let res = (|| -> Result<()> {
        // Load base grid topology (buses, branches, generators, loads)
        let network = importers::load_grid_from_arrow(grid_file)?;

        // Configure grouping behavior: each unique (scenario_id, time) becomes a separate graph
        let cfg = FeaturizeGnnConfig {
            group_by_scenario: *group_by_scenario,
            group_by_time: *group_by_time,
            ..Default::default()
        };

        // Export GNN features: nodes (buses), edges (branches with flows), graphs (metadata)
        featurize_gnn_dc(
            &network,
            Path::new(flows),
            Path::new(out),
            &partitions,
            &cfg,
        )?;

        Ok(())
    })();

    let mut params = vec![
        ("grid_file".to_string(), grid_file.to_string()),
        ("flows".to_string(), flows.to_string()),
        ("out".to_string(), out.to_string()),
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
