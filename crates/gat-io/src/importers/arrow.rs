use std::{collections::HashMap, convert::TryFrom, path::Path};

use crate::arrow_validator::{
    BranchRecord, BusRecord, GeneratorRecord, LoadRecord, NetworkData, };
use crate::arrow_schema::{COST_MODEL_NONE, COST_MODEL_PIECEWISE, COST_MODEL_POLYNOMIAL};
use anyhow::{anyhow, Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
    Transformer, TransformerId,
};
use polars::prelude::{DataFrame, NamedFrom, PolarsResult, Series};

pub fn export_network_to_arrow(network: &Network, output_dir: impl AsRef<Path>) -> Result<()> {
    let writer = crate::exporters::ArrowDirectoryWriter::new(output_dir)?;
    writer.write_network(network, None, None)
}

pub fn load_grid_from_arrow(input_dir: impl AsRef<Path>) -> Result<Network> {
    let reader = crate::exporters::ArrowDirectoryReader::open(input_dir)?;
    network_from_directory_reader(&reader)
}

fn network_from_directory_reader(
    reader: &crate::exporters::ArrowDirectoryReader,
) -> Result<Network> {
    let loaded_tables = reader.load_tables()?;

    let buses_df = loaded_tables
        .get("buses")
        .context("missing 'buses' table in Arrow network directory")?;
    let generators_df = loaded_tables
        .get("generators")
        .context("missing 'generators' table in Arrow network directory")?;
    let loads_df = loaded_tables
        .get("loads")
        .context("missing 'loads' table in Arrow network directory")?;
    let branches_df = loaded_tables
        .get("branches")
        .context("missing 'branches' table in Arrow network directory")?;

    let mut network = Network::new();
    let mut bus_node_map: HashMap<i64, NodeIndex> = HashMap::new();

    // =========================================================================
    // 1. Load Buses
    // =========================================================================
    let bus_id_col = buses_df.column("id")?.i64()?;
    let bus_name_col = buses_df.column("name")?.utf8()?;
    let bus_voltage_kv_col = buses_df.column("voltage_kv")?.f64()?;
    let bus_voltage_pu_col = buses_df.column("voltage_pu")?.f64()?;
    let bus_angle_rad_col = buses_df.column("angle_rad")?.f64()?;
    let bus_vmin_pu_col = buses_df.column("vmin_pu")?.f64()?;
    let bus_vmax_pu_col = buses_df.column("vmax_pu")?.f64()?;
    let bus_area_id_col = buses_df.column("area_id")?.i64()?;
    let bus_zone_id_col = buses_df.column("zone_id")?.i64()?;

    for row in 0..buses_df.height() {
        let id_value = bus_id_col.get(row).context("bus id missing")?;
        let name = bus_name_col.get(row).context("bus name missing")?;
        let voltage_kv = bus_voltage_kv_col
            .get(row)
            .context("bus voltage_kv missing")?;
        let voltage_pu = bus_voltage_pu_col
            .get(row)
            .context("bus voltage_pu missing")?;
        let angle_rad = bus_angle_rad_col
            .get(row)
            .context("bus angle_rad missing")?;
        let vmin_pu = bus_vmin_pu_col.get(row);
        let vmax_pu = bus_vmax_pu_col.get(row);
        let area_id = bus_area_id_col.get(row);
        let zone_id = bus_zone_id_col.get(row);

        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(usize::try_from(id_value).context("bus id must be non-negative")?),
            name: name.to_string(),
            voltage_kv,
            voltage_pu,
            angle_rad,
            vmin_pu,
            vmax_pu,
            area_id: area_id.map(|v| v as i64),
            zone_id: zone_id.map(|v| v as i64),
            ..Bus::default()
        }));
        bus_node_map.insert(id_value, node_idx);
    }

    // =========================================================================
    // 2. Load Generators
    // =========================================================================
    let gen_id_col = generators_df.column("id")?.i64()?;
    let gen_name_col = generators_df.column("name")?.utf8()?;
    let gen_bus_col = generators_df.column("bus")?.i64()?;
    let gen_status_col = generators_df.column("status")?.bool()?;
    let gen_active_power_mw_col = generators_df.column("active_power_mw")?.f64()?;
    let gen_reactive_power_mvar_col = generators_df.column("reactive_power_mvar")?.f64()?;
    let gen_pmin_mw_col = generators_df.column("pmin_mw")?.f64()?;
    let gen_pmax_mw_col = generators_df.column("pmax_mw")?.f64()?;
    let gen_qmin_mvar_col = generators_df.column("qmin_mvar")?.f64()?;
    let gen_qmax_mvar_col = generators_df.column("qmax_mvar")?.f64()?;
    let gen_voltage_setpoint_pu_col = generators_df.column("voltage_setpoint_pu")?.f64()?;
    let gen_mbase_mva_col = generators_df.column("mbase_mva")?.f64()?;
    let gen_cost_model_col = generators_df.column("cost_model")?.i32()?;
    let gen_cost_startup_col = generators_df.column("cost_startup")?.f64()?;
    let gen_cost_shutdown_col = generators_df.column("cost_shutdown")?.f64()?;
    // cost_coeffs and cost_values are list types, handled separately
    let gen_cost_coeffs_col = generators_df.column("cost_coeffs")?.list()?;
    let gen_cost_values_col = generators_df.column("cost_values")?.list()?;
    let gen_is_synchronous_condenser_col =
        generators_df.column("is_synchronous_condenser")?.bool()?;

    for row in 0..generators_df.height() {
        let id_value = gen_id_col.get(row).context("generator id missing")?;
        let name = gen_name_col.get(row).context("generator name missing")?;
        let bus_value = gen_bus_col
            .get(row)
            .context("generator bus reference missing")?;
        let status = gen_status_col
            .get(row)
            .context("generator status missing")?;
        let active_power_mw = gen_active_power_mw_col
            .get(row)
            .context("generator active_power_mw missing")?;
        let reactive_power_mvar = gen_reactive_power_mvar_col
            .get(row)
            .context("generator reactive_power_mvar missing")?;
        let pmin_mw = gen_pmin_mw_col
            .get(row)
            .context("generator pmin_mw missing")?;
        let pmax_mw = gen_pmax_mw_col
            .get(row)
            .context("generator pmax_mw missing")?;
        let qmin_mvar = gen_qmin_mvar_col
            .get(row)
            .context("generator qmin_mvar missing")?;
        let qmax_mvar = gen_qmax_mvar_col
            .get(row)
            .context("generator qmax_mvar missing")?;
        let voltage_setpoint_pu = gen_voltage_setpoint_pu_col.get(row);
        let mbase_mva = gen_mbase_mva_col.get(row);
        let cost_model_code = gen_cost_model_col
            .get(row)
            .context("generator cost_model missing")?;
        let cost_startup = gen_cost_startup_col.get(row);
        let cost_shutdown = gen_cost_shutdown_col.get(row);
        let is_synchronous_condenser = gen_is_synchronous_condenser_col
            .get(row)
            .context("generator is_synchronous_condenser missing")?;

        let cost_model = match cost_model_code {
            COST_MODEL_NONE => gat_core::CostModel::NoCost,
            COST_MODEL_PIECEWISE => {
                let coeffs_series = gen_cost_coeffs_col
                    .get_as_series(row)
                    .context("piecewise cost_coeffs missing")?;
                let values_series = gen_cost_values_col
                    .get_as_series(row)
                    .context("piecewise cost_values missing")?;

                let coeffs: Vec<f64> = coeffs_series
                    .f64()?
                    .into_iter()
                    .flatten()
                    .collect();
                let values: Vec<f64> = values_series
                    .f64()?
                    .into_iter()
                    .flatten()
                    .collect();

                gat_core::CostModel::PiecewiseLinear(
                    coeffs.into_iter().zip(values.into_iter()).collect(),
                )
            }
            COST_MODEL_POLYNOMIAL => {
                let coeffs_series = gen_cost_coeffs_col
                    .get_as_series(row)
                    .context("polynomial cost_coeffs missing")?;
                let coeffs: Vec<f64> = coeffs_series
                    .f64()?
                    .into_iter()
                    .flatten()
                    .collect();
                gat_core::CostModel::Polynomial(coeffs)
            }
            _ => gat_core::CostModel::NoCost,
        };

        if !bus_node_map.contains_key(&bus_value) {
            return Err(anyhow!(
                "generator {} references unknown bus {}",
                id_value,
                bus_value
            ));
        }

        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(usize::try_from(id_value).context("gen id must be non-negative")?),
            name: name.to_string(),
            bus: BusId::new(usize::try_from(bus_value).context("gen bus id must be non-negative")?),
            status,
            active_power_mw,
            reactive_power_mvar,
            pmin_mw,
            pmax_mw,
            qmin_mvar,
            qmax_mvar,
            voltage_setpoint_pu,
            mbase_mva,
            cost_model,
            cost_startup,
            cost_shutdown,
            is_synchronous_condenser,
        }));
    }

    // =========================================================================
    // 3. Load Loads
    // =========================================================================
    let load_id_col = loads_df.column("id")?.i64()?;
    let load_name_col = loads_df.column("name")?.utf8()?;
    let load_bus_col = loads_df.column("bus")?.i64()?;
    let load_status_col = loads_df.column("status")?.bool()?;
    let load_active_power_mw_col = loads_df.column("active_power_mw")?.f64()?;
    let load_reactive_power_mvar_col = loads_df.column("reactive_power_mvar")?.f64()?;

    for row in 0..loads_df.height() {
        let id_value = load_id_col.get(row).context("load id missing")?;
        let name = load_name_col.get(row).context("load name missing")?;
        let bus_value = load_bus_col
            .get(row)
            .context("load bus reference missing")?;
        let status = load_status_col.get(row).context("load status missing")?;
        let active_power_mw = load_active_power_mw_col
            .get(row)
            .context("load active_power_mw missing")?;
        let reactive_power_mvar = load_reactive_power_mvar_col
            .get(row)
            .context("load reactive_power_mvar missing")?;

        if !status {
            continue;
        }

        if !bus_node_map.contains_key(&bus_value) {
            return Err(anyhow!(
                "load {} references unknown bus {}",
                id_value,
                bus_value
            ));
        }

        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(usize::try_from(id_value).context("load id must be non-negative")?),
            name: name.to_string(),
            bus: BusId::new(
                usize::try_from(bus_value).context("load bus id must be non-negative")?,
            ),
            active_power_mw,
            reactive_power_mvar,
        }));
    }

    // =========================================================================
    // 4. Load Branches
    // =========================================================================
    let branch_id_col = branches_df.column("id")?.i64()?;
    let branch_name_col = branches_df.column("name")?.utf8()?;
    let branch_element_type_col = branches_df.column("element_type")?.utf8()?;
    let branch_from_bus_col = branches_df.column("from_bus")?.i64()?;
    let branch_to_bus_col = branches_df.column("to_bus")?.i64()?;
    let branch_status_col = branches_df.column("status")?.bool()?;
    let branch_resistance_pu_col = branches_df.column("resistance_pu")?.f64()?;
    let branch_reactance_pu_col = branches_df.column("reactance_pu")?.f64()?;
    let branch_charging_b_pu_col = branches_df.column("charging_b_pu")?.f64()?;
    let branch_tap_ratio_col = branches_df.column("tap_ratio")?.f64()?;
    let branch_phase_shift_rad_col = branches_df.column("phase_shift_rad")?.f64()?;
    let branch_rating_a_mva_col = branches_df.column("rate_a_mva")?.f64()?;
    let branch_rating_b_mva_col = branches_df.column("rate_b_mva")?.f64()?;
    let branch_rating_c_mva_col = branches_df.column("rate_c_mva")?.f64()?;
    let branch_angle_min_rad_col = branches_df.column("angle_min_rad")?.f64()?;
    let branch_angle_max_rad_col = branches_df.column("angle_max_rad")?.f64()?;

    for row in 0..branches_df.height() {
        let id_value = branch_id_col.get(row).context("branch id missing")?;
        let name = branch_name_col.get(row).context("branch name missing")?;
        let element_type = branch_element_type_col
            .get(row)
            .context("branch element_type missing")?;
        let from_bus_value = branch_from_bus_col
            .get(row)
            .context("branch from_bus missing")?;
        let to_bus_value = branch_to_bus_col
            .get(row)
            .context("branch to_bus missing")?;
        let status = branch_status_col
            .get(row)
            .context("branch status missing")?;
        let resistance_pu = branch_resistance_pu_col
            .get(row)
            .context("branch resistance_pu missing")?;
        let reactance_pu = branch_reactance_pu_col
            .get(row)
            .context("branch reactance_pu missing")?;
        let charging_b_pu = branch_charging_b_pu_col
            .get(row)
            .context("branch charging_b_pu missing")?;
        let tap_ratio = branch_tap_ratio_col
            .get(row)
            .context("branch tap_ratio missing")?;
        let phase_shift_rad = branch_phase_shift_rad_col
            .get(row)
            .context("branch phase_shift_rad missing")?;
        let rating_a_mva = branch_rating_a_mva_col.get(row);
        let rating_b_mva = branch_rating_b_mva_col.get(row);
        let rating_c_mva = branch_rating_c_mva_col.get(row);
        let angle_min_rad = branch_angle_min_rad_col.get(row);
        let angle_max_rad = branch_angle_max_rad_col.get(row);

        let from_idx = bus_node_map.get(&from_bus_value).with_context(|| {
            format!(
                "branch {} references unknown from bus {}",
                id_value, from_bus_value
            )
        })?;
        let to_idx = bus_node_map.get(&to_bus_value).with_context(|| {
            format!(
                "branch {} references unknown to bus {}",
                id_value, to_bus_value
            )
        })?;

        match element_type {
            "line" => {
                let branch = Branch {
                    id: BranchId::new(
                        usize::try_from(id_value).context("branch id must be non-negative")?,
                    ),
                    name: name.to_string(),
                    from_bus: BusId::new(
                        usize::try_from(from_bus_value)
                            .context("branch from_bus id must be non-negative")?,
                    ),
                    to_bus: BusId::new(
                        usize::try_from(to_bus_value)
                            .context("branch to_bus id must be non-negative")?,
                    ),
                    status,
                    resistance: resistance_pu,
                    reactance: reactance_pu,
                    charging_b_pu,
                    tap_ratio,
                    phase_shift_rad,
                    s_max_mva: rating_a_mva, // Use rating_a_mva as s_max_mva for now
                    rating_a_mva,
                    rating_b_mva,
                    rating_c_mva,
                    angle_min_rad,
                    angle_max_rad,
                    ..Branch::default()
                };
                network
                    .graph
                    .add_edge(*from_idx, *to_idx, Edge::Branch(branch));
            }
            "transformer" => {
                let transformer = Transformer {
                    id: TransformerId::new(
                        usize::try_from(id_value).context("transformer id must be non-negative")?,
                    ),
                    name: name.to_string(),
                    from_bus: BusId::new(
                        usize::try_from(from_bus_value)
                            .context("transformer from_bus id must be non-negative")?,
                    ),
                    to_bus: BusId::new(
                        usize::try_from(to_bus_value)
                            .context("transformer to_bus id must be non-negative")?,
                    ),
                    ratio: tap_ratio,
                };
                network
                    .graph
                    .add_edge(*from_idx, *to_idx, Edge::Transformer(transformer));
            }
            _ => return Err(anyhow!("Unknown element type: {}", element_type)),
        }
    }
    // NetworkValidator::validate(&network_to_validator_data(&network))
    //     .context("imported network failed integrity validation")?;

    Ok(network)
}

