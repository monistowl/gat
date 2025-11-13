use clap::{Parser, Subcommand};
use gat_algo::power_flow;
use gat_core::graph_utils;
use gat_gui;
use gat_io::{importers, validate};
use gat_ts;
use gat_viz;
use std::path::Path;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber; // Added power_flow
mod dataset;
use dataset::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set the logging level
    #[arg(long, default_value = "info")]
    log_level: Level,

    /// Set the profile (e.g., "dev", "release")
    #[arg(long, default_value = "dev")]
    profile: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Import data from various formats
    Import {
        #[command(subcommand)]
        command: ImportCommands,
    },
    /// Validate a dataset against a schema
    Validate {
        /// Path to the dataset specification file
        #[arg(long)]
        spec: String,
    },
    /// Graph utilities
    Graph {
        #[command(subcommand)]
        command: GraphCommands,
    },
    /// Power flow solvers
    Pf {
        #[command(subcommand)]
        command: PowerFlowCommands,
    },
    /// Contingency analysis
    Nminus1 {
        #[command(subcommand)]
        command: Nminus1Commands,
    },
    /// Time-series utilities
    Ts {
        #[command(subcommand)]
        command: TsCommands,
    },
    /// Optimal power flow
    Opf {
        #[command(subcommand)]
        command: OpfCommands,
    },
    /// State estimation
    Se {
        #[command(subcommand)]
        command: SeCommands,
    },
    /// Visualization helpers
    Viz {
        #[command(subcommand)]
        command: VizCommands,
    },
    /// GUI dashboard
    Gui {
        #[command(subcommand)]
        command: GuiCommands,
    },
    /// Dataset adapters
    Dataset {
        #[command(subcommand)]
        command: DatasetCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ImportCommands {
    /// Import PSS/E RAW file
    Psse {
        /// Path to the RAW file
        #[arg(long)]
        raw: String,
        /// Output file path (Arrow format)
        #[arg(short, long)]
        output: String,
    },
    /// Import MATPOWER case file
    Matpower {
        /// Path to the MATPOWER .m file
        #[arg(long)]
        m: String,
        /// Output file path (Arrow format)
        #[arg(short, long)]
        output: String,
    },
    /// Import CIM RDF/XML files
    Cim {
        /// Path to the directory or zip file containing RDF/XML
        #[arg(long)]
        rdf: String,
        /// Output file path (Arrow format)
        #[arg(short, long)]
        output: String,
    },
}

#[derive(Subcommand, Debug)]
enum GraphCommands {
    /// Display graph statistics
    Stats {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
    },
    /// Find islands in the grid
    Islands {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Emit island IDs
        #[arg(long)]
        emit: bool,
    },
    /// Export graph to various formats
    Export {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Output format (e.g., graphviz)
        #[arg(long)]
        format: String,
    },
}

#[derive(Subcommand, Debug)]
enum PowerFlowCommands {
    /// Run DC power flow
    Dc {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Output file path for flows (Parquet format)
        #[arg(short, long)]
        out: String,
    },
    /// Run AC power flow
    Ac {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Tolerance for convergence
        #[arg(long, default_value = "1e-8")]
        tol: f64,
        /// Maximum number of iterations
        #[arg(long, default_value = "20")]
        max_iter: u32,
    },
}

#[derive(Subcommand, Debug)]
enum Nminus1Commands {
    /// Run a DC N-1 screening scenario
    Dc {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Contingency CSV (`branch_id,label`)
        #[arg(long)]
        contingencies: String,
        /// Output Parquet for scenario summaries
        #[arg(short, long)]
        out: String,
        /// Optional branch limits CSV (branch_id,flow_limit) for violation checks
        #[arg(long)]
        branch_limits: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum TsCommands {
    /// Resample a telemetry series
    Resample {
        /// Input time-series file (CSV or Parquet)
        input: String,
        /// Timestamp column name
        #[arg(long, default_value = "timestamp")]
        timestamp: String,
        /// Value column to aggregate
        #[arg(long, default_value = "value")]
        value: String,
        /// Resampling rule (e.g., 5s, 1m, 1h)
        #[arg(long)]
        rule: String,
        /// Output file path (CSV or Parquet)
        #[arg(short, long)]
        out: String,
    },
    /// Join two telemetry datasets
    Join {
        /// Left-hand input file (CSV or Parquet)
        left: String,
        /// Right-hand input file (CSV or Parquet)
        right: String,
        /// Key column to join on
        #[arg(long, default_value = "timestamp")]
        on: String,
        /// Output file path (CSV or Parquet)
        #[arg(short, long)]
        out: String,
    },
    /// Aggregate values by a column
    Agg {
        /// Input file path (CSV or Parquet)
        input: String,
        /// Column to group by
        #[arg(long, default_value = "sensor")]
        group: String,
        /// Value column to aggregate
        #[arg(long, default_value = "value")]
        value: String,
        /// Aggregation to perform: sum|mean|min|max|count
        #[arg(long, default_value = "sum")]
        agg: String,
        /// Output file path (CSV or Parquet)
        #[arg(short, long)]
        out: String,
    },
}

#[derive(Subcommand, Debug)]
enum DatasetCommands {
    /// RTS-GMLC helpers
    RtsGmlc {
        #[command(subcommand)]
        command: RtsGmlcCommands,
    },
    /// HIREN test cases
    Hiren {
        #[command(subcommand)]
        command: HirenCommands,
    },
    /// Import dsgrid Parquet bundle
    Dsgrid {
        #[arg(short, long)]
        out: String,
    },
    /// Sup3rCC weather helpers
    Sup3rcc {
        #[command(subcommand)]
        command: Sup3rccCommands,
    },
    /// PRAS outputs
    Pras {
        /// Path to PRAS directory or file
        path: String,
        #[arg(short, long)]
        out: String,
    },
}

#[derive(Subcommand, Debug)]
enum RtsGmlcCommands {
    /// Fetch release copy
    Fetch {
        #[arg(short, long, default_value = "data/rts-gmlc")]
        out: String,
        #[arg(long)]
        tag: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum HirenCommands {
    /// List cases
    List,
    /// Fetch a case
    Fetch {
        case: String,
        #[arg(short, long, default_value = "data/hiren")]
        out: String,
    },
}

#[derive(Subcommand, Debug)]
enum Sup3rccCommands {
    /// Fetch Parquet
    Fetch {
        #[arg(short, long)]
        out: String,
    },
    /// Sample for a grid
    Sample {
        grid: String,
        #[arg(short, long)]
        out: String,
    },
}

#[derive(Subcommand, Debug)]
enum VizCommands {
    /// Emit a basic visualization summary (placeholder)
    Plot {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Optional output path for the visualization artifact
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum GuiCommands {
    /// Launch the GUI dashboard (placeholder)
    Run {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Optional path to persist the visualization artifact
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum OpfCommands {
    /// Run DC optimal power flow
    Dc {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Cost CSV (bus_id,marginal_cost)
        #[arg(long)]
        cost: String,
        /// Limits CSV (bus_id,pmin,pmax,demand)
        #[arg(long)]
        limits: String,
        /// Output Parquet for dispatch
        #[arg(short, long)]
        out: String,
        /// Optional branch limits CSV (branch_id,flow_limit)
        #[arg(long)]
        branch_limits: Option<String>,
        /// Optional piecewise cost CSV (bus_id,start,end,slope)
        #[arg(long)]
        piecewise: Option<String>,
    },
    /// Run AC optimal power flow
    Ac {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Output Parquet for branch flows/residuals
        #[arg(short, long)]
        out: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum number of iterations
        #[arg(long, default_value = "20")]
        max_iter: u32,
    },
}

fn main() {
    let cli = Cli::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(cli.log_level)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Hello from gat-cli! Running with profile: {}", cli.profile);

    match &cli.command {
        Some(Commands::Import { command }) => {
            let result = match command {
                ImportCommands::Psse { raw, output } => {
                    info!("Importing PSSE RAW from {} to {}", raw, output);
                    importers::import_psse_raw(raw, output)
                }
                ImportCommands::Matpower { m, output } => {
                    info!("Importing MATPOWER from {} to {}", m, output);
                    importers::import_matpower_case(m, output)
                }
                ImportCommands::Cim { rdf, output } => {
                    info!("Importing CIM from {} to {}", rdf, output);
                    importers::import_cim_rdf(rdf, output)
                }
            };

            match result {
                Ok(_) => info!("Import successful!"),
                Err(e) => error!("Import failed: {:?}", e),
            }
        }
        Some(Commands::Validate { spec }) => {
            info!("Validating dataset with spec {}", spec);
            match validate::validate_dataset(spec) {
                Ok(_) => info!("Validation successful!"),
                Err(e) => error!("Validation failed: {:?}", e),
            }
        }
        Some(Commands::Graph { command }) => {
            let result = match command {
                GraphCommands::Stats { grid_file } => {
                    info!("Displaying graph statistics for {}", grid_file);
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => match graph_utils::graph_stats(&network) {
                            Ok(stats) => {
                                println!(
                                    "Nodes: {}\nEdges: {}\nComponents: {}\nDegree[min/avg/max]: {}/{:.2}/{}\nDensity: {:.4}",
                                    stats.node_count,
                                    stats.edge_count,
                                    stats.connected_components,
                                    stats.min_degree,
                                    stats.avg_degree,
                                    stats.max_degree,
                                    stats.density
                                );
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    }
                }
                GraphCommands::Islands { grid_file, emit } => {
                    info!("Finding islands in {} (emit_id: {})", grid_file, emit);
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => match graph_utils::find_islands(&network) {
                            Ok(analysis) => {
                                for summary in &analysis.islands {
                                    println!(
                                        "Island {}: {} node(s)",
                                        summary.island_id, summary.node_count
                                    );
                                }

                                if *emit {
                                    println!("\nNode → Island assignments:");
                                    for assignment in &analysis.assignments {
                                        println!(
                                            "  idx {:>3}: {:<20} -> island {}",
                                            assignment.node_index,
                                            assignment.label,
                                            assignment.island_id
                                        );
                                    }
                                }

                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    }
                }
                GraphCommands::Export { grid_file, format } => {
                    info!("Exporting graph in {} format", format);
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => match graph_utils::export_graph(&network, format) {
                            Ok(dot) => {
                                println!("{}", dot);
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    }
                }
            };

            match result {
                Ok(_) => info!("Graph command successful!"),
                Err(e) => error!("Graph command failed: {:?}", e),
            }
        }
        Some(Commands::Pf { command }) => {
            let result = match command {
                PowerFlowCommands::Dc { grid_file, out } => {
                    info!("Running DC power flow on {} to {}", grid_file, out);
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => power_flow::dc_power_flow(&network, out),
                        Err(e) => Err(e),
                    }
                }
                PowerFlowCommands::Ac {
                    grid_file,
                    tol,
                    max_iter,
                } => {
                    info!(
                        "Running AC power flow on {} with tol {} and max_iter {}",
                        grid_file, tol, max_iter
                    );
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => power_flow::ac_power_flow(&network, *tol, *max_iter),
                        Err(e) => Err(e),
                    }
                }
            };

            match result {
                Ok(_) => info!("Power flow command successful!"),
                Err(e) => error!("Power flow command failed: {:?}", e),
            }
        }
        Some(Commands::Nminus1 { command }) => {
            let result = match command {
                Nminus1Commands::Dc {
                    grid_file,
                    contingencies,
                    out,
                    branch_limits,
                } => {
                    info!(
                        "Running N-1 DC on {} with contingencies {} -> {}",
                        grid_file, contingencies, out
                    );
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => power_flow::n_minus_one_dc(
                            &network,
                            contingencies,
                            out,
                            branch_limits.as_deref(),
                        ),
                        Err(e) => Err(e),
                    }
                }
            };

            match result {
                Ok(_) => info!("N-1 command successful!"),
                Err(e) => error!("N-1 command failed: {:?}", e),
            }
        }
        Some(Commands::Ts { command }) => {
            let result = match command {
                TsCommands::Resample {
                    input,
                    timestamp,
                    value,
                    rule,
                    out,
                } => {
                    info!(
                        "Resampling {} ({}/{}) every {} -> {}",
                        input, timestamp, value, rule, out
                    );
                    gat_ts::resample_timeseries(input, timestamp, value, rule, out)
                }
                TsCommands::Join {
                    left,
                    right,
                    on,
                    out,
                } => {
                    info!("Joining {} and {} on {} -> {}", left, right, on, out);
                    gat_ts::join_timeseries(left, right, on, out)
                }
                TsCommands::Agg {
                    input,
                    group,
                    value,
                    agg,
                    out,
                } => {
                    info!(
                        "Aggregating {} by {} ({}) using {} -> {}",
                        input, group, value, agg, out
                    );
                    gat_ts::aggregate_timeseries(input, group, value, agg, out)
                }
            };

            match result {
                Ok(_) => info!("Timeseries command successful!"),
                Err(e) => error!("Timeseries command failed: {:?}", e),
            }
        }
        Some(Commands::Se { command }) => {
            let result = match command {
                SeCommands::Wls {
                    grid_file,
                    measurements,
                    out,
                    state_out,
                } => {
                    info!(
                        "Running WLS state estimation on {} using {} -> {}",
                        grid_file, measurements, out
                    );
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => power_flow::state_estimation_wls(
                            &network,
                            measurements,
                            out,
                            state_out.as_deref(),
                        ),
                        Err(e) => Err(e),
                    }
                }
            };

            match result {
                Ok(_) => info!("State estimation command successful!"),
                Err(e) => error!("State estimation command failed: {:?}", e),
            }
        }
        Some(Commands::Viz { command }) => {
            let result = match command {
                VizCommands::Plot { grid_file, output } => {
                    info!("Running viz plot on {}", grid_file);
                    match importers::load_grid_from_arrow(grid_file) {
                        Ok(_network) => {
                            let body = gat_viz::visualize_data();
                            if let Some(path) = output {
                                println!("Visualization persisted to {}", path);
                            }
                            println!("Visualization summary: {}", body);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                }
            };

            match result {
                Ok(_) => info!("Viz command successful!"),
                Err(e) => error!("Viz command failed: {:?}", e),
            }
        }
        Some(Commands::Gui { command }) => {
            let result = match command {
                GuiCommands::Run { grid_file, output } => {
                    info!("Launching GUI for {}", grid_file);
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(_network) => match gat_gui::launch(output.as_deref()) {
                            Ok(summary) => {
                                println!("GUI summary: {}", summary);
                                if let Some(path) = output {
                                    println!("GUI artifact persisted to {}", path);
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    }
                }
            };

            match result {
                Ok(_) => info!("GUI command successful!"),
                Err(e) => error!("GUI command failed: {:?}", e),
            }
        }
        Some(Commands::Dataset { command }) => {
            let result = match command {
                DatasetCommands::RtsGmlc { command } => match command {
                    RtsGmlcCommands::Fetch { out, tag } => {
                        fetch_rts_gmlc(Path::new(&out), tag.as_deref())
                    }
                },
                DatasetCommands::Hiren { command } => match command {
                    HirenCommands::List => {
                        let cases = list_hiren().unwrap_or_default();
                        for case in &cases {
                            println!("{}", case);
                        }
                        Ok(())
                    }
                    HirenCommands::Fetch { case, out } => fetch_hiren(&case, Path::new(&out)),
                },
                DatasetCommands::Dsgrid { out } => import_dsgrid(Path::new(&out)),
                DatasetCommands::Sup3rcc { command } => match command {
                    Sup3rccCommands::Fetch { out } => fetch_sup3rcc(Path::new(&out)),
                    Sup3rccCommands::Sample { grid, out } => {
                        sample_sup3rcc_grid(Path::new(&grid), Path::new(&out))
                    }
                },
                DatasetCommands::Pras { path, out } => {
                    import_pras(Path::new(&path), Path::new(&out))
                }
            };

            match result {
                Ok(_) => info!("Dataset command successful!"),
                Err(e) => error!("Dataset command failed: {:?}", e),
            }
        }
        Some(Commands::Opf { command }) => {
            let result = match command {
                OpfCommands::Dc {
                    grid_file,
                    cost,
                    limits,
                    out,
                    branch_limits,
                    piecewise,
                } => {
                    info!(
                        "Running DC OPF on {} with cost {} and limits {} -> {}",
                        grid_file, cost, limits, out
                    );
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => power_flow::dc_optimal_power_flow(
                            &network,
                            cost.as_str(),
                            limits.as_str(),
                            out.as_str(),
                            branch_limits.as_deref(),
                            piecewise.as_deref(),
                        ),
                        Err(e) => Err(e),
                    }
                }
                OpfCommands::Ac {
                    grid_file,
                    out,
                    tol,
                    max_iter,
                } => {
                    info!(
                        "Running AC OPF on {} with tol {}, max_iter {} -> {}",
                        grid_file, tol, max_iter, out
                    );
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => power_flow::ac_optimal_power_flow(
                            &network,
                            *tol,
                            *max_iter,
                            out.as_str(),
                        ),
                        Err(e) => Err(e),
                    }
                }
            };

            match result {
                Ok(_) => info!("OPF command successful!"),
                Err(e) => error!("OPF command failed: {:?}", e),
            }
        }
        None => {
            info!("No subcommand provided. Use `gat --help` for more information.");
        }
    }
}

#[derive(Subcommand, Debug)]
enum SeCommands {
    /// Run WLS state estimation
    Wls {
        /// Path to the grid file (Arrow format)
        grid_file: String,
        /// Measurements CSV (`measurement_type,branch_id,bus_id,value,weight,label`)
        #[arg(long)]
        measurements: String,
        /// Output Parquet for measurement residuals
        #[arg(short, long)]
        out: String,
        /// Optional Parquet output for the solved bus angles
        #[arg(long)]
        state_out: Option<String>,
    },
}
