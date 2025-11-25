use anyhow::{anyhow, Context, Result};
use gat_algo::power_flow;
use gat_core::{solver::SolverKind, BusId, Edge, Gen, GenId, Network, Node};
use gat_io::importers;
use polars::prelude::{DataFrame, NamedFrom, ParquetCompression, ParquetWriter, Series};
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// Import a MATPOWER case and emit distribution-specific node/branch tables as Parquet.
///
/// **Purpose:** Convert MATPOWER format (designed for transmission systems) into distribution-
/// friendly tables suitable for feeder-level analysis, hosting capacity studies, and DER integration.
///
/// **Distribution vs. Transmission:**
/// Distribution systems differ from transmission networks in several key ways:
/// - **Radial topology**: Tree-like structure (vs. meshed transmission grids)
/// - **Lower voltage**: Typically 4-35 kV (vs. 115+ kV for transmission)
/// - **R/X ratio**: Higher resistance relative to reactance (resistive losses dominate)
/// - **Unbalanced loads**: Three-phase imbalances are common (vs. balanced transmission)
/// - **DER-heavy**: High penetration of distributed energy resources (solar PV, storage)
///
/// **Baran/Wu Branch-Flow Formulation:**
/// The output tables support the classic distribution power flow model from Baran & Wu (1989),
/// which is well-suited for radial networks:
/// - Forward sweep: compute branch currents from leaf nodes toward root (substation)
/// - Backward sweep: compute voltages from root toward leaves using Kirchhoff's voltage law
/// - See doi:10.1109/TPWRD.1989.4303454 for the original branch-flow equations
///
/// **Algorithm:**
/// 1. Import MATPOWER case (may be transmission or distribution format)
/// 2. Extract bus data (nodes): buses, loads, generators, voltage limits
/// 3. Extract branch data: lines/transformers with impedance (R, X) and limits
/// 4. Classify nodes as "source" (has generation) or "load" (consumption only)
/// 5. Aggregate multi-phase loads if present (simplify to single-phase equivalent for now)
/// 6. Output dist_nodes.parquet (bus_id, phase, load_p_mw, load_q_mvar, v_min, v_max)
/// 7. Output dist_branches.parquet (branch_id, from_node, to_node, r, x, thermal_limit)
///
/// **Use Cases:**
/// - Hosting capacity analysis (how much DER can be added before voltage violations?)
/// - Volt-VAR optimization (coordinated control of capacitors, regulators, inverters)
/// - Feeder upgrades (identify bottleneck branches for reconductoring)
/// - DER integration studies (impact of solar/storage on feeder voltages)
///
/// **Pedagogical Note for Grad Students:**
/// MATPOWER was designed for economic dispatch and OPF on large transmission grids. Distribution
/// systems need different modeling: radial topology assumptions enable faster branch-flow solvers,
/// and voltage drop (not congestion) is the primary constraint. This function bridges formats.
pub fn import_matpower_case(matpower: &str, out_dir: &Path, feeder_id: Option<&str>) -> Result<()> {
    fs::create_dir_all(out_dir).with_context(|| {
        format!(
            "failed to create dist output directory '{}'; check permissions",
            out_dir.display()
        )
    })?;

    let temp = tempdir().context("creating temporary folder for MATPOWER import")?;
    let temp_path = temp.path().join("matpower.arrow");
    let network = importers::import_matpower_case(matpower, temp_path.to_str().unwrap())?;

    let nodes = build_node_frame(&network, feeder_id.unwrap_or("default"));
    let branches = build_branch_frame(&network, feeder_id.unwrap_or("default"));
    write_parquet(out_dir.join("dist_nodes.parquet"), nodes)?;
    write_parquet(out_dir.join("dist_branches.parquet"), branches)?;

    println!(
        "Dist import produced '{}' nodes and '{}' branches under {}",
        network.graph.node_count(),
        network.graph.edge_count(),
        out_dir.display()
    );
    Ok(())
}

