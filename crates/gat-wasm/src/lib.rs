//! WASM bindings for GAT web interface
//!
//! Provides browser-compatible functions for:
//! - Parsing MATPOWER files
//! - Running DC optimal power flow
//! - Accessing built-in IEEE test cases
//! - Arrow IPC export for zero-copy JS interop

mod arrow_export;

use std::collections::HashMap;

use gat_algo::{OpfMethod, OpfSolver};
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Kilovolts, Load, LoadId, Megavars,
    MegavoltAmperes, Megawatts, Network, Node, NodeIndex, PerUnit, Radians, Shunt, ShuntId,
};
use gat_io::wasm_parsers::{parse_matpower_string, MatpowerCase};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Initialize the WASM module (called automatically on load)
#[wasm_bindgen(start)]
pub fn init() {
    // Set up better panic messages in browser console
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Parsed network summary returned to JavaScript
#[derive(Serialize)]
pub struct NetworkSummary {
    pub version: String,
    pub base_mva: f64,
    pub bus_count: usize,
    pub gen_count: usize,
    pub branch_count: usize,
    pub gencost_count: usize,
    pub total_load_mw: f64,
    pub total_gen_mw: f64,
}

impl From<&MatpowerCase> for NetworkSummary {
    fn from(case: &MatpowerCase) -> Self {
        let total_load_mw: f64 = case.bus.iter().map(|b| b.pd).sum();
        let total_gen_mw: f64 = case
            .gen
            .iter()
            .filter(|g| g.gen_status > 0)
            .map(|g| g.pg)
            .sum();

        NetworkSummary {
            version: case.version.clone(),
            base_mva: case.base_mva,
            bus_count: case.bus.len(),
            gen_count: case.gen.len(),
            branch_count: case.branch.len(),
            gencost_count: case.gencost.len(),
            total_load_mw,
            total_gen_mw,
        }
    }
}

/// Parse a MATPOWER file content and return network summary as JSON
#[wasm_bindgen]
pub fn parse_matpower(content: &str) -> Result<String, JsValue> {
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let summary = NetworkSummary::from(&case);
    serde_json::to_string(&summary).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// DC-OPF solution returned to JavaScript
#[derive(Serialize)]
pub struct DcOpfResult {
    pub converged: bool,
    pub objective_value: f64,
    pub solve_time_ms: u128,
    pub method: String,
    pub generator_dispatch: HashMap<String, f64>,
    pub bus_angles_deg: HashMap<String, f64>,
    pub branch_flows_mw: HashMap<String, f64>,
    pub bus_lmp: HashMap<String, f64>,
    pub total_generation_mw: f64,
    pub total_load_mw: f64,
}

/// Run DC optimal power flow on MATPOWER content and return results as JSON
///
/// The DC-OPF uses a linear approximation that:
/// - Assumes voltage magnitudes ≈ 1.0 p.u.
/// - Linearizes power flow using susceptance (B') matrix
/// - Minimizes generator cost subject to flow constraints
#[wasm_bindgen]
pub fn run_dc_power_flow(content: &str) -> Result<String, JsValue> {
    // 1. Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. Convert to Network graph
    let network = matpower_to_network(&case)
        .map_err(|e| JsValue::from_str(&format!("Network error: {e}")))?;

    // 3. Run DC-OPF solver
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);
    let solution = solver
        .solve(&network)
        .map_err(|e| JsValue::from_str(&format!("Solver error: {e}")))?;

    // 4. Compute summary stats
    let total_gen: f64 = solution.generator_p.values().sum();
    let total_load: f64 = case.bus.iter().map(|b| b.pd).sum();

    // 5. Package results
    let result = DcOpfResult {
        converged: solution.converged,
        objective_value: solution.objective_value,
        solve_time_ms: solution.solve_time_ms,
        method: format!("{:?}", solution.method_used),
        generator_dispatch: solution.generator_p,
        bus_angles_deg: solution
            .bus_voltage_ang
            .into_iter()
            .map(|(k, v)| (k, v.to_degrees()))
            .collect(),
        branch_flows_mw: solution.branch_p_flow,
        bus_lmp: solution.bus_lmp,
        total_generation_mw: total_gen,
        total_load_mw: total_load,
    };

    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Convert a parsed MatpowerCase to a gat_core::Network
///
/// This mirrors the logic in gat-io/src/importers/matpower.rs but works
/// directly on the parsed structs rather than reading from files.
fn matpower_to_network(case: &MatpowerCase) -> Result<Network, String> {
    let mut network = Network::new();

    // Map MATPOWER bus_i → NodeIndex for connecting branches
    let mut bus_index_map: HashMap<usize, NodeIndex> = HashMap::new();

    // 1. Add buses (MATPOWER bus types: 1=PQ, 2=PV, 3=slack)
    for bus in &case.bus {
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(bus.bus_i),
            name: format!("Bus{}", bus.bus_i),
            base_kv: Kilovolts(bus.base_kv),
            voltage_pu: PerUnit(bus.vm),
            angle_rad: Radians(bus.va.to_radians()),
            vmin_pu: Some(PerUnit(bus.vmin)),
            vmax_pu: Some(PerUnit(bus.vmax)),
            area_id: Some(bus.area as i64),
            zone_id: Some(bus.zone as i64),
        }));
        bus_index_map.insert(bus.bus_i, node_idx);

        // Add load if bus has nonzero Pd/Qd
        if bus.pd.abs() > 1e-9 || bus.qd.abs() > 1e-9 {
            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(bus.bus_i),
                name: format!("Load@Bus{}", bus.bus_i),
                bus: BusId::new(bus.bus_i),
                active_power: Megawatts(bus.pd),
                reactive_power: Megavars(bus.qd),
            }));
        }

        // Add shunt if bus has nonzero Gs/Bs
        if bus.gs.abs() > 1e-9 || bus.bs.abs() > 1e-9 {
            // MATPOWER Gs/Bs are in MW/Mvar at 1.0 p.u. voltage
            // Convert to per-unit: divide by base_mva
            network.graph.add_node(Node::Shunt(Shunt {
                id: ShuntId::new(bus.bus_i),
                name: format!("Shunt@Bus{}", bus.bus_i),
                bus: BusId::new(bus.bus_i),
                gs_pu: bus.gs / case.base_mva,
                bs_pu: bus.bs / case.base_mva,
                status: true,
            }));
        }
    }

    // 2. Add generators with cost functions
    for (gen_idx, gen) in case.gen.iter().enumerate() {
        let gen_id = gen_idx + 1;

        // Build cost model from gencost data (if available)
        let cost_model = if gen_idx < case.gencost.len() {
            let gc = &case.gencost[gen_idx];
            if gc.model == 2 {
                // Polynomial cost
                // MATPOWER stores coefficients highest-order first: [cn, ..., c1, c0]
                // gat_core expects lowest-order first: [c0, c1, ..., cn]
                let mut coeffs: Vec<f64> = gc.cost.iter().rev().cloned().collect();
                // Trim to actual ncost terms
                coeffs.truncate(gc.ncost as usize);
                CostModel::Polynomial(coeffs)
            } else if gc.model == 1 {
                // Piecewise linear: pairs of (mw, cost)
                let points: Vec<(f64, f64)> = gc.cost.chunks(2).map(|c| (c[0], c[1])).collect();
                CostModel::PiecewiseLinear(points)
            } else {
                CostModel::NoCost
            }
        } else {
            CostModel::NoCost
        };

        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(gen_id),
            name: format!("Gen{}@Bus{}", gen_id, gen.gen_bus),
            bus: BusId::new(gen.gen_bus),
            active_power: Megawatts(gen.pg),
            reactive_power: Megavars(gen.qg),
            pmin: Megawatts(gen.pmin),
            pmax: Megawatts(gen.pmax),
            qmin: Megavars(gen.qmin),
            qmax: Megavars(gen.qmax),
            status: gen.gen_status > 0,
            voltage_setpoint: Some(PerUnit(gen.vg)),
            mbase: Some(MegavoltAmperes(gen.mbase)),
            cost_startup: None,
            cost_shutdown: None,
            cost_model,
            is_synchronous_condenser: false,
        }));
    }

    // 3. Add branches (transmission lines and transformers)
    for (br_idx, br) in case.branch.iter().enumerate() {
        let branch_id = br_idx + 1;

        // Get node indices for from/to buses
        let from_idx = *bus_index_map.get(&br.f_bus).ok_or_else(|| {
            format!(
                "Branch {} references unknown from_bus {}",
                branch_id, br.f_bus
            )
        })?;
        let to_idx = *bus_index_map.get(&br.t_bus).ok_or_else(|| {
            format!(
                "Branch {} references unknown to_bus {}",
                branch_id, br.t_bus
            )
        })?;

        // Determine if it's a transformer (tap != 0 and != 1, or has phase shift)
        let is_transformer =
            (br.tap.abs() > 1e-9 && (br.tap - 1.0).abs() > 1e-9) || br.shift.abs() > 1e-9;

        // Thermal rating (use rate_a if nonzero, otherwise None)
        let s_max = if br.rate_a > 0.0 {
            Some(MegavoltAmperes(br.rate_a))
        } else {
            None
        };

        network.graph.add_edge(
            from_idx,
            to_idx,
            Edge::Branch(Branch {
                id: BranchId::new(branch_id),
                name: format!("Branch{}_{}_{}", branch_id, br.f_bus, br.t_bus),
                from_bus: BusId::new(br.f_bus),
                to_bus: BusId::new(br.t_bus),
                resistance: br.br_r,
                reactance: br.br_x,
                tap_ratio: if br.tap.abs() < 1e-9 { 1.0 } else { br.tap },
                phase_shift: Radians(br.shift.to_radians()),
                charging_b: PerUnit(br.br_b),
                s_max,
                rating_a: if br.rate_a > 0.0 {
                    Some(MegavoltAmperes(br.rate_a))
                } else {
                    None
                },
                rating_b: if br.rate_b > 0.0 {
                    Some(MegavoltAmperes(br.rate_b))
                } else {
                    None
                },
                rating_c: if br.rate_c > 0.0 {
                    Some(MegavoltAmperes(br.rate_c))
                } else {
                    None
                },
                status: br.br_status > 0,
                angle_min: Some(Radians(br.angmin.to_radians())),
                angle_max: Some(Radians(br.angmax.to_radians())),
                element_type: if is_transformer {
                    "transformer".to_string()
                } else {
                    "line".to_string()
                },
                is_phase_shifter: br.shift.abs() > 1e-9,
            }),
        );
    }

    Ok(network)
}

