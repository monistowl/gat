use anyhow::Result;
use gat_core::{Network, Node, Bus};
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

    // Bus data section
    write_bus_section(network, &mut output);

    Ok(output)
}

/// Write bus data section in PSS/E v33 format
fn write_bus_section(network: &Network, output: &mut String) {
    // Collect buses
    let mut buses: Vec<&Bus> = network.graph.node_weights()
        .filter_map(|n| if let Node::Bus(b) = n { Some(b) } else { None })
        .collect();
    buses.sort_by_key(|b| b.id.value());

    for bus in buses {
        // PSS/E v33 bus format:
        // I, 'NAME', BASKV, IDE, AREA, ZONE, OWNER, VM, VA, NVHI, NVLO, EVHI, EVLO
        let ide = 1; // PQ bus by default
        let area = bus.area_id.unwrap_or(1);
        let zone = bus.zone_id.unwrap_or(1);
        let vmax = bus.vmax_pu.unwrap_or(1.1);
        let vmin = bus.vmin_pu.unwrap_or(0.9);

        output.push_str(&format!(
            "{},'{:12}',{:.1},{},{},{},1,{:.4},{:.2},{:.4},{:.4},{:.4},{:.4}\n",
            bus.id.value(),
            &bus.name[..bus.name.len().min(12)],
            bus.voltage_kv,
            ide,
            area,
            zone,
            bus.voltage_pu,
            0.0, // voltage angle
            vmax,
            vmin,
            vmax,
            vmin,
        ));
    }
    output.push_str("0 / END OF BUS DATA, BEGIN LOAD DATA\n");
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

    #[test]
    fn export_psse_includes_bus_data() {
        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "SLACK".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            vmax_pu: Some(1.05),
            vmin_pu: Some(0.95),
            area_id: Some(1),
            zone_id: Some(1),
            ..Bus::default()
        }));
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "LOAD".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));

        let output = export_to_psse_string(&network, "test").unwrap();
        // Check bus section marker
        assert!(output.contains("0 / END OF BUS DATA"));
        // Check bus 1 data (fields: I, NAME, BASKV, IDE, AREA, ZONE, OWNER, VM, VA)
        assert!(output.contains("1,'SLACK"));
        assert!(output.contains("138.0"));
    }
}