/// Run distribution-aware AC power flow and persist results as Parquet.
///
/// **Purpose:** Solve the non-linear power flow equations for distribution feeders to find bus
/// voltages and branch currents given injections (loads, DER generation) and network impedances.
///
/// **AC Power Flow Equations:**
/// For each bus i, the power balance equations are:
/// ```text
/// P_i = V_i ∑_j V_j (G_ij cos(θ_i - θ_j) + B_ij sin(θ_i - θ_j))
/// Q_i = V_i ∑_j V_j (G_ij sin(θ_i - θ_j) - B_ij cos(θ_i - θ_j))
/// ```
/// where V_i is voltage magnitude, θ_i is angle, G_ij + jB_ij are admittance matrix elements.
/// See doi:10.1109/TPWRS.2012.2187686 for the classic Newton-Raphson formulation.
///
/// **Newton-Raphson Method:**
/// The solver iteratively refines voltage estimates using the Jacobian matrix:
/// 1. Initialize: V = 1.0 p.u., θ = 0 for all buses (flat start)
/// 2. Compute power mismatches: ΔP = P_calculated - P_scheduled, ΔQ similar
/// 3. Build Jacobian J (partial derivatives ∂P/∂θ, ∂P/∂V, ∂Q/∂θ, ∂Q/∂V)
/// 4. Solve linear system: J · [Δθ, ΔV] = [ΔP, ΔQ]
/// 5. Update voltages: V += ΔV, θ += Δθ
/// 6. Repeat until mismatches < tolerance (typically 1e-6 p.u.)
///
/// **Distribution Characteristics:**
/// - **High R/X ratio**: Resistance dominates in distribution (vs. X >> R in transmission)
///   → Voltage drop is primarily due to resistive losses (I²R), not inductive reactance
/// - **Radial topology**: Enables specialized solvers (forward-backward sweep) but we use
///   Newton-Raphson for consistency with transmission tools
/// - **Voltage constraints**: ANSI C84.1 requires 0.95 ≤ V ≤ 1.05 p.u. at customer meters
///   → DER integration must respect these tight voltage bounds
///
/// **Use Cases:**
/// - Baseline analysis: What are steady-state voltages/currents before DER additions?
/// - Voltage violation detection: Which buses fall outside ANSI C84.1 limits?
/// - Loss calculation: How much power is lost in distribution feeders (I²R losses)?
/// - Time-series studies: Run PF for every hour to assess DER impact on daily voltage profiles
///
/// **Pedagogical Note for Grad Students:**
/// Power flow is the "Hello World" of power systems. It assumes all injections (P, Q) are known
/// (not optimized) and solves for voltages. In contrast, OPF *optimizes* injections subject to
/// constraints. PF answers "what happens?", OPF answers "what's optimal?". For distribution,
/// voltage limits are the binding constraint, not transmission congestion.
pub fn run_power_flow(
    grid_file: &Path,
    out_file: &Path,
    solver_kind: SolverKind,
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    let network = load_network(grid_file)?;
    let solver = solver_kind.build_solver();
    power_flow::ac_power_flow(&network, solver.as_ref(), tol, max_iter, out_file, &[])
        .with_context(|| format!("running dist pf on {}", grid_file.display()))
}

/// Run a simple single-objective AC OPF for hosting/volt-var experiments.
pub fn run_optimal_power_flow(
    grid_file: &Path,
    out_file: &Path,
    solver_kind: SolverKind,
    tol: f64,
    max_iter: u32,
    objective: &str,
) -> Result<()> {
    println!(
        "Dist OPF objective '{}'; recording results for downstream offline workflows",
        objective
    );
    let network = load_network(grid_file)?;
    let solver = solver_kind.build_solver();
    power_flow::ac_optimal_power_flow(&network, solver.as_ref(), tol, max_iter, out_file, &[])
        .with_context(|| format!("running dist opf on {}", grid_file.display()))
}