fn network_to_dataframe(network: &Network) -> PolarsResult<DataFrame> {
    let mut element_type: Vec<String> = Vec::new();
    let mut element_id: Vec<i64> = Vec::new();
    let mut element_name: Vec<String> = Vec::new();
    let mut voltage_kv: Vec<Option<f64>> = Vec::new();
    let mut from_bus: Vec<Option<i64>> = Vec::new();
    let mut to_bus: Vec<Option<i64>> = Vec::new();
    let mut resistance: Vec<Option<f64>> = Vec::new();
    let mut reactance: Vec<Option<f64>> = Vec::new();
    let mut tap_ratio: Vec<Option<f64>> = Vec::new();
    let mut phase_shift_rad: Vec<Option<f64>> = Vec::new();
    let mut charging_b_pu: Vec<Option<f64>> = Vec::new();
    let mut s_max_mva: Vec<Option<f64>> = Vec::new();
    let mut status: Vec<Option<bool>> = Vec::new();
    let mut rating_a_mva: Vec<Option<f64>> = Vec::new();
    let mut active_power: Vec<Option<f64>> = Vec::new();
    let mut reactive_power: Vec<Option<f64>> = Vec::new();

    for node_idx in network.graph.node_indices() {
        match &network.graph[node_idx] {
            Node::Bus(bus) => {
                element_type.push("bus".to_string());
                element_id.push(bus.id.value() as i64);
                element_name.push(bus.name.clone());
                voltage_kv.push(Some(bus.voltage_kv));
                from_bus.push(None);
                to_bus.push(None);
                resistance.push(None);
                reactance.push(None);
                tap_ratio.push(None);
                phase_shift_rad.push(None);
                charging_b_pu.push(None);
                s_max_mva.push(None);
                status.push(None);
                rating_a_mva.push(None);
                active_power.push(None);
                reactive_power.push(None);
            }
            Node::Gen(gen) => {
                element_type.push("gen".to_string());
                element_id.push(gen.id.value() as i64);
                element_name.push(gen.name.clone());
                voltage_kv.push(None);
                from_bus.push(Some(gen.bus.value() as i64));
                to_bus.push(None);
                resistance.push(None);
                reactance.push(None);
                tap_ratio.push(None);
                phase_shift_rad.push(None);
                charging_b_pu.push(None);
                s_max_mva.push(None);
                status.push(None);
                rating_a_mva.push(None);
                active_power.push(Some(gen.active_power_mw));
                reactive_power.push(Some(gen.reactive_power_mvar));
            }
            Node::Load(load) => {
                element_type.push("load".to_string());
                element_id.push(load.id.value() as i64);
                element_name.push(load.name.clone());
                voltage_kv.push(None);
                from_bus.push(Some(load.bus.value() as i64));
                to_bus.push(None);
                resistance.push(None);
                reactance.push(None);
                tap_ratio.push(None);
                phase_shift_rad.push(None);
                charging_b_pu.push(None);
                s_max_mva.push(None);
                status.push(None);
                rating_a_mva.push(None);
                active_power.push(Some(load.active_power_mw));
                reactive_power.push(Some(load.reactive_power_mvar));
            }
        }
    }

    for edge_idx in network.graph.edge_indices() {
        let edge = &network.graph[edge_idx];
        match edge {
            Edge::Branch(branch) => {
                element_type.push("branch".to_string());
                element_id.push(branch.id.value() as i64);
                element_name.push(branch.name.clone());
                voltage_kv.push(None);
                from_bus.push(Some(branch.from_bus.value() as i64));
                to_bus.push(Some(branch.to_bus.value() as i64));
                resistance.push(Some(branch.resistance));
                reactance.push(Some(branch.reactance));
                tap_ratio.push(Some(branch.tap_ratio));
                phase_shift_rad.push(Some(branch.phase_shift_rad));
                charging_b_pu.push(Some(branch.charging_b_pu));
                s_max_mva.push(branch.s_max_mva);
                status.push(Some(branch.status));
                rating_a_mva.push(branch.rating_a_mva);
                active_power.push(None);
                reactive_power.push(None);
            }
            Edge::Transformer(tx) => {
                element_type.push("transformer".to_string());
                element_id.push(tx.id.value() as i64);
                element_name.push(tx.name.clone());
                voltage_kv.push(None);
                from_bus.push(Some(tx.from_bus.value() as i64));
                to_bus.push(Some(tx.to_bus.value() as i64));
                resistance.push(None);
                reactance.push(None);
                tap_ratio.push(None);
                phase_shift_rad.push(None);
                charging_b_pu.push(None);
                s_max_mva.push(None);
                status.push(None);
                rating_a_mva.push(None);
                active_power.push(None);
                reactive_power.push(None);
            }
        }
    }

    DataFrame::new(vec![
        Series::new("type", element_type),
        Series::new("id", element_id),
        Series::new("name", element_name),
        Series::new("voltage_kv", voltage_kv),
        Series::new("from_bus", from_bus),
        Series::new("to_bus", to_bus),
        Series::new("resistance", resistance),
        Series::new("reactance", reactance),
        Series::new("tap_ratio", tap_ratio),
        Series::new("phase_shift_rad", phase_shift_rad),
        Series::new("charging_b_pu", charging_b_pu),
        Series::new("s_max_mva", s_max_mva),
        Series::new("status", status),
        Series::new("rating_a_mva", rating_a_mva),
        Series::new("active_power_mw", active_power),
        Series::new("reactive_power_mvar", reactive_power),
    ])
}

