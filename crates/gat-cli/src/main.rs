use clap::Parser;
use clap_complete::{generate, Shell};
use dirs::config_dir;
use gat_algo::{power_flow, LpSolverKind};
use gat_core::{graph_utils, solver::SolverKind};
use gat_io::{importers, validate};
use rayon::ThreadPoolBuilder;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tabwriter::TabWriter;
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber; // Added power_flow
mod dataset;
mod runs;
use dataset::*;
use gat_cli::{
    cli::{
        build_cli_command, AnalyticsCommands, Cli, Commands, DatasetCommands, GraphCommands,
        GuiCommands, HirenCommands, ImportCommands, Nminus1Commands, OpfCommands,
        PowerFlowCommands, PublicDatasetCommands, RtsGmlcCommands, RunFormat, RunsCommands,
        SeCommands, Sup3rccCommands, TsCommands, TuiCommands, VizCommands,
    },
    manifest,
};
use gat_viz::layout::layout_network;
use manifest::{record_manifest, ManifestEntry};
use runs::{discover_runs, resolve_manifest, summaries, RunRecord};
use serde_json;

fn configure_threads(spec: &str) {
    let count = if spec.eq_ignore_ascii_case("auto") {
        num_cpus::get()
    } else {
        spec.parse().unwrap_or_else(|_| num_cpus::get())
    };
    let _ = ThreadPoolBuilder::new().num_threads(count).build_global();
}

fn record_run(out: &str, command: &str, params: &[(&str, &str)]) {
    if let Err(err) = record_manifest(Path::new(out), command, params) {
        eprintln!("Failed to record run manifest: {err}");
    }
}

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
        return Err(anyhow::anyhow!("resumed run failed with {}", status));
    }
    Ok(())
}

