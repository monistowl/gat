//! Tauri commands for GAT demo frontend.
//!
//! These commands expose the GAT solver capabilities to the Svelte frontend,
//! converting internal data structures to JSON for visualization.

use std::path::{Path, PathBuf};

use std::collections::HashMap;

use gat_algo::opf::ac_nlp::SparseYBus;
use gat_algo::power_flow::ac_pf::AcPowerFlowSolver;
use gat_core::solver::{FaerSolver, LinearSystemBackend};
use gat_core::{Edge, Network, Node};
use gat_io::importers::{parse_matpower, Format};
use serde::{Deserialize, Serialize};

/// Information about an available case file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseInfo {
    pub name: String,
    pub path: String,
    pub buses: Option<usize>,
}

/// Bus data for frontend visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusJson {
    pub id: usize,
    pub name: String,
    #[serde(rename = "type")]
    pub bus_type: String,
    pub vm: f64,
    pub va: f64,
    pub p_load: f64,
    pub q_load: f64,
    pub voltage_kv: f64,
}

/// Branch data for frontend visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchJson {
    pub from: usize,
    pub to: usize,
    pub r: f64,
    pub x: f64,
    pub b: f64,
    pub p_flow: f64,
    pub loading_pct: f64,
    pub status: bool,
}

/// Generator data for frontend visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorJson {
    pub bus: usize,
    pub p_gen: f64,
    pub q_gen: f64,
    #[serde(rename = "type")]
    pub gen_type: String,
}

/// Complete network data for frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkJson {
    pub name: String,
    pub buses: Vec<BusJson>,
    pub branches: Vec<BranchJson>,
    pub generators: Vec<GeneratorJson>,
    pub base_mva: f64,
}

/// Convert a GAT Network to the frontend JSON format.
fn network_to_json(network: &Network, name: &str) -> NetworkJson {
    let mut buses = Vec::new();
    let mut generators = Vec::new();
    let mut branches = Vec::new();

    // Maps from NodeIndex to BusId for branch lookups
    let mut bus_id_map = std::collections::HashMap::new();
    // Track loads per bus
    let mut bus_loads: std::collections::HashMap<usize, (f64, f64)> =
        std::collections::HashMap::new();

    // First pass: collect buses, generators, and loads
    for node_idx in network.graph.node_indices() {
        let node = &network.graph[node_idx];
        match node {
            Node::Bus(bus) => {
                let bus_id = bus.id.value();
                bus_id_map.insert(node_idx, bus_id);

                buses.push(BusJson {
                    id: bus_id,
                    name: bus.name.clone(),
                    bus_type: "PQ".to_string(), // Will be updated if has generator
                    vm: bus.voltage_pu,
                    va: bus.angle_rad.to_degrees(),
                    p_load: 0.0,
                    q_load: 0.0,
                    voltage_kv: bus.voltage_kv,
                });
            }
            Node::Gen(gen) => {
                let bus_id = gen.bus.value();
                generators.push(GeneratorJson {
                    bus: bus_id,
                    p_gen: gen.active_power_mw,
                    q_gen: gen.reactive_power_mvar,
                    gen_type: "thermal".to_string(),
                });
                // Mark bus as PV (assume first gen's bus is slack if it's the largest)
                for b in &mut buses {
                    if b.id == bus_id && b.bus_type == "PQ" {
                        b.bus_type = "PV".to_string();
                    }
                }
            }
            Node::Load(load) => {
                let bus_id = load.bus.value();
                let entry = bus_loads.entry(bus_id).or_insert((0.0, 0.0));
                entry.0 += load.active_power_mw;
                entry.1 += load.reactive_power_mvar;
            }
            Node::Shunt(_) => {}
        }
    }

    // Update bus loads
    for bus in &mut buses {
        if let Some(&(p, q)) = bus_loads.get(&bus.id) {
            bus.p_load = p;
            bus.q_load = q;
        }
    }

    // Second pass: collect branches
    for edge_idx in network.graph.edge_indices() {
        let edge = &network.graph[edge_idx];

        match edge {
            Edge::Branch(branch) => {
                branches.push(BranchJson {
                    from: branch.from_bus.value(),
                    to: branch.to_bus.value(),
                    r: branch.resistance,
                    x: branch.reactance,
                    b: branch.charging_b_pu,
                    p_flow: 0.0,      // Filled after power flow solve
                    loading_pct: 0.0, // Filled after power flow solve
                    status: branch.status,
                });
            }
            Edge::Transformer(xfmr) => {
                branches.push(BranchJson {
                    from: xfmr.from_bus.value(),
                    to: xfmr.to_bus.value(),
                    r: 0.0,  // Transformer - impedance modeled differently
                    x: 0.01, // Small reactance placeholder
                    b: 0.0,
                    p_flow: 0.0,
                    loading_pct: 0.0,
                    status: true,
                });
            }
        }
    }

    NetworkJson {
        name: name.to_string(),
        buses,
        branches,
        generators,
        base_mva: 100.0, // Standard base MVA
    }
}

/// List available test cases from the pglib-opf directory.
#[tauri::command]
pub fn list_cases() -> Result<Vec<CaseInfo>, String> {
    // Path: crates/gat-gui/src-tauri -> crates/gat-gui -> crates -> workspace root
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.join("data/pglib-opf"))
        .ok_or("Could not find data directory")?;

    let mut cases = Vec::new();

    if data_dir.exists() {
        for entry in std::fs::read_dir(&data_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                let case_file = path.join("case.m");
                if case_file.exists() {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    // Extract bus count from name if present
                    let buses = extract_bus_count(&name);

                    cases.push(CaseInfo {
                        name: name.clone(),
                        path: case_file.to_string_lossy().to_string(),
                        buses,
                    });
                }
            }
        }
    }

    // Sort by bus count (smallest first)
    cases.sort_by_key(|c| c.buses.unwrap_or(usize::MAX));

    Ok(cases)
}

