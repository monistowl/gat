use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Context, Result};
use gat_cli::cli::ImportCommands;
use gat_io::helpers::{validate_network, ImportDiagnostics, Severity, ValidationConfig};
use gat_io::importers::{self, Format};

use crate::commands::telemetry::record_run_timed_with_diagnostics;

/// Verbosity level for import diagnostics
#[derive(Clone, Copy)]
pub enum Verbosity {
    Normal,  // Just counts
    Verbose, // Grouped warnings
    Debug,   // Line-level details
}

impl From<u8> for Verbosity {
    fn from(v: u8) -> Self {
        match v {
            0 => Verbosity::Normal,
            1 => Verbosity::Verbose,
            _ => Verbosity::Debug,
        }
    }
}

pub fn handle(
    command: &ImportCommands,
    verbose: u8,
    strict: bool,
    run_validation: bool,
) -> Result<()> {
    let verbosity = Verbosity::from(verbose);

    // Extract format and input path from command
    let (format, input_path, output) = match command {
        ImportCommands::Auto { input, output } => {
            // Auto-detect format from file
            let path = Path::new(input);
            let (detected_format, confidence) = Format::detect(path)
                .ok_or_else(|| anyhow::anyhow!(
                    "Could not detect format for '{}'. Supported extensions: .m (MATPOWER), .raw (PSS/E), .rdf/.xml (CIM), .json (pandapower)",
                    input
                ))?;

            let confidence_str = match confidence {
                importers::Confidence::High => "high confidence",
                importers::Confidence::Medium => "medium confidence",
                importers::Confidence::Low => "low confidence",
            };
            eprintln!("Detected format: {} ({})", detected_format, confidence_str);

            (detected_format, input.as_str(), output.as_deref())
        }
        ImportCommands::Psse { raw, output } => (Format::Psse, raw.as_str(), output.as_deref()),
        ImportCommands::Matpower { m, output } => (Format::Matpower, m.as_str(), output.as_deref()),
        ImportCommands::Cim { rdf, output } => (Format::Cim, rdf.as_str(), output.as_deref()),
        ImportCommands::Pandapower { json, output } => {
            (Format::Pandapower, json.as_str(), output.as_deref())
        }
    };

    // Run the unified import workflow
    run_import(
        format,
        input_path,
        output,
        verbosity,
        strict,
        run_validation,
    )
}

/// Unified import workflow for all formats.
fn run_import(
    format: Format,
    input: &str,
    output: Option<&str>,
    verbosity: Verbosity,
    strict: bool,
    run_validation: bool,
) -> Result<()> {
    let start = Instant::now();

    // Prepare paths
    let (input_path, output_path) =
        prepare_import(input, output, format.extensions(), format.friendly_name())?;

    // Parse using the format's parser
    let mut result = format.parse(&input_path)?;

    // Run post-import validation if requested
    if run_validation {
        let config = ValidationConfig::default();
        validate_network(&result.network, &mut result.diagnostics, &config);
    }

    // Write the network to Arrow
    importers::export_network_to_arrow(&result.network, &output_path)?;

    // Print diagnostics based on verbosity
    print_diagnostics(&result.diagnostics, verbosity, run_validation);

    // Record run manifest with diagnostics
    record_run_timed_with_diagnostics(
        &output_path,
        &format!("import {}", format.command_name()),
        &[("input", &input_path), ("output", &output_path)],
        start,
        &result.diagnostics,
    );

    // In strict mode, fail if there are any warnings
    if strict && result.diagnostics.has_issues() {
        bail!(
            "Import failed in strict mode ({} warning(s), {} error(s))",
            result.diagnostics.warning_count(),
            result.diagnostics.error_count()
        );
    }

    Ok(())
}

