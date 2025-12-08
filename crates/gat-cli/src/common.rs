//! Common CLI types and utilities shared across commands.
//!
//! This module provides standardized enums and argument types to ensure
//! consistent flag naming and behavior across all gat CLI commands.

use anyhow::{Context, Result};
use clap::ValueEnum;
use gat_core::Network;
use gat_io::importers::load_grid_from_arrow;
use serde::Serialize;
use std::io::{self, Write};
use std::path::PathBuf;

/// Output format for tabular/structured data.
///
/// Commands that produce structured output should use this enum to allow
/// users to choose their preferred format for piping and processing.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable ASCII table (default for interactive use)
    #[default]
    Table,
    /// JSON object or array (pipe-friendly, structured)
    Json,
    /// JSON Lines - one JSON object per line (streaming-friendly)
    Jsonl,
    /// Comma-separated values (pipe to awk/cut/etc)
    Csv,
}

/// File output format for power flow commands.
///
/// Used for commands that write analysis results to files, allowing users
/// to choose between binary (Parquet) and text formats (JSON, CSV).
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileOutputFormat {
    /// Apache Parquet columnar format (default, efficient for large datasets)
    #[default]
    Parquet,
    /// JSON object or array (human-readable, structured)
    Json,
    /// Comma-separated values (widely compatible)
    Csv,
}

impl OutputFormat {
    /// Returns true if this format is machine-readable (suitable for piping)
    pub fn is_machine_readable(&self) -> bool {
        matches!(self, Self::Json | Self::Jsonl | Self::Csv)
    }
}

/// Solver method for OPF problems.
///
/// Unified enum replacing the various string-based `--method` flags.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OpfMethod {
    /// Merit-order economic dispatch (no network constraints)
    Economic,
    /// DC optimal power flow (LP with B-matrix)
    Dc,
    /// Second-order cone relaxation of AC-OPF
    #[default]
    Socp,
    /// Full nonlinear AC-OPF (penalty method + L-BFGS or IPOPT)
    Ac,
}

/// Flow calculation mode (DC vs AC power flow).
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowMode {
    /// DC power flow (linear, angle-only)
    #[default]
    Dc,
    /// AC power flow (nonlinear, voltage + angle)
    Ac,
}

/// Linear algebra solver backend.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinearSolver {
    /// Gaussian elimination (simple, reliable)
    #[default]
    Gauss,
    /// Faer library (fast, modern)
    Faer,
}

/// LP/QP optimizer backend.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Optimizer {
    /// Clarabel interior-point solver (pure Rust, default)
    #[default]
    Clarabel,
    /// HiGHS solver (requires feature flag)
    Highs,
}

/// NLP solver backend for AC-OPF.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NlpSolver {
    /// L-BFGS quasi-Newton (pure Rust, default)
    #[default]
    Lbfgs,
    /// IPOPT interior-point (requires solver-ipopt feature)
    Ipopt,
}

/// Thermal rating type for branch limits.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RatingType {
    /// Rate A - normal operating limit
    #[default]
    RateA,
    /// Rate B - short-term emergency limit
    RateB,
    /// Rate C - long-term emergency limit
    RateC,
}

/// Output format for GNN featurization.
///
/// Supports multiple formats for compatibility with different ML frameworks.
/// - **Arrow**: GAT native format (Parquet tables for nodes, edges, graphs)
/// - **NeuripsJson**: NeurIPS PowerGraph benchmark format (graph-per-file JSON)
/// - **PytorchGeometric**: PyTorch Geometric compatible format (COO tensors as JSON)
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GnnOutputFormat {
    /// GAT native Arrow/Parquet (default, supports partitioning)
    #[default]
    Arrow,
    /// NeurIPS PowerGraph benchmark JSON format
    NeuripsJson,
    /// PyTorch Geometric compatible JSON (COO edge_index, x, edge_attr)
    PytorchGeometric,
}

/// Input source that can be a file path or stdin.
#[derive(Clone, Debug)]
pub enum InputSource {
    /// Read from a file path
    File(PathBuf),
    /// Read from stdin (specified as "-")
    Stdin,
}

impl InputSource {
    /// Parse from a string argument. "-" means stdin, anything else is a file path.
    pub fn parse(s: &str) -> Self {
        if s == "-" {
            Self::Stdin
        } else {
            Self::File(PathBuf::from(s))
        }
    }

    /// Returns true if this is stdin
    pub fn is_stdin(&self) -> bool {
        matches!(self, Self::Stdin)
    }

    /// Get the path if this is a file, None if stdin
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::File(p) => Some(p),
            Self::Stdin => None,
        }
    }
}

/// Output destination that can be a file path or stdout.
#[derive(Clone, Debug)]
pub enum OutputDest {
    /// Write to a file path
    File(PathBuf),
    /// Write to stdout (specified as "-")
    Stdout,
}

impl OutputDest {
    /// Parse from a string argument. "-" means stdout, anything else is a file path.
    pub fn parse(s: &str) -> Self {
        if s == "-" {
            Self::Stdout
        } else {
            Self::File(PathBuf::from(s))
        }
    }

    /// Returns true if this is stdout
    pub fn is_stdout(&self) -> bool {
        matches!(self, Self::Stdout)
    }
}

/// Common solver parameters shared across commands.
#[derive(Clone, Debug)]
pub struct SolverParams {
    /// Convergence tolerance
    pub tol: f64,
    /// Maximum iterations
    pub max_iter: u32,
    /// Threading hint ("auto" or numeric)
    pub threads: String,
}