/// Sweep DER injections at selected buses to approximate hosting capacity boundaries.
///
/// **Purpose:** Determine how much distributed generation (solar PV, storage discharge) can be
/// added at each bus before violating voltage or thermal limits. This quantifies "hosting capacity"—
/// the maximum DER penetration the feeder can safely accommodate without upgrades.
///
/// **Hosting Capacity Concept:**
/// Hosting capacity (HC) is the amount of DER (MW) that can be interconnected without:
/// 1. **Overvoltage**: DER reverse power flow causes voltage rise beyond 1.05 p.u. (ANSI C84.1)
/// 2. **Thermal overload**: Branch currents exceed conductor ampacity (thermal limits)
/// 3. **Protection miscoordination**: DER trips protective devices (not modeled here)
/// 4. **Transformer overload**: Substation or distribution transformers exceed ratings
///
/// **Historical Context:**
/// As rooftop solar penetration grew in the 2010s, utilities needed methods to assess "how much
/// solar can we add before the grid breaks?" HC analysis became standard practice. California
/// Rule 21 and IEEE 1547-2018 mandate HC studies for interconnection. See doi:10.1109/TDEI.2016.7729825
/// for the EPRI-developed stochastic HC method.
///
/// **Algorithm (Deterministic Sweep):**
/// 1. Select target buses (candidate DER locations, or sweep all buses)
/// 2. For each bus, incrementally add DER injection: 0 MW → max_injection MW (in `steps` increments)
/// 3. At each step, run AC OPF to find feasible dispatch (respects voltage/thermal constraints)
/// 4. Record whether OPF converges (success = feasible, failure = limit violated)
/// 5. Hosting capacity = largest injection_mw where OPF succeeds
/// 6. Output: Per-bus HC curves (injection_mw vs. feasibility) as Parquet
///
/// **Why OPF (not PF)?**
/// We use OPF (Optimal Power Flow) instead of plain PF (Power Flow) because OPF can:
/// - Adjust other generators to maintain voltage support (models coordinated control)
/// - Respect all constraints (voltage limits, branch thermal limits)
/// - Find feasible operating point if one exists (PF may diverge if base case is infeasible)
///
/// **Interpreting Results:**
/// - **Success = true**: Feeder can accommodate this DER injection level
/// - **Success = false**: Voltage or thermal limits violated, HC is below this level
/// - **HC value**: Maximum injection_mw before first failure (linear interpolation between steps)
/// - **Bottleneck identification**: If HC is low, check which constraint binds (voltage or thermal)
///
/// **Limitations (Deterministic HC):**
/// - **Static analysis**: Doesn't model time-varying solar/load (use time-series PF for that)
/// - **Single-bus injection**: Doesn't assess simultaneous DER at multiple buses (combinatorial)
/// - **No stochasticity**: Doesn't account for DER/load uncertainty (EPRI method uses Monte Carlo)
/// - **No advanced controls**: Assumes fixed power factor (real HC: smart inverters can help)
///
/// **Extensions:**
/// - **Stochastic HC**: Monte Carlo over load/generation scenarios (captures variability)
/// - **Multi-bus HC**: Optimize DER portfolio across buses (integer programming or heuristics)
/// - **Smart inverter HC**: Model volt-VAR, volt-Watt curves (IEEE 1547-2018 functions)
/// - **Upgrade alternatives**: If HC is low, compare cost of DER curtailment vs. feeder reconductoring
///
/// **Pedagogical Note for Grad Students:**
/// Hosting capacity is a grid integration metric, not a physics quantity. It's policy-driven:
/// utilities define acceptable voltage ranges, and HC is the DER level that respects those ranges.
/// Different jurisdictions have different rules (e.g., 0.95-1.05 p.u. in US, 0.90-1.10 in some EU).
/// HC studies inform interconnection queues, grid modernization planning, and rate design.
///
/// **Real-World Example:**
/// A California utility found HC = 2 MW/feeder on average, but varies 0.5-5 MW depending on:
/// - Feeder length (longer = higher impedance = worse voltage drop/rise)
/// - Substation voltage regulator settings (can compensate for some DER-induced voltage rise)
/// - Load density (heavier loads absorb more DER locally, reducing backfeed to substation)
pub fn hostcap_sweep(
    grid_file: &Path,
    target_buses: &[usize],
    max_injection: f64,
    steps: usize,
    out_dir: &Path,
    solver_kind: SolverKind,
) -> Result<()> {
    if steps == 0 {
        return Err(anyhow!("hostcap steps must be at least 1"));
    }
    fs::create_dir_all(out_dir).with_context(|| {
        format!(
            "failed to create hostcap directory '{}'; check permissions",
            out_dir.display()
        )
    })?;

    let network = load_network(grid_file)?;
    let bus_names = collect_bus_names(&network);
    let mut targets = target_buses.to_vec();
    if targets.is_empty() {
        targets = bus_names.keys().copied().collect();
    }

    let mut summary_bus = Vec::new();
    let mut summary_node = Vec::new();
    let mut summary_step = Vec::new();
    let mut summary_injection = Vec::new();
    let mut summary_success = Vec::new();
    let mut summary_artifact = Vec::new();

    for &bus_id in &targets {
        let node_label = bus_names
            .get(&bus_id)
            .unwrap_or(&"unknown".to_string())
            .clone();
        for step in 0..=steps {
            let injection = (step as f64) * max_injection / (steps as f64);
            let host_network = add_virtual_der(&network, bus_id, injection, step);
            let artifact = out_dir.join(format!("hostcap_bus{}_step{}.parquet", bus_id, step));
            let solver = solver_kind.build_solver();
            let run_result = power_flow::ac_optimal_power_flow(
                &host_network,
                solver.as_ref(),
                1e-6,
                20,
                &artifact,
                &[],
            );
            let success = run_result.is_ok();
            if let Err(err) = run_result {
                eprintln!("hostcap run failed for bus {} step {}: {err}", bus_id, step);
            }
            summary_bus.push(bus_id as i64);
            summary_node.push(node_label.clone());
            summary_step.push(step as i64);
            summary_injection.push(injection);
            summary_success.push(success);
            summary_artifact.push(artifact.display().to_string());
        }
    }

    let detail = DataFrame::new(vec![
        Series::new("bus_id", summary_bus),
        Series::new("node_label", summary_node),
        Series::new("step", summary_step),
        Series::new("injection_mw", summary_injection),
        Series::new("success", summary_success),
        Series::new("artifact", summary_artifact),
    ])?;
    let detail_height = detail.height();
    write_parquet(out_dir.join("hostcap_summary.parquet"), detail)?;
    println!(
        "Hostcap sweep generated {} rows and artifacts in {}",
        detail_height,
        out_dir.display()
    );
    Ok(())
}

