use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use libc;
use tempfile::tempdir;

use gat_io::{
    exporters::{
        formats::{
            export_network_to_cim, export_network_to_matpower, export_network_to_pandapower,
            export_network_to_powermodels, export_network_to_psse,
        },
        write_network_to_arrow_directory, ExportMetadata,
    },
    importers::{load_grid_from_arrow_with_manifest, Format},
};

use gat_cli::cli::{ConvertCommands, ConvertFormat};

pub fn handle(command: &ConvertCommands) -> Result<()> {
    match command {
        ConvertCommands::Format {
            input,
            from,
            to,
            output,
            force,
            verbose,
            strict,
        } => convert_format(input, *from, *to, output.as_deref(), *force, *verbose, *strict),
    }
}

fn convert_format(
    input: &str,
    from: Option<ConvertFormat>,
    to: ConvertFormat,
    output: Option<&str>,
    force: bool,
    verbose: bool,
    strict: bool,
) -> Result<()> {
    let input_path = PathBuf::from(input);
    if !input_path.exists() {
        bail!("Input '{}' does not exist", input);
    }

    // Keep temp directory alive for the entire function
    let _temp_guard;
    let arrow_source = if is_arrow_input(&input_path, from) {
        input_path.clone()
    } else {
        let format = match detect_input_format(&input_path, from)? {
            Some(fmt) => fmt,
            None => {
                bail!("Unable to determine import format for '{}'", input);
            }
        };

        let arrow_temp =
            tempdir().context("creating temporary directory for intermediate Arrow dataset")?;
        let arrow_path = arrow_temp.path().join("converted.arrow");
        fs::create_dir_all(&arrow_path)
            .with_context(|| format!("creating temp arrow directory {}", arrow_path.display()))?;
        let result = format.parse(input)?;

        // Show import diagnostics in verbose mode
        if verbose {
            let diag = &result.diagnostics;
            if diag.has_issues() {
                eprintln!("\n📋 Import Diagnostics:");
                eprintln!("   Warnings: {}, Errors: {}", diag.warning_count(), diag.error_count());
                for issue in &diag.issues {
                    let icon = if issue.severity == gat_io::helpers::Severity::Error {
                        "❌"
                    } else {
                        "⚠️"
                    };
                    eprintln!("   {} [{}] {}", icon, issue.category, issue.message);
                }
                eprintln!();
            } else {
                eprintln!("📋 Import completed with no warnings or errors");
            }
        }

        // Strict mode: fail if any warnings or errors occurred
        if strict && result.diagnostics.has_issues() {
            let diag = &result.diagnostics;
            let messages: Vec<String> = diag.issues.iter().map(|i| i.message.clone()).collect();
            bail!(
                "Strict mode: import produced {} warning(s), {} error(s):\n  - {}",
                diag.warning_count(),
                diag.error_count(),
                messages.join("\n  - ")
            );
        }

        write_network_to_arrow_directory(&result.network, &arrow_path)?;
        _temp_guard = arrow_temp; // Keep temp dir alive
        arrow_path
    };

    match to {
        ConvertFormat::Arrow => {
            let final_path = resolve_output_path(output, &input_path, ConvertFormat::Arrow)?;
            finalize_arrow_output(&arrow_source, &final_path, force)
        }
        ConvertFormat::Matpower => {
            let final_path = resolve_output_path(output, &input_path, ConvertFormat::Matpower)?;
            export_from_arrow(&arrow_source, &final_path, to, force)
        }
        ConvertFormat::Psse => {
            let final_path = resolve_output_path(output, &input_path, ConvertFormat::Psse)?;
            export_from_arrow(&arrow_source, &final_path, to, force)
        }
        ConvertFormat::Cim => {
            let final_path = resolve_output_path(output, &input_path, ConvertFormat::Cim)?;
            export_from_arrow(&arrow_source, &final_path, to, force)
        }
        ConvertFormat::Pandapower => {
            let final_path = resolve_output_path(output, &input_path, ConvertFormat::Pandapower)?;
            export_from_arrow(&arrow_source, &final_path, to, force)
        }
        ConvertFormat::Powermodels => {
            let final_path = resolve_output_path(output, &input_path, ConvertFormat::Powermodels)?;
            export_from_arrow(&arrow_source, &final_path, to, force)
        }
    }
}

fn is_arrow_input(input_path: &Path, override_format: Option<ConvertFormat>) -> bool {
    if matches!(override_format, Some(ConvertFormat::Arrow)) {
        return true;
    }
    input_path.is_dir() && input_path.join("manifest.json").exists()
}

