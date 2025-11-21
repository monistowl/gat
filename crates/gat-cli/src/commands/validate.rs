use std::time::Instant;

use anyhow::Result;
use gat_io::validate;

use crate::commands::telemetry::record_run_timed;

pub fn handle(spec: &str) -> Result<()> {
    let start = Instant::now();
    let res = validate::validate_dataset(spec);
    record_run_timed(spec, "validate dataset", &[("spec", spec)], start, &res);
    res
}
