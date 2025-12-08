//! Multi-area reliability analysis command

use anyhow::{Context, Result};
use gat_algo::canos_multiarea::{AreaId, Corridor, MultiAreaMonteCarlo, MultiAreaSystem};
use gat_cli::cli::AnalyticsCommands;
use gat_io::importers;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use crate::commands::util::configure_threads;

#[derive(Debug, Deserialize)]
struct CorridorInput {
    id: usize,
    area_a: usize,
    area_b: usize,
    capacity_mw: f64,
    #[serde(default = "default_failure_rate")]
    failure_rate: f64,
}

fn default_failure_rate() -> f64 {
    0.01
}

#[derive(Debug, Serialize)]
struct MultiAreaOutput {
    areas: Vec<AreaMetrics>,
    system_lole: f64,
    system_eue: f64,
    samples: usize,
    solve_time_ms: u64,
}

#[derive(Debug, Serialize)]
struct AreaMetrics {
    area_id: usize,
    lole: f64,
    eue: f64,
}

pub fn handle(command: &AnalyticsCommands) -> Result<()> {
    let AnalyticsCommands::MultiArea {
        areas_dir,
        corridors,
        samples,
        seed,
        out,
        threads,
    } = command
    else {
        unreachable!();
    };

    configure_threads(threads);

    // Load corridor definitions
    let corridors_vec: Vec<CorridorInput> = {
        let file = File::open(corridors)
            .with_context(|| format!("opening corridors file: {}", corridors))?;
        serde_json::from_reader(BufReader::new(file)).context("parsing corridors JSON")?
    };

    // Discover area networks in directory
    let areas_path = Path::new(areas_dir);
    let mut area_networks: HashMap<usize, String> = HashMap::new();
    for entry in std::fs::read_dir(areas_path)
        .with_context(|| format!("reading areas directory: {}", areas_dir))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Check if it's an Arrow directory (check for buses.arrow, buses.parquet, or manifest.json)
            if path.join("buses.arrow").exists()
                || path.join("buses.parquet").exists()
                || path.join("manifest.json").exists()
            {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Try parsing directly, or strip .arrow/.parquet extension first
                    let name_without_ext = name
                        .strip_suffix(".arrow")
                        .or_else(|| name.strip_suffix(".parquet"))
                        .unwrap_or(name);

                    if let Ok(area_id) = name_without_ext.parse::<usize>() {
                        area_networks.insert(area_id, path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    if area_networks.is_empty() {
        anyhow::bail!("No area networks found in {}", areas_dir);
    }

    println!("Found {} areas", area_networks.len());

    // Build multi-area system
    let mut system = MultiAreaSystem::new();
    for (area_id, network_path) in &area_networks {
        let network = importers::load_grid_from_arrow(network_path)
            .with_context(|| format!("loading area {} network", area_id))?;
        system
            .add_area(AreaId(*area_id), network)
            .with_context(|| format!("adding area {}", area_id))?;
    }

    for c in &corridors_vec {
        let corridor = Corridor::new(c.id, AreaId(c.area_a), AreaId(c.area_b), c.capacity_mw)
            .with_failure_rate(c.failure_rate);
        system
            .add_corridor(corridor)
            .with_context(|| format!("adding corridor {}", c.id))?;
    }

    println!(
        "Multi-area system: {} areas, {} corridors",
        system.num_areas(),
        system.num_corridors()
    );

    // Run Monte Carlo
    let start = std::time::Instant::now();
    let mc = MultiAreaMonteCarlo::new(*samples);
    let mc = if let Some(_s) = seed {
        // Note: The API doesn't currently have with_seed, so we'll document this
        // and use the default for now
        println!("Warning: seed parameter not yet implemented in MultiAreaMonteCarlo; results will not be reproducible");
        mc
    } else {
        mc
    };
    let results = mc
        .compute_multiarea_reliability(&system)
        .context("running multi-area Monte Carlo")?;
    let solve_time = start.elapsed();

    // Build output
    let mut area_metrics: Vec<AreaMetrics> = results
        .area_lole
        .iter()
        .map(|(area_id, lole)| AreaMetrics {
            area_id: area_id.0,
            lole: *lole,
            eue: *results.area_eue.get(area_id).unwrap_or(&0.0),
        })
        .collect();

    // Sort by area_id for consistent output
    area_metrics.sort_by_key(|a| a.area_id);

    // Calculate system-wide metrics
    // System LOLE = max of area LOLEs (most constrained area determines system reliability)
    let system_lole = results
        .area_lole
        .values()
        .copied()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);
    // System EUE = sum of area EUEs (total unserved energy)
    let system_eue = results.area_eue.values().sum::<f64>();

    let output = MultiAreaOutput {
        areas: area_metrics,
        system_lole,
        system_eue,
        samples: *samples,
        solve_time_ms: solve_time.as_millis() as u64,
    };

    // Write output
    let json = serde_json::to_string_pretty(&output).context("serializing results")?;
    let mut file = File::create(out).context("creating output file")?;
    file.write_all(json.as_bytes()).context("writing output")?;

    println!("\nMulti-Area Reliability Results:");
    println!("  System LOLE: {:.2} hours/year", output.system_lole);
    println!("  System EUE: {:.2} MWh/year", output.system_eue);
    println!("  Samples: {}", samples);
    println!("  Time: {} ms", output.solve_time_ms);
    println!("\nPer-area results:");
    for area in &output.areas {
        println!(
            "  Area {}: LOLE={:.2} h/yr, EUE={:.2} MWh/yr",
            area.area_id, area.lole, area.eue
        );
    }
    println!("\nResults written to {}", out);

    Ok(())
}