/// Extract bus count from case name like "pglib_opf_case14_ieee" -> 14
fn extract_bus_count(name: &str) -> Option<usize> {
    // Look for "case" followed by digits
    if let Some(idx) = name.find("case") {
        let rest = &name[idx + 4..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse().ok()
    } else {
        None
    }
}

/// Load a case file and return network data for visualization.
#[tauri::command]
pub fn load_case(path: &str) -> Result<NetworkJson, String> {
    let path_obj = Path::new(path);

    // Detect format and parse
    let result = if let Some((format, _)) = Format::detect(path_obj) {
        format.parse(path).map_err(|e| e.to_string())?
    } else {
        // Default to MATPOWER
        parse_matpower(path).map_err(|e| e.to_string())?
    };

    // Extract case name from path
    let name = path_obj
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(network_to_json(&result.network, &name))
}

/// Power flow solution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerFlowResult {
    pub buses: Vec<BusJson>,
    pub branches: Vec<BranchJson>,
    pub converged: bool,
    pub iterations: usize,
    pub max_mismatch: f64,
    pub solve_time_ms: f64,
}

/// Y-bus matrix entry for visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YbusEntry {
    pub row: usize,
    pub col: usize,
    pub g: f64, // Conductance (real part)
    pub b: f64, // Susceptance (imaginary part)
    pub magnitude: f64,
    pub from_bus_id: usize,
    pub to_bus_id: usize,
}

/// Y-bus matrix result for frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YbusJson {
    pub n_bus: usize,
    pub entries: Vec<YbusEntry>,
    pub bus_ids: Vec<usize>, // Bus ID for each row/column index
}

/// Build Y-bus admittance matrix for visualization.
#[tauri::command]
pub fn get_ybus(path: &str) -> Result<YbusJson, String> {
    let path_obj = Path::new(path);

    // Parse the case
    let result = if let Some((format, _)) = Format::detect(path_obj) {
        format.parse(path).map_err(|e| e.to_string())?
    } else {
        parse_matpower(path).map_err(|e| e.to_string())?
    };

    let network = &result.network;

    // Build bus ID list (in the order they appear in the graph)
    let mut bus_ids: Vec<usize> = Vec::new();
    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            bus_ids.push(bus.id.value());
        }
    }

    // Build sparse Y-bus
    let ybus = SparseYBus::from_network(network).map_err(|e| e.to_string())?;

    // Collect non-zero entries
    let mut entries = Vec::new();
    let n_bus = ybus.n_bus();

    // Use a set to track unique (row, col) pairs since G and B might have different sparsity
    let mut visited = std::collections::HashSet::new();

    for row in 0..n_bus {
        // Collect indices from G matrix
        for (col, _) in ybus.g_row_iter(row) {
            if visited.insert((row, col)) {
                let g = ybus.g(row, col);
                let b = ybus.b(row, col);
                let magnitude = (g * g + b * b).sqrt();

                entries.push(YbusEntry {
                    row,
                    col,
                    g,
                    b,
                    magnitude,
                    from_bus_id: bus_ids[row],
                    to_bus_id: bus_ids[col],
                });
            }
        }

        // Also check B matrix for entries that might not be in G
        for (col, _) in ybus.b_row_iter(row) {
            if visited.insert((row, col)) {
                let g = ybus.g(row, col);
                let b = ybus.b(row, col);
                let magnitude = (g * g + b * b).sqrt();

                entries.push(YbusEntry {
                    row,
                    col,
                    g,
                    b,
                    magnitude,
                    from_bus_id: bus_ids[row],
                    to_bus_id: bus_ids[col],
                });
            }
        }
    }

    // Sort by row, then column for consistent display
    entries.sort_by_key(|e| (e.row, e.col));

    Ok(YbusJson {
        n_bus,
        entries,
        bus_ids,
    })
}

/// Solve power flow for a loaded case.
#[tauri::command]
pub fn solve_power_flow(path: &str) -> Result<PowerFlowResult, String> {
    let path_obj = Path::new(path);
    let start = std::time::Instant::now();

    // Parse the case
    let result = if let Some((format, _)) = Format::detect(path_obj) {
        format.parse(path).map_err(|e| e.to_string())?
    } else {
        parse_matpower(path).map_err(|e| e.to_string())?
    };

    let network = &result.network;

    // Run power flow
    let solver = AcPowerFlowSolver::new()
        .with_tolerance(1e-6)
        .with_max_iterations(25);

    let pf_solution = solver.solve(network).map_err(|e| e.to_string())?;
    let solve_time = start.elapsed().as_secs_f64() * 1000.0;

    // Build response with updated voltages
    let mut buses = Vec::new();
    let mut bus_loads: std::collections::HashMap<usize, (f64, f64)> =
        std::collections::HashMap::new();

    // Collect loads first
    for node in network.graph.node_weights() {
        if let Node::Load(load) = node {
            let bus_id = load.bus.value();
            let entry = bus_loads.entry(bus_id).or_insert((0.0, 0.0));
            entry.0 += load.active_power_mw;
            entry.1 += load.reactive_power_mvar;
        }
    }

    // Build bus list with solved voltages
    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            let bus_id = bus.id;
            let vm = pf_solution
                .bus_voltage_magnitude
                .get(&bus_id)
                .copied()
                .unwrap_or(bus.voltage_pu);
            let va = pf_solution
                .bus_voltage_angle
                .get(&bus_id)
                .copied()
                .unwrap_or(bus.angle_rad)
                .to_degrees();

            let bus_type = pf_solution
                .bus_types
                .get(&bus_id)
                .map(|t| match t {
                    gat_algo::power_flow::ac_pf::BusType::Slack => "slack",
                    gat_algo::power_flow::ac_pf::BusType::PV => "PV",
                    gat_algo::power_flow::ac_pf::BusType::PQ => "PQ",
                })
                .unwrap_or("PQ")
                .to_string();

            let (p_load, q_load) = bus_loads
                .get(&bus_id.value())
                .copied()
                .unwrap_or((0.0, 0.0));

            buses.push(BusJson {
                id: bus_id.value(),
                name: bus.name.clone(),
                bus_type,
                vm,
                va,
                p_load,
                q_load,
                voltage_kv: bus.voltage_kv,
            });
        }
    }

    // Compute branch flows from solved voltages using AC power flow equations
    // P_ij = V_i² * g_ij - V_i * V_j * (g_ij * cos(θ_ij) + b_ij * sin(θ_ij))
    // where g_ij + j*b_ij = -Y_ij (off-diagonal admittance)
    let base_mva = 100.0;
    let mut branches = Vec::new();

    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            if !branch.status {
                branches.push(BranchJson {
                    from: branch.from_bus.value(),
                    to: branch.to_bus.value(),
                    r: branch.resistance,
                    x: branch.reactance,
                    b: branch.charging_b_pu,
                    p_flow: 0.0,
                    loading_pct: 0.0,
                    status: false,
                });
                continue;
            }

            let from_id = gat_core::BusId::new(branch.from_bus.value());
            let to_id = gat_core::BusId::new(branch.to_bus.value());

            // Get solved voltages
            let v_from = pf_solution
                .bus_voltage_magnitude
                .get(&from_id)
                .copied()
                .unwrap_or(1.0);
            let v_to = pf_solution
                .bus_voltage_magnitude
                .get(&to_id)
                .copied()
                .unwrap_or(1.0);
            let theta_from = pf_solution
                .bus_voltage_angle
                .get(&from_id)
                .copied()
                .unwrap_or(0.0);
            let theta_to = pf_solution
                .bus_voltage_angle
                .get(&to_id)
                .copied()
                .unwrap_or(0.0);

            // Branch admittance: y = 1/(r + jx), then g = r/(r² + x²), b = -x/(r² + x²)
            let z_sq = branch.resistance.powi(2) + branch.reactance.powi(2);
            let g_series = if z_sq > 1e-12 {
                branch.resistance / z_sq
            } else {
                0.0
            };
            let b_series = if z_sq > 1e-12 {
                -branch.reactance / z_sq
            } else {
                -1e6
            };

            // Angle difference
            let theta_ij = theta_from - theta_to - branch.phase_shift_rad;
            let cos_t = theta_ij.cos();
            let sin_t = theta_ij.sin();

            // Active power flow from i to j (in per-unit)
            // P_ij = V_i² * g_ij - V_i * V_j * (g_ij * cos(θ_ij) + b_ij * sin(θ_ij))
            let tap = branch.tap_ratio.max(0.01);
            let p_flow_pu = (v_from.powi(2) / tap.powi(2)) * g_series
                - (v_from * v_to / tap) * (g_series * cos_t + b_series * sin_t);
            let p_flow = p_flow_pu * base_mva;

            // Loading percentage based on rating
            let loading_pct = branch
                .rating_a_mva
                .map(|rating| (p_flow.abs() / rating * 100.0).min(999.0))
                .unwrap_or(0.0);

            branches.push(BranchJson {
                from: branch.from_bus.value(),
                to: branch.to_bus.value(),
                r: branch.resistance,
                x: branch.reactance,
                b: branch.charging_b_pu,
                p_flow,
                loading_pct,
                status: branch.status,
            });
        }
    }

    Ok(PowerFlowResult {
        buses,
        branches,
        converged: pf_solution.converged,
        iterations: pf_solution.iterations,
        max_mismatch: pf_solution.max_mismatch,
        solve_time_ms: solve_time,
    })
}

