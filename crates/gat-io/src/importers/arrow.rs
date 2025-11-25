use std::{collections::HashMap, convert::TryFrom, fs::File};

use anyhow::{anyhow, Context, Result};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node, NodeIndex,
    Transformer, TransformerId,
};
use polars::prelude::{
    DataFrame, IpcReader, IpcWriter, NamedFrom, PolarsResult, SerReader, SerWriter, Series,
};

pub(super) fn write_network_to_arrow(network: &Network, output_file: &str) -> Result<()> {
    let mut df = network_to_dataframe(network).context("building DataFrame for network export")?;
    let mut file = File::create(output_file).with_context(|| {
        format!(
            "creating Arrow output '{}'; ensure directory exists",
            output_file
        )
    })?;
    IpcWriter::new(&mut file)
        .finish(&mut df)
        .context("writing Arrow output file")?;
    Ok(())
}

pub fn export_network_to_arrow(network: &Network, output_file: &str) -> Result<()> {
    write_network_to_arrow(network, output_file)
}

pub fn load_grid_from_arrow(grid_file: &str) -> Result<Network> {
    let file = File::open(grid_file)
        .with_context(|| format!("opening Arrow dataset '{}'; ensure it exists", grid_file))?;
    let reader = IpcReader::new(file);
    let df = reader
        .finish()
        .context("reading Arrow IPC dataset for grid import")?;
    dataframe_to_network(&df).context("converting Arrow dataset into Network graph")
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
        Series::new("active_power_mw", active_power),
        Series::new("reactive_power_mvar", reactive_power),
    ])
}

