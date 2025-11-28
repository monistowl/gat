use anyhow::{bail, Result};
#[cfg(feature = "docs")]
use chrono::Utc;
use clap::{Parser, Subcommand};
#[cfg(feature = "docs")]
use serde::Serialize;
#[cfg(feature = "docs")]
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about = "Utility tasks for docs automation", long_about = None)]
struct Opt {
    #[command(subcommand)]
    command: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Documentation helpers (use `doc` + subcommand)
    #[command(subcommand)]
    Doc(DocCommand),
    /// `xtask doc:all`
    #[command(name = "doc:all")]
    DocAll,
    /// `xtask doc:cli`
    #[command(name = "doc:cli")]
    DocCli,
    /// `xtask doc:schemas`
    #[command(name = "doc:schemas")]
    DocSchemas,
    /// `xtask doc:site`
    #[command(name = "doc:site")]
    DocSite,
    /// Release helpers that rely on the canonical metadata
    #[command(subcommand)]
    Release(ReleaseCommand),
    /// Native solver build helpers
    #[command(subcommand)]
    Solver(SolverCommand),
    /// `xtask build-solver <name>` - shorthand for solver build
    #[command(name = "build-solver")]
    BuildSolver {
        /// Solver name (ipopt, highs, cbc, bonmin, couenne, symphony)
        solver: String,
        /// Install to ~/.gat/solvers/ after building
        #[arg(long)]
        install: bool,
    },
}

#[derive(Subcommand)]
enum ReleaseCommand {
    /// Print release metadata such as os/arch/version/tarball
    Info {
        /// Variant to describe (`headless` or `full`)
        #[arg(long, default_value = "headless")]
        variant: String,
        /// Optional version override (defaults to workspace release version)
        #[arg(long)]
        version: Option<String>,
    },
}

#[derive(Subcommand)]
enum SolverCommand {
    /// Build a native solver wrapper crate
    Build {
        /// Solver name (ipopt, highs, cbc, bonmin, couenne, symphony)
        solver: String,
        /// Install to ~/.gat/solvers/ after building
        #[arg(long)]
        install: bool,
    },
    /// List available solver crates and their build status
    List,
    /// Clean built solver binaries
    Clean {
        /// Solver name (or "all" to clean all)
        solver: String,
    },
}

#[derive(Subcommand)]
enum DocCommand {
    /// Run all doc generators
    All,
    /// Emit CLI Markdown/man exports
    Cli,
    /// Emit JSON schemas
    Schemas,
    /// Build a minimal site scaffold
    Site,
}

fn main() -> Result<()> {
    let opts = Opt::parse();
    match opts.command {
        Task::Doc(cmd) => match cmd {
            DocCommand::All => doc_all(),
            DocCommand::Cli => doc_cli(),
            DocCommand::Schemas => doc_schemas(),
            DocCommand::Site => doc_site(),
        },
        Task::DocAll => doc_all(),
        Task::DocCli => doc_cli(),
        Task::DocSchemas => doc_schemas(),
        Task::DocSite => doc_site(),
        Task::Release(cmd) => match cmd {
            ReleaseCommand::Info { variant, version } => {
                run_release_info(&variant, version.as_deref())
            }
        },
        Task::Solver(cmd) => match cmd {
            SolverCommand::Build { solver, install } => build_solver(&solver, install),
            SolverCommand::List => list_solvers(),
            SolverCommand::Clean { solver } => clean_solver(&solver),
        },
        Task::BuildSolver { solver, install } => build_solver(&solver, install),
    }
}

fn doc_all() -> Result<()> {
    #[cfg(feature = "docs")]
    {
        docgen::write_all_docs()
    }

    #[cfg(not(feature = "docs"))]
    {
        missing_docs_feature("doc:all")
    }
}

fn doc_cli() -> Result<()> {
    #[cfg(feature = "docs")]
    {
        docgen::write_cli_docs()
    }

    #[cfg(not(feature = "docs"))]
    {
        missing_docs_feature("doc:cli")
    }
}

