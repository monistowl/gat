use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::AllocCommands;

use crate::commands::telemetry::record_run_timed;
use crate::commands::util::parse_partitions;
use gat_algo::alloc_kpi::compute_kpi_contributions;

/// Handle `gat alloc kpi` command: simple contribution analysis for KPI changes.
///
/// **Purpose:** Approximates the contribution of control actions/portfolios to KPI improvements
/// using gradient-based sensitivity or linear approximations. This is a stepping stone towards
/// full SHAP (SHapley Additive exPlanations) or Partition SHAP explainability.
///
/// **Explainability Context:**
/// When evaluating scenarios with different policy/control settings (DR programs, DER dispatch,
/// carbon caps), we want to attribute changes in reliability KPIs (LOLE, EUE, violations) to
/// specific interventions. This helps answer "which controls contributed most to improvement?"
///
/// **Algorithm (v0):**
/// 1. Load KPI results across multiple scenarios (scenario_id, kpi_value)
/// 2. Load scenario metadata (which controls were active in each scenario)
/// 3. Compute finite differences or gradients: ΔKPIcontrol_i = KPI(control_i=on) - KPI(baseline)
/// 4. Rank controls by contribution magnitude
/// 5. Output contribution table: control_id, kpi_delta, contribution_pct
///
/// **Pedagogical Note for Grad Students:**
/// This is a simplified attribution method. Full SHAP considers all possible coalitions of
/// features and computes marginal contributions, satisfying fairness axioms (symmetry, dummy,
/// additivity). Our v0 uses naive differences or linear regression coefficients, which are
/// computationally cheap but may misattribute correlated controls. See Lundberg & Lee (2017),
/// "A Unified Approach to Interpreting Model Predictions" (doi:10.5555/3295222.3295230) for
/// SHAP theory, and doi:10.1038/s42256-019-0138-9 for applications.
///
/// **Future Extensions:**
/// - Implement Partition SHAP for hierarchical control structures (zone-level vs. resource-level)
/// - Add counterfactual scenarios: "what if control X was off?"
/// - Integrate with cost allocation: attribute both KPI improvements and cost impacts
///
/// **Example Usage:**
/// ```bash
/// # 1. Run scenarios with different control settings
/// gat batch opf --scenarios ./scenarios/control_variants.yaml --out ./outputs/opf_results.parquet
///
/// # 2. Compute KPI metrics for each scenario
/// gat analytics reliability --flows ./outputs/opf_results.parquet --out ./outputs/kpi_metrics.parquet
///
/// # 3. Attribute KPI changes to control actions
/// gat alloc kpi \
///   --kpi-results ./outputs/kpi_metrics.parquet \
///   --scenario-meta ./scenarios/control_metadata.yaml \
///   --out ./outputs/kpi_contributions.parquet
/// ```
///
/// **Status:** This is a stub implementation (to be completed in gat-09h issue).
pub fn handle(command: &AllocCommands) -> Result<()> {
    let AllocCommands::Kpi {
        kpi_results,
        scenario_meta,
        out,
        out_partitions,
    } = command
    else {
        unreachable!();
    };

    let partitions = parse_partitions(out_partitions.as_ref());
    let start = Instant::now();

    let res = (|| -> Result<()> {
        // Compute KPI contribution analysis using finite differences
        let summary = compute_kpi_contributions(
            Path::new(kpi_results),
            Path::new(scenario_meta),
            None, // baseline_scenario_id (auto-detect)
            Path::new(out),
            &partitions,
        )?;

        // Print summary statistics
        println!(
            "KPI contribution analysis: {} KPIs × {} controls = {} contributions",
            summary.num_kpis,
            summary.num_controls,
            summary.num_kpis * summary.num_controls
        );
        println!("  Output: {}", out);

        Ok(())
    })();

    // Record run telemetry
    let params = [
        ("kpi_results".to_string(), kpi_results.to_string()),
        ("scenario_meta".to_string(), scenario_meta.to_string()),
        ("out".to_string(), out.to_string()),
        (
            "out_partitions".to_string(),
            out_partitions.as_deref().unwrap_or("").to_string(),
        ),
    ];

    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    record_run_timed(out, "alloc kpi", &param_refs, start, &res);
    res
}
