use gat_cli::manifest::{
    record_manifest, record_manifest_with_diagnostics, ManifestTelemetry, Param,
};
use gat_io::helpers::ImportDiagnostics;
use std::{env, path::Path, time::Instant};

const TELEMETRY_ENV_KEYS: &[&str] = &[
    "GAT_VARIANT",
    "GAT_ENV",
    "GAT_RELEASE_VERSION",
    "GITHUB_RUN_ID",
    "GITHUB_WORKFLOW",
    "GITHUB_JOB",
    "GITHUB_REF",
    "GITHUB_SHA",
];

fn collect_telemetry_env() -> Vec<Param> {
    TELEMETRY_ENV_KEYS
        .iter()
        .filter_map(|key| {
            env::var(key).ok().map(|value| Param {
                name: key.to_string(),
                value,
            })
        })
        .collect()
}

fn correlation_id() -> Option<String> {
    env::var("GAT_CORRELATION_ID")
        .or_else(|_| env::var("GITHUB_RUN_ID"))
        .ok()
}

fn record_run_with_status(
    out: &str,
    command: &str,
    params: &[(&str, &str)],
    status: &str,
    duration_ms: Option<u128>,
) {
    let telemetry = ManifestTelemetry {
        status: status.to_string(),
        duration_ms,
        env: collect_telemetry_env(),
        correlation_id: correlation_id(),
    };
    if let Err(err) = record_manifest(Path::new(out), command, params, telemetry) {
        eprintln!("Failed to record run manifest: {err}");
    }
}

pub fn record_run(out: &str, command: &str, params: &[(&str, &str)]) {
    record_run_with_status(out, command, params, "success", None);
}

pub fn record_run_timed(
    out: &str,
    command: &str,
    params: &[(&str, &str)],
    start: Instant,
    result: &anyhow::Result<()>,
) {
    let duration_ms = start.elapsed().as_millis();
    let status = if result.is_ok() { "success" } else { "failure" };
    record_run_with_status(out, command, params, status, Some(duration_ms));
}

/// Record a run with diagnostics from an import operation.
/// The diagnostics are serialized to JSON and stored in the manifest.
pub fn record_run_timed_with_diagnostics(
    out: &str,
    command: &str,
    params: &[(&str, &str)],
    start: Instant,
    diagnostics: &ImportDiagnostics,
) {
    let duration_ms = start.elapsed().as_millis();
    let telemetry = ManifestTelemetry {
        status: "success".to_string(),
        duration_ms: Some(duration_ms),
        env: collect_telemetry_env(),
        correlation_id: correlation_id(),
    };
    let diag_json = serde_json::to_value(diagnostics).ok();
    if let Err(err) =
        record_manifest_with_diagnostics(Path::new(out), command, params, telemetry, diag_json)
    {
        eprintln!("Failed to record run manifest: {err}");
    }
}
