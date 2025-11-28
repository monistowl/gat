use anyhow::Result;
use gat_cli::cli::BenchmarkCommands;

pub mod baseline;
pub mod common;
pub mod opfdata;
pub mod pfdelta;
pub mod pglib;

pub fn handle(command: &BenchmarkCommands) -> Result<()> {
    match command {
        BenchmarkCommands::Pfdelta {
            pfdelta_root,
            case,
            contingency,
            max_cases,
            out,
            threads,
            mode,
            tol,
            max_iter,
            diagnostics_log,
            strict,
        } => pfdelta::handle(
            pfdelta_root,
            case.as_deref(),
            contingency,
            *max_cases,
            out,
            threads,
            mode,
            *tol,
            *max_iter,
            diagnostics_log.as_deref(),
            *strict,
        ),
        BenchmarkCommands::Pglib {
            pglib_dir,
            baseline,
            case_filter,
            max_cases,
            out,
            threads,
            method,
            tol,
            max_iter,
            enhanced,
        } => pglib::handle(
            pglib_dir,
            baseline.as_deref(),
            case_filter.as_deref(),
            *max_cases,
            out,
            threads,
            method,
            *tol,
            *max_iter,
            *enhanced,
        ),
        BenchmarkCommands::Opfdata {
            opfdata_dir,
            case_filter,
            max_cases,
            out,
            threads,
            method,
            tol,
            max_iter,
            diagnostics_log,
            strict,
        } => opfdata::handle(
            opfdata_dir,
            case_filter.as_deref(),
            *max_cases,
            out,
            threads,
            method,
            *tol,
            *max_iter,
            diagnostics_log.as_deref(),
            *strict,
        ),
    }
}
