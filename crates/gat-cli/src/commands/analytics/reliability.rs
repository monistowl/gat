use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::AnalyticsCommands;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;

/// Handle `gat analytics reliability` command: compute LOLE, EUE, and thermal violations.
///
/// **Purpose:** Converts batch PF/OPF outputs into reliability adequacy metrics used for
/// resource adequacy (RA) accreditation and KPI prediction. Computes Loss of Load Expectation
/// (LOLE), Energy Unserved (EUE), and thermal violation counts. See doi:10.1109/TPWRS.2012.2187686
/// for reliability metrics in power systems.
pub fn handle(command: &AnalyticsCommands) -> Result<()> {
    let AnalyticsCommands::Reliability {
        batch_manifest,
        flows,
        branch_limits,
        scenario_weights,
        out,
        out_partitions,
        unserved_threshold,
    } = command else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();
    let mut summary = None;

    let res = (|| -> Result<()> {
        let result = gat_algo::reliability_metrics(
            batch_manifest.as_deref().map(Path::new),
            flows.as_deref().map(Path::new),
            branch_limits.as_deref(),
            scenario_weights.as_deref(),
            Path::new(out),
            &partitions,
            *unserved_threshold,
        )?;
        summary = Some(result);
        Ok(())
    })();

    // Build params list, including summary stats if available
    let mut params = vec![
        ("out".to_string(), out.to_string()),
        (
            "unserved_threshold_mw".to_string(),
            unserved_threshold.to_string(),
        ),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
    ];
    if let Some(ref s) = summary {
        // Print summary and add to params (using ref to avoid move)
        println!(
            "Reliability metrics: {} scenarios × {} time periods = {} cases, LOLE={:.2} hours, EUE={:.2} MWh, {} violations -> {}",
            s.num_scenarios,
            s.num_time_periods,
            s.total_cases,
            s.lole_hours,
            s.eue_mwh,
            s.thermal_violations,
            out
        );
        params.push(("num_scenarios".to_string(), s.num_scenarios.to_string()));
        params.push(("num_time_periods".to_string(), s.num_time_periods.to_string()));
        params.push(("total_cases".to_string(), s.total_cases.to_string()));
        params.push(("lole_hours".to_string(), s.lole_hours.to_string()));
        params.push(("eue_mwh".to_string(), s.eue_mwh.to_string()));
        params.push(("thermal_violations".to_string(), s.thermal_violations.to_string()));
    }
    if let Some(ref m) = batch_manifest {
        params.push(("batch_manifest".to_string(), m.to_string()));
    }
    if let Some(ref f) = flows {
        params.push(("flows".to_string(), f.to_string()));
    }
    if let Some(ref bl) = branch_limits {
        params.push(("branch_limits".to_string(), bl.to_string()));
    }
    if let Some(ref sw) = scenario_weights {
        params.push(("scenario_weights".to_string(), sw.to_string()));
    }

    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "analytics reliability", &param_refs, start, &res);
    res
}
