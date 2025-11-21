use std::time::Instant;

use anyhow::Result;
use gat_cli::cli::TsCommands;
use gat_ts::{aggregate_timeseries, join_timeseries, resample_timeseries};

use crate::commands::telemetry::record_run_timed;
use crate::parse_partitions;

pub fn handle(command: &TsCommands) -> Result<()> {
    match command {
        TsCommands::Resample {
            input,
            timestamp,
            value,
            rule,
            out,
            out_partitions,
        } => {
            let start = Instant::now();
            let partitions = parse_partitions(out_partitions.as_ref());
            let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
            let res = resample_timeseries(input, timestamp, value, rule, out, &partitions);
            record_run_timed(
                out,
                "ts resample",
                &[
                    ("input", input),
                    ("timestamp", timestamp),
                    ("value", value),
                    ("rule", rule),
                    ("out", out),
                    ("out_partitions", partition_spec.as_str()),
                ],
                start,
                &res,
            );
            res
        }
        TsCommands::Join {
            left,
            right,
            on,
            out,
            out_partitions,
        } => {
            let start = Instant::now();
            let partitions = parse_partitions(out_partitions.as_ref());
            let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
            let res = join_timeseries(left, right, on, out, &partitions);
            record_run_timed(
                out,
                "ts join",
                &[
                    ("left", left),
                    ("right", right),
                    ("on", on),
                    ("out", out),
                    ("out_partitions", partition_spec.as_str()),
                ],
                start,
                &res,
            );
            res
        }
        TsCommands::Agg {
            input,
            group,
            value,
            agg,
            out,
            out_partitions,
        } => {
            let start = Instant::now();
            let partitions = parse_partitions(out_partitions.as_ref());
            let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
            let res = aggregate_timeseries(input, group, value, agg, out, &partitions);
            record_run_timed(
                out,
                "ts agg",
                &[
                    ("input", input),
                    ("group", group),
                    ("value", value),
                    ("agg", agg),
                    ("out", out),
                    ("out_partitions", partition_spec.as_str()),
                ],
                start,
                &res,
            );
            res
        }
    }
}