// Convert in-memory Network into validator-friendly records
fn network_to_validator_data(network: &Network) -> NetworkData {
    let mut data = NetworkData::default();

    for node in network.graph.node_weights() {
        match node {
            Node::Bus(bus) => data.buses.push(BusRecord {
                id: bus.id.value() as i64,
            }),
            Node::Gen(gen) => {
                let (cost_model, cost_coeffs, cost_values) = match &gen.cost_model {
                    gat_core::CostModel::NoCost => (0, Vec::new(), Vec::new()),
                    gat_core::CostModel::PiecewiseLinear(points) => {
                        let mut xs = Vec::with_capacity(points.len());
                        let mut ys = Vec::with_capacity(points.len());
                        for (x, y) in points {
                            xs.push(*x);
                            ys.push(*y);
                        }
                        (1, xs, ys)
                    }
                    gat_core::CostModel::Polynomial(coeffs) => (2, coeffs.clone(), Vec::new()),
                };

                data.generators.push(GeneratorRecord {
                    id: gen.id.value() as i64,
                    bus: gen.bus.value() as i64,
                    cost_model,
                    cost_coeffs,
                    cost_values,
                });
            }
            Node::Load(load) => data.loads.push(LoadRecord {
                id: load.id.value() as i64,
                bus: load.bus.value() as i64,
            }),
        }
    }

    for edge in network.graph.edge_weights() {
        if let Edge::Branch(branch) = edge {
            data.branches.push(BranchRecord {
                id: branch.id.value() as i64,
                from_bus: branch.from_bus.value() as i64,
                to_bus: branch.to_bus.value() as i64,
            });
        }
    }

    data
}
