//! Common CLI types and utilities shared across commands.
//!
//! This module provides standardized enums and argument types to ensure
//! consistent flag naming and behavior across all gat CLI commands.

use clap::ValueEnum;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_source_parse() {
        assert!(InputSource::parse("-").is_stdin());
        assert!(!InputSource::parse("file.arrow").is_stdin());
        assert_eq!(
            InputSource::parse("test.arrow").path().unwrap().to_str().unwrap(),
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
}
