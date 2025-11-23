use anyhow::Result;
use gat_algo::elcc::elcc_estimation;
use gat_cli::cli::AnalyticsCommands;
use std::path::Path;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;

pub fn handle(command: &AnalyticsCommands) -> Result<()> {
    let AnalyticsCommands::Elcc {
        resource_profiles,
        reliability_metrics,
        out,
        out_partitions,
        max_jobs,
    } = command
    else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();
    let mut summary = None;

    let res = (|| -> Result<()> {
        let result = elcc_estimation(
            Path::new(resource_profiles),
            Path::new(reliability_metrics),
            Path::new(out),
            &partitions,
            *max_jobs,
        )?;
        summary = Some(result);
        Ok(())
    })();

    let mut params = vec![
        (
            "resource_profiles".to_string(),
            resource_profiles.to_string(),
        ),
        (
            "reliability_metrics".to_string(),
            reliability_metrics.to_string(),
        ),
        ("out".to_string(), out.to_string()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
        ("max_jobs".to_string(), max_jobs.to_string()),
    ];
    if let Some(ref s) = summary {
        println!(
            "ELCC results: {} resource classes, {} output rows written to {}",
            s.num_resource_classes, s.num_output_rows, out
        );
        params.push((
            "num_resource_classes".to_string(),
            s.num_resource_classes.to_string(),
        ));
        params.push(("num_output_rows".to_string(), s.num_output_rows.to_string()));
    }

    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "analytics elcc", &param_refs, start, &res);
    res
}
