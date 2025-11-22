use std::path::Path;
use std::time::Instant;

use crate::commands::telemetry::record_run_timed;
use anyhow::Result;
use gat_scenarios::{load_spec_from_path, validate};

pub fn handle(spec: &str) -> Result<()> {
    let start = Instant::now();
    let res = (|| -> Result<()> {
        let path = Path::new(spec);
        let set = load_spec_from_path(path)?;
        validate(&set)?;
        println!("Scenario spec validated successfully");
        Ok(())
    })();
    record_run_timed(spec, "scenarios validate", &[("spec", spec)], start, &res);
    res
}
