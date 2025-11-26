use anyhow::Result;
use gat_core::Network;
use std::path::Path;
use std::fs;

/// Export network to PSS/E RAW format string (v33)
pub fn export_to_psse_string(network: &Network, case_name: &str) -> Result<String> {
    let mut output = String::new();

    // Header line 1: IC, SBASE, REV, XFRRAT, NXFRAT, BASFRQ
    output.push_str(&format!("0,   100.00, 33, 0, 0, 60.00 / {}\n", case_name));
    // Header lines 2-3 (comments)
    output.push_str(&format!("{}\n", case_name));
    output.push_str("Exported by gat\n");

    Ok(output)
}

/// Export network to PSS/E RAW file
pub fn export_to_psse(network: &Network, path: &Path, case_name: &str) -> Result<()> {
    let content = export_to_psse_string(network, case_name)?;
    fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gat_core::{Network, Node, Bus, BusId};

    #[test]
    fn export_psse_creates_valid_header() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));

        let output = export_to_psse_string(&network, "test_case").unwrap();
        assert!(output.contains("0,   100.00, 33"));  // PSS/E v33 header
        assert!(output.contains("test_case"));
    }
}