fn doc_schemas() -> Result<()> {
    #[cfg(feature = "docs")]
    {
        docgen::write_schemas()
    }

    #[cfg(not(feature = "docs"))]
    {
        missing_docs_feature("doc:schemas")
    }
}

fn doc_site() -> Result<()> {
    let book_dir = Path::new("site/book");
    fs::create_dir_all(book_dir)?;
    let summary = r#"# Summary
- [Guide](../docs/guide/overview.md)
- [CLI reference](../docs/cli/gat.md)
- [Schemas](../docs/schemas/manifest.schema.json)
- [Arrow schema](../docs/schemas/flows.schema.json)
"#;
    fs::write(book_dir.join("SUMMARY.md"), summary)?;

    let index = r#"# GAT Docs Site
This book indexes the auto-generated docs that the MCC server exposes.

- [CLI reference](../docs/cli/gat.md)
- [Guide](../docs/guide/overview.md)
- [Manifest schema](../docs/schemas/manifest.schema.json)
- [Branch flow schema](../docs/schemas/flows.schema.json)
"#;
    fs::write(book_dir.join("index.md"), index)?;

    Ok(())
}

#[cfg(feature = "docs")]
#[derive(Serialize)]
struct DocIndex {
    default: String,
    updated_at: String,
    versions: Vec<DocVersion>,
}

#[cfg(feature = "docs")]
#[derive(Serialize)]
struct DocVersion {
    name: String,
    uri: String,
    generated_at: String,
}

#[cfg(feature = "docs")]
fn write_docs_index() -> Result<()> {
    let generated_at = Utc::now().to_rfc3339();
    let mut versions = vec![DocVersion {
        name: "latest".to_string(),
        uri: "/doc/index.md".to_string(),
        generated_at: generated_at.clone(),
    }];
    if let Ok(tagged) = std::env::var("GAT_DOCS_VERSION") {
        let tagged = tagged.trim();
        if !tagged.is_empty() && tagged != "latest" {
            versions.push(DocVersion {
                name: tagged.to_string(),
                uri: format!("/doc/index.md?version={tagged}"),
                generated_at: generated_at.clone(),
            });
        }
    }

    let index = DocIndex {
        default: "latest".to_string(),
        updated_at: generated_at,
        versions,
    };
    let path = Path::new("docs/index.json");
    ensure_parent(path)?;
    fs::write(path, serde_json::to_string_pretty(&index)?)?;
    Ok(())
}

#[cfg(feature = "docs")]
fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(not(feature = "docs"))]
fn missing_docs_feature(task: &str) -> Result<()> {
    bail!(
        "{task} requires building xtask with `--features docs` (e.g. `cargo run -p xtask --features docs -- {task}`)",
        task = task
    )
}

fn run_release_info(variant: &str, version: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("bash");
    cmd.arg("scripts/platform-info.sh")
        .arg("--variant")
        .arg(variant);
    if let Some(version) = version {
        cmd.arg("--version").arg(version);
    }
    let status = cmd.status()?.success();
    if !status {
        bail!("platform-info.sh failed");
    }
    Ok(())
}

// ============================================================================
// Solver build infrastructure
// ============================================================================

/// Information about a native solver wrapper crate.
struct SolverInfo {
    name: &'static str,
    crate_name: &'static str,
    description: &'static str,
    native_lib: &'static str,
}

const SOLVER_INFOS: &[SolverInfo] = &[
    SolverInfo {
        name: "ipopt",
        crate_name: "gat-ipopt",
        description: "Interior point optimizer for large-scale NLP",
        native_lib: "libipopt",
    },
    SolverInfo {
        name: "highs",
        crate_name: "gat-highs",
        description: "High-performance LP/MIP solver",
        native_lib: "libhighs",
    },
    SolverInfo {
        name: "cbc",
        crate_name: "gat-cbc",
        description: "COIN-OR branch and cut MIP solver",
        native_lib: "libCbc",
    },
    SolverInfo {
        name: "bonmin",
        crate_name: "gat-bonmin",
        description: "Basic Open-source Nonlinear Mixed Integer",
        native_lib: "libbonmin",
    },
    SolverInfo {
        name: "couenne",
        crate_name: "gat-couenne",
        description: "Convex Over and Under ENvelopes for Nonlinear Estimation",
        native_lib: "libcouenne",
    },
    SolverInfo {
        name: "symphony",
        crate_name: "gat-symphony",
        description: "COIN-OR parallel MIP solver",
        native_lib: "libSym",
    },
];

