use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result};
use gat_cli::cli::ImportCommands;
use gat_io::importers;

use crate::commands::telemetry::record_run_timed;

pub fn handle(command: &ImportCommands) -> Result<()> {
    match command {
        ImportCommands::Psse { raw, output } => {
            let start = Instant::now();
            let (raw_path, output_path) =
                prepare_import(raw, output.as_deref(), &["raw"], "PSS/E RAW")?;
            let res = importers::import_psse_raw(&raw_path, &output_path).map(|_| ());
            record_run_timed(
                &output_path,
                "import psse",
                &[("raw", &raw_path), ("output", &output_path)],
                start,
                &res,
            );
            res
        }
        ImportCommands::Matpower { m, output } => {
            let start = Instant::now();
            let (case_path, output_path) = prepare_import(
                m,
                output.as_deref(),
                &["m", "mat", "matpower"],
                "MATPOWER case",
            )?;
            let res = importers::import_matpower_case(&case_path, &output_path).map(|_| ());
            record_run_timed(
                &output_path,
                "import matpower",
                &[("case", &case_path), ("output", &output_path)],
                start,
                &res,
            );
            res
        }
        ImportCommands::Cim { rdf, output } => {
            let start = Instant::now();
            let (rdf_path, output_path) =
                prepare_import(rdf, output.as_deref(), &["rdf", "xml"], "CIM RDF/XML")?;
            let res = importers::import_cim_rdf(&rdf_path, &output_path).map(|_| ());
            record_run_timed(
                &output_path,
                "import cim",
                &[("rdf", &rdf_path), ("output", &output_path)],
                start,
                &res,
            );
            res
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
