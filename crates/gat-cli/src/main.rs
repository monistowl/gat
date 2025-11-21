use clap::Parser;
use clap_complete::{generate, Shell};
use gat_algo::{power_flow, LpSolverKind};
use gat_core::{graph_utils, solver::SolverKind};
use gat_io::{importers, validate};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
#[cfg(feature = "tui")]
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tabwriter::TabWriter;
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber; // Added power_flow
mod dataset;
mod runs;
use crate::commands::telemetry::record_run;
use crate::commands::util::{configure_threads, parse_partitions};
use crate::commands::{adms, analytics, derms, dist, pf, se, ts};
use dataset::*;
#[cfg(feature = "tui")]
use dirs::config_dir;
#[cfg(feature = "gui")]
use gat_cli::cli::GuiCommands;
#[cfg(feature = "tui")]
use gat_cli::cli::TuiCommands;
#[cfg(feature = "viz")]
use gat_cli::cli::VizCommands;
use gat_cli::{
    cli::{
        build_cli_command, Cli, Commands, DatasetCommands, GraphCommands, HirenCommands,
        ImportCommands, Nminus1Commands, OpfCommands, PublicDatasetCommands, RtsGmlcCommands,
        RunFormat, RunsCommands, Sup3rccCommands,
    },
    manifest,
};
#[cfg(feature = "viz")]
use gat_viz::layout::layout_network;
use manifest::ManifestEntry;
use runs::{discover_runs, resolve_manifest, summaries, RunRecord};
mod commands;

fn resume_manifest(manifest: &ManifestEntry) -> anyhow::Result<()> {
    let mut args: Vec<String> = manifest
        .command
        .split_whitespace()
        .map(String::from)
        .collect();
    for param in &manifest.params {
        match param.name.as_str() {
            "grid_file" => args.push(param.value.clone()),
            _ => {
                args.push(format!("--{}", param.name));
                args.push(param.value.clone());
            }
        }
    }
    let exe = env::current_exe()?;
    let status = Command::new(exe).args(&args).status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("resumed run failed with {status}"));
    }
    Ok(())
}

fn describe_manifest(manifest: &ManifestEntry) {
    println!(
        "Manifest {} (cmd: `{}` @ v{} from {})",
        manifest.run_id, manifest.command, manifest.version, manifest.timestamp
    );
    if let Some(seed) = &manifest.seed {
        println!("Seed: {seed}");
    }
    if !manifest.params.is_empty() {
        println!("Parameters:");
        for param in &manifest.params {
            println!("  {} = {}", param.name, param.value);
        }
    }
    if !manifest.inputs.is_empty() {
        println!("Inputs:");
        for input in &manifest.inputs {
            let hash = input.hash.as_deref().unwrap_or("unknown");
            println!("  {} ({})", input.path, hash);
        }
    }
    if !manifest.outputs.is_empty() {
        println!("Outputs:");
        for output in &manifest.outputs {
            println!("  {output}");
        }
    }
    if manifest.chunk_map.is_empty() {
        println!("Chunk map entries: 0");
    } else {
        println!("Chunk map:");
        for chunk in &manifest.chunk_map {
            let when = chunk.completed_at.as_deref().unwrap_or("pending");
            println!("  {} -> {} ({})", chunk.id, chunk.status, when);
        }
    }
}

fn run_list(root: &Path, format: RunFormat) -> anyhow::Result<()> {
    let records = discover_runs(root)?;
    match format {
        RunFormat::Plain => print_run_table(&records),
        RunFormat::Json => print_run_json(&records),
    }
}

fn print_run_table(records: &[RunRecord]) -> anyhow::Result<()> {
    let mut writer = TabWriter::new(io::stdout());
    writeln!(writer, "RUN ID\tCOMMAND\tTIMESTAMP\tVERSION\tMANIFEST")?;
    for record in records {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}",
            record.manifest.run_id,
            record.manifest.command,
            record.manifest.timestamp,
            record.manifest.version,
            record.path.display()
        )?;
    }
    writer.flush()?;
    Ok(())
}

