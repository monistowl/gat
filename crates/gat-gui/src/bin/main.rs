use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

type CliResult = Result<()>;

#[derive(Parser, Debug)]
#[command(
    name = "gat-gui",
    about = "Composite launcher for GAT GUI experiences."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List GUI apps packaged in this launcher.
    List,
    /// Launch the GAT Notebook experience.
    Notebook {
        /// Workspace folder to bootstrap for the notebook session.
        #[arg(long, value_name = "PATH", default_value = "./gat-notebook")]
        workspace: PathBuf,
        /// Port to advertise for the notebook server.
        #[arg(long, value_name = "PORT", default_value_t = 8787)]
        port: u16,
        /// Whether to request that the browser be opened after launch.
        #[arg(long)]
        open_browser: bool,
        /// Optional path to write a structured launch summary.
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

fn main() -> CliResult {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            for app in gat_gui::available_apps() {
                println!("- {}: {} (cmd: {})", app.name, app.description, app.command);
            }
        }
        Commands::Notebook {
            workspace,
            port,
            open_browser,
            output,
        } => {
            let options = gat_notebook::NotebookOptions {
                workspace,
                port,
                open_browser,
            };
            let report = gat_gui::launch(gat_gui::GuiApp::Notebook(options))?;
            println!("{}", report.summary());

            if let Some(path) = output {
                std::fs::write(path, report.summary())?;
            }
        }
    }

    Ok(())
}