fn print_diagnostics(diag: &ImportDiagnostics, verbosity: Verbosity, validated: bool) {
    // Always print stats
    let validation_marker = if validated { " [validated]" } else { "" };
    println!(
        "✓ Imported {} buses, {} branches, {} generators, {} loads{}",
        diag.stats.buses,
        diag.stats.branches,
        diag.stats.generators,
        diag.stats.loads,
        validation_marker
    );

    let warning_count = diag.warning_count();
    let error_count = diag.error_count();

    if warning_count == 0 && error_count == 0 {
        return;
    }

    match verbosity {
        Verbosity::Normal => {
            if warning_count > 0 || error_count > 0 {
                println!(
                    "⚠ {} warning(s), {} error(s) (use -v for details)",
                    warning_count, error_count
                );
            }
        }
        Verbosity::Verbose => {
            // Group warnings by category and message
            let mut grouped: HashMap<(&str, &str), usize> = HashMap::new();
            for issue in &diag.issues {
                let key = (issue.category.as_str(), issue.message.as_str());
                *grouped.entry(key).or_insert(0) += 1;
            }

            if !grouped.is_empty() {
                println!("⚠ {} issue(s):", warning_count + error_count);
                for ((category, message), count) in grouped {
                    if count > 1 {
                        println!("  - {}: {} ({} occurrences)", category, message, count);
                    } else {
                        println!("  - {}: {}", category, message);
                    }
                }
            }
        }
        Verbosity::Debug => {
            // Show every issue with line numbers
            if !diag.issues.is_empty() {
                println!("⚠ {} issue(s):", warning_count + error_count);
                for issue in &diag.issues {
                    let severity_marker = match issue.severity {
                        Severity::Warning => "⚠",
                        Severity::Error => "✗",
                    };
                    match (&issue.line, &issue.entity) {
                        (Some(line), Some(entity)) => {
                            println!(
                                "  {} Line {}: {} ({})",
                                severity_marker, line, issue.message, entity
                            );
                        }
                        (Some(line), None) => {
                            println!("  {} Line {}: {}", severity_marker, line, issue.message);
                        }
                        (None, Some(entity)) => {
                            println!("  {} {}: {}", severity_marker, entity, issue.message);
                        }
                        (None, None) => {
                            println!("  {} {}", severity_marker, issue.message);
                        }
                    }
                }
            }
        }
    }
}

fn prepare_import(
    input: &str,
    output: Option<&str>,
    expected_extensions: &[&str],
    friendly_name: &str,
) -> Result<(String, String)> {
    let input_path = PathBuf::from(input);
    ensure_input_file(&input_path, expected_extensions, friendly_name)?;

    let output_path = normalize_output_path(output, &input_path, "arrow")?;

    Ok((to_string_path(&input_path), to_string_path(&output_path)))
}

fn ensure_input_file(path: &Path, expected_extensions: &[&str], friendly_name: &str) -> Result<()> {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    if !canonical.exists() {
        anyhow::bail!(
            "Input file '{}' not found. Hint: double-check the path or run 'ls {}' to verify its location.",
            path.display(),
            path.parent()
                .map(|p| p.display().to_string())
                .filter(|p| !p.is_empty())
                .unwrap_or_else(|| ".".to_string())
        );
    }

    if let Some(ext) = canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    {
        if !expected_extensions.is_empty()
            && !expected_extensions
                .iter()
                .any(|expected| ext.eq_ignore_ascii_case(expected))
        {
            anyhow::bail!(
                "{} '{}' has extension '.{}' which does not match the expected [{}]. Hint: ensure you are using a valid {} file.",
                friendly_name,
                path.display(),
                ext,
                expected_extensions.join(", ."),
                friendly_name,
            );
        }
    } else {
        eprintln!(
            "Warning: could not detect a file extension for '{}'; proceeding without strict format checks.",
            path.display()
        );
    }

    Ok(())
}

fn normalize_output_path(
    output: Option<&str>,
    input_path: &Path,
    default_ext: &str,
) -> Result<PathBuf> {
    let mut path = output
        .map(PathBuf::from)
        .unwrap_or_else(|| input_path.with_extension(default_ext));

    if path.extension().is_none() {
        path.set_extension(default_ext);
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "creating parent directory '{}' for output file {}",
                    parent.display(),
                    path.display()
                )
            })?;
        }
    }

    Ok(path)
}

fn to_string_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