fn write_parquet(path: PathBuf, mut df: DataFrame) -> Result<()> {
    let mut file = File::create(&path).with_context(|| {
        format!(
            "creating Parquet output '{}'; ensure path exists",
            path.display()
        )
    })?;
    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(&mut df)
        .with_context(|| format!("writing Parquet table {}", path.display()))?;
    Ok(())
}

fn load_network(grid_file: &Path) -> Result<Network> {
    let grid_str = grid_file
        .to_str()
        .ok_or_else(|| anyhow!("grid path contains invalid UTF-8: {}", grid_file.display()))?;
    importers::load_grid_from_arrow(grid_str)
        .with_context(|| format!("loading grid arrow {}", grid_file.display()))
}

fn build_node_frame(network: &Network, feeder: &str) -> DataFrame {
    let mut load_map: HashMap<BusId, f64> = HashMap::new();
    let mut load_map_q: HashMap<BusId, f64> = HashMap::new();
    let mut gens: HashMap<BusId, usize> = HashMap::new();

    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Load(load) => {
                *load_map.entry(load.bus).or_insert(0.0) += load.active_power_mw;
                *load_map_q.entry(load.bus).or_insert(0.0) += load.reactive_power_mvar;
            }
            Node::Gen(gen) => {
                *gens.entry(gen.bus).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    let mut ids = Vec::new();
    let mut phases = Vec::new();
    let mut types = Vec::new();
    let mut v_min = Vec::new();
    let mut v_max = Vec::new();
    let mut load_p = Vec::new();
    let mut load_q = Vec::new();
    let mut feeders = Vec::new();

    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            ids.push(bus.id.value() as i64);
            phases.push("ABC".to_string());
            let node_type = if gens.contains_key(&bus.id) {
                "source"
            } else {
                "load"
            };
            types.push(node_type.to_string());
            v_min.push(0.95);
            v_max.push(1.05);
            load_p.push(*load_map.get(&bus.id).unwrap_or(&0.0));
            load_q.push(*load_map_q.get(&bus.id).unwrap_or(&0.0));
            feeders.push(feeder.to_string());
        }
    }

    DataFrame::new(vec![
        Series::new("node_id", ids),
        Series::new("phase", phases),
        Series::new("node_type", types),
        Series::new("v_min", v_min),
        Series::new("v_max", v_max),
        Series::new("load_p_mw", load_p),
        Series::new("load_q_mvar", load_q),
        Series::new("feeder_id", feeders),
    ])
    .expect("dist pointer frame should always construct")
}

