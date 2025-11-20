#![cfg(feature = "docs")]

use clap_mangen::Man;
use clap_markdown::help_markdown;

use crate::{build_cli_command, cli::Cli, manifest};

pub struct CliDocs {
    pub markdown: String,
    pub manpage: Vec<u8>,
}

pub struct DocArtifacts {
    pub cli: CliDocs,
    pub manifest_schema: serde_json::Value,
}

pub fn generate_doc_artifacts() -> DocArtifacts {
    let markdown = help_markdown::<Cli>();

    let command = build_cli_command();
    let mut man_buf = Vec::new();
    Man::new(command)
        .render(&mut man_buf)
        .expect("man rendering");

    DocArtifacts {
        cli: CliDocs {
            markdown,
            manpage: man_buf,
        },
        manifest_schema: manifest::manifest_json_schema(),
    }
}
