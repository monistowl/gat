//! Native solver management commands.
//!
//! Provides CLI commands for listing, installing, and uninstalling
//! native solver plugins.

use anyhow::{anyhow, Result};
use gat_cli::cli::SolverCommands;
use gat_cli::install::{config, gat_home, solvers_state};
use std::path::PathBuf;

/// Handle solver subcommands.
pub fn handle(command: &SolverCommands) -> Result<()> {
    match command {
        SolverCommands::List => list_solvers(),
        SolverCommands::Install { solver, force } => install_solver(solver, *force),
        SolverCommands::Uninstall { solver } => uninstall_solver(solver),
        SolverCommands::Status => show_status(),
    }
}

/// Metadata for native solvers.
struct SolverMeta {
    name: &'static str,
    binary: &'static str,
    description: &'static str,
}

const NATIVE_SOLVERS: &[SolverMeta] = &[
    SolverMeta {
        name: "ipopt",
        binary: "gat-ipopt",
        description: "NLP interior-point optimizer",
    },
    SolverMeta {
        name: "highs",
        binary: "gat-highs",
        description: "LP/MIP high-performance",
    },
    SolverMeta {
        name: "cbc",
        binary: "gat-cbc",
        description: "MIP branch-and-cut",
    },
    SolverMeta {
        name: "bonmin",
        binary: "gat-bonmin",
        description: "MINLP branch-and-bound",
    },
    SolverMeta {
        name: "couenne",
        binary: "gat-couenne",
        description: "Global optimization",
    },
    SolverMeta {
        name: "symphony",
        binary: "gat-symphony",
        description: "Parallel MIP",
    },
];

/// List available and installed solvers.
fn list_solvers() -> Result<()> {
    println!("Available solvers:");
    println!();

    // Pure-Rust solvers (always available)
    println!("Pure-Rust (always available):");
    println!("  clarabel    Conic solver (SOCP, SDP)");
    println!("  lbfgs       Quasi-Newton NLP optimizer");
    println!();

    // Native solvers
    println!("Native solvers (require installation):");

    let solvers_dir = get_solvers_dir()?;
    let state = solvers_state::load_solvers_state()?;

    for meta in NATIVE_SOLVERS {
        let binary_path = solvers_dir.join(meta.binary);

        // Check both file existence and state registration
        let (status, version) = if let Some(info) = state.installed.get(meta.name) {
            if info.enabled && binary_path.exists() {
                ("[installed]".to_string(), Some(info.version.clone()))
            } else if !info.enabled {
                ("[disabled]".to_string(), Some(info.version.clone()))
            } else {
                ("[missing binary]".to_string(), None)
            }
        } else if binary_path.exists() {
            ("[unregistered]".to_string(), None)
        } else {
            ("[not installed]".to_string(), None)
        };

        let version_str = version.map(|v| format!(" v{}", v)).unwrap_or_default();
        println!(
            "  {:<12} {:<30} {}{}",
            meta.name, meta.description, status, version_str
        );
    }

    println!();
    println!("Install native solvers with: gat solver install <name>");
    println!("Or build from source: cargo xtask build-solver <name> --install");

    Ok(())
}

/// Install a native solver.
fn install_solver(solver: &str, force: bool) -> Result<()> {
    let solver_lower = solver.to_lowercase();

    // Validate solver name
    let valid_solvers = ["ipopt", "highs", "cbc", "bonmin", "couenne", "symphony"];
    if !valid_solvers.contains(&solver_lower.as_str()) {
        return Err(anyhow!(
            "Unknown solver '{}'. Valid options: {}",
            solver,
            valid_solvers.join(", ")
        ));
    }

    let solvers_dir = get_solvers_dir()?;
    let binary_name = format!("gat-{}", solver_lower);
    let binary_path = solvers_dir.join(&binary_name);

    if binary_path.exists() && !force {
        println!(
            "Solver '{}' is already installed at {}",
            solver,
            binary_path.display()
        );
        println!("Use --force to reinstall.");
        return Ok(());
    }

    // Create solvers directory if needed
    std::fs::create_dir_all(&solvers_dir)?;

    println!("Installing solver '{}'...", solver);
    println!();

    // For now, explain how to manually install
    // In the future, this would download pre-built binaries or build from source
    println!("Native solver installation is not yet automated.");
    println!();
    println!("To install {} manually:", solver);
    println!(
        "  1. Build the {} crate: cargo build -p {} --release",
        binary_name, binary_name
    );
    println!("  2. Copy the binary to: {}", binary_path.display());
    println!();
    println!(
        "Or use the xtask helper: cargo xtask build-solver {}",
        solver_lower
    );

    Ok(())
}

/// Uninstall a native solver.
fn uninstall_solver(solver: &str) -> Result<()> {
    let solver_lower = solver.to_lowercase();
    let solvers_dir = get_solvers_dir()?;
    let binary_name = format!("gat-{}", solver_lower);
    let binary_path = solvers_dir.join(&binary_name);

    let mut removed_binary = false;
    let mut removed_state = false;

    // Remove binary if it exists
    if binary_path.exists() {
        std::fs::remove_file(&binary_path)?;
        removed_binary = true;
    }

    // Remove from state registry
    if solvers_state::unregister_solver(&solver_lower)? {
        removed_state = true;
    }

    if removed_binary || removed_state {
        println!("Uninstalled solver '{}'.", solver);
        if removed_state {
            println!("  Removed from solver registry.");
        }
        if removed_binary {
            println!("  Removed binary: {}", binary_path.display());
        }
    } else {
        println!("Solver '{}' is not installed.", solver);
    }

    Ok(())
}

/// Show solver configuration status.
fn show_status() -> Result<()> {
    let config = config::load_gat_config()?;
    let state = solvers_state::load_solvers_state()?;

    println!("Solver configuration:");
    println!();
    println!(
        "  Native solvers enabled: {}",
        if config.solvers.native_enabled {
            "yes"
        } else {
            "no"
        }
    );
    println!("  Default LP solver:      {}", config.solvers.default_lp);
    println!("  Default NLP solver:     {}", config.solvers.default_nlp);
    println!(
        "  Timeout:                {} seconds",
        config.solvers.timeout_seconds
    );
    println!(
        "  Max iterations:         {}",
        config.solvers.max_iterations
    );
    println!();

    // Show installed solvers from registry
    if !state.installed.is_empty() {
        println!("Registered solvers (protocol v{}):", state.protocol_version);
        for (name, info) in &state.installed {
            let enabled_str = if info.enabled { "" } else { " [disabled]" };
            println!(
                "  {:<12} v{:<10} {}{}",
                name,
                info.version,
                info.binary_path.display(),
                enabled_str
            );
        }
        println!();
    } else {
        println!("No native solvers registered.");
        println!();
    }

    if !config.solvers.native_enabled {
        println!("To enable native solvers, add this to ~/.gat/config/gat.toml:");
        println!();
        println!("  [solvers]");
        println!("  native_enabled = true");
    }

    Ok(())
}

/// Get the solvers directory path.
fn get_solvers_dir() -> Result<PathBuf> {
    let gat_home = gat_home::gat_home()?;
    Ok(gat_home.join("solvers"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_solvers_dir() {
        let result = get_solvers_dir();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("solvers"));
    }
}
