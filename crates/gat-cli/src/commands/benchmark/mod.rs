use anyhow::Result;
use gat_cli::cli::BenchmarkCommands;
use gat_cli::common::OpfMethod;

pub mod baseline;
pub mod compare;
pub mod dplib;
pub mod dss2;
pub mod opfdata;
pub mod pfdelta;
pub mod pglib;
pub mod summary;

/// Convert CLI OpfMethod enum to algo crate's OpfMethod enum
fn cli_method_to_algo(method: OpfMethod) -> gat_algo::opf::OpfMethod {
    match method {
        OpfMethod::Economic => gat_algo::opf::OpfMethod::EconomicDispatch,
        OpfMethod::Dc => gat_algo::opf::OpfMethod::DcOpf,
        OpfMethod::Socp => gat_algo::opf::OpfMethod::SocpRelaxation,
        OpfMethod::Ac => gat_algo::opf::OpfMethod::AcOpf,
    }
}

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
            solver,
        } => pglib::handle(
            pglib_dir,
            baseline.as_deref(),
            case_filter.as_deref(),
            *max_cases,
            out,
            threads,
            cli_method_to_algo(*method),
            *tol,
            *max_iter,
            *enhanced,
            solver,
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
            solver,
        } => opfdata::handle(
            opfdata_dir,
            case_filter.as_deref(),
            *max_cases,
            out,
            threads,
            cli_method_to_algo(*method),
            *tol,
            *max_iter,
            diagnostics_log.as_deref(),
            *strict,
            solver,
        ),
        BenchmarkCommands::Summary { csv } => summary::handle(csv),
        BenchmarkCommands::Compare { before, after } => compare::handle(before, after),
        BenchmarkCommands::Dplib {
            pglib_dir,
            case_filter,
            max_cases,
            out,
            threads,
            num_partitions,
            max_iter,
            tol,
            rho,
            subproblem_method,
        } => dplib::handle(
            pglib_dir,
            case_filter.as_deref(),
            *max_cases,
            out,
            threads,
            *num_partitions,
            *max_iter,
            *tol,
            *rho,
            subproblem_method,
        ),
        BenchmarkCommands::Dss2 {
            out,
            trials,
            noise_std,
            load_scale,
            num_flow,
            num_injection,
            seed,
            threads,
        } => dss2::handle(
            out,
            *trials,
            *noise_std,
            *load_scale,
            *num_flow,
            *num_injection,
            *seed,
            threads,
        ),
    }
}