fn dataframe_to_network(df: &DataFrame) -> Result<Network> {
    let type_col = df
        .column("type")
        .context("missing 'type' column in grid arrow file")?
        .utf8()
        .context("'type' column must be utf8")?;
    let id_col = df
        .column("id")
        .context("missing 'id' column in grid arrow file")?
        .i64()
        .context("'id' column must be integers")?;
    let name_col = df
        .column("name")
        .context("missing 'name' column in grid arrow file")?
        .utf8()
        .context("'name' column must be utf8")?;
    let voltage_col = df
        .column("voltage_kv")
        .context("missing 'voltage_kv' column in grid arrow file")?
        .f64()
        .context("'voltage_kv' column must be float64")?;
    let from_col = df
        .column("from_bus")
        .context("missing 'from_bus' column in grid arrow file")?
        .i64()
        .context("'from_bus' column must be integers")?;
    let to_col = df
        .column("to_bus")
        .context("missing 'to_bus' column in grid arrow file")?
        .i64()
        .context("'to_bus' column must be integers")?;
    let resistance_col = df
        .column("resistance")
        .context("missing 'resistance' column in grid arrow file")?
        .f64()
        .context("'resistance' column must be float64")?;
    let reactance_col = df
        .column("reactance")
        .context("missing 'reactance' column in grid arrow file")?
        .f64()
        .context("'reactance' column must be float64")?;
    let active_power_col = df
        .column("active_power_mw")
        .ok()
        .and_then(|series| series.f64().ok());
    let reactive_power_col = df
        .column("reactive_power_mvar")
        .ok()
        .and_then(|series| series.f64().ok());

    let mut network = Network::new();
    let mut bus_index_map: HashMap<i64, NodeIndex> = HashMap::new();

    for row in 0..df.height() {
        if type_col.get(row) == Some("bus") {
            let id_value = id_col
                .get(row)
                .context("grid row missing id while reconstructing buses")?;
            let name = name_col
                .get(row)
                .context("grid row missing name while reconstructing buses")?;
            let voltage = voltage_col
                .get(row)
                .context("grid row missing voltage for bus reconstruction")?;
            let node_index = network.graph.add_node(Node::Bus(Bus {
                id: BusId::new(usize::try_from(id_value).context("bus id must be non-negative")?),
                name: name.to_string(),
                voltage_kv: voltage,
            }));
            bus_index_map.insert(id_value, node_index);
        }
    }

    for row in 0..df.height() {
        match type_col.get(row) {
            Some("gen") => {
                let name = name_col
                    .get(row)
                    .context("grid row missing name for generator")?;
                let bus_value = from_col
                    .get(row)
                    .context("generator row missing bus reference")?;
                let bus_id = usize::try_from(bus_value).context("bus id must be non-negative")?;
                if !bus_index_map.contains_key(&bus_value) {
                    return Err(anyhow!("generator references unknown bus {}", bus_value));
                }
                let id_value = id_col
                    .get(row)
                    .context("grid row missing id for generator")?;
                let active_power = active_power_col
                    .as_ref()
                    .and_then(|col| col.get(row))
                    .unwrap_or(0.0);
                let reactive_power = reactive_power_col
                    .as_ref()
                    .and_then(|col| col.get(row))
                    .unwrap_or(0.0);

                network.graph.add_node(Node::Gen(Gen {
                    id: GenId::new(
                        usize::try_from(id_value).context("gen id must be non-negative")?,
                    ),
                    name: name.to_string(),
                    bus: BusId::new(bus_id),
                    active_power_mw: active_power,
                    reactive_power_mvar: reactive_power,
                    pmin_mw: 0.0,
                    pmax_mw: f64::INFINITY,
                    qmin_mvar: f64::NEG_INFINITY,
                    qmax_mvar: f64::INFINITY,
                    cost_model: gat_core::CostModel::NoCost,
                }));
            }
            Some("load") => {
                let name = name_col
                    .get(row)
                    .context("grid row missing name for load")?;
                let bus_value = from_col
                    .get(row)
                    .context("load row missing bus reference")?;
                let bus_id = usize::try_from(bus_value).context("bus id must be non-negative")?;
                if !bus_index_map.contains_key(&bus_value) {
                    return Err(anyhow!("load references unknown bus {}", bus_value));
                }
                let id_value = id_col.get(row).context("load row missing id")?;
                let active_power = active_power_col
                    .as_ref()
                    .and_then(|col| col.get(row))
                    .unwrap_or(0.0);
                let reactive_power = reactive_power_col
                    .as_ref()
                    .and_then(|col| col.get(row))
                    .unwrap_or(0.0);

                network.graph.add_node(Node::Load(Load {
                    id: LoadId::new(
                        usize::try_from(id_value).context("load id must be non-negative")?,
                    ),
                    name: name.to_string(),
                    bus: BusId::new(bus_id),
                    active_power_mw: active_power,
                    reactive_power_mvar: reactive_power,
                }));
            }
            _ => {}
        }
    }

    let mut branch_counter = 0usize;
    let mut transformer_counter = 0usize;
    for row in 0..df.height() {
        match type_col.get(row) {
            Some("branch") => {
                let name = name_col
                    .get(row)
                    .context("grid row missing name for branch")?;
                let from_bus = from_col.get(row).context("branch row missing from_bus")?;
                let to_bus = to_col.get(row).context("branch row missing to_bus")?;
                let resistance = resistance_col.get(row).unwrap_or(0.0);
                let reactance = reactance_col.get(row).unwrap_or(0.0);
                let from_idx = bus_index_map
                    .get(&from_bus)
                    .with_context(|| format!("branch references unknown from bus {}", from_bus))?;
                let to_idx = bus_index_map
                    .get(&to_bus)
                    .with_context(|| format!("branch references unknown to bus {}", to_bus))?;

                let branch = Branch {
                    id: BranchId::new(branch_counter),
                    name: name.to_string(),
                    from_bus: BusId::new(
                        usize::try_from(from_bus).context("bus id must be non-negative")?,
                    ),
                    to_bus: BusId::new(
                        usize::try_from(to_bus).context("bus id must be non-negative")?,
                    ),
                    resistance,
                    reactance,
                };

                network
                    .graph
                    .add_edge(*from_idx, *to_idx, Edge::Branch(branch));
                branch_counter += 1;
            }
            Some("transformer") => {
                let name = name_col
                    .get(row)
                    .context("grid row missing name for transformer")?;
                let from_bus = from_col
                    .get(row)
                    .context("transformer row missing from_bus")?;
                let to_bus = to_col.get(row).context("transformer row missing to_bus")?;
                let from_idx = bus_index_map.get(&from_bus).with_context(|| {
                    format!("transformer references unknown from bus {}", from_bus)
                })?;
                let to_idx = bus_index_map
                    .get(&to_bus)
                    .with_context(|| format!("transformer references unknown to bus {}", to_bus))?;

                let transformer = Transformer {
                    id: TransformerId::new(transformer_counter),
                    name: name.to_string(),
                    from_bus: BusId::new(
                        usize::try_from(from_bus).context("bus id must be non-negative")?,
                    ),
                    to_bus: BusId::new(
                        usize::try_from(to_bus).context("bus id must be non-negative")?,
                    ),
                    ratio: 1.0,
                };

                network
                    .graph
                    .add_edge(*from_idx, *to_idx, Edge::Transformer(transformer));
                transformer_counter += 1;
            }
            _ => continue,
        }
    }

    Ok(network)
}