fn get_solver_info(name: &str) -> Option<&'static SolverInfo> {
    SOLVER_INFOS.iter().find(|s| s.name == name.to_lowercase())
}

fn get_gat_solvers_dir() -> Result<std::path::PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".gat").join("solvers"))
}

fn build_solver(solver: &str, install: bool) -> Result<()> {
    let info = get_solver_info(solver).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown solver '{}'. Valid solvers: {}",
            solver,
            SOLVER_INFOS
                .iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    println!("Building {} ({})...", info.crate_name, info.description);
    println!();

    // Check if the crate exists
    let crate_path = Path::new("crates").join(info.crate_name);
    if !crate_path.exists() {
        println!("Note: {} crate does not exist yet.", info.crate_name);
        println!();
        println!("To create the solver wrapper crate:");
        println!("  1. Create crates/{}/Cargo.toml", info.crate_name);
        println!("  2. Implement the solver IPC protocol from gat-solver-common");
        println!("  3. Link against {} native library", info.native_lib);
        println!();
        println!("See docs/architecture/native-solver-plugins.md for details.");
        return Ok(());
    }

    // Build the crate in release mode
    let status = Command::new("cargo")
        .args(["build", "-p", info.crate_name, "--release"])
        .status()?;

    if !status.success() {
        bail!("Failed to build {}", info.crate_name);
    }

    println!();
    println!("Successfully built {}!", info.crate_name);

    // Install if requested
    if install {
        install_solver_binary(info)?;
    } else {
        println!();
        println!(
            "To install, run: cargo xtask build-solver {} --install",
            solver
        );
        println!(
            "Or copy target/release/{} to ~/.gat/solvers/",
            info.crate_name
        );
    }

    Ok(())
}

fn install_solver_binary(info: &SolverInfo) -> Result<()> {
    let solvers_dir = get_gat_solvers_dir()?;
    fs::create_dir_all(&solvers_dir)?;

    let source = Path::new("target/release").join(info.crate_name);
    let dest = solvers_dir.join(info.crate_name);

    if !source.exists() {
        bail!(
            "Binary not found at {}. Build the solver first.",
            source.display()
        );
    }

    fs::copy(&source, &dest)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms)?;
    }

    // Register solver in solvers.toml
    register_solver_in_state(info)?;

    println!();
    println!("Installed {} to {}", info.crate_name, dest.display());
    println!();
    println!("To enable native solvers, add to ~/.gat/config/gat.toml:");
    println!("  [solvers]");
    println!("  native_enabled = true");

    Ok(())
}