/// DC Power flow solution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcPowerFlowResult {
    pub buses: Vec<BusJson>,
    pub branches: Vec<BranchJson>,
    pub converged: bool,
    pub solve_time_ms: f64,
}

/// Solve DC power flow for a loaded case.
///
/// DC power flow is a linearized approximation that:
/// - Assumes voltage magnitudes are 1.0 p.u. (flat voltage profile)
/// - Ignores reactive power and losses
/// - Solves B'θ = P for bus angles
/// - Computes branch flows from angle differences
///
/// Much faster than AC but less accurate. Good for screening and contingency analysis.
#[tauri::command]
pub fn solve_dc_power_flow(path: &str) -> Result<DcPowerFlowResult, String> {
    let path_obj = Path::new(path);
    let start = std::time::Instant::now();

    // Parse the case
    let result = if let Some((format, _)) = Format::detect(path_obj) {
        format.parse(path).map_err(|e| e.to_string())?
    } else {
        parse_matpower(path).map_err(|e| e.to_string())?
    };

    let network = &result.network;
    let solver = FaerSolver;

    // Build bus susceptance matrix and get bus ordering
    let (bus_ids, bus_idx_map, susceptance) = build_bus_susceptance(network);
    let node_count = bus_ids.len();

    // Collect net injections (P_gen - P_load) per bus in MW, then convert to per-unit
    let base_mva = 100.0;
    let mut injections: HashMap<usize, f64> = HashMap::new();

    for node in network.graph.node_weights() {
        match node {
            Node::Gen(gen) => {
                let bus_id = gen.bus.value();
                *injections.entry(bus_id).or_default() += gen.active_power_mw / base_mva;
            }
            Node::Load(load) => {
                let bus_id = load.bus.value();
                *injections.entry(bus_id).or_default() -= load.active_power_mw / base_mva;
            }
            _ => {}
        }
    }

    // Solve B'θ = P for bus angles (excluding slack bus at index 0)
    let angles = if node_count <= 1 {
        bus_ids.iter().map(|&id| (id, 0.0)).collect()
    } else {
        // Build reduced susceptance matrix (exclude slack bus)
        let reduced_size = node_count - 1;
        let mut b_reduced: Vec<Vec<f64>> = vec![vec![0.0; reduced_size]; reduced_size];
        let mut p_reduced = vec![0.0; reduced_size];

        for i in 0..reduced_size {
            let bus_i = bus_ids[i + 1]; // Skip slack bus (index 0)
            p_reduced[i] = *injections.get(&bus_i).unwrap_or(&0.0);

            for j in 0..reduced_size {
                let bus_j = bus_ids[j + 1];
                // Get susceptance from full matrix
                if let Some(&row_idx) = bus_idx_map.get(&bus_i) {
                    if let Some(&col_idx) = bus_idx_map.get(&bus_j) {
                        b_reduced[i][j] = susceptance[row_idx][col_idx];
                    }
                }
            }
        }

        // Solve using linear system solver
        let theta_reduced = solver
            .solve(&b_reduced, &p_reduced)
            .map_err(|e| format!("DC power flow solve failed: {}", e))?;

        // Build full angle map (slack bus = 0)
        let mut angles: HashMap<usize, f64> = HashMap::new();
        angles.insert(bus_ids[0], 0.0); // Slack bus
        for (i, &theta) in theta_reduced.iter().enumerate() {
            angles.insert(bus_ids[i + 1], theta);
        }
        angles
    };

    let solve_time = start.elapsed().as_secs_f64() * 1000.0;

    // Build bus response with DC voltages (1.0 p.u., solved angles)
    let mut buses = Vec::new();
    let mut bus_loads: HashMap<usize, (f64, f64)> = HashMap::new();

    for node in network.graph.node_weights() {
        if let Node::Load(load) = node {
            let bus_id = load.bus.value();
            let entry = bus_loads.entry(bus_id).or_insert((0.0, 0.0));
            entry.0 += load.active_power_mw;
            entry.1 += load.reactive_power_mvar;
        }
    }

    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            let bus_id = bus.id.value();
            let va = angles.get(&bus_id).copied().unwrap_or(0.0).to_degrees();
            let (p_load, q_load) = bus_loads.get(&bus_id).copied().unwrap_or((0.0, 0.0));

            buses.push(BusJson {
                id: bus_id,
                name: bus.name.clone(),
                bus_type: "PQ".to_string(), // DC doesn't distinguish types
                vm: 1.0,                    // DC assumes flat voltage
                va,
                p_load,
                q_load,
                voltage_kv: bus.voltage_kv,
            });
        }
    }

    // Compute branch flows from angle differences
    let mut branches = Vec::new();
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            if !branch.status {
                continue;
            }
            let theta_from = angles.get(&branch.from_bus.value()).copied().unwrap_or(0.0);
            let theta_to = angles.get(&branch.to_bus.value()).copied().unwrap_or(0.0);

            // Flow = (θ_from - θ_to - phase_shift) / x * base_mva
            let reactance = (branch.reactance * branch.tap_ratio).abs().max(1e-6);
            let flow_pu = ((theta_from - theta_to) - branch.phase_shift_rad) / reactance;
            let p_flow = flow_pu * base_mva;

            // Loading percentage based on rating
            let loading_pct = branch
                .rating_a_mva
                .map(|rating| (p_flow.abs() / rating * 100.0).min(999.0))
                .unwrap_or(0.0);

            branches.push(BranchJson {
                from: branch.from_bus.value(),
                to: branch.to_bus.value(),
                r: branch.resistance,
                x: branch.reactance,
                b: branch.charging_b_pu,
                p_flow,
                loading_pct,
                status: branch.status,
            });
        }
    }

    Ok(DcPowerFlowResult {
        buses,
        branches,
        converged: true, // DC always converges (single linear solve)
        solve_time_ms: solve_time,
    })
}

