//! Transmission Expansion Planning CLI commands

use anyhow::{Context, Result};
use gat_algo::tep::{solve_tep, CandidateId, TepProblemBuilder, TepSolverConfig};
use gat_cli::cli::TepCommands;
use gat_core::BusId;
use gat_io::importers;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};

use crate::commands::util::configure_threads;

/// Candidate line input format (JSON)
#[derive(Debug, Clone, Deserialize)]
struct CandidateInput {
    from_bus: usize,
    to_bus: usize,
    reactance_pu: f64,
    capacity_mw: f64,
    investment_cost: f64,
    #[serde(default)]
    name: Option<String>,
}

/// Solution output format
#[derive(Debug, Serialize)]
struct TepOutput {
    status: String,
    total_cost: f64,
    investment_cost: f64,
    operating_cost: f64,
    lines_built: Vec<LineBuildOutput>,
    solve_time_ms: u64,
}

#[derive(Debug, Serialize)]
struct LineBuildOutput {
    name: String,
    from_bus: usize,
    to_bus: usize,
    circuits_built: usize,
    investment_cost: f64,
}

pub fn handle(command: &TepCommands) -> Result<()> {
    match command {
        TepCommands::Solve {
            network,
            candidates,
            budget,
            out,
            threads,
        } => {
            configure_threads(threads);
            handle_solve(network, candidates, *budget, out)
        }
        TepCommands::Validate { candidates } => handle_validate(candidates),
    }
}

fn handle_solve(
    network_path: &str,
    candidates_path: &str,
    budget: Option<f64>,
    out_path: &str,
) -> Result<()> {
    // Warn if budget parameter provided
    if budget.is_some() {
        eprintln!("Warning: budget parameter is not yet implemented and will be ignored");
    }

    // Load network
    let network = importers::load_grid_from_arrow(network_path)
        .context("loading network from Arrow directory")?;

    println!("Network loaded: {} buses", network.stats().num_buses);

    // Load candidate lines
    let candidates: Vec<CandidateInput> = {
        let file = File::open(candidates_path)
            .with_context(|| format!("opening candidates file: {}", candidates_path))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).context("parsing candidates JSON")?
    };

    println!("Loaded {} candidate lines", candidates.len());

    // Build TEP problem and track candidate ID mapping
    let mut builder = TepProblemBuilder::new(network);
    let mut candidate_map: HashMap<CandidateId, CandidateInput> = HashMap::new();
    let mut next_id = 1; // CandidateId starts at 1

    for c in &candidates {
        let name = c
            .name
            .clone()
            .unwrap_or_else(|| format!("Candidate {}-{}", c.from_bus, c.to_bus));
        builder = builder.candidate(
            name,
            BusId::new(c.from_bus),
            BusId::new(c.to_bus),
            c.reactance_pu,
            c.capacity_mw,
            c.investment_cost,
        );
        candidate_map.insert(CandidateId::new(next_id), c.clone());
        next_id += 1;
    }

    let problem = builder.build();

    println!(
        "TEP problem: {} buses, {} candidates",
        problem.num_buses(),
        problem.num_candidates()
    );

    // Solve
    let config = TepSolverConfig::default();
    let start = std::time::Instant::now();
    let solution = solve_tep(&problem, &config).context("solving TEP problem")?;
    let solve_time = start.elapsed();

    // Build output
    let lines_built: Vec<LineBuildOutput> = solution
        .build_decisions
        .iter()
        .map(|d| {
            let candidate = candidate_map
                .get(&d.candidate_id)
                .expect("candidate_id should exist in map");
            LineBuildOutput {
                name: d.name.clone(),
                from_bus: candidate.from_bus,
                to_bus: candidate.to_bus,
                circuits_built: d.circuits_to_build,
                investment_cost: d.investment_cost,
            }
        })
        .collect();

    let output = TepOutput {
        status: if solution.optimal {
            "optimal".to_string()
        } else {
            "feasible".to_string()
        },
        total_cost: solution.total_cost,
        investment_cost: solution.investment_cost,
        operating_cost: solution.operating_cost,
        lines_built,
        solve_time_ms: solve_time.as_millis() as u64,
    };

    // Write output
    let json = serde_json::to_string_pretty(&output).context("serializing solution")?;
    let mut file = File::create(out_path).context("creating output file")?;
    file.write_all(json.as_bytes()).context("writing output")?;

    // Print summary
    println!("\nTEP Solution:");
    println!("  Status: {}", output.status);
    println!("  Total cost: ${:.2}", output.total_cost);
    println!("  Investment: ${:.2}", output.investment_cost);
    println!("  Operating: ${:.2}", output.operating_cost);
    println!(
        "  Lines built: {}/{}",
        output
            .lines_built
            .iter()
            .filter(|l| l.circuits_built > 0)
            .count(),
        output.lines_built.len()
    );
    println!("  Solve time: {} ms", output.solve_time_ms);
    println!("\nResults written to {}", out_path);

    Ok(())
}

fn handle_validate(candidates_path: &str) -> Result<()> {
    let file = File::open(candidates_path)
        .with_context(|| format!("opening candidates file: {}", candidates_path))?;
    let reader = BufReader::new(file);
    let candidates: Vec<CandidateInput> =
        serde_json::from_reader(reader).context("parsing candidates JSON")?;

    println!("Candidate lines file is valid");
    println!("  {} candidate lines", candidates.len());

    for (i, c) in candidates.iter().enumerate() {
        if c.reactance_pu <= 0.0 {
            println!("  Warning: Candidate {} has non-positive reactance", i + 1);
        }
        if c.capacity_mw <= 0.0 {
            println!("  Warning: Candidate {} has non-positive capacity", i + 1);
        }
        if c.investment_cost < 0.0 {
            println!(
                "  Warning: Candidate {} has negative investment cost",
                i + 1
            );
        }
    }

    Ok(())
}
