#[cfg(not(feature = "docs"))]
use anyhow::bail;
use anyhow::Result;
#[cfg(feature = "docs")]
use chrono::Utc;
use clap::{Parser, Subcommand};
#[cfg(feature = "docs")]
use serde::Serialize;
#[cfg(feature = "docs")]
use serde_json::json;
use std::fs;
use std::path::Path;

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
