use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use serde::Serialize;
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
    doc_cli()?;
    doc_schemas()?;
    doc_site()?;
    write_docs_index()?;
    Ok(())
}

fn doc_cli() -> Result<()> {
    let markdown = clap_markdown::help_markdown::<gat_cli::cli::Cli>();
    let cli_md = Path::new("docs/cli/gat.md");
    ensure_parent(cli_md)?;
    fs::write(cli_md, markdown)?;

    let command = gat_cli::build_cli_command();
    let mut man_buf = Vec::new();
    clap_mangen::Man::new(command.clone()).render(&mut man_buf)?;
    let man_path = Path::new("docs/man/gat.1");
    ensure_parent(man_path)?;
    fs::write(man_path, man_buf)?;

    Ok(())
}

fn doc_schemas() -> Result<()> {
    let manifest_schema = gat_cli::manifest::manifest_json_schema();
    let manifest_path = Path::new("docs/schemas/manifest.schema.json");
    ensure_parent(manifest_path)?;
    fs::write(
        manifest_path,
        serde_json::to_string_pretty(&manifest_schema)?,
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

#[derive(Serialize)]
struct DocIndex {
    default: String,
    updated_at: String,
    versions: Vec<DocVersion>,
}

#[derive(Serialize)]
struct DocVersion {
    name: String,
    uri: String,
    generated_at: String,
}

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

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
