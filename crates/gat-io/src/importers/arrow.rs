use std::{collections::HashMap, convert::TryFrom, path::Path};

use crate::arrow_manifest::ArrowManifest;
use crate::arrow_schema::{COST_MODEL_NONE, COST_MODEL_PIECEWISE, COST_MODEL_POLYNOMIAL};
use anyhow::{anyhow, Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
    Transformer, TransformerId,
};

pub fn export_network_to_arrow(network: &Network, output_dir: impl AsRef<Path>) -> Result<()> {
    let writer = crate::exporters::ArrowDirectoryWriter::new(output_dir)?;
    writer.write_network(network, None, None)
}

pub fn load_grid_from_arrow(input_dir: impl AsRef<Path>) -> Result<Network> {
    load_grid_from_arrow_with_manifest(input_dir).map(|(network, _)| network)
}

pub fn load_grid_from_arrow_with_manifest(
    input_dir: impl AsRef<Path>,
) -> Result<(Network, ArrowManifest)> {
    let reader = crate::exporters::ArrowDirectoryReader::open(&input_dir)?;
    let manifest = reader.manifest().clone();
    let network = network_from_directory_reader(&reader)?;
    Ok((network, manifest))
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
            base_kv: gat_core::Kilovolts(voltage_kv),
            voltage_pu: gat_core::PerUnit(voltage_pu),
            angle_rad: gat_core::Radians(angle_rad),
            vmin_pu: vmin_pu.map(gat_core::PerUnit),
            vmax_pu: vmax_pu.map(gat_core::PerUnit),
            area_id: area_id.map(|v| v as i64),
            zone_id: zone_id.map(|v| v as i64),
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

                let coeffs: Vec<f64> = coeffs_series.f64()?.into_iter().flatten().collect();
                let values: Vec<f64> = values_series.f64()?.into_iter().flatten().collect();

                gat_core::CostModel::PiecewiseLinear(
                    coeffs.into_iter().zip(values.into_iter()).collect(),
                )
            }
            COST_MODEL_POLYNOMIAL => {
                let coeffs_series = gen_cost_coeffs_col
                    .get_as_series(row)
                    .context("polynomial cost_coeffs missing")?;
                let coeffs: Vec<f64> = coeffs_series.f64()?.into_iter().flatten().collect();
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
            active_power: gat_core::Megawatts(active_power_mw),
            reactive_power: gat_core::Megavars(reactive_power_mvar),
            pmin: gat_core::Megawatts(pmin_mw),
            pmax: gat_core::Megawatts(pmax_mw),
            qmin: gat_core::Megavars(qmin_mvar),
            qmax: gat_core::Megavars(qmax_mvar),
            voltage_setpoint: voltage_setpoint_pu.map(gat_core::PerUnit),
            mbase: mbase_mva.map(gat_core::MegavoltAmperes),
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
            active_power: gat_core::Megawatts(active_power_mw),
            reactive_power: gat_core::Megavars(reactive_power_mvar),
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
                    charging_b: gat_core::PerUnit(charging_b_pu),
                    tap_ratio,
                    phase_shift: gat_core::Radians(phase_shift_rad),
                    s_max: rating_a_mva.map(gat_core::MegavoltAmperes), // Use rating_a_mva as s_max for now
                    rating_a: rating_a_mva.map(gat_core::MegavoltAmperes),
                    rating_b: rating_b_mva.map(gat_core::MegavoltAmperes),
                    rating_c: rating_c_mva.map(gat_core::MegavoltAmperes),
                    angle_min: angle_min_rad.map(gat_core::Radians),
                    angle_max: angle_max_rad.map(gat_core::Radians),
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