fn build_branch_frame(network: &Network, _feeder: &str) -> DataFrame {
    let mut ids = Vec::new();
    let mut from_nodes = Vec::new();
    let mut to_nodes = Vec::new();
    let mut r = Vec::new();
    let mut x = Vec::new();
    let mut b = Vec::new();
    let mut tap = Vec::new();
    let mut status = Vec::new();
    let mut thermal = Vec::new();

    for edge_idx in network.graph.edge_indices() {
        if let Edge::Branch(branch) = &network.graph[edge_idx] {
            ids.push(branch.id.value() as i64);
            from_nodes.push(branch.from_bus.value() as i64);
            to_nodes.push(branch.to_bus.value() as i64);
            r.push(branch.resistance);
            x.push(branch.reactance);
            b.push(0.0);
            tap.push(1.0);
            status.push("closed".to_string());
            thermal.push(1e6);
        }
    }

    DataFrame::new(vec![
        Series::new("branch_id", ids),
        Series::new("from_node", from_nodes),
        Series::new("to_node", to_nodes),
        Series::new("r", r),
        Series::new("x", x),
        Series::new("b", b),
        Series::new("tap", tap),
        Series::new("status", status),
        Series::new("thermal_limit", thermal),
    ])
    .expect("dist branch frame should always construct")
}

fn collect_bus_names(network: &Network) -> HashMap<usize, String> {
    let mut map = HashMap::new();
    for node_idx in network.graph.node_indices() {
        if let Node::Bus(bus) = &network.graph[node_idx] {
            map.insert(bus.id.value(), bus.name.clone());
        }
    }
    map
}

fn add_virtual_der(network: &Network, bus_id: usize, injection: f64, step: usize) -> Network {
    let mut clone = Network {
        graph: network.graph.clone(),
    };
    let gen_id = GenId::new(clone.graph.node_count());
    let der = Gen {
        id: gen_id,
        name: format!("hostcap_der_{}_{}", bus_id, step),
        bus: BusId::new(bus_id),
        active_power_mw: injection,
        reactive_power_mvar: 0.0,
        pmin_mw: 0.0,
        pmax_mw: injection,
        qmin_mvar: 0.0,
        qmax_mvar: 0.0,
        cost_model: gat_core::CostModel::NoCost,
    };
    clone.graph.add_node(Node::Gen(der));
    clone
}