fn register_solver_in_state(info: &SolverInfo) -> Result<()> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    let state_path = home.join(".gat").join("config").join("solvers.toml");

    // Read existing state or create new
    let mut state: toml::Table = if state_path.exists() {
        let content = fs::read_to_string(&state_path)?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        toml::Table::new()
    };

    // Ensure protocol_version
    state
        .entry("protocol_version".to_string())
        .or_insert(toml::Value::Integer(1));

    // Get or create installed table
    let installed = state
        .entry("installed".to_string())
        .or_insert(toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid solvers.toml format"))?;

    // Add/update solver entry
    let mut solver_entry = toml::Table::new();
    solver_entry.insert(
        "version".to_string(),
        toml::Value::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    solver_entry.insert(
        "binary_path".to_string(),
        toml::Value::String(
            home.join(".gat")
                .join("solvers")
                .join(info.crate_name)
                .to_string_lossy()
                .to_string(),
        ),
    );
    solver_entry.insert(
        "installed_at".to_string(),
        toml::Value::String(chrono::Utc::now().to_rfc3339()),
    );

    installed.insert(info.name.to_string(), toml::Value::Table(solver_entry));

    // Write back
    fs::create_dir_all(state_path.parent().unwrap())?;
    fs::write(&state_path, toml::to_string_pretty(&state)?)?;

    Ok(())
}

fn list_solvers() -> Result<()> {
    println!("Available native solver wrappers:");
    println!();

    let solvers_dir = get_gat_solvers_dir()?;

    for info in SOLVER_INFOS {
        let crate_path = Path::new("crates").join(info.crate_name);
        let binary_path = solvers_dir.join(info.crate_name);

        let crate_status = if crate_path.exists() {
            "crate exists"
        } else {
            "not implemented"
        };

        let install_status = if binary_path.exists() {
            "[installed]"
        } else {
            "[not installed]"
        };

        println!(
            "  {:<10} {:<50} {} {}",
            info.name, info.description, crate_status, install_status
        );
    }

    println!();
    println!("Build with: cargo xtask build-solver <name>");
    println!("Install with: cargo xtask build-solver <name> --install");

    Ok(())
}

fn clean_solver(solver: &str) -> Result<()> {
    if solver == "all" {
        println!("Cleaning all solver binaries...");
        let solvers_dir = get_gat_solvers_dir()?;
        if solvers_dir.exists() {
            for info in SOLVER_INFOS {
                let binary_path = solvers_dir.join(info.crate_name);
                if binary_path.exists() {
                    fs::remove_file(&binary_path)?;
                    println!("  Removed {}", binary_path.display());
                }
            }
        }
        println!("Done.");
        return Ok(());
    }

    let info = get_solver_info(solver).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown solver '{}'. Valid solvers: {} (or 'all')",
            solver,
            SOLVER_INFOS
                .iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let solvers_dir = get_gat_solvers_dir()?;
    let binary_path = solvers_dir.join(info.crate_name);

    if binary_path.exists() {
        fs::remove_file(&binary_path)?;
        println!("Removed {}", binary_path.display());
    } else {
        println!("{} is not installed.", info.name);
    }

    Ok(())
}

#[cfg(feature = "docs")]
mod docgen {
    use super::*;
    use gat_cli::docs::{generate_doc_artifacts, CliDocs, DocArtifacts};

    pub fn write_all_docs() -> Result<()> {
        let artifacts = generate_doc_artifacts();
        write_cli_files(&artifacts.cli)?;
        write_schema_files(&artifacts)?;
        super::doc_site()?;
        super::write_docs_index()
    }

    pub fn write_cli_docs() -> Result<()> {
        let artifacts = generate_doc_artifacts();
        write_cli_files(&artifacts.cli)
    }

    pub fn write_schemas() -> Result<()> {
        let artifacts = generate_doc_artifacts();
        write_schema_files(&artifacts)
    }

    fn write_cli_files(cli_docs: &CliDocs) -> Result<()> {
        let cli_md = Path::new("docs/cli/gat.md");
        ensure_parent(cli_md)?;
        fs::write(cli_md, &cli_docs.markdown)?;

        let man_path = Path::new("docs/man/gat.1");
        ensure_parent(man_path)?;
        fs::write(man_path, &cli_docs.manpage)?;

        Ok(())
    }

    fn write_schema_files(artifacts: &DocArtifacts) -> Result<()> {
        let manifest_path = Path::new("docs/schemas/manifest.schema.json");
        ensure_parent(manifest_path)?;
        fs::write(
            manifest_path,
            serde_json::to_string_pretty(&artifacts.manifest_schema)?,
        )?;

        let flows_schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Branch flows",
            "type": "object",
            "properties": {
                "branch_id": { "type": "integer" },
                "from_bus": { "type": "integer" },
                "to_bus": { "type": "integer" },
                "flow_mw": { "type": "number" }
            },
            "required": ["branch_id", "from_bus", "to_bus", "flow_mw"],
            "additionalProperties": false
        });
        let flows_path = Path::new("docs/schemas/flows.schema.json");
        ensure_parent(flows_path)?;
        fs::write(flows_path, serde_json::to_string_pretty(&flows_schema)?)?;

        Ok(())
    }
}
