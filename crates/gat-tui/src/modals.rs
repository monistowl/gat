/// Pre-configured modal templates for new CLI commands.
///
/// Each modal is a CommandModal with pre-filled command text, help text,
/// and execution mode suitable for the specific command.

use crate::ui::{CommandModal, ExecutionMode};

/// Scenarios: Materialize command modal
pub fn scenarios_materialize_modal() -> CommandModal {
    CommandModal::new(
        "Materialize Scenarios",
        "Load YAML template, materialize all scenarios into manifest",
        'm',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat scenarios materialize",
        "--template scenarios.yaml",
        "--output manifest.json",
    ])
}

/// Scenarios: Validate command modal
pub fn scenarios_validate_modal() -> CommandModal {
    CommandModal::new(
        "Validate Scenarios",
        "Validate scenario specification for syntax and correctness",
        'v',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text(["gat scenarios validate", "--spec scenarios.yaml"])
}

/// Batch: Power flow command modal
pub fn batch_pf_modal() -> CommandModal {
    CommandModal::new(
        "Batch Power Flow",
        "Run DC power flow on all scenarios in manifest",
        'p',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat batch pf",
        "--manifest manifest.json",
        "--max-jobs 4",
    ])
}

/// Batch: Optimal power flow command modal
pub fn batch_opf_modal() -> CommandModal {
    CommandModal::new(
        "Batch Optimal Power Flow",
        "Run OPF on all scenarios with reliability metrics",
        'o',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat batch opf",
        "--manifest manifest.json",
        "--max-jobs 4",
    ])
}

/// Featurize: GNN features command modal
pub fn featurize_gnn_modal() -> CommandModal {
    CommandModal::new(
        "GNN Featurization",
        "Export grid topology as Graph Neural Network features",
        'g',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat featurize gnn",
        "--grid-file case.arrow",
        "--group-by zone",
        "--output gnn_features.parquet",
    ])
}

/// Featurize: KPI features command modal
pub fn featurize_kpi_modal() -> CommandModal {
    CommandModal::new(
        "KPI Featurization",
        "Generate KPI training features from batch outputs",
        'k',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat featurize kpi",
        "--batch-root ./batch_results",
        "--output kpi_features.parquet",
    ])
}

/// Allocation: Congestion rents command modal
pub fn alloc_rents_modal() -> CommandModal {
    CommandModal::new(
        "Congestion Rents",
        "Decompose OPF results into congestion rents and surplus",
        'r',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat alloc rents",
        "--opf-file opf_results.parquet",
        "--output rents.parquet",
    ])
}

/// Allocation: KPI contribution command modal
pub fn alloc_kpi_modal() -> CommandModal {
    CommandModal::new(
        "KPI Contribution",
        "Compute sensitivity of KPIs to control actions",
        'c',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat alloc kpi",
        "--opf-file opf_results.parquet",
        "--kpi-file kpi_features.parquet",
        "--output contribution.parquet",
    ])
}

/// Geo: Spatial join command modal
pub fn geo_join_modal() -> CommandModal {
    CommandModal::new(
        "Spatial Join",
        "Map buses/feeders to spatial polygons (census tracts, etc.)",
        'j',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat geo join",
        "--grid-file case.arrow",
        "--polygons polygons.geoparquet",
        "--method point_in_polygon",
        "--output bus_polygons.parquet",
    ])
}

/// Geo: Geospatial featurization command modal
pub fn geo_featurize_modal() -> CommandModal {
    CommandModal::new(
        "Spatial Features",
        "Generate polygon-level spatial-temporal features",
        'f',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat geo featurize",
        "--flows flows.parquet",
        "--bus-polygons bus_polygons.parquet",
        "--lag-days 7",
        "--output spatial_features.parquet",
    ])
}

/// Analytics: Deliverability Score command modal
pub fn analytics_ds_modal() -> CommandModal {
    CommandModal::new(
        "Deliverability Score",
        "Compute deliverability score for each resource",
        'd',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat analytics ds",
        "--grid-file case.arrow",
        "--flows flows.parquet",
        "--output ds_scores.parquet",
    ])
}

/// Analytics: Reliability metrics command modal
pub fn analytics_reliability_modal() -> CommandModal {
    CommandModal::new(
        "Reliability Metrics",
        "Compute LOLE, EUE, and thermal violations",
        'e',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat analytics reliability",
        "--manifest manifest.json",
        "--flows flows.parquet",
        "--output reliability.json",
    ])
}

/// Analytics: ELCC estimation command modal
pub fn analytics_elcc_modal() -> CommandModal {
    CommandModal::new(
        "ELCC Estimation",
        "Estimate Equivalent Load Carrying Capability",
        'l',
    )
    .with_mode(ExecutionMode::DryRun)
    .with_command_text([
        "gat analytics elcc",
        "--profiles resource_profiles.parquet",
        "--reliability reliability.json",
        "--output elcc_estimates.parquet",
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenarios_materialize_modal() {
        let modal = scenarios_materialize_modal();
        assert_eq!(modal.title, "Materialize Scenarios");
        assert_eq!(modal.run_hotkey, 'm');
    }

    #[test]
    fn test_batch_pf_modal() {
        let modal = batch_pf_modal();
        assert_eq!(modal.title, "Batch Power Flow");
        assert_eq!(modal.run_hotkey, 'p');
    }

    #[test]
    fn test_all_modals_exist() {
        let _m1 = scenarios_validate_modal();
        let _m2 = batch_opf_modal();
        let _m3 = featurize_gnn_modal();
        let _m4 = featurize_kpi_modal();
        let _m5 = alloc_rents_modal();
        let _m6 = alloc_kpi_modal();
        let _m7 = geo_join_modal();
        let _m8 = geo_featurize_modal();
        let _m9 = analytics_ds_modal();
        let _m10 = analytics_reliability_modal();
        let _m11 = analytics_elcc_modal();
        // All 11 modals instantiate successfully
    }
}
