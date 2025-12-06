use clap::{CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::Shell;
use gat_io::importers::Format;
use std::path::PathBuf;

use crate::common::OutputFormat;

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
        /// Increase verbosity (-v for warnings summary, -vv for line details)
        #[arg(short, long, action = clap::ArgAction::Count, global = true)]
        verbose: u8,
        /// Fail if any warnings are encountered
        #[arg(long, global = true)]
        strict: bool,
        /// Run post-import validation (topology, references, physical sanity)
        #[arg(long, global = true)]
        validate: bool,
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
    /// Scenario definitions and materialization workflows
    Scenarios {
        #[command(subcommand)]
        command: ScenariosCommands,
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
    /// Inspect the local environment and report common setup issues
    Doctor {},
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
    /// Distribution modeling helpers
    Dist {
        #[command(subcommand)]
        command: DistCommands,
    },
    /// DERMS analytics
    Derms {
        #[command(subcommand)]
        command: DermsCommands,
    },
    /// ADMS reliability workflows
    Adms {
        #[command(subcommand)]
        command: AdmsCommands,
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
    /// Native solver management (install, list, status)
    Solver {
        #[command(subcommand)]
        command: SolverCommands,
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
    /// Feature extraction for ML models (GNN, KPI predictors, etc.)
    Featurize {
        #[command(subcommand)]
        command: FeaturizeCommands,
    },
    /// Geo-spatial tools (GIS joins, polygon mapping, spatial features)
    Geo {
        #[command(subcommand)]
        command: GeoCommands,
    },
    /// Allocation and settlement tools (congestion rents, cost attribution)
    Alloc {
        #[command(subcommand)]
        command: AllocCommands,
    },
    /// Scenario batch runners for PF/OPF (CANOS-style fan-out)
    Batch {
        #[command(subcommand)]
        command: BatchCommands,
    },
    /// Benchmarking suites for OPF/PF solvers
    Benchmark {
        #[command(subcommand)]
        command: BenchmarkCommands,
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
    /// Release version helpers
    Version {
        #[command(subcommand)]
        command: VersionCommands,
    },
    /// Convert between power system formats via the Arrow schema
    Convert {
        #[command(subcommand)]
        command: ConvertCommands,
    },
    /// Deep-dive network inspection and analysis
    Inspect {
        #[command(subcommand)]
        command: InspectCommands,
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
    /// Auto-detect format and import (based on file extension and content)
    Auto {
        /// Path to the input file
        input: String,
        /// Output directory path (Arrow format). Defaults to input filename with .arrow extension.
        #[arg(short = 'o', long = "out", visible_alias = "output")]
        out: Option<String>,
    },
    /// Import PSS/E RAW file
    Psse {
        /// Path to the RAW file
        #[arg(long)]
        raw: String,
        /// Output directory path (Arrow format). Defaults to input filename with .arrow extension.
        #[arg(short = 'o', long = "out", visible_alias = "output")]
        out: Option<String>,
    },
    /// Import MATPOWER case file
    Matpower {
        /// Path to the MATPOWER .m file
        #[arg(long)]
        m: String,
        /// Output directory path (Arrow format). Defaults to input filename with .arrow extension.
        #[arg(short = 'o', long = "out", visible_alias = "output")]
        out: Option<String>,
    },
    /// Import CIM RDF file
    Cim {
        /// Path to the CIM RDF file
        #[arg(long)]
        rdf: String,
        /// Output directory path (Arrow format). Defaults to input filename with .arrow extension.
        #[arg(short = 'o', long = "out", visible_alias = "output")]
        out: Option<String>,
    },
    /// Import pandapower JSON file
    Pandapower {
        /// Path to the pandapower JSON file
        #[arg(long)]
        json: String,
        /// Output directory path (Arrow format). Defaults to input filename with .arrow extension.
        #[arg(short = 'o', long = "out", visible_alias = "output")]
        out: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConvertCommands {
    /// Convert between supported formats using Arrow as an intermediate representation
    Format {
        /// Path to the input file or Arrow directory
        input: String,
        /// Input format override (auto-detect by default)
        #[arg(long, value_enum)]
        from: Option<ConvertFormat>,
        /// Target format for conversion
        #[arg(long, value_enum)]
        to: ConvertFormat,
        /// Output directory/file. Defaults to input filename with target format extension.
        #[arg(short = 'o', long = "out", visible_alias = "output")]
        out: Option<String>,
        /// Overwrite existing output without prompting
        #[arg(long)]
        force: bool,
        /// Show import diagnostics (warnings about defaults, validation issues)
        #[arg(short, long)]
        verbose: bool,
        /// Fail if any import warnings or errors occur (for CI pipelines)
        #[arg(long)]
        strict: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ConvertFormat {
    /// Arrow directory
    Arrow,
    /// MATPOWER case
    Matpower,
    /// PSS/E RAW file
    Psse,
    /// CIM RDF/XML file
    Cim,
    /// pandapower JSON file
    Pandapower,
    /// PowerModels.jl JSON file
    Powermodels,
}

impl ConvertFormat {
    pub fn to_import_format(self) -> Option<Format> {
        match self {
            ConvertFormat::Arrow => None,
            ConvertFormat::Matpower => Some(Format::Matpower),
            ConvertFormat::Psse => Some(Format::Psse),
            ConvertFormat::Cim => Some(Format::Cim),
            ConvertFormat::Pandapower => Some(Format::Pandapower),
            ConvertFormat::Powermodels => Some(Format::PowerModels),
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum InspectCommands {
    /// Show network summary statistics (buses, branches, generators, loads)
    Summary {
        /// Path to Arrow directory or importable file
        input: String,
    },
    /// List all generators with their bus assignments and limits
    Generators {
        /// Path to Arrow directory or importable file
        input: String,
        /// Filter by bus ID
        #[arg(long)]
        bus: Option<usize>,
        /// Output format
        #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Table)]
        format: OutputFormat,
    },
    /// List all branches with their endpoints and parameters
    Branches {
        /// Path to Arrow directory or importable file
        input: String,
        /// Filter by rating less than threshold (MVA)
        #[arg(long)]
        rating_lt: Option<f64>,
        /// Output format
        #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Table)]
        format: OutputFormat,
    },
    /// Show power balance analysis (total generation capacity vs load)
    PowerBalance {
        /// Path to Arrow directory or importable file
        input: String,
    },
    /// Dump network data as JSON (for scripting)
    Json {
        /// Path to Arrow directory or importable file
        input: String,
        /// Pretty-print the JSON output
        #[arg(long)]
        pretty: bool,
    },
    /// Analyze branch thermal limits to identify potential bottlenecks
    Thermal {
        /// Path to Arrow directory or importable file
        input: String,
        /// Only show branches with rating below this threshold (MVA)
        #[arg(long)]
        threshold: Option<f64>,
        /// Output format
        #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Table)]
        format: OutputFormat,
    },
}

#[derive(Subcommand, Debug)]
pub enum ScenariosCommands {
    /// Validate a scenario specification (YAML/JSON)
    Validate {
        /// Path to the scenario spec template
        #[arg(long, value_hint = ValueHint::FilePath)]
        spec: String,
    },
    /// List normalized scenarios inside a spec
    List {
        /// Path to the scenario spec
        #[arg(long, value_hint = ValueHint::FilePath)]
        spec: String,
        /// Output format
        #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Table)]
        format: OutputFormat,
    },
    /// Expand templated scenarios into fully resolved definitions
    Expand {
        /// Path to the scenario spec
        #[arg(long, value_hint = ValueHint::FilePath)]
        spec: String,
        /// Optional base grid file to override the spec
        #[arg(long, value_hint = ValueHint::FilePath)]
        grid_file: Option<String>,
        /// Path for the expanded output (JSON or YAML)
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
    },
    /// Materialize per-scenario grids and produce a manifest
    Materialize {
        /// Path to the scenario spec
        #[arg(long, value_hint = ValueHint::FilePath)]
        spec: String,
        /// Optional base grid file (overrides spec)
        #[arg(long, value_hint = ValueHint::FilePath)]
        grid_file: Option<String>,
        /// Directory where scenario grids and manifest are written
        #[arg(short = 'd', long = "out-dir", visible_alias = "output-dir", value_hint = ValueHint::DirPath)]
        out_dir: String,
        /// Drop outaged components from the exported grids
        #[arg(long, default_value_t = true)]
        drop_outaged: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum BatchCommands {
    /// Run DC/AC PF for every scenario in a manifest (CANOS-style fan-out).
    ///
    /// Executes power flow analysis in parallel across all scenarios defined in the manifest.
    /// Produces a batch_manifest.json with per-job timing statistics (min/max/mean/median/p95)
    /// and a rich console summary showing job counts, timing distribution, and any failures.
    /// See doi:10.1109/TPWRS.2007.899019 for DC power flow fundamentals.
    Pf {
        /// Scenario manifest JSON generated by `gat scenarios materialize`
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: String,
        /// Output directory root for job outputs
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        out: String,
        /// Flow mode (`dc` or `ac`)
        #[arg(long, default_value = "dc")]
        mode: String,
        /// Linear solver (`gauss`/`faer`, etc.)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Threading hint for global Rayon pool
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
        /// Maximum number of jobs to execute in parallel (0 = auto)
        #[arg(long, default_value_t = 0)]
        max_jobs: usize,
        /// Partition columns for Parquet outputs (optional)
        #[arg(long)]
        out_partitions: Option<String>,
        /// AC tolerance in per unit
        #[arg(long, default_value_t = 1e-6)]
        tol: f64,
        /// Maximum AC solver iterations
        #[arg(long, default_value_t = 50)]
        max_iter: usize,
    },
    /// Run DC/AC OPF for every scenario (CANOS-ready reliability stats).
    ///
    /// Executes optimal power flow in parallel across all scenarios. Produces a batch_manifest.json
    /// with per-job timing statistics (min/max/mean/median/p95), solver iterations, and convergence
    /// status. Console output includes a rich summary with job counts, timing distribution,
    /// AC solver statistics when applicable, and failed job details.
    Opf {
        /// Scenario manifest JSON
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: String,
        /// Output directory root
        #[arg(short, long, value_hint = ValueHint::DirPath)]
        out: String,
        /// OPF mode (`dc` or `ac`)
        #[arg(long, default_value = "dc")]
        mode: String,
        /// Main solver (gauss/faer)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// LP solver for DC OPF
        #[arg(long, default_value = "clarabel")]
        lp_solver: String,
        /// Cost CSV (required for DC OPF)
        #[arg(long, value_hint = ValueHint::FilePath, default_value = "")]
        cost: String,
        /// Limits CSV (required for DC OPF)
        #[arg(long, value_hint = ValueHint::FilePath, default_value = "")]
        limits: String,
        /// Optional branch limits
        #[arg(long, value_hint = ValueHint::FilePath)]
        branch_limits: Option<String>,
        /// Optional piecewise cost segments
        #[arg(long, value_hint = ValueHint::FilePath)]
        piecewise: Option<String>,
        /// Threading hint for global pool
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
        /// Maximum concurrent OPF jobs (0 = auto)
        #[arg(long, default_value_t = 0)]
        max_jobs: usize,
        /// Partition columns for Parquet outputs
        #[arg(long)]
        out_partitions: Option<String>,
        /// Iteration tolerance
        #[arg(long, default_value_t = 1e-6)]
        tol: f64,
        /// Maximum iterations
        #[arg(long, default_value_t = 50)]
        max_iter: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum BenchmarkCommands {
    /// Run PFDelta AC OPF benchmark suite
    Pfdelta {
        /// Root directory containing PFDelta dataset
        #[arg(long, value_hint = ValueHint::DirPath)]
        pfdelta_root: String,
        /// Specific test case to benchmark (14, 30, 57, 118, 500, 2000)
        #[arg(long)]
        case: Option<String>,
        /// Contingency type to run (n, n-1, n-2, or all)
        #[arg(long, default_value = "all")]
        contingency: String,
        /// Maximum number of test cases to run (0 = all)
        #[arg(long, default_value_t = 0)]
        max_cases: usize,
        /// Output CSV path for results
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
        /// Number of parallel solver threads (auto = CPU count)
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
        /// Solve mode: pf (power flow) or opf (optimal power flow)
        #[arg(long, default_value = "opf")]
        mode: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum AC solver iterations
        #[arg(long, default_value_t = 20)]
        max_iter: u32,
        /// Path for JSONL diagnostics output (import warnings, validation issues)
        #[arg(long, value_hint = ValueHint::FilePath)]
        diagnostics_log: Option<String>,
        /// Fail if any case has import warnings (for CI quality gates)
        #[arg(long)]
        strict: bool,
    },
    /// Run PGLib-OPF benchmark suite (MATPOWER format)
    Pglib {
        /// Directory containing PGLib MATPOWER (.m) files
        #[arg(long, value_hint = ValueHint::DirPath)]
        pglib_dir: String,
        /// Optional baseline CSV for objective comparison
        #[arg(long, value_hint = ValueHint::FilePath)]
        baseline: Option<String>,
        /// Filter cases by name pattern (e.g., "case14", "case118")
        #[arg(long)]
        case_filter: Option<String>,
        /// Maximum number of cases to run (0 = all)
        #[arg(long, default_value_t = 0)]
        max_cases: usize,
        /// Output CSV path for results
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
        /// Number of parallel solver threads (auto = CPU count)
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
        /// OPF method: ac, socp, dc, economic (default: socp)
        #[arg(long, default_value = "socp")]
        method: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum AC solver iterations
        #[arg(long, default_value_t = 200)]
        max_iter: u32,
        /// Use enhanced SOCP (OBBT + QC envelopes for tighter relaxation)
        #[arg(long)]
        enhanced: bool,
        /// Native solver preference for AC-OPF: none, prefer, require
        /// - none: use pure Rust L-BFGS (default)
        /// - prefer: use IPOPT if available, fall back to L-BFGS
        /// - require: require IPOPT, fail if unavailable
        #[arg(long, default_value = "none")]
        solver: String,
    },
    /// Run OPFData benchmark suite (GNN-format JSON)
    Opfdata {
        /// Directory containing OPFData JSON files
        #[arg(long, value_hint = ValueHint::DirPath)]
        opfdata_dir: String,
        /// Filter samples by file path pattern
        #[arg(long)]
        case_filter: Option<String>,
        /// Maximum number of samples to run (0 = all)
        #[arg(long, default_value_t = 0)]
        max_cases: usize,
        /// Output CSV path for results
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
        /// Number of parallel solver threads (auto = CPU count)
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
        /// OPF method: ac, socp, dc, economic (default: socp)
        #[arg(long, default_value = "socp")]
        method: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum AC solver iterations
        #[arg(long, default_value_t = 200)]
        max_iter: u32,
        /// Path for JSONL diagnostics output (import warnings, validation issues)
        #[arg(long, value_hint = ValueHint::FilePath)]
        diagnostics_log: Option<String>,
        /// Fail if any sample has import warnings (for CI quality gates)
        #[arg(long)]
        strict: bool,
        /// Native solver preference for AC-OPF: none, prefer, require
        /// - none: use pure Rust L-BFGS (default)
        /// - prefer: use IPOPT if available, fall back to L-BFGS
        /// - require: require IPOPT, fail if unavailable
        #[arg(long, default_value = "none")]
        solver: String,
    },
    /// Display summary statistics from a benchmark CSV
    Summary {
        /// Path to benchmark CSV file
        #[arg(value_hint = ValueHint::FilePath)]
        csv: String,
    },
    /// Compare two benchmark CSV files to show improvements/regressions
    Compare {
        /// Path to "before" benchmark CSV
        #[arg(value_hint = ValueHint::FilePath)]
        before: String,
        /// Path to "after" benchmark CSV
        #[arg(value_hint = ValueHint::FilePath)]
        after: String,
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
    /// Validate network data for consistency and physical sanity
    Validate {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Enable strict mode (fail on any warnings)
        #[arg(long)]
        strict: bool,
        /// Skip topology checks (connectivity, islands)
        #[arg(long)]
        skip_topology: bool,
        /// Increase verbosity (-v for details)
        #[arg(short, long, action = clap::ArgAction::Count)]
        verbose: u8,
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
    /// Run DC power flow.
    ///
    /// Solves the linearized DC power flow equations (B'θ = P) and outputs branch flows
    /// to Parquet. Console output shows a rich summary with bus counts, generation/load
    /// totals, and branch flow statistics (range, max absolute flow).
    Dc {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Output file path for flows (Parquet format)
        #[arg(short, long)]
        out: String,
        /// Threading hint (`auto` or integer)
        #[arg(short = 't', long, default_value = "auto")]
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
    /// Run AC power flow.
    ///
    /// Solves the nonlinear AC power flow equations using iterative methods. Outputs
    /// branch flows to Parquet. Console output shows a rich summary with solver parameters,
    /// bus counts, generation/load totals, and branch flow statistics.
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
        #[arg(short = 't', long, default_value = "auto")]
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
        /// Enforce generator Q limits (PV-PQ bus switching)
        #[arg(long)]
        q_limits: bool,
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
        #[arg(short = 't', long, default_value = "auto")]
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
pub enum DistCommands {
    /// Import MATPOWER into distribution tables
    Import {
        /// Source MATPOWER case file
        #[arg(long)]
        m: String,
        /// Output directory for dist tables
        #[arg(short = 'd', long = "out-dir", visible_alias = "output-dir")]
        out_dir: String,
        /// Optional feeder identifier to annotate the tables
        #[arg(long)]
        feeder_id: Option<String>,
    },
    /// Run a distribution AC power flow
    Pf {
        /// Grid file (Arrow format)
        #[arg(long)]
        grid_file: String,
        /// Output Parquet path
        #[arg(long)]
        out: String,
        /// Solver (`gauss`, `clarabel`, `highs`)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum iterations
        #[arg(long, default_value = "20")]
        max_iter: u32,
    },
    /// Run a simple hosting-capacity OPF for distribution feeders
    Opf {
        /// Grid file (Arrow format)
        #[arg(long)]
        grid_file: String,
        /// Output Parquet path
        #[arg(long)]
        out: String,
        /// Objective descriptor
        #[arg(long, default_value = "loss")]
        objective: String,
        /// Solver to use
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum iterations
        #[arg(long, default_value = "20")]
        max_iter: u32,
    },
    /// Sweep hosting capacity over selected buses
    Hostcap {
        /// Grid file (Arrow format)
        #[arg(long)]
        grid_file: String,
        /// Output directory for artifacts
        #[arg(short = 'd', long = "out-dir", visible_alias = "output-dir")]
        out_dir: String,
        /// Bus IDs to target (comma-separated or repeated)
        #[arg(long, value_delimiter = ',')]
        bus: Vec<usize>,
        /// Maximum injection per bus
        #[arg(long, default_value = "2.0")]
        max_injection: f64,
        /// Number of steps per bus
        #[arg(long, default_value = "8")]
        steps: usize,
        /// Solver to use
        #[arg(long, default_value = "gauss")]
        solver: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum DermsCommands {
    /// Build DER flexibility envelopes
    Envelope {
        /// Grid Arrow file
        #[arg(long)]
        grid_file: String,
        /// DER asset Parquet
        #[arg(long)]
        assets: String,
        /// Output Parquet path
        #[arg(short, long)]
        out: String,
        /// Grouping key (agg_id or bus)
        #[arg(long)]
        group_by: Option<String>,
    },
    /// Produce a scheduling recommendation
    Schedule {
        /// DER asset Parquet
        #[arg(long)]
        assets: String,
        /// Price series Parquet
        #[arg(long)]
        price_series: String,
        /// Output Parquet path
        #[arg(short, long)]
        out: String,
        /// Objective name (for logging)
        #[arg(long, default_value = "median-price")]
        objective: String,
    },
    /// Run a stress-test over randomized price perturbations
    StressTest {
        /// DER asset Parquet
        #[arg(long)]
        assets: String,
        /// Price series Parquet
        #[arg(long)]
        price_series: String,
        /// Output directory for scans
        #[arg(short = 'd', long = "out-dir", visible_alias = "output-dir")]
        out_dir: String,
        /// Number of scenarios to sample
        #[arg(long, default_value = "16")]
        scenarios: usize,
        /// Optional RNG seed
        #[arg(long)]
        seed: Option<u64>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AdmsCommands {
    /// Run FLISR reliability sampling
    FlisrSim {
        /// Grid file (Arrow)
        #[arg(long)]
        grid_file: String,
        /// Reliability catalog Parquet
        #[arg(long)]
        reliability: String,
        /// Output directory for FLISR artifacts
        #[arg(short = 'd', long = "out-dir", visible_alias = "output-dir")]
        out_dir: String,
        /// Number of scenarios to sample
        #[arg(long, default_value = "3")]
        scenarios: usize,
        /// Solver to use
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum iterations
        #[arg(long, default_value = "20")]
        max_iter: u32,
    },
    /// Volt/VAR planning runs
    VvoPlan {
        /// Grid file (Arrow)
        #[arg(long)]
        grid_file: String,
        /// Output directory for day-type artifacts
        #[arg(short = 'd', long = "out-dir", visible_alias = "output-dir")]
        out_dir: String,
        /// Day types (comma-separated)
        #[arg(long, default_value = "low,high")]
        day_types: String,
        /// Solver to use
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum iterations
        #[arg(long, default_value = "20")]
        max_iter: u32,
    },
    /// Monte Carlo outage evaluation
    OutageMc {
        /// Reliability catalog Parquet
        #[arg(long)]
        reliability: String,
        /// Output directory
        #[arg(short = 'd', long = "out-dir", visible_alias = "output-dir")]
        out_dir: String,
        /// Sample count
        #[arg(long, default_value = "20")]
        samples: usize,
        /// Optional RNG seed
        #[arg(long)]
        seed: Option<u64>,
    },
    /// State estimation checks
    StateEstimation {
        /// Grid file (Arrow)
        #[arg(long)]
        grid_file: String,
        /// Measurements CSV
        #[arg(long)]
        measurements: String,
        /// Output Parquet for measurement residuals
        #[arg(long)]
        out: String,
        /// Optional output for estimated state
        #[arg(long)]
        state_out: Option<String>,
        /// Solver to use
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Slack bus override
        #[arg(long)]
        slack_bus: Option<usize>,
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
    /// Fetch generator capacity and location data from EIA API
    #[command(about = "Download U.S. generator data from EIA")]
    Eia {
        /// EIA API key
        #[arg(long)]
        api_key: String,

        /// Output file path (supports .csv, .parquet)
        #[arg(short, long)]
        output: String,
    },
    /// Fetch carbon intensity data from Ember Climate API
    #[command(about = "Download carbon intensity and renewable data from Ember")]
    Ember {
        /// Region code (e.g., "US-West", "GB", "DE")
        #[arg(long)]
        region: String,

        /// Start date in YYYY-MM-DD format
        #[arg(long)]
        start_date: String,

        /// End date in YYYY-MM-DD format
        #[arg(long)]
        end_date: String,

        /// Output file path (supports .csv, .parquet)
        #[arg(short, long)]
        output: String,
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
    /// Deliverability Score (DS) computation for resource adequacy accreditation
    ///
    /// Computes DC-approximate deliverability scores: the fraction of nameplate capacity
    /// that can be delivered before branch thermal limits are violated. Used in RA accreditation
    /// where DS × ELCC determines effective capacity. See doi:10.1109/TPWRS.2007.899019 for DC flow.
    Ds {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// CSV file with bus capacity limits (bus_id, pmax)
        #[arg(long)]
        limits: String,
        /// CSV file with branch thermal limits (branch_id, flow_limit)
        #[arg(long)]
        branch_limits: String,
        /// Parquet file with branch flows from DC PF/OPF (must have branch_id, flow_mw)
        #[arg(long)]
        flows: String,
        /// Output file path for DS table (Parquet)
        #[arg(short, long)]
        out: String,
        /// Reference/slack bus ID for PTDF computation
        #[arg(long, default_value_t = 1)]
        sink_bus: usize,
        /// Solver to use (gauss, faer, etc.)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Threading hint (`auto` or integer)
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
    },
    /// Reliability metrics (LOLE, EUE, thermal violations) from batch outputs
    ///
    /// Computes Loss of Load Expectation (LOLE), Energy Unserved (EUE), and thermal violation
    /// counts from batch PF/OPF results. Used for resource adequacy assessment and KPI prediction.
    /// See doi:10.1109/TPWRS.2012.2187686 for reliability metrics in power systems.
    Reliability {
        /// Batch manifest JSON from `gat batch` (alternative to --flows)
        #[arg(long)]
        batch_manifest: Option<String>,
        /// Parquet file with branch flows (alternative to --batch-manifest)
        #[arg(long)]
        flows: Option<String>,
        /// CSV file with branch thermal limits (branch_id, flow_limit)
        #[arg(long)]
        branch_limits: Option<String>,
        /// CSV file with scenario weights/probabilities (scenario_id, weight)
        #[arg(long)]
        scenario_weights: Option<String>,
        /// Output file path for reliability metrics table (Parquet)
        #[arg(short, long)]
        out: String,
        /// Minimum unserved load to count as LOLE event (MW, default 0.1)
        #[arg(long, default_value_t = 0.1)]
        unserved_threshold: f64,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
    },
    /// Estimate Equivalent Load Carrying Capability (ELCC)
    Elcc {
        /// Parquet file with resource profiles (asset_id, class_id, time, capacity)
        #[arg(long)]
        resource_profiles: String,
        /// Parquet file with reliability metrics (from `gat analytics reliability`)
        #[arg(long)]
        reliability_metrics: String,
        /// Output file path for ELCC estimates (Parquet)
        #[arg(short, long)]
        out: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
        /// Number of parallel jobs (0 for auto)
        #[arg(long, default_value_t = 0)]
        max_jobs: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum AllocCommands {
    /// Compute congestion rents and surplus decomposition from OPF results
    ///
    /// Analyzes OPF outputs (LMPs, flows, injections) to decompose system surplus into congestion
    /// rents, generator revenues, and load payments. Provides the numerical backbone for allocation
    /// and settlement frameworks. See doi:10.1109/TPWRS.2003.820692 for LMP-based congestion analysis.
    Rents {
        /// Parquet file with OPF results (must have: bus_id, lmp, injection_mw, flow_mw)
        #[arg(long, value_hint = ValueHint::FilePath)]
        opf_results: String,
        /// Path to the grid topology file (Arrow format, for branch mapping)
        #[arg(long, value_hint = ValueHint::FilePath)]
        grid_file: String,
        /// Optional tariff/margin parameters CSV (resource_id, tariff_rate)
        #[arg(long, value_hint = ValueHint::FilePath)]
        tariffs: Option<String>,
        /// Output file path for congestion rents table (Parquet)
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
        /// Partition columns (comma separated, e.g., "scenario_id,time")
        #[arg(long)]
        out_partitions: Option<String>,
    },
    /// Simple contribution analysis for KPI changes across scenarios
    ///
    /// Approximates the contribution of control actions/portfolios to KPI improvements using
    /// gradient-based sensitivity or linear approximations. A stepping stone towards full SHAP
    /// explainability. See doi:10.1038/s42256-019-0138-9 for SHAP and model explanations.
    Kpi {
        /// Parquet file with KPI results (must have: scenario_id, kpi_value)
        #[arg(long, value_hint = ValueHint::FilePath)]
        kpi_results: String,
        /// Parquet file with scenario metadata (scenario_id, control flags, policy settings)
        #[arg(long, value_hint = ValueHint::FilePath)]
        scenario_meta: String,
        /// Output file path for contribution analysis (Parquet)
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
        /// Partition columns (comma separated, e.g., "scenario_id")
        #[arg(long)]
        out_partitions: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum GeoCommands {
    /// Map buses/feeders to spatial polygons (tracts, zip codes, neighborhoods)
    ///
    /// Performs spatial joins between power grid topology (buses, feeders) and GIS polygons
    /// (census tracts, zip codes, planning areas, etc.). Produces polygon_id ↔ bus_id mapping
    /// tables for downstream spatial aggregation. Supports point-in-polygon tests, Voronoi
    /// tessellation, and k-nearest-neighbor assignment. Compatible with GeoParquet format.
    /// See doi:10.3390/ijgi9020102 for spatial joins in energy systems GIS.
    Join {
        /// Path to the grid topology file (Arrow format, must have bus_id, lat, lon)
        #[arg(long, value_hint = ValueHint::FilePath)]
        grid_file: String,
        /// Path to the GIS polygon file (GeoParquet, Shapefile, or GeoJSON with polygon geometries)
        #[arg(long, value_hint = ValueHint::FilePath)]
        polygons: String,
        /// Spatial join method: "point_in_polygon", "voronoi", or "knn"
        #[arg(long, default_value = "point_in_polygon")]
        method: String,
        /// For knn method: number of nearest polygons to assign (default 1)
        #[arg(long, default_value_t = 1)]
        k: usize,
        /// Output file path for bus-to-polygon mapping table (Parquet: bus_id, polygon_id, distance)
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
        /// Partition columns (comma separated, e.g., "polygon_id")
        #[arg(long)]
        out_partitions: Option<String>,
    },
    /// Produce time-series feature tables keyed by (polygon_id, time)
    ///
    /// Aggregates time-series grid metrics (load, voltage, violations, etc.) to spatial polygons
    /// using the bus-to-polygon mapping from `gat geo join`. Computes lags, rolling statistics,
    /// event flags, and seasonal features for spatial forecasting models. Outputs polygon-level
    /// feature fabric for demand forecasting, reliability prediction, and spatial planning.
    /// See doi:10.1016/j.energy.2020.117515 for spatial-temporal load forecasting.
    Featurize {
        /// Path to the bus-to-polygon mapping table (output from `gat geo join`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        mapping: String,
        /// Path to time-series grid metrics (Parquet with bus_id, time, values)
        #[arg(long, value_hint = ValueHint::FilePath)]
        timeseries: String,
        /// Lag periods to compute (comma separated, e.g., "1,7,24" for 1-hour, 7-hour, 24-hour lags)
        #[arg(long)]
        lags: Option<String>,
        /// Rolling window sizes (comma separated, e.g., "7,24,168" for 7h, 24h, 168h windows)
        #[arg(long)]
        windows: Option<String>,
        /// Compute seasonal features (day-of-week, hour-of-day, month-of-year flags)
        #[arg(long, default_value_t = true)]
        seasonal: bool,
        /// Output file path for polygon-level features (Parquet: polygon_id, time, features)
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
        /// Partition columns (comma separated, e.g., "polygon_id,time")
        #[arg(long)]
        out_partitions: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum FeaturizeCommands {
    /// Export grid topology and flows as GNN-ready graph features
    ///
    /// Converts power grid data into graph-structured features for Graph Neural Networks (GNNs).
    /// Produces node features (buses with static topology + dynamic injections), edge features
    /// (branches with impedance + flows), and graph metadata. Compatible with PyTorch Geometric,
    /// DGL, and other GNN frameworks. See doi:10.1109/TPWRS.2020.3041234 for GNNs in power systems.
    Gnn {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Parquet file with branch flows from PF/OPF (must have branch_id, flow_mw)
        #[arg(long)]
        flows: String,
        /// Output directory root for feature tables (nodes, edges, graphs)
        #[arg(short, long)]
        out: String,
        /// Partition columns (comma separated, e.g., "graph_id,scenario_id")
        #[arg(long)]
        out_partitions: Option<String>,
        /// Group flows by scenario_id (if present in flows)
        #[arg(long, default_value_t = true)]
        group_by_scenario: bool,
        /// Group flows by time (if present in flows)
        #[arg(long, default_value_t = true)]
        group_by_time: bool,
    },
    /// Generate KPI training/evaluation feature tables for ML prediction models
    ///
    /// Aggregates batch PF/OPF outputs and reliability metrics into wide feature tables suitable
    /// for training probabilistic KPI predictors (TabNet, NGBoost, gradient boosting, etc.).
    /// Outputs are keyed by (scenario_id, time, zone) and include system stress indicators,
    /// policy flags, and reliability metrics. The "X" features for predicting reliability KPIs.
    Kpi {
        /// Root directory containing batch PF/OPF outputs (flows, LMPs, violations)
        #[arg(long, value_hint = ValueHint::DirPath)]
        batch_root: String,
        /// Optional reliability metrics file (output from `gat analytics reliability`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        reliability: Option<String>,
        /// Optional scenario metadata file (YAML/JSON with policy flags, weather, etc.)
        #[arg(long, value_hint = ValueHint::FilePath)]
        scenario_meta: Option<String>,
        /// Output file path for KPI features (Parquet format)
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        out: String,
        /// Partition columns (comma separated, e.g., "scenario_id,time")
        #[arg(long)]
        out_partitions: Option<String>,
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
pub enum VersionCommands {
    /// Sync release metadata
    Sync {
        /// Tag name to validate (leading `v` is stripped)
        #[arg(long)]
        tag: Option<String>,
        /// Write manifest JSON describing the resolved version/tag
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: Option<PathBuf>,
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
    /// Run DC optimal power flow.
    ///
    /// Solves the DC-OPF linear program to minimize generation cost subject to power
    /// balance and branch flow constraints. Console output shows a rich summary with
    /// total cost, total demand, top generator dispatches, and branch flow statistics.
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
        #[arg(short = 't', long, default_value = "auto")]
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
    /// Run AC optimal power flow (fast-decoupled, linear approximation)
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
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
        /// Solver to use (gauss, faer)
        #[arg(long, default_value = "gauss")]
        solver: String,
        /// Partition columns (comma separated)
        #[arg(long)]
        out_partitions: Option<String>,
    },
    /// Run full nonlinear AC-OPF with cost optimization
    ///
    /// Uses penalty method + L-BFGS to solve the full nonlinear AC optimal power
    /// flow problem including voltage magnitudes, angles, and generator dispatch.
    /// Minimizes total generation cost subject to power balance and physical limits.
    AcNlp {
        /// Path to the grid data file (Arrow format)
        grid_file: String,
        /// Output JSON file for dispatch results
        #[arg(short, long)]
        out: String,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-4")]
        tol: f64,
        /// Maximum number of iterations
        #[arg(long, default_value = "200")]
        max_iter: u32,
        /// Warm-start method: flat, dc, socp
        #[arg(long, default_value = "flat")]
        warm_start: String,
        /// Threading hint (`auto` or integer)
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
        /// NLP solver to use: lbfgs (default), ipopt (requires solver-ipopt feature)
        #[arg(long, default_value = "lbfgs")]
        solver: String,
    },
    /// Run OPF directly on a MATPOWER file with full solver options.
    ///
    /// Swiss-army-knife command for validation and testing. Accepts MATPOWER files
    /// directly (no separate cost/limits CSV needed), supports all solver methods
    /// and warm-start strategies, and provides rich console output with optional
    /// JSON export.
    Run {
        /// Path to MATPOWER (.m) file or directory containing .m file
        input: String,
        /// OPF method: economic, dc, socp, ac [default: socp]
        #[arg(long, default_value = "socp")]
        method: String,
        /// NLP solver for AC-OPF: lbfgs, ipopt [default: lbfgs]
        #[arg(long, default_value = "lbfgs")]
        solver: String,
        /// Warm-start strategy for AC-OPF: flat, dc, socp, cascaded [default: flat]
        #[arg(long, default_value = "flat")]
        warm_start: String,
        /// Enable OBBT + QC envelopes for tighter SOCP relaxation
        #[arg(long)]
        enhanced: bool,
        /// Convergence tolerance
        #[arg(long, default_value = "1e-6")]
        tol: f64,
        /// Maximum iterations
        #[arg(long, default_value = "200")]
        max_iter: u32,
        /// Solver timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
        /// Compare objective against PGLib baseline CSV
        #[arg(long)]
        baseline: Option<String>,
        /// Include constraint violation details in output
        #[arg(long)]
        output_violations: bool,
        /// Write JSON solution to file
        #[arg(short, long)]
        out: Option<String>,
        /// Threading hint (`auto` or integer)
        #[arg(short = 't', long, default_value = "auto")]
        threads: String,
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
        #[arg(short = 't', long, default_value = "auto")]
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

#[derive(Subcommand, Debug)]
pub enum SolverCommands {
    /// List available and installed solvers
    List,
    /// Install a native solver plugin
    Install {
        /// Solver name (ipopt, highs, cbc, bonmin, couenne, symphony)
        solver: String,
        /// Force reinstall if already installed
        #[arg(long)]
        force: bool,
    },
    /// Uninstall a native solver plugin
    Uninstall {
        /// Solver name to uninstall
        solver: String,
    },
    /// Show solver configuration status
    Status,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum RunFormat {
    Plain,
    Json,
}

pub fn build_cli_command() -> clap::Command {
    Cli::command()
}
