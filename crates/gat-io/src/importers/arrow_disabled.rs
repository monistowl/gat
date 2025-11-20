use anyhow::{bail, Result};
use gat_core::Network;

pub(super) fn write_network_to_arrow(_network: &Network, _output_file: &str) -> Result<()> {
    bail!("Arrow IPC support is disabled; build with the 'ipc' feature to enable it")
}

pub fn load_grid_from_arrow(_grid_file: &str) -> Result<Network> {
    bail!("Arrow IPC support is disabled; build with the 'ipc' feature to enable it")
}