/// Build bus susceptance matrix B' for DC power flow.
/// Returns (bus_ids, bus_idx_map, susceptance_matrix as 2D Vec).
fn build_bus_susceptance(network: &Network) -> (Vec<usize>, HashMap<usize, usize>, Vec<Vec<f64>>) {
    // Collect all bus IDs
    let mut bus_ids: Vec<usize> = network
        .graph
        .node_weights()
        .filter_map(|n| {
            if let Node::Bus(bus) = n {
                Some(bus.id.value())
            } else {
                None
            }
        })
        .collect();
    bus_ids.sort();

    let n = bus_ids.len();
    let bus_idx_map: HashMap<usize, usize> =
        bus_ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();

    // Build dense susceptance matrix B' (imaginary part of Y-bus, negated off-diagonals)
    let mut b_matrix = vec![vec![0.0; n]; n];

    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            if !branch.status {
                continue;
            }
            let from_idx = bus_idx_map.get(&branch.from_bus.value());
            let to_idx = bus_idx_map.get(&branch.to_bus.value());

            if let (Some(&i), Some(&j)) = (from_idx, to_idx) {
                let reactance = (branch.reactance * branch.tap_ratio).abs().max(1e-6);
                let b = 1.0 / reactance;

                // Off-diagonal: -b
                b_matrix[i][j] -= b;
                b_matrix[j][i] -= b;

                // Diagonal: +b (from this branch)
                b_matrix[i][i] += b;
                b_matrix[j][j] += b;
            }
        }
    }

    (bus_ids, bus_idx_map, b_matrix)
}

// ============================================================================
// N-1 Contingency Analysis
// ============================================================================

/// A single contingency result (one branch outage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContingencyResult {
    /// Branch that was removed (from-to bus IDs)
    pub outage_from: usize,
    pub outage_to: usize,
    /// Whether this contingency causes any violations
    pub has_violations: bool,
    /// Branches that are overloaded (loading > 100%)
    pub overloaded_branches: Vec<OverloadedBranch>,
    /// Maximum loading percentage across all branches
    pub max_loading_pct: f64,
    /// Solve succeeded (false if island created)
    pub solved: bool,
}

/// An overloaded branch in a contingency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverloadedBranch {
    pub from: usize,
    pub to: usize,
    pub loading_pct: f64,
    pub flow_mw: f64,
    pub rating_mva: f64,
}

/// N-1 contingency analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N1ContingencyResult {
    pub total_contingencies: usize,
    pub contingencies_with_violations: usize,
    pub contingencies_failed: usize,
    pub results: Vec<ContingencyResult>,
    pub worst_contingency: Option<ContingencyResult>,
    pub solve_time_ms: f64,
}