impl Default for SolverParams {
    fn default() -> Self {
        Self {
            tol: 1e-6,
            max_iter: 100,
            threads: "auto".to_string(),
        }
    }
}

/// Write data as JSON to the given writer.
pub fn write_json<W: Write, T: Serialize>(
    data: &T,
    writer: &mut W,
    pretty: bool,
) -> io::Result<()> {
    if pretty {
        serde_json::to_writer_pretty(&mut *writer, data).map_err(io::Error::other)?;
    } else {
        serde_json::to_writer(&mut *writer, data).map_err(io::Error::other)?;
    }
    writeln!(writer)?;
    Ok(())
}

/// Write data as JSON Lines (one JSON object per line) to the given writer.
pub fn write_jsonl<W: Write, T: Serialize>(data: &[T], writer: &mut W) -> io::Result<()> {
    for item in data {
        serde_json::to_writer(&mut *writer, item).map_err(io::Error::other)?;
        writeln!(writer)?;
    }
    Ok(())
}

/// Write JSON array data as CSV to the given writer.
/// Assumes all objects have the same keys.
pub fn write_csv_from_json<W: Write>(data: &[serde_json::Value], writer: &mut W) -> io::Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    // Extract headers from first object
    let first = &data[0];
    let headers: Vec<&str> = match first.as_object() {
        Some(obj) => obj.keys().map(|s| s.as_str()).collect(),
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected JSON objects",
            ))
        }
    };

    // Write header row
    writeln!(writer, "{}", headers.join(","))?;

    // Write data rows
    for item in data {
        if let Some(obj) = item.as_object() {
            let values: Vec<String> = headers
                .iter()
                .map(|h| {
                    obj.get(*h)
                        .map(|v| match v {
                            serde_json::Value::String(s) => {
                                // Escape quotes and wrap in quotes if contains comma
                                if s.contains(',') || s.contains('"') {
                                    format!("\"{}\"", s.replace('"', "\"\""))
                                } else {
                                    s.clone()
                                }
                            }
                            serde_json::Value::Null => String::new(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default()
                })
                .collect();
            writeln!(writer, "{}", values.join(","))?;
        }
    }
    Ok(())
}

/// Load a network from a file path or stdin (Arrow format).
///
/// For File variant: Uses the existing ArrowDirectoryReader to load the network.
/// For Stdin variant: Returns an error indicating stdin support is not yet implemented.
///
/// # Arguments
///
/// * `source` - Input source (file path or stdin)
///
/// # Returns
///
/// A `Network` object loaded from the Arrow format.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened or read
/// - The Arrow data is invalid or corrupted
/// - Stdin is specified (not yet implemented)
pub fn load_network(source: &InputSource) -> Result<Network> {
    match source {
        InputSource::File(path) => load_grid_from_arrow(path)
            .with_context(|| format!("Failed to load network from: {}", path.display())),
        InputSource::Stdin => {
            anyhow::bail!("Stdin input not yet implemented. Use a file path.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_source_parse() {
        assert!(InputSource::parse("-").is_stdin());
        assert!(!InputSource::parse("file.arrow").is_stdin());
        assert_eq!(
            InputSource::parse("test.arrow")
                .path()
                .unwrap()
                .to_str()
                .unwrap(),
            "test.arrow"
        );
    }

    #[test]
    fn test_output_dest_parse() {
        assert!(OutputDest::parse("-").is_stdout());
        assert!(!OutputDest::parse("out.parquet").is_stdout());
    }

    #[test]
    fn test_output_format_machine_readable() {
        assert!(!OutputFormat::Table.is_machine_readable());
        assert!(OutputFormat::Json.is_machine_readable());
        assert!(OutputFormat::Jsonl.is_machine_readable());
        assert!(OutputFormat::Csv.is_machine_readable());
    }

    #[test]
    fn test_load_network_from_file() {
        // Test with a sample file path - only test if file exists
        let test_paths = [
            "test_data/matpower/case9.arrow",
            "../../test_data/matpower/case9.arrow",
        ];

        for path in &test_paths {
            let source = InputSource::File(PathBuf::from(path));
            if source.path().unwrap().exists() {
                let result = load_network(&source);
                assert!(result.is_ok(), "Failed to load network from {}", path);
                return; // Test passed with at least one path
            }
        }
        // Skip test if no test file exists
        eprintln!("Skipping test_load_network_from_file: no test file found");
    }

    #[test]
    fn test_load_network_from_stdin_not_implemented() {
        let source = InputSource::Stdin;
        let result = load_network(&source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Stdin input not yet implemented"));
    }
}

#[cfg(test)]
mod format_writer_tests {
    use super::*;

    #[test]
    fn test_write_json_to_string() {
        let data = vec![
            serde_json::json!({"id": 1, "name": "Gen1"}),
            serde_json::json!({"id": 2, "name": "Gen2"}),
        ];
        let mut output = Vec::new();
        write_json(&data, &mut output, false).unwrap();
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("Gen1"));
        assert!(result.contains("Gen2"));
    }

    #[test]
    fn test_write_jsonl_to_string() {
        let data = vec![serde_json::json!({"id": 1}), serde_json::json!({"id": 2})];
        let mut output = Vec::new();
        write_jsonl(&data, &mut output).unwrap();
        let result = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = result.trim().lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_write_csv_from_json() {
        let data = vec![
            serde_json::json!({"id": 1, "name": "A"}),
            serde_json::json!({"id": 2, "name": "B"}),
        ];
        let mut output = Vec::new();
        write_csv_from_json(&data, &mut output).unwrap();
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("id,name") || result.contains("name,id"));
        assert!(result.contains("1"));
        assert!(result.contains("A"));
    }
}
