use clap::{CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Set the logging level
    #[arg(long, default_value = "info")]
    pub log_level: tracing::Level,

    /// Set the profile (e.g., "dev", "release")
    #[arg(long, default_value = "dev")]
    pub profile: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
    /// Generate shell completion scripts
    Completions {
        /// Shell type
        #[arg(value_enum)]
        shell: Shell,
        /// Write output to a file instead of stdout
        #[arg(short, long)]
        out: Option<PathBuf>,
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
    #[cfg(feature = "viz")]
    Viz {
        #[command(subcommand)]
        command: VizCommands,
    },
    /// Grid analytics helpers (PTDF, sensitivities, etc.)
    Analytics {
        #[command(subcommand)]
        command: AnalyticsCommands,
    },
    /// GUI dashboard
    #[cfg(feature = "gui")]
    Gui {
        #[command(subcommand)]
        command: GuiCommands,
    },
    /// Run management
    Runs {
        #[command(subcommand)]
        command: RunsCommands,
    },
    /// Dataset adapters
    Dataset {
        #[command(subcommand)]
        command: DatasetCommands,
    },
    /// Helpers for the terminal dashboard
    #[cfg(feature = "tui")]
    Tui {
        #[command(subcommand)]
        command: TuiCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum ImportCommands {
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
    /// Import CIM RDF file
    Cim {
        /// Path to the CIM RDF file
        #[arg(long)]
        rdf: String,
        /// Output file path (Arrow format)
        #[arg(short, long)]
        output: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum GraphCommands {
    /// Graph stats summary
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
        #[arg(long, default_value = "graphviz")]
        format: String,
        /// Optional output file path
        #[arg(short, long)]
        out: Option<String>,
    },
    #[cfg(feature = "viz")]
    /// Compute force-directed layout for visualization
    Visualize {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Number of simulation iterations
        #[arg(long, default_value_t = 150)]
        iterations: usize,
        /// Optional output file path
        #[arg(short, long)]
        out: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum PowerFlowCommands {
    /// Run DC power flow
    Dc {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Output file path for flows (Parquet format)
        #[arg(short, long)]
        out: String,
        /// Threading hint (`auto` or integer)
        #[arg(long, default_value = "auto")]
        threads: String,
        /// Solver to use (gauss, faer)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// LP solver for the cost minimization (clarabel, coin_cbc, highs)
        #[arg(long, default_value = "clarabel")]
        lp_solver: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
    },
    /// Run AC power flow
    Ac {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Output file path for flows (Parquet format)
        #[arg(short, long)]
        out: String,
        /// Tolerance for convergence
        #[arg(long, default_value = "1e-8")]
        tol: f64,
        /// Maximum number of iterations
        #[arg(long, default_value = "20")]
        max_iter: u32,
        /// Threading hint (`auto` or integer)
        #[arg(long, default_value = "auto")]
        threads: String,
        /// Solver to use (gauss, faer)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// LP solver for the cost minimization (clarabel, coin_cbc, highs)
        #[arg(long, default_value = "clarabel")]
        lp_solver: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum Nminus1Commands {
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
        /// Threads: `auto` or numeric
        #[arg(long, default_value = "auto")]
        threads: String,
        /// Solver to use (gauss, faer)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum TsCommands {
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
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
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
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
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
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum DatasetCommands {
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
    /// Public dataset catalog
    Public {
        #[command(subcommand)]
        command: PublicDatasetCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum AnalyticsCommands {
    /// PTDF sensitivity for a source→sink transfer
    Ptdf {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Injection bus ID
        #[arg(long)]
        source: usize,
        /// Withdrawal bus ID
        #[arg(long)]
        sink: usize,
        /// Transfer size in MW (defaults to 1 MW)
        #[arg(long, default_value = "1.0")]
        transfer: f64,
        /// Output file path for branch PTDF table (Parquet)
        #[arg(short, long, default_value = "ptdf.parquet")]
        out: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
        /// Threading hint (`auto` or integer)
        #[arg(long, default_value = "auto")]
        threads: String,
        /// Solver to use (gauss, faer, etc.)
        #[arg(long, default_value = "gauss")]
        solver: String,
    },
}

#[derive(Subcommand, Debug)]
#[cfg(feature = "tui")]
pub enum TuiCommands {
    /// Write a default gat-tui config file
    Config {
        /// Output path, defaults to ~/.config/gat-tui/config.toml
        #[arg(short, long)]
        out: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum RunsCommands {
    /// List recorded runs
    List {
        /// Root path(s) to scan for run manifests
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Output format for the listing
        #[arg(long, value_enum, default_value_t = RunFormat::Plain)]
        format: RunFormat,
    },
    /// Describe a recorded run
    Describe {
        /// Manifest path or run_id alias
        target: String,
        /// Root path where manifests are scanned (used when target is a run_id)
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Output format for the description
        #[arg(long, value_enum, default_value_t = RunFormat::Plain)]
        format: RunFormat,
    },
    /// Resume a long run from a manifest
    Resume {
        /// Root path where manifests are scanned (used when the manifest argument is a run_id)
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Manifest JSON path or run_id alias
        manifest: String,
        /// Actually re-run the command recorded in the manifest
        #[arg(long)]
        execute: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum RtsGmlcCommands {
    /// Fetch release copy
    Fetch {
        #[arg(short, long, default_value = "data/rts-gmlc")]
        out: String,
        #[arg(long)]
        tag: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum HirenCommands {
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
pub enum Sup3rccCommands {
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
pub enum PublicDatasetCommands {
    /// List curated public datasets we know how to fetch
    List {
        /// Filter datasets by tag
        #[arg(long)]
        tag: Option<String>,
        /// Search term that matches dataset id or description
        #[arg(long)]
        query: Option<String>,
    },
    /// Show metadata about a curated dataset
    Describe {
        /// Dataset ID (see `gat dataset public list`)
        id: String,
    },
    /// Fetch a curated dataset by ID
    Fetch {
        /// Dataset ID (see `gat dataset public list`)
        id: String,
        /// Directory to stage the download (defaults to ~/.cache/gat/datasets or data/public)
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        out: Option<String>,
        /// Force re-download if the file already exists
        #[arg(long)]
        force: bool,
        /// Try to extract the dataset if it's a zip archive
        #[arg(long)]
        extract: bool,
    },
}

#[derive(Subcommand, Debug)]
#[cfg(feature = "viz")]
pub enum VizCommands {
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
#[cfg(feature = "gui")]
pub enum GuiCommands {
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
pub enum OpfCommands {
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
        /// Threading hint (`auto` or integer)
        #[arg(long, default_value = "auto")]
        threads: String,
        /// Solver to use (gauss, faer)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// LP solver for the cost minimization (clarabel, coin_cbc, highs)
        #[arg(long, default_value = "clarabel")]
        lp_solver: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
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
        /// Threading hint (`auto` or integer)
        #[arg(long, default_value = "auto")]
        threads: String,
        /// Solver to use (gauss, faer)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum SeCommands {
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
        /// Threading hint (`auto` or integer)
        #[arg(long, default_value = "auto")]
        threads: String,
        /// Solver to use (gauss, faer)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
        /// Slack bus ID (defaults to lowest bus ID)
        #[arg(long)]
        slack_bus: Option<usize>,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum RunFormat {
    Plain,
    Json,
}

pub fn build_cli_command() -> clap::Command {
    Cli::command()
}