/// Run N-1 contingency analysis using DC power flow.
///
/// For each branch, temporarily remove it and solve DC power flow
/// to check if remaining branches become overloaded.
#[tauri::command]
pub fn run_n1_contingency(path: &str) -> Result<N1ContingencyResult, String> {
    let path_obj = Path::new(path);
    let start = std::time::Instant::now();

    // Parse the case
    let result = if let Some((format, _)) = Format::detect(path_obj) {
        format.parse(path).map_err(|e| e.to_string())?
    } else {
        parse_matpower(path).map_err(|e| e.to_string())?
    };

    let network = &result.network;
    let solver = FaerSolver;
    let base_mva = 100.0;

    // Collect all active branches with their info
    let mut branches_info: Vec<(usize, usize, f64, f64, Option<f64>, f64)> = Vec::new();
    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            if branch.status {
                let reactance = (branch.reactance * branch.tap_ratio).abs().max(1e-6);
                branches_info.push((
                    branch.from_bus.value(),
                    branch.to_bus.value(),
                    reactance,
                    branch.phase_shift_rad,
                    branch.rating_a_mva,
                    branch.tap_ratio,
                ));
            }
        }
    }

    // Collect bus info and injections
    let mut bus_ids: Vec<usize> = network
        .graph
        .node_weights()
        .filter_map(|n| {
            if let Node::Bus(bus) = n {
                Some(bus.id.value())
            } else {
                None
            }
        })
        .collect();
    bus_ids.sort();

    let n = bus_ids.len();
    let bus_idx_map: HashMap<usize, usize> =
        bus_ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();

    // Collect net injections
    let mut injections: HashMap<usize, f64> = HashMap::new();
    for node in network.graph.node_weights() {
        match node {
            Node::Gen(gen) => {
                let bus_id = gen.bus.value();
                *injections.entry(bus_id).or_default() += gen.active_power_mw / base_mva;
            }
            Node::Load(load) => {
                let bus_id = load.bus.value();
                *injections.entry(bus_id).or_default() -= load.active_power_mw / base_mva;
            }
            _ => {}
        }
    }

    let mut results = Vec::new();
    let mut worst_loading = 0.0;
    let mut worst_result: Option<ContingencyResult> = None;

    // For each branch, run contingency analysis
    for outage_idx in 0..branches_info.len() {
        let (outage_from, outage_to, _, _, _, _) = branches_info[outage_idx];

        // Build susceptance matrix excluding this branch
        let mut b_matrix = vec![vec![0.0; n]; n];
        for (idx, &(from, to, reactance, _, _, _)) in branches_info.iter().enumerate() {
            if idx == outage_idx {
                continue; // Skip the outaged branch
            }
            if let (Some(&i), Some(&j)) = (bus_idx_map.get(&from), bus_idx_map.get(&to)) {
                let b = 1.0 / reactance;
                b_matrix[i][j] -= b;
                b_matrix[j][i] -= b;
                b_matrix[i][i] += b;
                b_matrix[j][j] += b;
            }
        }

        // Solve for angles (excluding slack bus at index 0)
        let angles: HashMap<usize, f64> = if n <= 1 {
            bus_ids.iter().map(|&id| (id, 0.0)).collect()
        } else {
            let reduced_size = n - 1;
            let mut b_reduced: Vec<Vec<f64>> = vec![vec![0.0; reduced_size]; reduced_size];
            let mut p_reduced = vec![0.0; reduced_size];

            for i in 0..reduced_size {
                let bus_i = bus_ids[i + 1];
                p_reduced[i] = *injections.get(&bus_i).unwrap_or(&0.0);
                for j in 0..reduced_size {
                    let bus_j = bus_ids[j + 1];
                    if let (Some(&row_idx), Some(&col_idx)) =
                        (bus_idx_map.get(&bus_i), bus_idx_map.get(&bus_j))
                    {
                        b_reduced[i][j] = b_matrix[row_idx][col_idx];
                    }
                }
            }

            // Try to solve - may fail if contingency creates an island
            match solver.solve(&b_reduced, &p_reduced) {
                Ok(theta_reduced) => {
                    let mut angles: HashMap<usize, f64> = HashMap::new();
                    angles.insert(bus_ids[0], 0.0);
                    for (i, &theta) in theta_reduced.iter().enumerate() {
                        angles.insert(bus_ids[i + 1], theta);
                    }
                    angles
                }
                Err(_) => {
                    // Contingency creates island or singular matrix
                    results.push(ContingencyResult {
                        outage_from,
                        outage_to,
                        has_violations: true,
                        overloaded_branches: vec![],
                        max_loading_pct: 999.0,
                        solved: false,
                    });
                    continue;
                }
            }
        };

        // Calculate flows on remaining branches
        let mut overloaded = Vec::new();
        let mut max_loading = 0.0;

        for (idx, &(from, to, reactance, phase_shift, rating, _)) in
            branches_info.iter().enumerate()
        {
            if idx == outage_idx {
                continue;
            }

            let theta_from = angles.get(&from).copied().unwrap_or(0.0);
            let theta_to = angles.get(&to).copied().unwrap_or(0.0);
            let flow_pu = (theta_from - theta_to - phase_shift) / reactance;
            let flow_mw = flow_pu * base_mva;

            let loading_pct = rating
                .map(|r| (flow_mw.abs() / r * 100.0).min(999.0))
                .unwrap_or(0.0);

            if loading_pct > max_loading {
                max_loading = loading_pct;
            }

            if loading_pct > 100.0 {
                overloaded.push(OverloadedBranch {
                    from,
                    to,
                    loading_pct,
                    flow_mw,
                    rating_mva: rating.unwrap_or(0.0),
                });
            }
        }

        let has_violations = !overloaded.is_empty();
        let contingency = ContingencyResult {
            outage_from,
            outage_to,
            has_violations,
            overloaded_branches: overloaded,
            max_loading_pct: max_loading,
            solved: true,
        };

        if max_loading > worst_loading {
            worst_loading = max_loading;
            worst_result = Some(contingency.clone());
        }

        results.push(contingency);
    }

    let solve_time = start.elapsed().as_secs_f64() * 1000.0;
    let contingencies_with_violations = results.iter().filter(|r| r.has_violations).count();
    let contingencies_failed = results.iter().filter(|r| !r.solved).count();

    // Sort results: violations first, then by max loading descending
    results.sort_by(|a, b| {
        if a.has_violations != b.has_violations {
            b.has_violations.cmp(&a.has_violations)
        } else {
            b.max_loading_pct
                .partial_cmp(&a.max_loading_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    Ok(N1ContingencyResult {
        total_contingencies: branches_info.len(),
        contingencies_with_violations,
        contingencies_failed,
        results,
        worst_contingency: worst_result,
        solve_time_ms: solve_time,
    })
}

// ============================================================================
// Configuration Management
// ============================================================================

/// Solver configuration for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SolverConfigJson {
    pub native_enabled: bool,
    pub default_lp: String,
    pub default_nlp: String,
    pub timeout_seconds: u64,
    pub max_iterations: u32,
}

/// Logging configuration for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoggingConfigJson {
    pub level: String,
}

/// Data paths configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataConfigJson {
    pub grid_cache: String,
    pub results_dir: String,
}

/// UI configuration for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiConfigJson {
    pub theme: String,
    pub enable_animations: bool,
}

/// Complete application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfigJson {
    pub solvers: SolverConfigJson,
    pub logging: LoggingConfigJson,
    pub data: DataConfigJson,
    pub ui: UiConfigJson,
}

/// Get the GAT config file path.
fn gat_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".gat")
        .join("config")
        .join("gat.toml")
}

