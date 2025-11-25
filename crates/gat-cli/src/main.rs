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
    adms, alloc, analytics, completions, datasets, derms, dist, doctor, featurize, geo, graph,
    import, nminus1, opf, pf, se, ts, validate, version,
};
use gat_cli::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(cli.log_level)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Hello from gat-cli! Running with profile: {}", cli.profile);

    match &cli.command {
        Some(Commands::Import { command }) => run_and_log("import", || import::handle(command)),
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