/// List available built-in test cases
#[wasm_bindgen]
pub fn list_builtin_cases() -> String {
    // Available IEEE test cases
    r#"["ieee14", "ieee30", "ieee57", "ieee118"]"#.to_string()
}

/// Get the content of a built-in test case by name
#[wasm_bindgen]
pub fn get_builtin_case(name: &str) -> Result<String, JsValue> {
    // For now, return a small embedded case
    // In the future, these will be compiled into the WASM binary
    match name {
        "ieee14" => Ok(IEEE14_CASE.to_string()),
        _ => Err(JsValue::from_str(&format!(
            "Case '{}' not found. Available: ieee14",
            name
        ))),
    }
}

/// SOCP-OPF result returned to JavaScript
#[derive(Serialize)]
pub struct SocpOpfResult {
    pub converged: bool,
    pub objective_value: f64,
    pub solve_time_ms: u128,
    pub method: String,
    pub generator_dispatch: HashMap<String, f64>,
    pub generator_reactive: HashMap<String, f64>,
    pub bus_voltage_mag: HashMap<String, f64>,
    pub bus_voltage_ang_deg: HashMap<String, f64>,
    pub branch_flows_mw: HashMap<String, f64>,
    pub branch_reactive_flows_mvar: HashMap<String, f64>,
    pub bus_lmp: HashMap<String, f64>,
    pub total_generation_mw: f64,
    pub total_load_mw: f64,
    pub total_losses_mw: f64,
}

