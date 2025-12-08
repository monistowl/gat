use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

mod commands;
mod dataset;
mod runs;

use crate::commands::batch as command_batch;
use crate::commands::benchmark as command_benchmark;
#[cfg(feature = "gui")]
use crate::commands::gui;
use crate::commands::runs as command_runs;
use crate::commands::scenarios as command_scenarios;
#[cfg(feature = "tui")]
use crate::commands::tui;
#[cfg(feature = "viz")]
use crate::commands::viz;
use crate::commands::{
    adms, alloc, analytics, completions, convert, datasets, derms, dist, doctor, featurize, geo,
    graph, import, inspect, nminus1, opf, pf, se, solver, tep, ts, validate, version,
};
use gat_cli::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(cli.log_level)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Hello from gat-cli! Running with profile: {}", cli.profile);

    // Handle --gpu flag
    if cli.gpu {
        #[cfg(feature = "gpu")]
        {
            if gat_gpu::is_gpu_available() {
                info!("GPU acceleration enabled");
            } else {
                error!("GPU requested but no GPU available");
                std::process::exit(1);
            }
        }
        #[cfg(not(feature = "gpu"))]
        {
            error!("GPU requested but gat-cli was not compiled with `gpu` feature. Rebuild with `--features gpu`.");
            std::process::exit(1);
        }
    }

    match &cli.command {
        Some(Commands::Import {
            command,
            verbose,
            strict,
            validate,
        }) => run_and_log("import", || {
            import::handle(command, *verbose, *strict, *validate)
        }),
        Some(Commands::Validate { spec }) => run_and_log("validate", || validate::handle(spec)),
        Some(Commands::Graph { command }) => run_and_log("graph", || graph::handle(command)),
        Some(Commands::Scenarios { command }) => {
            run_and_log("scenarios", || command_scenarios::handle(command))
        }
        Some(Commands::Batch { command }) => {
            run_and_log("batch", || command_batch::handle(command))
        }
        Some(Commands::Benchmark { command }) => {
            run_and_log("benchmark", || command_benchmark::handle(command))
        }
        Some(Commands::Dataset { command }) => run_and_log("dataset", || datasets::handle(command)),
        Some(Commands::Doctor {}) => run_and_log("doctor", doctor::handle),
        Some(Commands::Analytics { command }) => {
            run_and_log("analytics", || analytics::handle(command))
        }
        Some(Commands::Featurize { command }) => {
            run_and_log("featurize", || featurize::handle(command))
        }
        Some(Commands::Alloc { command }) => run_and_log("alloc", || alloc::handle(command)),
        Some(Commands::Geo { command }) => run_and_log("geo", || geo::handle(command)),
        Some(Commands::Pf { command }) => run_and_log("pf", || pf::handle(command)),
        Some(Commands::Nminus1 { command }) => run_and_log("nminus1", || nminus1::handle(command)),
        Some(Commands::Ts { command }) => run_and_log("ts", || ts::handle(command)),
        Some(Commands::Dist { command }) => run_and_log("dist", || dist::handle(command)),
        Some(Commands::Derms { command }) => run_and_log("derms", || derms::handle(command)),
        Some(Commands::Adms { command }) => run_and_log("adms", || adms::handle(command)),
        Some(Commands::Opf { command }) => run_and_log("opf", || opf::handle(command)),
        Some(Commands::Se { command }) => run_and_log("se", || se::handle(command)),
        Some(Commands::Solver { command }) => run_and_log("solver", || solver::handle(command)),
        Some(Commands::Tep { command }) => run_and_log("tep", || tep::handle(command)),
        Some(Commands::Convert { command }) => run_and_log("convert", || convert::handle(command)),
        Some(Commands::Inspect { command }) => run_and_log("inspect", || inspect::handle(command)),
        Some(Commands::Runs { command }) => run_and_log("runs", || command_runs::handle(command)),
        #[cfg(feature = "gui")]
        Some(Commands::Gui { command }) => run_and_log("gui", || gui::handle(command)),
        #[cfg(feature = "viz")]
        Some(Commands::Viz { command }) => run_and_log("viz", || viz::handle(command)),
        Some(Commands::Completions { shell, out }) => run_and_log("completions", || {
            completions::handle(*shell, out.as_deref())
        }),
        #[cfg(feature = "tui")]
        Some(Commands::Tui { command }) => run_and_log("tui", || tui::handle(command)),
        Some(Commands::Version { command }) => run_and_log("version", || version::handle(command)),
        None => {
            info!("No subcommand provided. Use `gat --help` for more information.");
        }
    }
}

fn run_and_log(name: &str, action: impl FnOnce() -> anyhow::Result<()>) {
    match action() {
        Ok(_) => info!("{name} command successful!"),
        Err(err) => error!("{name} command failed: {:?}", err),
    }
}
