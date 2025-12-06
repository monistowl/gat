//! # gat-cli: Command-Line Interface
//!
//! The primary user-facing interface for GAT (Grid Analysis Toolkit), providing command-line access
//! to power system analysis workflows.
//!
//! ## Design Philosophy
//!
//! **Modular Command Structure**: Top-level commands map to functional domains (import, pf, opf, analytics, etc.).
//! Each domain has its own command submodule with hierarchical subcommands.
//!
//! **Composable Workflows**: Commands can be chained in shell scripts and CI/CD pipelines. All commands
//! accept file paths as arguments and produce machine-parsable output (JSON, Parquet, CSV).
//!
//! **Reproducibility**: Every command execution can be saved as a run manifest that captures all parameters,
//! allowing exact reproduction via `gat runs resume`.
//!
//! ## Quick Start: Import and Analyze a Case
//!
//! ```bash
//! # 1. Import a MATPOWER case
//! gat import matpower --m case14.m -o grid.arrow
//!
//! # 2. Run DC power flow
//! gat pf dc grid.arrow -o flows.parquet
//!
//! # 3. Query results with DuckDB
//! duckdb "SELECT * FROM read_parquet('flows.parquet') LIMIT 10"
//! ```
//!
//! ## Command Structure
//!
//! ```text
//! gat
//! ├── import <format>     # Data import (matpower, psse, cim, pandapower)
//! ├── pf                  # Power flow (dc, ac)
//! ├── opf                 # Optimal power flow (dc, ac, unit commitment)
//! ├── nminus1             # Contingency analysis
//! ├── batch               # Batch mode (parallel execution)
//! ├── scenarios           # Scenario management (yaml, materialize)
//! ├── ts                  # Time-series operations (solve, forecast, stats)
//! ├── graph               # Network analysis (stats, islands, connectivity)
//! ├── analytics           # Advanced analytics (reliability, deliverability, elcc)
//! ├── dataset             # Dataset management (validate, info)
//! ├── geo                 # Spatial analysis (join, featurize)
//! ├── se                  # State estimation (estimate, observability)
//! ├── dist                # Distribution analysis (pf, vvo, flisr)
//! ├── runs                # Run management (list, resume, show)
//! └── version             # Version info
//! ```
//!
//! ## Output Formats
//!
//! All analysis commands produce **Apache Parquet** output:
//! - **Advantage**: Columnar compression, multi-language support (Python, R, Spark, DuckDB)
//! - **Supported Readers**:
//!   - `polars` (Python/Rust)
//!   - `pandas` (Python)
//!   - `duckdb` (SQL queries)
//!   - Apache Spark
//!
//! Example workflow:
//! ```bash
//! # Run analysis
//! gat pf dc grid.arrow -o flows.parquet
//!
//! # Analyze in Python
//! python3 << 'EOF'
//! import polars as pl
//! flows = pl.read_parquet("flows.parquet")
//! violations = flows.filter(pl.col("power_mw") > pl.col("limit_mva"))
//! print(f"Violations: {len(violations)}")
//! EOF
//! ```
//!
//! ## Modules
//!
//! - [`cli`] - Command structures and arg parsing (via clap)
//! - [`install`] - Installation and setup utilities
//! - [`manifest`] - Run manifest management (for reproducibility)
//! - [`docs`] - (feature-gated) Command documentation generation
//!
//! ## Feature Flags
//!
//! - `default`: CLI only (core analysis commands)
//! - `tui`: Terminal UI mode (`gat tui` command)
//! - `gui`: GUI mode (`gat gui` command)
//! - `viz`: Visualization commands (`gat viz` command)
//! - `docs`: Documentation generation utilities
//!
//! ## Integration with gat-io and gat-core
//!
//! The CLI layer orchestrates:
//! 1. **Import** (gat-io) - Parse external formats to internal `Network` representation
//! 2. **Analysis** (gat-core) - Run solvers and topological analysis
//! 3. **Export** (gat-io/Arrow) - Write results to Parquet for downstream tools
//!
//! ## Example: Custom Analysis Pipeline
//!
//! ```bash
//! #!/bin/bash
//!
//! # Import multiple cases
//! for case in case9.m case14.m case30.m; do
//!   gat import matpower --m $case -o ${case%.m}.arrow
//! done
//!
//! # Run contingency analysis on all
//! gat batch pf \
//!   --manifest scenarios.json \
//!   --threads 8 \
//!   -o results/
//!
//! # Analyze with custom Python script
//! python3 analyze_results.py results/*.parquet
//! ```
//!
//! ## Error Handling
//!
//! - Exit code 0 on success
//! - Exit code 1 on user errors (invalid input, missing files)
//! - Exit code 2 on system errors (out of memory, I/O failures)
//! - Detailed error messages to stderr

pub mod cli;
pub mod common;
#[cfg(feature = "docs")]
pub mod docs;
pub mod install;
pub mod manifest;

#[cfg(feature = "gui")]
pub use cli::GuiCommands;
#[cfg(feature = "tui")]
pub use cli::TuiCommands;
#[cfg(feature = "viz")]
pub use cli::VizCommands;
pub use cli::{
    build_cli_command, Cli, Commands, DatasetCommands, GraphCommands, HirenCommands,
    ImportCommands, InspectCommands, Nminus1Commands, OpfCommands, PowerFlowCommands, RunsCommands,
    ScenariosCommands, SeCommands, Sup3rccCommands, TsCommands, VersionCommands,
};