/// Run SOCP (Second-Order Cone Programming) relaxation OPF on MATPOWER content
///
/// SOCP provides a convex relaxation of AC-OPF that:
/// - Uses the branch-flow model (Baran-Wu / Farivar-Low)
/// - Handles quadratic generator costs
/// - Respects voltage and thermal limits
/// - Is more accurate than DC-OPF, especially for losses and reactive power
#[wasm_bindgen]
pub fn run_socp_power_flow(content: &str) -> Result<String, JsValue> {
    // 1. Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. Convert to Network graph
    let network = matpower_to_network(&case)
        .map_err(|e| JsValue::from_str(&format!("Network error: {e}")))?;

    // 3. Run SOCP-OPF solver
    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let solution = solver
        .solve(&network)
        .map_err(|e| JsValue::from_str(&format!("Solver error: {e}")))?;

    // 4. Compute summary stats
    let total_gen: f64 = solution.generator_p.values().sum();
    let total_load: f64 = case.bus.iter().map(|b| b.pd).sum();

    // 5. Package results
    let result = SocpOpfResult {
        converged: solution.converged,
        objective_value: solution.objective_value,
        solve_time_ms: solution.solve_time_ms,
        method: format!("{:?}", solution.method_used),
        generator_dispatch: solution.generator_p,
        generator_reactive: solution.generator_q,
        bus_voltage_mag: solution.bus_voltage_mag,
        bus_voltage_ang_deg: solution
            .bus_voltage_ang
            .into_iter()
            .map(|(k, v)| (k, v.to_degrees()))
            .collect(),
        branch_flows_mw: solution.branch_p_flow,
        branch_reactive_flows_mvar: solution.branch_q_flow,
        bus_lmp: solution.bus_lmp,
        total_generation_mw: total_gen,
        total_load_mw: total_load,
        total_losses_mw: solution.total_losses_mw,
    };

    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Economic dispatch result returned to JavaScript
#[derive(Serialize)]
pub struct EconomicDispatchResult {
    pub converged: bool,
    pub objective_value: f64,
    pub solve_time_ms: u128,
    pub method: String,
    pub generator_dispatch: HashMap<String, f64>,
    pub total_generation_mw: f64,
    pub total_load_mw: f64,
    pub estimated_losses_mw: f64,
}

/// Run merit-order economic dispatch on MATPOWER content
///
/// Economic dispatch minimizes generation cost without network constraints:
/// - Ranks generators by marginal cost (merit order)
/// - Dispatches cheapest generators first
/// - Does not consider transmission limits or losses explicitly
/// - Fastest method, useful for initial estimates
#[wasm_bindgen]
pub fn run_economic_dispatch(content: &str) -> Result<String, JsValue> {
    // 1. Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. Convert to Network graph
    let network = matpower_to_network(&case)
        .map_err(|e| JsValue::from_str(&format!("Network error: {e}")))?;

    // 3. Run Economic Dispatch solver
    let solver = OpfSolver::new().with_method(OpfMethod::EconomicDispatch);
    let solution = solver
        .solve(&network)
        .map_err(|e| JsValue::from_str(&format!("Solver error: {e}")))?;

    // 4. Compute summary stats
    let total_gen: f64 = solution.generator_p.values().sum();
    let total_load: f64 = case.bus.iter().map(|b| b.pd).sum();

    // 5. Package results
    let result = EconomicDispatchResult {
        converged: solution.converged,
        objective_value: solution.objective_value,
        solve_time_ms: solution.solve_time_ms,
        method: format!("{:?}", solution.method_used),
        generator_dispatch: solution.generator_p,
        total_generation_mw: total_gen,
        total_load_mw: total_load,
        estimated_losses_mw: solution.total_losses_mw,
    };

    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Network analysis result returned to JavaScript
#[derive(Serialize)]
pub struct NetworkAnalysis {
    pub bus_count: usize,
    pub gen_count: usize,
    pub branch_count: usize,
    pub load_count: usize,
    pub total_load_mw: f64,
    pub total_gen_capacity_mw: f64,
    pub total_online_gen_mw: f64,
    pub reserve_margin_pct: f64,
    pub transformers: usize,
    pub lines: usize,
    pub voltage_levels_kv: Vec<f64>,
    pub areas: Vec<i64>,
}

/// Analyze network structure and capacity
///
/// Returns detailed network statistics including:
/// - Component counts (buses, generators, branches)
/// - Capacity margins and reserves
/// - Voltage levels and areas
#[wasm_bindgen]
pub fn analyze_network(content: &str) -> Result<String, JsValue> {
    // 1. Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. Compute statistics
    let total_load_mw: f64 = case.bus.iter().map(|b| b.pd).sum();
    let total_gen_capacity: f64 = case.gen.iter().map(|g| g.pmax).sum();
    let online_gens: Vec<_> = case.gen.iter().filter(|g| g.gen_status > 0).collect();
    let total_online_gen: f64 = online_gens.iter().map(|g| g.pg).sum();
    let online_capacity: f64 = online_gens.iter().map(|g| g.pmax).sum();

    // Reserve margin: (capacity - load) / load * 100
    let reserve_margin = if total_load_mw > 0.0 {
        (online_capacity - total_load_mw) / total_load_mw * 100.0
    } else {
        0.0
    };

    // Count transformers vs lines
    let transformers = case
        .branch
        .iter()
        .filter(|b| (b.tap.abs() > 1e-9 && (b.tap - 1.0).abs() > 1e-9) || b.shift.abs() > 1e-9)
        .count();
    let lines = case.branch.len() - transformers;

    // Unique voltage levels
    let mut voltage_levels: Vec<f64> = case.bus.iter().map(|b| b.base_kv).collect();
    voltage_levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    voltage_levels.dedup();

    // Unique areas
    let mut areas: Vec<i64> = case.bus.iter().map(|b| b.area as i64).collect();
    areas.sort();
    areas.dedup();

    // Load count (buses with non-zero Pd)
    let load_count = case.bus.iter().filter(|b| b.pd.abs() > 1e-9).count();

    let result = NetworkAnalysis {
        bus_count: case.bus.len(),
        gen_count: case.gen.len(),
        branch_count: case.branch.len(),
        load_count,
        total_load_mw,
        total_gen_capacity_mw: total_gen_capacity,
        total_online_gen_mw: total_online_gen,
        reserve_margin_pct: reserve_margin,
        transformers,
        lines,
        voltage_levels_kv: voltage_levels,
        areas,
    };

    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compare OPF methods on the same network
///
/// Runs DC-OPF, SOCP, and Economic Dispatch, returning comparison results
#[wasm_bindgen]
pub fn compare_methods(content: &str) -> Result<String, JsValue> {
    #[derive(Serialize)]
    struct MethodComparison {
        method: String,
        converged: bool,
        objective_value: f64,
        solve_time_ms: u128,
        total_losses_mw: f64,
    }

    #[derive(Serialize)]
    struct ComparisonResult {
        methods: Vec<MethodComparison>,
        total_load_mw: f64,
    }

    // Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let network = matpower_to_network(&case)
        .map_err(|e| JsValue::from_str(&format!("Network error: {e}")))?;
    let total_load: f64 = case.bus.iter().map(|b| b.pd).sum();

    let mut methods = Vec::new();

    // Run DC-OPF
    if let Ok(sol) = OpfSolver::new()
        .with_method(OpfMethod::DcOpf)
        .solve(&network)
    {
        methods.push(MethodComparison {
            method: "DC-OPF".to_string(),
            converged: sol.converged,
            objective_value: sol.objective_value,
            solve_time_ms: sol.solve_time_ms,
            total_losses_mw: sol.total_losses_mw,
        });
    }

    // Run SOCP
    if let Ok(sol) = OpfSolver::new()
        .with_method(OpfMethod::SocpRelaxation)
        .solve(&network)
    {
        methods.push(MethodComparison {
            method: "SOCP".to_string(),
            converged: sol.converged,
            objective_value: sol.objective_value,
            solve_time_ms: sol.solve_time_ms,
            total_losses_mw: sol.total_losses_mw,
        });
    }

    // Run Economic Dispatch
    if let Ok(sol) = OpfSolver::new()
        .with_method(OpfMethod::EconomicDispatch)
        .solve(&network)
    {
        methods.push(MethodComparison {
            method: "Economic Dispatch".to_string(),
            converged: sol.converged,
            objective_value: sol.objective_value,
            solve_time_ms: sol.solve_time_ms,
            total_losses_mw: sol.total_losses_mw,
        });
    }

    let result = ComparisonResult {
        methods,
        total_load_mw: total_load,
    };

    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Network layout result returned to JavaScript
/// Compatible with Cytoscape.js and other graph visualization libraries
#[derive(Serialize)]
pub struct NetworkLayout {
    /// Nodes with computed x,y positions
    pub nodes: Vec<LayoutNode>,
    /// Edges connecting nodes
    pub edges: Vec<LayoutEdgeData>,
}

#[derive(Serialize)]
pub struct LayoutNode {
    /// Bus ID
    pub id: usize,
    /// Bus label/name
    pub label: String,
    /// X coordinate (force-directed position)
    pub x: f32,
    /// Y coordinate (force-directed position)
    pub y: f32,
}

#[derive(Serialize)]
pub struct LayoutEdgeData {
    /// Source bus ID
    pub source: usize,
    /// Target bus ID
    pub target: usize,
}

/// Compute force-directed layout for network visualization
///
/// Uses the Fruchterman-Reingold algorithm to compute aesthetically pleasing
/// node positions. The output is compatible with Cytoscape.js and other
/// graph visualization libraries.
///
/// Parameters:
/// - content: MATPOWER case file content
/// - iterations: Number of simulation iterations (default: 100, more = better but slower)
///
/// Returns JSON with nodes (id, label, x, y) and edges (source, target)
#[wasm_bindgen]
pub fn compute_layout(content: &str, iterations: Option<u32>) -> Result<String, JsValue> {
    // 1. Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. Convert to Network graph
    let network = matpower_to_network(&case)
        .map_err(|e| JsValue::from_str(&format!("Network error: {e}")))?;

    // 3. Run force-directed layout
    let iters = iterations.unwrap_or(100) as usize;
    let layout_result = gat_viz::layout::layout_network(&network, iters);

    // 4. Convert to our output format (rename fields for JS compatibility)
    let result = NetworkLayout {
        nodes: layout_result
            .nodes
            .into_iter()
            .map(|n| LayoutNode {
                id: n.id,
                label: n.label,
                x: n.x,
                y: n.y,
            })
            .collect(),
        edges: layout_result
            .edges
            .into_iter()
            .map(|e| LayoutEdgeData {
                source: e.from,
                target: e.to,
            })
            .collect(),
    };

    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// OPF Arrow result containing binary Arrow IPC tables for each result type
///
/// This structure is returned to JavaScript as an object with typed arrays.
/// Each table can be loaded directly by Apache Arrow JS or DuckDB-WASM.
#[wasm_bindgen(getter_with_clone)]
pub struct OpfArrowTables {
    /// Arrow IPC bytes for generator dispatch (p_mw, q_mvar)
    #[wasm_bindgen(readonly)]
    pub generators: Vec<u8>,
    /// Arrow IPC bytes for bus voltages (v_mag, v_ang_deg, lmp)
    #[wasm_bindgen(readonly)]
    pub buses: Vec<u8>,
    /// Arrow IPC bytes for branch flows (p_flow_mw, q_flow_mvar)
    #[wasm_bindgen(readonly)]
    pub branches: Vec<u8>,
    /// JSON summary (converged, objective, solve_time, totals)
    #[wasm_bindgen(readonly)]
    pub summary: String,
}

/// Run DC-OPF and return results as Arrow IPC tables
///
/// This is the high-performance variant of run_dc_power_flow that returns
/// binary Arrow IPC data instead of JSON. The Arrow tables can be:
/// - Loaded directly by Apache Arrow JS for visualization
/// - Queried with SQL via DuckDB-WASM
/// - Transformed with Arquero for complex analysis
///
/// Returns an OpfArrowTables object with:
/// - generators: Arrow table with gen_id, p_mw, q_mvar columns
/// - buses: Arrow table with bus_id, v_mag, v_ang_deg, lmp columns
/// - branches: Arrow table with branch_id, p_flow_mw, q_flow_mvar columns
/// - summary: JSON string with metadata (converged, objective_value, etc.)
#[wasm_bindgen]
pub fn run_dc_opf_arrow(content: &str) -> Result<OpfArrowTables, JsValue> {
    // 1. Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. Convert to Network graph
    let network = matpower_to_network(&case)
        .map_err(|e| JsValue::from_str(&format!("Network error: {e}")))?;

    // 3. Run DC-OPF solver
    let solver = OpfSolver::new().with_method(OpfMethod::DcOpf);
    let solution = solver
        .solve(&network)
        .map_err(|e| JsValue::from_str(&format!("Solver error: {e}")))?;

    // 4. Compute summary stats
    let total_gen: f64 = solution.generator_p.values().sum();
    let total_load: f64 = case.bus.iter().map(|b| b.pd).sum();

    // 5. Convert to Arrow IPC
    let generators =
        arrow_export::generators_to_arrow(&solution.generator_p, &solution.generator_q)
            .map_err(|e| JsValue::from_str(&format!("Arrow generator error: {e}")))?;

    let bus_ang_deg: HashMap<String, f64> = solution
        .bus_voltage_ang
        .iter()
        .map(|(k, v)| (k.clone(), v.to_degrees()))
        .collect();
    let buses =
        arrow_export::buses_to_arrow(&solution.bus_voltage_mag, &bus_ang_deg, &solution.bus_lmp)
            .map_err(|e| JsValue::from_str(&format!("Arrow bus error: {e}")))?;

    let branches =
        arrow_export::branches_to_arrow(&solution.branch_p_flow, &solution.branch_q_flow)
            .map_err(|e| JsValue::from_str(&format!("Arrow branch error: {e}")))?;

    // 6. Create summary JSON
    let summary = arrow_export::OpfSummary {
        converged: solution.converged,
        objective_value: solution.objective_value,
        solve_time_ms: solution.solve_time_ms,
        method: format!("{:?}", solution.method_used),
        total_generation_mw: total_gen,
        total_load_mw: total_load,
        total_losses_mw: solution.total_losses_mw,
    };
    let summary_json =
        serde_json::to_string(&summary).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(OpfArrowTables {
        generators,
        buses,
        branches,
        summary: summary_json,
    })
}

/// Run SOCP-OPF and return results as Arrow IPC tables
///
/// High-performance variant returning Arrow IPC data for zero-copy JS interop.
#[wasm_bindgen]
pub fn run_socp_opf_arrow(content: &str) -> Result<OpfArrowTables, JsValue> {
    // 1. Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. Convert to Network graph
    let network = matpower_to_network(&case)
        .map_err(|e| JsValue::from_str(&format!("Network error: {e}")))?;

    // 3. Run SOCP-OPF solver
    let solver = OpfSolver::new().with_method(OpfMethod::SocpRelaxation);
    let solution = solver
        .solve(&network)
        .map_err(|e| JsValue::from_str(&format!("Solver error: {e}")))?;

    // 4. Compute summary stats
    let total_gen: f64 = solution.generator_p.values().sum();
    let total_load: f64 = case.bus.iter().map(|b| b.pd).sum();

    // 5. Convert to Arrow IPC
    let generators =
        arrow_export::generators_to_arrow(&solution.generator_p, &solution.generator_q)
            .map_err(|e| JsValue::from_str(&format!("Arrow generator error: {e}")))?;

    let bus_ang_deg: HashMap<String, f64> = solution
        .bus_voltage_ang
        .iter()
        .map(|(k, v)| (k.clone(), v.to_degrees()))
        .collect();
    let buses =
        arrow_export::buses_to_arrow(&solution.bus_voltage_mag, &bus_ang_deg, &solution.bus_lmp)
            .map_err(|e| JsValue::from_str(&format!("Arrow bus error: {e}")))?;

    let branches =
        arrow_export::branches_to_arrow(&solution.branch_p_flow, &solution.branch_q_flow)
            .map_err(|e| JsValue::from_str(&format!("Arrow branch error: {e}")))?;

    // 6. Create summary JSON
    let summary = arrow_export::OpfSummary {
        converged: solution.converged,
        objective_value: solution.objective_value,
        solve_time_ms: solution.solve_time_ms,
        method: format!("{:?}", solution.method_used),
        total_generation_mw: total_gen,
        total_load_mw: total_load,
        total_losses_mw: solution.total_losses_mw,
    };
    let summary_json =
        serde_json::to_string(&summary).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(OpfArrowTables {
        generators,
        buses,
        branches,
        summary: summary_json,
    })
}

/// Run SOCP-OPF with relaxed tolerances for faster interactive use
///
/// This is a faster variant of SOCP-OPF that uses relaxed convergence tolerances:
/// - tolerance: 1e-4 (vs 1e-6 default) - 100x more relaxed
/// - max_iterations: 50 (vs 100 default) - earlier termination
///
/// Typically 50-70% faster than standard SOCP while maintaining engineering accuracy.
/// Results are accurate to ~0.01% which is sufficient for visualization and analysis.
#[wasm_bindgen]
pub fn run_socp_opf_fast_arrow(content: &str) -> Result<OpfArrowTables, JsValue> {
    // 1. Parse MATPOWER content
    let case = parse_matpower_string(content).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. Convert to Network graph
    let network = matpower_to_network(&case)
        .map_err(|e| JsValue::from_str(&format!("Network error: {e}")))?;

    // 3. Run SOCP-OPF solver with relaxed tolerances
    let solver = OpfSolver::new()
        .with_method(OpfMethod::SocpRelaxation)
        .with_tolerance(1e-4) // Relaxed from 1e-6
        .with_max_iterations(50); // Reduced from 100
    let solution = solver
        .solve(&network)
        .map_err(|e| JsValue::from_str(&format!("Solver error: {e}")))?;

    // 4. Compute summary stats
    let total_gen: f64 = solution.generator_p.values().sum();
    let total_load: f64 = case.bus.iter().map(|b| b.pd).sum();

    // 5. Convert to Arrow IPC
    let generators =
        arrow_export::generators_to_arrow(&solution.generator_p, &solution.generator_q)
            .map_err(|e| JsValue::from_str(&format!("Arrow generator error: {e}")))?;

    let bus_ang_deg: HashMap<String, f64> = solution
        .bus_voltage_ang
        .iter()
        .map(|(k, v)| (k.clone(), v.to_degrees()))
        .collect();
    let buses =
        arrow_export::buses_to_arrow(&solution.bus_voltage_mag, &bus_ang_deg, &solution.bus_lmp)
            .map_err(|e| JsValue::from_str(&format!("Arrow bus error: {e}")))?;

    let branches =
        arrow_export::branches_to_arrow(&solution.branch_p_flow, &solution.branch_q_flow)
            .map_err(|e| JsValue::from_str(&format!("Arrow branch error: {e}")))?;

    // 6. Create summary JSON (mark as "fast" variant in method string)
    let summary = arrow_export::OpfSummary {
        converged: solution.converged,
        objective_value: solution.objective_value,
        solve_time_ms: solution.solve_time_ms,
        method: "SocpRelaxationFast".to_string(),
        total_generation_mw: total_gen,
        total_load_mw: total_load,
        total_losses_mw: solution.total_losses_mw,
    };
    let summary_json =
        serde_json::to_string(&summary).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(OpfArrowTables {
        generators,
        buses,
        branches,
        summary: summary_json,
    })
}

/// IEEE 14-bus test case (embedded)
const IEEE14_CASE: &str = r#"
function mpc = case14
mpc.version = '2';
mpc.baseMVA = 100;

%% bus data
mpc.bus = [
    1   3   0       0       0   0   1   1.06    0       0   1   1.06    0.94;
    2   2   21.7    12.7    0   0   1   1.045  -4.98    0   1   1.06    0.94;
    3   2   94.2    19      0   0   1   1.01   -12.72   0   1   1.06    0.94;
    4   1   47.8   -3.9     0   0   1   1.019  -10.33   0   1   1.06    0.94;
    5   1   7.6     1.6     0   0   1   1.02   -8.78    0   1   1.06    0.94;
    6   2   11.2    7.5     0   0   1   1.07   -14.22   0   1   1.06    0.94;
    7   1   0       0       0   0   1   1.062  -13.37   0   1   1.06    0.94;
    8   2   0       0       0   0   1   1.09   -13.36   0   1   1.06    0.94;
    9   1   29.5    16.6    0   19  1   1.056  -14.94   0   1   1.06    0.94;
    10  1   9       5.8     0   0   1   1.051  -15.1    0   1   1.06    0.94;
    11  1   3.5     1.8     0   0   1   1.057  -14.79   0   1   1.06    0.94;
    12  1   6.1     1.6     0   0   1   1.055  -15.07   0   1   1.06    0.94;
    13  1   13.5    5.8     0   0   1   1.05   -15.16   0   1   1.06    0.94;
    14  1   14.9    5       0   0   1   1.036  -16.04   0   1   1.06    0.94;
];

%% generator data
mpc.gen = [
    1   232.4   -16.9   10  0   1.06    100 1   332.4   0;
    2   40      42.4    50  -40 1.045   100 1   140     0;
    3   0       23.4    40  0   1.01    100 1   100     0;
    6   0       12.2    24  -6  1.07    100 1   100     0;
    8   0       17.4    24  -6  1.09    100 1   100     0;
];

%% branch data
mpc.branch = [
    1   2   0.01938 0.05917 0.0528  9900    0   0   0   0   1   -360    360;
    1   5   0.05403 0.22304 0.0492  9900    0   0   0   0   1   -360    360;
    2   3   0.04699 0.19797 0.0438  9900    0   0   0   0   1   -360    360;
    2   4   0.05811 0.17632 0.034   9900    0   0   0   0   1   -360    360;
    2   5   0.05695 0.17388 0.0346  9900    0   0   0   0   1   -360    360;
    3   4   0.06701 0.17103 0.0128  9900    0   0   0   0   1   -360    360;
    4   5   0.01335 0.04211 0       9900    0   0   0   0   1   -360    360;
    4   7   0       0.20912 0       9900    0   0   0.978   0   1   -360    360;
    4   9   0       0.55618 0       9900    0   0   0.969   0   1   -360    360;
    5   6   0       0.25202 0       9900    0   0   0.932   0   1   -360    360;
    6   11  0.09498 0.19890 0       9900    0   0   0   0   1   -360    360;
    6   12  0.12291 0.25581 0       9900    0   0   0   0   1   -360    360;
    6   13  0.06615 0.13027 0       9900    0   0   0   0   1   -360    360;
    7   8   0       0.17615 0       9900    0   0   0   0   1   -360    360;
    7   9   0.11001 0.20640 0       9900    0   0   0   0   1   -360    360;
    9   10  0.03181 0.08450 0       9900    0   0   0   0   1   -360    360;
    9   14  0.12711 0.27038 0       9900    0   0   0   0   1   -360    360;
    10  11  0.08205 0.19207 0       9900    0   0   0   0   1   -360    360;
    12  13  0.22092 0.19988 0       9900    0   0   0   0   1   -360    360;
    13  14  0.17093 0.34802 0       9900    0   0   0   0   1   -360    360;
];

%% generator cost data
mpc.gencost = [
    2   0   0   3   0.0430293   20  0;
    2   0   0   3   0.25        20  0;
    2   0   0   3   0.01        40  0;
    2   0   0   3   0.01        40  0;
    2   0   0   3   0.01        40  0;
];
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_list_builtin_cases() {
        let cases = list_builtin_cases();
        assert!(cases.contains("ieee14"));
    }

    #[wasm_bindgen_test]
    fn test_get_builtin_case_ieee14() {
        let case = get_builtin_case("ieee14").unwrap();
        assert!(case.contains("case14"));
        assert!(case.contains("baseMVA"));
        assert!(case.contains("mpc.bus"));
    }

    #[wasm_bindgen_test]
    fn test_get_builtin_case_invalid() {
        let result = get_builtin_case("nonexistent");
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_parse_matpower_ieee14() {
        let case_content = get_builtin_case("ieee14").unwrap();
        let result = parse_matpower(&case_content).unwrap();

        // Parse the JSON result
        let summary: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(summary["bus_count"], 14);
        assert_eq!(summary["gen_count"], 5);
        assert_eq!(summary["branch_count"], 20);
        assert_eq!(summary["base_mva"], 100.0);
    }

    #[wasm_bindgen_test]
    fn test_dc_opf_ieee14() {
        let case_content = get_builtin_case("ieee14").unwrap();
        let result = run_dc_power_flow(&case_content).unwrap();

        // Parse the JSON result
        let opf: serde_json::Value = serde_json::from_str(&result).unwrap();

        // Check convergence
        assert_eq!(opf["converged"], true);

        // Check objective is reasonable (should be positive cost)
        let obj = opf["objective_value"].as_f64().unwrap();
        assert!(obj > 0.0, "Objective should be positive: {}", obj);

        // Check generation matches load (with small tolerance)
        let total_gen = opf["total_generation_mw"].as_f64().unwrap();
        let total_load = opf["total_load_mw"].as_f64().unwrap();
        assert!(
            (total_gen - total_load).abs() < 1.0,
            "Generation {} should match load {}",
            total_gen,
            total_load
        );

        // Check we have generator dispatch results
        let gen_dispatch = opf["generator_dispatch"].as_object().unwrap();
        assert!(!gen_dispatch.is_empty(), "Should have generator dispatch");

        // Check we have bus angles
        let bus_angles = opf["bus_angles_deg"].as_object().unwrap();
        assert!(!bus_angles.is_empty(), "Should have bus angles");
    }
}