fn detect_input_format(
    input_path: &Path,
    override_format: Option<ConvertFormat>,
) -> Result<Option<Format>> {
    if let Some(cf) = override_format {
        return Ok(cf.to_import_format());
    }
    if input_path.is_dir() {
        return Ok(None);
    }

    if let Some((format, _)) = Format::detect(input_path) {
        Ok(Some(format))
    } else {
        Ok(None)
    }
}

fn resolve_output_path(
    output: Option<&str>,
    input: &Path,
    target: ConvertFormat,
) -> Result<PathBuf> {
    if let Some(output) = output {
        return Ok(PathBuf::from(output));
    }

    let stem = input
        .file_stem()
        .or_else(|| input.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("converted");

    let default = match target {
        ConvertFormat::Arrow => format!("{stem}.arrow"),
        ConvertFormat::Matpower => format!("{stem}.m"),
        ConvertFormat::Psse => format!("{stem}.raw"),
        ConvertFormat::Cim => format!("{stem}.rdf"),
        ConvertFormat::Pandapower => format!("{stem}_pp.json"),
        ConvertFormat::Powermodels => format!("{stem}_pm.json"),
    };

    Ok(PathBuf::from(default))
}

fn export_from_arrow(
    arrow_dir: &Path,
    output_path: &Path,
    target_format: ConvertFormat,
    force: bool,
) -> Result<()> {
    // Check if output exists
    if output_path.exists() && !force {
        bail!(
            "Output file '{}' already exists; use --force to overwrite",
            output_path.display()
        );
    }

    // Load network from Arrow and capture manifest metadata
    let (network, manifest) = load_grid_from_arrow_with_manifest(arrow_dir).with_context(|| {
        format!(
            "loading network from Arrow directory: {}",
            arrow_dir.display()
        )
    })?;
    let metadata = ExportMetadata::from_manifest(&manifest);

    // Export to target format
    match target_format {
        ConvertFormat::Arrow => {
            unreachable!("Arrow export should be handled by finalize_arrow_output")
        }
        ConvertFormat::Matpower => {
            export_network_to_matpower(&network, output_path, Some(&metadata))
                .with_context(|| format!("exporting to MATPOWER: {}", output_path.display()))?;
        }
        ConvertFormat::Psse => {
            export_network_to_psse(&network, output_path, Some(&metadata))
                .with_context(|| format!("exporting to PSS/E: {}", output_path.display()))?;
        }
        ConvertFormat::Cim => {
            export_network_to_cim(&network, output_path, Some(&metadata))
                .with_context(|| format!("exporting to CIM: {}", output_path.display()))?;
        }
        ConvertFormat::Pandapower => {
            export_network_to_pandapower(&network, output_path, Some(&metadata))
                .with_context(|| format!("exporting to pandapower: {}", output_path.display()))?;
        }
        ConvertFormat::Powermodels => {
            export_network_to_powermodels(&network, output_path, Some(&metadata))
                .with_context(|| format!("exporting to PowerModels: {}", output_path.display()))?;
        }
    }

    println!(
        "✓ Converted to {} format: {}",
        other_string(target_format),
        output_path.display()
    );
    Ok(())
}

fn finalize_arrow_output(src: &Path, dest: &Path, force: bool) -> Result<()> {
    if dest.exists() {
        if force {
            fs::remove_dir_all(dest).with_context(|| {
                format!("removing existing output directory {}", dest.display())
            })?;
        } else {
            bail!(
                "Destination '{}' already exists; use --force to overwrite",
                dest.display()
            );
        }
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory for {}", dest.display()))?;
    }

    match fs::rename(src, dest) {
        Ok(_) => Ok(()),
        Err(err) => {
            if is_cross_device(&err) {
                copy_dir_all(src, dest)?;
                fs::remove_dir_all(src).with_context(|| {
                    format!("removing temporary arrow directory {}", src.display())
                })?;
                Ok(())
            } else {
                Err(err).context("renaming Arrow directory")
            }
        }
    }
}

fn copy_dir_all(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let to_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &to_path)?;
        } else {
            fs::copy(entry.path(), &to_path)?;
        }
    }
    Ok(())
}

fn is_cross_device(err: &std::io::Error) -> bool {
    err.raw_os_error() == Some(libc::EXDEV)
}

fn other_string(format: ConvertFormat) -> &'static str {
    match format {
        ConvertFormat::Arrow => "Arrow",
        ConvertFormat::Matpower => "MATPOWER",
        ConvertFormat::Psse => "PSS/E",
        ConvertFormat::Cim => "CIM",
        ConvertFormat::Pandapower => "pandapower",
        ConvertFormat::Powermodels => "PowerModels.jl",
    }
}
