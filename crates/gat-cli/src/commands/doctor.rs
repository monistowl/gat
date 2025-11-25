use std::env;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use tabwriter::TabWriter;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Ok,
    Warn,
}

struct Check {
    name: &'static str,
    status: CheckStatus,
    detail: String,
}

impl Check {
    fn ok(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            status: CheckStatus::Ok,
            detail: detail.into(),
        }
    }

    fn warn(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            status: CheckStatus::Warn,
            detail: detail.into(),
        }
    }
}

pub fn handle() -> Result<()> {
    let checks = vec![
        check_path_entries(),
        check_solver_binaries(),
        check_cache_dir(),
        check_test_data(),
    ];

    let mut writer = TabWriter::new(Vec::new()).padding(2);
    writeln!(writer, "Check\tStatus\tDetails")?;
    for check in &checks {
        let status = match check.status {
            CheckStatus::Ok => "ok",
            CheckStatus::Warn => "warn",
        };
        writeln!(writer, "{}\t{}\t{}", check.name, status, check.detail)?;
    }
    writer.flush()?;
    let table = String::from_utf8(writer.into_inner()?)?;
    println!("{table}");

    if checks.iter().any(|c| c.status == CheckStatus::Warn) {
        eprintln!("Some checks reported warnings. Review the details above to complete setup.");
    }

    Ok(())
}

fn check_path_entries() -> Check {
    match env::var_os("PATH") {
        Some(path) => {
            let count = env::split_paths(&path).count();
            let detail = format!("PATH set with {count} entries");
            Check::ok("path", detail)
        }
        None => Check::warn("path", "PATH environment variable is not set"),
    }
}

fn check_solver_binaries() -> Check {
    const SOLVERS: &[&str] = &["highs", "cbc", "ipopt"];
    let missing: Vec<&str> = SOLVERS
        .iter()
        .copied()
        .filter(|solver| !command_on_path(solver))
        .collect();

    if missing.is_empty() {
        Check::ok("solvers", "HiGHS/CBC/IPOPT detected on PATH")
    } else if missing.len() == SOLVERS.len() {
        Check::warn(
            "solvers",
            "No optional solver binaries (HiGHS/CBC/IPOPT) found on PATH",
        )
    } else {
        Check::warn(
            "solvers",
            format!("Missing solver binaries: {}", missing.join(", ")),
        )
    }
}

fn check_cache_dir() -> Check {
    match dirs::cache_dir() {
        Some(base) => {
            let path = base.join("gat");
            if path.exists() {
                Check::ok("cache", format!("using cache directory at {}", path.display()))
            } else {
                Check::warn(
                    "cache",
                    format!("preferred cache directory {} does not exist yet", path.display()),
                )
            }
        }
        None => Check::warn("cache", "Could not resolve OS cache directory"),
    }
}

fn check_test_data() -> Check {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("test_data"));
    }
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        candidates.push(PathBuf::from(manifest_dir).join("../../test_data"));
    }

    if let Some(found) = candidates.into_iter().find(|p| p.exists()) {
        Check::ok(
            "test-data",
            format!("example datasets available at {}", found.display()),
        )
    } else {
        Check::warn(
            "test-data",
            "No local test_data directory found (set GAT_BIN to repo root for bundled samples)",
        )
    }
}

fn command_on_path(command: &str) -> bool {
    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths)
                .map(|p| p.join(command))
                .find(|candidate| candidate.is_file())
        })
        .is_some()
}

