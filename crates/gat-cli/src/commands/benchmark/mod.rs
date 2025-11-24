use anyhow::Result;
use gat_cli::cli::BenchmarkCommands;

pub mod pfdelta;

pub fn handle(command: &BenchmarkCommands) -> Result<()> {
    match command {
        BenchmarkCommands::Pfdelta {
            pfdelta_root,
            case,
            contingency,
            max_cases,
            out,
            threads,
            tol,
            max_iter,
        } => pfdelta::handle(
            pfdelta_root,
            case.as_deref(),
            contingency,
            *max_cases,
            out,
            threads,
            *tol,
            *max_iter,
        ),
    }
}