/// Load configuration from file or return defaults.
#[tauri::command]
pub fn get_config() -> Result<AppConfigJson, String> {
    let config_path = gat_config_path();

    // Try to load existing config
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;

        // Parse TOML - we'll manually extract fields since the structure might differ
        let toml_value: toml::Value = content
            .parse()
            .map_err(|e: toml::de::Error| e.to_string())?;

        let config = AppConfigJson {
            solvers: SolverConfigJson {
                native_enabled: toml_value
                    .get("solvers")
                    .and_then(|s| s.get("native_enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                default_lp: toml_value
                    .get("solvers")
                    .and_then(|s| s.get("default_lp"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("clarabel")
                    .to_string(),
                default_nlp: toml_value
                    .get("solvers")
                    .and_then(|s| s.get("default_nlp"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("lbfgs")
                    .to_string(),
                timeout_seconds: toml_value
                    .get("solvers")
                    .and_then(|s| s.get("timeout_seconds"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(300) as u64,
                max_iterations: toml_value
                    .get("solvers")
                    .and_then(|s| s.get("max_iterations"))
                    .and_then(|v| v.as_integer())
                    .unwrap_or(1000) as u32,
            },
            logging: LoggingConfigJson {
                level: toml_value
                    .get("logging")
                    .and_then(|l| l.get("level"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("info")
                    .to_string(),
            },
            data: DataConfigJson {
                grid_cache: toml_value
                    .get("data")
                    .and_then(|d| d.get("grid_cache"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("~/.gat/cache/grids")
                    .to_string(),
                results_dir: toml_value
                    .get("data")
                    .and_then(|d| d.get("results_dir"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("~/.gat/results")
                    .to_string(),
            },
            ui: UiConfigJson {
                theme: toml_value
                    .get("ui")
                    .and_then(|u| u.get("theme"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("dark")
                    .to_string(),
                enable_animations: toml_value
                    .get("ui")
                    .and_then(|u| u.get("enable_animations"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
            },
        };

        Ok(config)
    } else {
        // Return defaults
        Ok(AppConfigJson {
            solvers: SolverConfigJson {
                native_enabled: true,
                default_lp: "clarabel".to_string(),
                default_nlp: "lbfgs".to_string(),
                timeout_seconds: 300,
                max_iterations: 1000,
            },
            logging: LoggingConfigJson {
                level: "info".to_string(),
            },
            data: DataConfigJson {
                grid_cache: "~/.gat/cache/grids".to_string(),
                results_dir: "~/.gat/results".to_string(),
            },
            ui: UiConfigJson {
                theme: "dark".to_string(),
                enable_animations: true,
            },
        })
    }
}

/// Save configuration to file.
#[tauri::command]
pub fn save_config(config: AppConfigJson) -> Result<(), String> {
    let config_path = gat_config_path();

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Build TOML content
    let toml_content = format!(
        r#"# GAT Configuration File
# Generated by gat-demo

[solvers]
native_enabled = {}
default_lp = "{}"
default_nlp = "{}"
timeout_seconds = {}
max_iterations = {}

[logging]
level = "{}"

[data]
grid_cache = "{}"
results_dir = "{}"

[ui]
theme = "{}"
enable_animations = {}
"#,
        config.solvers.native_enabled,
        config.solvers.default_lp,
        config.solvers.default_nlp,
        config.solvers.timeout_seconds,
        config.solvers.max_iterations,
        config.logging.level,
        config.data.grid_cache,
        config.data.results_dir,
        config.ui.theme,
        config.ui.enable_animations,
    );

    std::fs::write(&config_path, toml_content).map_err(|e| e.to_string())?;

    Ok(())
}

/// Get the path to the config file.
#[tauri::command]
pub fn get_config_path() -> Result<String, String> {
    Ok(gat_config_path().to_string_lossy().to_string())
}

// ============================================================================
// Notebook Management
// ============================================================================

/// A demo notebook entry from the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookDemo {
    pub title: String,
    pub description: String,
    pub path: String,
}

/// Quick action from the notebook manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAction {
    pub label: String,
    pub command: String,
    pub notes: String,
}

/// Notebook manifest data for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookManifest {
    pub app: String,
    pub description: String,
    pub workspace: String,
    pub port: u16,
    pub notebooks_dir: String,
    pub datasets_dir: String,
    pub context_dir: String,
    pub demos: Vec<NotebookDemo>,
    pub quick_actions: Vec<QuickAction>,
    pub status_badges: Vec<String>,
}

/// Get the notebook workspace path.
fn notebook_workspace_path() -> PathBuf {
    // Path: crates/gat-gui/src-tauri -> crates/gat-gui -> crates -> workspace root
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_notebook = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.join("gat-notebook"));

    if let Some(path) = workspace_notebook {
        if path.exists() {
            return path;
        }
    }

    // Fall back to home directory
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".gat")
        .join("notebook")
}

/// Get the notebook manifest with demos and quick actions.
#[tauri::command]
pub fn get_notebook_manifest() -> Result<NotebookManifest, String> {
    let workspace = notebook_workspace_path();
    let manifest_path = workspace.join("notebook.manifest.json");

    // Try to read existing manifest
    if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        let demos: Vec<NotebookDemo> = json
            .get("demos")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(NotebookDemo {
                            title: v.get("title")?.as_str()?.to_string(),
                            description: v.get("description")?.as_str()?.to_string(),
                            path: v.get("path")?.as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        return Ok(NotebookManifest {
            app: json
                .get("app")
                .and_then(|v| v.as_str())
                .unwrap_or("gat-notebook")
                .to_string(),
            description: json
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("GAT research notebook")
                .to_string(),
            workspace: workspace.to_string_lossy().to_string(),
            port: json.get("port").and_then(|v| v.as_u64()).unwrap_or(8787) as u16,
            notebooks_dir: json
                .get("notebooks_dir")
                .and_then(|v| v.as_str())
                .unwrap_or("notebooks")
                .to_string(),
            datasets_dir: json
                .get("datasets_dir")
                .and_then(|v| v.as_str())
                .unwrap_or("datasets")
                .to_string(),
            context_dir: json
                .get("context_dir")
                .and_then(|v| v.as_str())
                .unwrap_or("context")
                .to_string(),
            demos,
            quick_actions: vec![
                QuickAction {
                    label: "Run DC Power Flow".to_string(),
                    command: "gat pf dc datasets/ieee14.arrow --out notebooks/pf_dc.parquet"
                        .to_string(),
                    notes: "Quick DC power flow on loaded grid".to_string(),
                },
                QuickAction {
                    label: "Run AC Power Flow".to_string(),
                    command: "gat pf ac datasets/ieee14.arrow --out notebooks/pf_ac.parquet"
                        .to_string(),
                    notes: "Full Newton-Raphson AC power flow".to_string(),
                },
                QuickAction {
                    label: "Batch Analysis".to_string(),
                    command: "gat batch pf --manifest datasets/runs/manifest.json --max-jobs 8"
                        .to_string(),
                    notes: "Run batch power flows from manifest".to_string(),
                },
            ],
            status_badges: vec![
                "Workspace ready".to_string(),
                "12 demo notebooks available".to_string(),
            ],
        });
    }

    // Return defaults if no manifest exists
    Ok(NotebookManifest {
        app: "gat-notebook".to_string(),
        description: "A research-grade notebook tuned for GAT runs, outputs, and RAG notes."
            .to_string(),
        workspace: workspace.to_string_lossy().to_string(),
        port: 8787,
        notebooks_dir: "notebooks".to_string(),
        datasets_dir: "datasets".to_string(),
        context_dir: "context".to_string(),
        demos: vec![
            NotebookDemo {
                title: "Power flow walkthrough".to_string(),
                description: "Import a grid, run DC/AC flows, and inspect violations.".to_string(),
                path: "notebooks/demos/power-flow.md".to_string(),
            },
            NotebookDemo {
                title: "Scenario + batch analysis".to_string(),
                description:
                    "Materialize scenarios and execute batch studies with limits and solver controls."
                        .to_string(),
                path: "notebooks/demos/scenario-batch.md".to_string(),
            },
            NotebookDemo {
                title: "Data validation and cleanup".to_string(),
                description:
                    "Validate topology, catch islands, and prepare a clean grid artifact for studies."
                        .to_string(),
                path: "notebooks/demos/validation.md".to_string(),
            },
            NotebookDemo {
                title: "Time-series and forecasting".to_string(),
                description:
                    "Run time-coupled OPF, stats, and forecasts with reusable Parquet outputs."
                        .to_string(),
                path: "notebooks/demos/time-series.md".to_string(),
            },
            NotebookDemo {
                title: "Contingency analysis".to_string(),
                description: "Run N-1 screening, triage violations, and capture remediation ideas."
                    .to_string(),
                path: "notebooks/demos/contingency-resilience.md".to_string(),
            },
            NotebookDemo {
                title: "Solver benchmarking".to_string(),
                description: "Compare OPF solvers, capture runtimes, and persist benchmarks."
                    .to_string(),
                path: "notebooks/demos/solver-benchmarks.md".to_string(),
            },
        ],
        quick_actions: vec![
            QuickAction {
                label: "Run DC Power Flow".to_string(),
                command: "gat pf dc datasets/ieee14.arrow --out notebooks/pf_dc.parquet"
                    .to_string(),
                notes: "Quick DC power flow on loaded grid".to_string(),
            },
            QuickAction {
                label: "Run AC Power Flow".to_string(),
                command: "gat pf ac datasets/ieee14.arrow --out notebooks/pf_ac.parquet"
                    .to_string(),
                notes: "Full Newton-Raphson AC power flow".to_string(),
            },
        ],
        status_badges: vec!["No workspace initialized".to_string()],
    })
}

/// Read a notebook file content.
#[tauri::command]
pub fn read_notebook(path: String) -> Result<String, String> {
    let workspace = notebook_workspace_path();
    let full_path = workspace.join(&path);

    if full_path.exists() {
        std::fs::read_to_string(&full_path).map_err(|e| e.to_string())
    } else {
        Err(format!("Notebook not found: {}", path))
    }
}

/// Initialize a notebook workspace (calls gat-notebook's seed logic).
#[tauri::command]
pub fn init_notebook_workspace(workspace_path: Option<String>) -> Result<String, String> {
    let workspace = workspace_path
        .map(PathBuf::from)
        .unwrap_or_else(notebook_workspace_path);

    // Create directories
    std::fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(workspace.join("notebooks")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(workspace.join("notebooks/demos")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(workspace.join("datasets")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(workspace.join("context")).map_err(|e| e.to_string())?;

    // Create a basic README
    let readme_path = workspace.join("README.md");
    if !readme_path.exists() {
        let readme = r#"# GAT Notebook Workspace

This folder mirrors the layout used by the Twinsong notebook experience, but tuned for
Grid Analysis Toolkit (GAT) workflows:

- Drop Arrow grids, Parquet runs, and YAML scenario specs under `datasets/`.
- Capture exploratory prompts and decisions inside `notebooks/`.
- Persist batch or RAG context in `context/`.
"#;
        std::fs::write(&readme_path, readme).map_err(|e| e.to_string())?;
    }

    Ok(workspace.to_string_lossy().to_string())
}

// ============================================================================
// Batch Job Management
// ============================================================================

use crate::state::AppState;
use gat_batch::{run_batch as batch_run, BatchJob, BatchRunnerConfig, TaskKind};
use uuid::Uuid;

/// Request to run a batch job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRunRequest {
    pub input_dir: String,
    pub output_dir: String,
    pub file_pattern: String,
    pub analysis_type: String, // "pf_dc" | "pf_ac" | "opf_dc" | "opf_ac"
    pub parallel_jobs: usize,
    pub tolerance: f64,
    pub max_iterations: usize,
}

/// Response from starting a batch run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRunResponse {
    pub run_id: String,
    pub total_jobs: usize,
}

/// Status response for a batch run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStatusResponse {
    pub status: String,
    pub completed: usize,
    pub total: usize,
    pub results: Option<Vec<JobResultJson>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResultJson {
    pub job_id: String,
    pub status: String,
    pub duration_ms: Option<f64>,
    pub error: Option<String>,
}

/// Start a batch job run.
#[tauri::command]
pub async fn run_batch_job(
    request: BatchRunRequest,
    state: tauri::State<'_, AppState>,
) -> Result<BatchRunResponse, String> {
    let run_id = Uuid::new_v4().to_string();

    // Discover input files matching pattern
    let input_path = std::path::Path::new(&request.input_dir);
    let pattern = request.file_pattern.replace("*", "");

    let mut jobs: Vec<BatchJob> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(input_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(&pattern) {
                        jobs.push(BatchJob {
                            job_id: name.to_string(),
                            scenario_id: name.to_string(),
                            time: None,
                            grid_file: path.clone(),
                            tags: vec![],
                            weight: 1.0,
                        });
                    }
                }
            }
        }
    }

    let total_jobs = jobs.len();
    if total_jobs == 0 {
        return Err(format!(
            "No files matching {} found in {}",
            request.file_pattern, request.input_dir
        ));
    }

    // Initialize run state
    {
        let mut runs = state.batch_runs.lock().unwrap();
        runs.insert(
            run_id.clone(),
            crate::state::BatchRun {
                run_id: run_id.clone(),
                status: "running".to_string(),
                completed: 0,
                total: total_jobs,
                results: None,
                error: None,
            },
        );
    }

    // Parse task type
    let task = match request.analysis_type.as_str() {
        "pf_dc" => TaskKind::PfDc,
        "pf_ac" => TaskKind::PfAc,
        "opf_dc" => TaskKind::OpfDc,
        "opf_ac" => TaskKind::OpfAc,
        _ => return Err(format!("Unknown analysis type: {}", request.analysis_type)),
    };

    // Clone for async move
    let run_id_clone = run_id.clone();
    let state_clone = state.batch_runs.clone();

    // Spawn background task
    std::thread::spawn(move || {
        // Use faer solver by default (fast, reliable)
        let solver = "faer"
            .parse::<gat_core::solver::SolverKind>()
            .unwrap_or_default();

        let config = BatchRunnerConfig {
            jobs,
            output_root: std::path::PathBuf::from(&request.output_dir),
            task,
            solver,
            lp_solver: None,
            partitions: vec![],
            tol: request.tolerance,
            max_iter: request.max_iterations,
            cost: None,
            limits: None,
            branch_limits: None,
            piecewise: None,
            threads: request.parallel_jobs,
        };

        match batch_run(&config) {
            Ok(summary) => {
                let mut runs = state_clone.lock().unwrap();
                if let Some(run) = runs.get_mut(&run_id_clone) {
                    run.status = "completed".to_string();
                    run.completed = summary.success + summary.failure;
                    run.results = Some(summary.jobs);
                }
            }
            Err(e) => {
                let mut runs = state_clone.lock().unwrap();
                if let Some(run) = runs.get_mut(&run_id_clone) {
                    run.status = "failed".to_string();
                    run.error = Some(e.to_string());
                }
            }
        }
    });

    Ok(BatchRunResponse { run_id, total_jobs })
}

/// Get status of a batch run.
#[tauri::command]
pub fn get_batch_status(
    run_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<BatchStatusResponse, String> {
    let runs = state.batch_runs.lock().unwrap();
    let run = runs
        .get(&run_id)
        .ok_or_else(|| format!("Run {} not found", run_id))?;

    Ok(BatchStatusResponse {
        status: run.status.clone(),
        completed: run.completed,
        total: run.total,
        results: run.results.as_ref().map(|jobs| {
            jobs.iter()
                .map(|j| JobResultJson {
                    job_id: j.job_id.clone(),
                    status: j.status.clone(),
                    duration_ms: j.duration_ms,
                    error: j.error.clone(),
                })
                .collect()
        }),
        error: run.error.clone(),
    })
}

// ============================================================================
// PTDF Computation
// ============================================================================

use gat_algo::contingency::lodf::compute_ptdf_matrix;

/// Request to compute PTDF factors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtdfRequest {
    pub network_path: String,
    pub injection_bus: usize,
    pub withdrawal_bus: usize,
}

/// PTDF result for a single branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtdfBranchResult {
    pub branch_id: usize,
    pub from_bus: usize,
    pub to_bus: usize,
    pub branch_name: String,
    pub ptdf_factor: f64,
    pub flow_change_mw: f64,
}

/// Response from PTDF computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtdfResponse {
    pub injection_bus: usize,
    pub withdrawal_bus: usize,
    pub transfer_mw: f64,
    pub branches: Vec<PtdfBranchResult>,
    pub compute_time_ms: f64,
}

/// Compute PTDF factors for a transfer between two buses.
#[tauri::command]
pub fn compute_ptdf(request: PtdfRequest) -> Result<PtdfResponse, String> {
    let start = std::time::Instant::now();
    let path = std::path::Path::new(&request.network_path);

    // Parse network (use existing pattern from commands.rs)
    let result = if let Some((format, _)) = Format::detect(path) {
        format
            .parse(&request.network_path)
            .map_err(|e| e.to_string())?
    } else {
        parse_matpower(&request.network_path).map_err(|e| e.to_string())?
    };

    let network = &result.network;

    // Compute PTDF matrix
    let ptdf_matrix = compute_ptdf_matrix(network).map_err(|e| e.to_string())?;

    // Get branch info for names
    let mut branch_info: HashMap<usize, (usize, usize, String)> = HashMap::new();
    for edge in network.graph.edge_references() {
        if let Edge::Branch(branch) = edge.weight() {
            branch_info.insert(
                branch.id.value(),
                (
                    branch.from_bus.value(),
                    branch.to_bus.value(),
                    branch.name.clone(),
                ),
            );
        }
    }

    // Calculate PTDF for the specified transfer
    // Transfer = inject at injection_bus, withdraw at withdrawal_bus
    // Net PTDF = PTDF[branch, injection] - PTDF[branch, withdrawal]
    let transfer_mw = 100.0; // Standard 100 MW transfer
    let mut branches: Vec<PtdfBranchResult> = Vec::new();

    for &branch_id in &ptdf_matrix.branch_ids {
        let ptdf_inject = ptdf_matrix
            .get(branch_id, request.injection_bus)
            .unwrap_or(0.0);
        let ptdf_withdraw = ptdf_matrix
            .get(branch_id, request.withdrawal_bus)
            .unwrap_or(0.0);
        let ptdf_factor = ptdf_inject - ptdf_withdraw;

        let (from_bus, to_bus, name) =
            branch_info
                .get(&branch_id)
                .cloned()
                .unwrap_or((0, 0, format!("Branch {}", branch_id)));

        branches.push(PtdfBranchResult {
            branch_id,
            from_bus,
            to_bus,
            branch_name: name,
            ptdf_factor,
            flow_change_mw: ptdf_factor * transfer_mw,
        });
    }

    // Sort by absolute PTDF factor descending
    branches.sort_by(|a, b| {
        b.ptdf_factor
            .abs()
            .partial_cmp(&a.ptdf_factor.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let compute_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    Ok(PtdfResponse {
        injection_bus: request.injection_bus,
        withdrawal_bus: request.withdrawal_bus,
        transfer_mw,
        branches,
        compute_time_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bus_count() {
        assert_eq!(extract_bus_count("pglib_opf_case14_ieee"), Some(14));
        assert_eq!(extract_bus_count("pglib_opf_case118_ieee"), Some(118));
        assert_eq!(extract_bus_count("pglib_opf_case9241_pegase"), Some(9241));
        assert_eq!(extract_bus_count("unknown_format"), None);
    }
}