fn print_run_json(records: &[RunRecord]) -> anyhow::Result<()> {
    let runs = summaries(records);
    serde_json::to_writer_pretty(io::stdout(), &runs)
        .map_err(|err| anyhow::anyhow!("serializing run list to JSON: {err}"))?;
    println!();
    Ok(())
}

fn generate_completions(shell: Shell, out: Option<&Path>) -> anyhow::Result<()> {
    let mut cmd = build_cli_command();
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        generate(shell, &mut cmd, "gat", &mut file);
        println!("Wrote {shell:?} completion to {}", path.display());
    } else {
        let stdout = &mut io::stdout();
        generate(shell, &mut cmd, "gat", stdout);
    }
    Ok(())
}

#[cfg(feature = "tui")]
const TUI_CONFIG_TEMPLATE: &str = "\
poll_secs=1
solver=gauss
verbose=false
command=cargo run -p gat-cli -- --help
";

#[cfg(feature = "tui")]
fn default_tui_config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("gat-tui").join("config.toml"))
}

#[cfg(feature = "tui")]
fn write_tui_config(out: Option<&str>) -> anyhow::Result<PathBuf> {
    let target = out
        .map(PathBuf::from)
        .or_else(default_tui_config_path)
        .ok_or_else(|| anyhow::anyhow!("unable to determine gat-tui config path"))?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, TUI_CONFIG_TEMPLATE)?;
    Ok(target)
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
                                println!("Graph statistics for {grid_file}:");
                                println!("  Nodes         : {}", stats.node_count);
                                println!("  Edges         : {}", stats.edge_count);
                                println!("  Components    : {}", stats.connected_components);
                                println!(
                                    "  Degree [min/avg/max]: {}/{:.2}/{}",
                                    stats.min_degree, stats.avg_degree, stats.max_degree
                                );
                                println!("  Density       : {:.4}", stats.density);
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
                GraphCommands::Export {
                    grid_file,
                    format,
                    out,
                } => {
                    info!("Exporting graph from {} in {} format", grid_file, format);
                    match importers::load_grid_from_arrow(grid_file.as_str()) {
                        Ok(network) => match graph_utils::export_graph(&network, format) {
                            Ok(dot) => {
                                if let Some(path) = out {
                                    if let Err(e) = fs::write(path, &dot) {
                                        Err(anyhow::anyhow!("writing graph export to {path}: {e}"))
                                    } else {
                                        println!("Graph exported to {path}");
                                        Ok(())
                                    }
                                } else {
                                    println!("{dot}");
                                    Ok(())
                                }
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    }
                }
                #[cfg(feature = "viz")]
                GraphCommands::Visualize {
                    grid_file,
                    iterations,
                    out,
                } => (|| -> anyhow::Result<()> {
                    info!(
                        "Visualizing graph {} (iterations {})",
                        grid_file, iterations
                    );
                    let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                    let layout = layout_network(&network, *iterations);
                    let payload = serde_json::to_string_pretty(&layout)
                        .map_err(|err| anyhow::anyhow!("serializing layout to JSON: {err}"))?;
                    if let Some(path) = out {
                        fs::write(path, &payload)
                            .map_err(|err| anyhow::anyhow!("writing layout to {path}: {err}"))?;
                        println!("Layout written to {path}");
                    } else {
                        println!("{payload}");
                    }
                    Ok(())
                })(),
            };

            match result {
                Ok(_) => info!("Graph command successful!"),
                Err(e) => error!("Graph command failed: {:?}", e),
            }
        }
        Some(Commands::Completions { shell, out }) => {
            let result = generate_completions(*shell, out.as_deref());
            match result {
                Ok(_) => info!("Completions generated"),
                Err(e) => error!("Completions generation failed: {:?}", e),
            }
        }
        Some(Commands::Pf { command }) => {
            let result = pf::handle(command);
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
                    threads,
                    solver,
                    out_partitions,
                } => {
                    configure_threads(threads);
                    info!(
                        "Running N-1 DC on {} with contingencies {} -> {}",
                        grid_file, contingencies, out
                    );
                    (|| -> anyhow::Result<()> {
                        let solver_kind = solver.parse::<SolverKind>()?;
                        let solver_impl = solver_kind.build_solver();
                        let partitions = parse_partitions(out_partitions.as_ref());
                        let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                        let out_path = Path::new(out);
                        let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                            Ok(network) => power_flow::n_minus_one_dc(
                                &network,
                                Arc::clone(&solver_impl),
                                contingencies,
                                out_path,
                                &partitions,
                                branch_limits.as_deref(),
                            ),
                            Err(e) => Err(e),
                        };
                        if res.is_ok() {
                            record_run(
                                out,
                                "nminus1 dc",
                                &[
                                    ("grid_file", grid_file),
                                    ("threads", threads),
                                    ("branch_limits", branch_limits.as_deref().unwrap_or("none")),
                                    ("out", out),
                                    ("solver", solver_kind.as_str()),
                                    ("out_partitions", partition_spec.as_str()),
                                ],
                            );
                        }
                        res
                    })()
                }
            };

            match result {
                Ok(_) => info!("N-1 command successful!"),
                Err(e) => error!("N-1 command failed: {:?}", e),
            }
        }
        Some(Commands::Ts { command }) => {
            let result = ts::handle(command);
            match result {
                Ok(_) => info!("Timeseries command successful!"),
                Err(e) => error!("Timeseries command failed: {:?}", e),
            }
        }
        Some(Commands::Dist { command }) => {
            let result = dist::handle(command);
            match result {
                Ok(_) => info!("Dist command successful!"),
                Err(e) => error!("Dist command failed: {:?}", e),
            }
        }
        Some(Commands::Derms { command }) => {
            let result = derms::handle(command);
            match result {
                Ok(_) => info!("DERMS command successful!"),
                Err(e) => error!("DERMS command failed: {:?}", e),
            }
        }
        Some(Commands::Adms { command }) => {
            let result = adms::handle(command);
            match result {
                Ok(_) => info!("ADMS command successful!"),
                Err(e) => error!("ADMS command failed: {:?}", e),
            }
        }
        Some(Commands::Se { command }) => {
            let result = se::handle(command);
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
        #[cfg(feature = "gui")]
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
                            println!("{case}");
                        }
                        Ok(())
                    }
                    HirenCommands::Fetch { case, out } => fetch_hiren(case, Path::new(&out)),
                },
                DatasetCommands::Dsgrid { out } => import_dsgrid(Path::new(&out)),
                DatasetCommands::Sup3rcc { command } => match command {
                    Sup3rccCommands::Fetch { out } => fetch_sup3rcc(Path::new(&out)),
                    Sup3rccCommands::Sample { grid, out } => {
                        sample_sup3rcc_grid(Path::new(&grid), Path::new(&out))
                    }
                },
                // Dataset catalog helpers plug directly into the public-fetch functions we added above.
                DatasetCommands::Public { command } => match command {
                    PublicDatasetCommands::List { tag, query } => {
                        let filter = PublicDatasetFilter {
                            tag: tag.clone(),
                            query: query.clone(),
                        };
                        list_public_datasets(&filter)
                    }
                    PublicDatasetCommands::Describe { id } => describe_public_dataset(id),
                    PublicDatasetCommands::Fetch {
                        id,
                        out,
                        force,
                        extract,
                    } => fetch_public_dataset(id, out.as_deref().map(Path::new), *extract, *force)
                        .map(|_| ()),
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
        Some(Commands::Analytics { command }) => {
            let result = analytics::handle(command);
            match result {
                Ok(_) => info!("Analytics command successful!"),
                Err(e) => error!("Analytics command failed: {:?}", e),
            }
        }
        Some(Commands::Runs { command }) => {
            let result = match command {
                RunsCommands::List { root, format } => run_list(root.as_path(), *format),
                RunsCommands::Describe {
                    target,
                    root,
                    format,
                } => (|| -> anyhow::Result<()> {
                    let record = resolve_manifest(root.as_path(), target.as_str())?;
                    match format {
                        RunFormat::Plain => describe_manifest(&record.manifest),
                        RunFormat::Json => {
                            serde_json::to_writer_pretty(io::stdout(), &record.manifest)
                                .map_err(|err| anyhow::anyhow!("serializing manifest: {err}"))?;
                            println!();
                        }
                    }
                    Ok(())
                })(),
                RunsCommands::Resume {
                    root,
                    manifest,
                    execute,
                } => (|| -> anyhow::Result<()> {
                    let record = resolve_manifest(root.as_path(), manifest.as_str())?;
                    describe_manifest(&record.manifest);
                    if *execute {
                        resume_manifest(&record.manifest)?;
                        println!("Manifest {} resumed", record.manifest.run_id);
                    } else {
                        println!("Manifest {} ready (not executed)", record.manifest.run_id);
                    }
                    Ok(())
                })(),
            };
            match result {
                Ok(_) => info!("Runs command successful!"),
                Err(e) => error!("Runs command failed: {:?}", e),
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
                    threads,
                    solver,
                    lp_solver,
                    out_partitions,
                } => {
                    configure_threads(threads);
                    info!(
                        "Running DC OPF on {} with cost {} and limits {} -> {}",
                        grid_file, cost, limits, out
                    );
                    (|| -> anyhow::Result<()> {
                        let solver_kind = solver.parse::<SolverKind>()?;
                        let solver_impl = solver_kind.build_solver();
                        let lp_solver_kind = lp_solver.parse::<LpSolverKind>()?;
                        let partitions = parse_partitions(out_partitions.as_ref());
                        let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                        let out_path = Path::new(out);
                        let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                            Ok(network) => power_flow::dc_optimal_power_flow(
                                &network,
                                solver_impl.as_ref(),
                                cost.as_str(),
                                limits.as_str(),
                                out_path,
                                &partitions,
                                branch_limits.as_deref(),
                                piecewise.as_deref(),
                                &lp_solver_kind,
                            ),
                            Err(e) => Err(e),
                        };
                        if res.is_ok() {
                            record_run(
                                out,
                                "opf dc",
                                &[
                                    ("grid_file", grid_file),
                                    ("threads", threads),
                                    ("solver", solver_kind.as_str()),
                                    ("lp_solver", lp_solver_kind.as_str()),
                                    ("out_partitions", partition_spec.as_str()),
                                ],
                            );
                        }
                        res
                    })()
                }
                OpfCommands::Ac {
                    grid_file,
                    out,
                    tol,
                    max_iter,
                    threads,
                    solver,
                    out_partitions,
                } => {
                    configure_threads(threads);
                    info!(
                        "Running AC OPF on {} with tol {}, max_iter {} -> {}",
                        grid_file, tol, max_iter, out
                    );
                    (|| -> anyhow::Result<()> {
                        let solver_kind = solver.parse::<SolverKind>()?;
                        let solver_impl = solver_kind.build_solver();
                        let partitions = parse_partitions(out_partitions.as_ref());
                        let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                        let out_path = Path::new(out);
                        let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                            Ok(network) => power_flow::ac_optimal_power_flow(
                                &network,
                                solver_impl.as_ref(),
                                *tol,
                                *max_iter,
                                out_path,
                                &partitions,
                            ),
                            Err(e) => Err(e),
                        };
                        if res.is_ok() {
                            record_run(
                                out,
                                "opf ac",
                                &[
                                    ("grid_file", grid_file),
                                    ("threads", threads),
                                    ("tol", &tol.to_string()),
                                    ("max_iter", &max_iter.to_string()),
                                    ("solver", solver_kind.as_str()),
                                    ("out_partitions", partition_spec.as_str()),
                                ],
                            );
                        }
                        res
                    })()
                }
            };

            match result {
                Ok(_) => info!("OPF command successful!"),
                Err(e) => error!("OPF command failed: {:?}", e),
            }
        }
        #[cfg(feature = "tui")]
        Some(Commands::Tui { command }) => match command {
            TuiCommands::Config { out } => match write_tui_config(out.as_deref()) {
                Ok(path) => info!("gat-tui config written to {}", path.display()),
                Err(err) => error!("failed to write gat-tui config: {}", err),
            },
        },
        None => {
            info!("No subcommand provided. Use `gat --help` for more information.");
        }
    }
}
