use std::{fs, io, path::Path};

use anyhow::Result;
use clap_complete::{generate, Shell};

use gat_cli::cli::build_cli_command;

pub fn handle(shell: Shell, out: Option<&Path>) -> Result<()> {
    let mut cmd = build_cli_command();
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        generate(shell, &mut cmd, "gat", &mut file);
        println!("Wrote {shell:?} completion to {}", path.display());
    } else {
        let stdout = &mut io::stdout();
        generate(shell, &mut cmd, "gat", stdout);
    }
    Ok(())
}