fn parse_partitions(spec: Option<&String>) -> Vec<String> {
    spec.map_or("", String::as_str)
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn describe_manifest(manifest: &ManifestEntry) {
    println!(
        "Manifest {} (cmd: `{}` @ v{} from {})",
        manifest.run_id, manifest.command, manifest.version, manifest.timestamp
    );
    if let Some(seed) = &manifest.seed {
        println!("Seed: {}", seed);
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
            println!("  {}", output);
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

const TUI_CONFIG_TEMPLATE: &str = "\
poll_secs=1
solver=gauss
verbose=false
command=cargo run -p gat-cli -- --help
";

fn default_tui_config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("gat-tui").join("config.toml"))
}

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
                                        Err(anyhow::anyhow!(
                                            "writing graph export to {}: {e}",
                                            path
                                        ))
                                    } else {
                                        println!("Graph exported to {}", path);
                                        Ok(())
                                    }
                                } else {
                                    println!("{}", dot);
                                    Ok(())
                                }
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    }
                }
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
                            .map_err(|err| anyhow::anyhow!("writing layout to {}: {err}", path))?;
                        println!("Layout written to {}", path);
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
            let result = match command {
                PowerFlowCommands::Dc {
                    grid_file,
                    out,
                    threads,
                    solver,
                    lp_solver,
                    out_partitions,
                } => {
                    configure_threads(threads);
                    info!("Running DC power flow on {} to {}", grid_file, out);
                    (|| -> anyhow::Result<()> {
                        let solver_kind = solver.parse::<SolverKind>()?;
                        let solver_impl = solver_kind.build_solver();
                        let _ = lp_solver;
                        let partitions = parse_partitions(out_partitions.as_ref());
                        let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                        let out_path = Path::new(out);
                        let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                            Ok(network) => power_flow::dc_power_flow(
                                &network,
                                solver_impl.as_ref(),
                                out_path,
                                &partitions,
                            ),
                            Err(e) => Err(e),
                        };
                        if res.is_ok() {
                            record_run(
                                out,
                                "pf dc",
                                &[
                                    ("grid_file", grid_file),
                                    ("out", out),
                                    ("threads", threads),
                                    ("solver", solver_kind.as_str()),
                                    ("out_partitions", partition_spec.as_str()),
                                ],
                            );
                        }
                        res
                    })()
                }
                PowerFlowCommands::Ac {
                    grid_file,
                    out,
                    tol,
                    max_iter,
                    threads,
                    solver,
                    lp_solver,
                    out_partitions,
                } => {
                    configure_threads(threads);
                    info!(
                        "Running AC power flow on {} with tol {} and max_iter {}",
                        grid_file, tol, max_iter
                    );
                    (|| -> anyhow::Result<()> {
                        let solver_kind = solver.parse::<SolverKind>()?;
                        let solver_impl = solver_kind.build_solver();
                        let _ = lp_solver;
                        let partitions = parse_partitions(out_partitions.as_ref());
                        let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                        let out_path = Path::new(out);
                        let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                            Ok(network) => power_flow::ac_power_flow(
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
                                "pf ac",
                                &[
                                    ("grid_file", grid_file),
                                    ("threads", threads),
                                    ("out", out),
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
            let result = match command {
                TsCommands::Resample {
                    input,
                    timestamp,
                    value,
                    rule,
                    out,
                    out_partitions,
                } => {
                    info!(
                        "Resampling {} ({}/{}) every {} -> {}",
                        input, timestamp, value, rule, out
                    );
                    let partitions = parse_partitions(out_partitions.as_ref());
                    let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                    let res = gat_ts::resample_timeseries(
                        input,
                        timestamp,
                        value,
                        rule,
                        out,
                        &partitions,
                    );
                    if res.is_ok() {
                        record_run(
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
                        );
                    }
                    res
                }
                TsCommands::Join {
                    left,
                    right,
                    on,
                    out,
                    out_partitions,
                } => {
                    info!("Joining {} and {} on {} -> {}", left, right, on, out);
                    let partitions = parse_partitions(out_partitions.as_ref());
                    let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                    let res = gat_ts::join_timeseries(left, right, on, out, &partitions);
                    if res.is_ok() {
                        record_run(
                            out,
                            "ts join",
                            &[
                                ("left", left),
                                ("right", right),
                                ("on", on),
                                ("out", out),
                                ("out_partitions", partition_spec.as_str()),
                            ],
                        );
                    }
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
                    info!(
                        "Aggregating {} by {} ({}) using {} -> {}",
                        input, group, value, agg, out
                    );
                    let partitions = parse_partitions(out_partitions.as_ref());
                    let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                    let res =
                        gat_ts::aggregate_timeseries(input, group, value, agg, out, &partitions);
                    if res.is_ok() {
                        record_run(
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
                        );
                    }
                    res
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
                    threads,
                    solver,
                    out_partitions,
                    slack_bus,
                } => {
                    configure_threads(threads);
                    info!(
                        "Running WLS state estimation on {} using {} -> {}",
                        grid_file, measurements, out
                    );
                    (|| -> anyhow::Result<()> {
                        let solver_kind = solver.parse::<SolverKind>()?;
                        let solver_impl = solver_kind.build_solver();
                        let partitions = parse_partitions(out_partitions.as_ref());
                        let partition_spec = out_partitions.as_deref().unwrap_or("").to_string();
                        let out_path = Path::new(out);
                        let state_path = state_out.as_deref().map(Path::new);
                        let res = match importers::load_grid_from_arrow(grid_file.as_str()) {
                            Ok(network) => power_flow::state_estimation_wls(
                                &network,
                                solver_impl.as_ref(),
                                measurements,
                                out_path,
                                &partitions,
                                state_path,
                                *slack_bus,
                            ),
                            Err(e) => Err(e),
                        };
                        if res.is_ok() {
                            let slack_spec = slack_bus.map(|id| id.to_string());
                            record_run(
                                out,
                                "se wls",
                                &[
                                    ("grid_file", grid_file),
                                    ("measurements", measurements.as_str()),
                                    ("threads", threads),
                                    ("solver", solver_kind.as_str()),
                                    ("out_partitions", partition_spec.as_str()),
                                    ("slack_bus", slack_spec.as_deref().unwrap_or("auto")),
                                ],
                            );
                        }
                        res
                    })()
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
            let result = match command {
                AnalyticsCommands::Ptdf {
                    grid_file,
                    source,
                    sink,
                    transfer,
                    out,
                    out_partitions,
                    threads,
                    solver,
                } => {
                    configure_threads(threads);
                    info!(
                        "Running PTDF analysis {}→{} ({:.3} MW) on {}",
                        source, sink, transfer, grid_file
                    );
                    // Wrap the entire PTDF workflow in a closure so we can use `?` and log once.
                    let res = (|| -> anyhow::Result<()> {
                        let solver_kind = solver.parse::<SolverKind>()?;
                        let solver_impl = solver_kind.build_solver();
                        let partitions = parse_partitions(out_partitions.as_ref());
                        let partition_spec = out_partitions.as_deref().unwrap_or("");
                        let network = importers::load_grid_from_arrow(grid_file.as_str())?;
                        power_flow::ptdf_analysis(
                            &network,
                            solver_impl.as_ref(),
                            *source,
                            *sink,
                            *transfer,
                            Path::new(out),
                            &partitions,
                        )?;
                        record_run(
                            out,
                            "analytics ptdf",
                            &[
                                ("grid_file", grid_file),
                                ("source", &source.to_string()),
                                ("sink", &sink.to_string()),
                                ("transfer", &transfer.to_string()),
                                ("solver", solver_kind.as_str()),
                                ("out_partitions", partition_spec),
                            ],
                        );
                        Ok(())
                    })();
                    res
                }
            };

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
