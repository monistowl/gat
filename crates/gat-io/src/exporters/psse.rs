use anyhow::Result;
use gat_core::{Network, Node, Bus, Load, Gen};
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

    // Load data section
    write_load_section(network, &mut output);

    // Generator data section
    write_generator_section(network, &mut output);

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

/// Write load data section in PSS/E v33 format
fn write_load_section(network: &Network, output: &mut String) {
    // Collect loads
    let mut loads: Vec<&Load> = network.graph.node_weights()
        .filter_map(|n| if let Node::Load(l) = n { Some(l) } else { None })
        .collect();
    loads.sort_by_key(|l| (l.bus.value(), l.id.value()));

    for (idx, load) in loads.iter().enumerate() {
        // PSS/E v33 load format:
        // I, ID, STATUS, AREA, ZONE, PL, QL, IP, IQ, YP, YQ, OWNER, SCALE, INTRPT
        // I = bus number
        // ID = load identifier (sequential 1-based in this case)
        // STATUS = 1 (in service)
        // AREA = 1 (default)
        // ZONE = 1 (default)
        // PL = active power (MW)
        // QL = reactive power (MVAr)
        // IP, IQ = constant current components (0.0 for constant power)
        // YP, YQ = constant admittance components (0.0 for constant power)
        // OWNER = 1 (default)
        // SCALE = 1 (no scaling)
        // INTRPT = 0 (not interruptible)
        output.push_str(&format!(
            "{},{},1,1,1,{:.1},{:.1},0.0,0.0,0.0,0.0,1,1,0\n",
            load.bus.value(),
            idx + 1,
            load.active_power_mw,
            load.reactive_power_mvar,
        ));
    }
    output.push_str("0 / END OF LOAD DATA, BEGIN FIXED SHUNT DATA\n");
}

/// Write generator data section in PSS/E v33 format
fn write_generator_section(network: &Network, output: &mut String) {
    // Collect generators
    let mut generators: Vec<&Gen> = network.graph.node_weights()
        .filter_map(|n| if let Node::Gen(g) = n { Some(g) } else { None })
        .collect();
    generators.sort_by_key(|g| (g.bus.value(), g.id.value()));

    for (idx, gen) in generators.iter().enumerate() {
        // PSS/E v33 generator format:
        // I, ID, PG, QG, QT, QB, VS, IREG, MBASE, ZR, ZX, RT, XT, GTAP, STAT, RMPCT, PT, PB, O1, F1...
        // I = bus number
        // ID = generator ID (sequential 1-based)
        // PG = active power output (MW)
        // QG = reactive power output (MVAr)
        // QT = max reactive power (MVAr)
        // QB = min reactive power (MVAr)
        // VS = voltage setpoint (pu)
        // IREG = 0 (no regulated bus)
        // MBASE = machine base MVA
        // ZR, ZX = 0.0 (source impedance)
        // RT, XT = 0.0 (transformer impedance)
        // GTAP = 1.0
        // STAT = 1 (in service) or 0 (out of service)
        // RMPCT = 100.0
        // PT = Pmax
        // PB = Pmin

        let vs = gen.voltage_setpoint_pu.unwrap_or(1.0);
        let mbase = gen.mbase_mva.unwrap_or(100.0);
        let stat = if gen.status { 1 } else { 0 };

        output.push_str(&format!(
            "{},{},{:.1},{:.1},{:.1},{:.1},{:.2},0,{:.1},0.0,0.0,0.0,0.0,1.0,{},100.0,{:.1},{:.1}\n",
            gen.bus.value(),
            idx + 1,
            gen.active_power_mw,
            gen.reactive_power_mvar,
            gen.qmax_mvar,
            gen.qmin_mvar,
            vs,
            mbase,
            stat,
            gen.pmax_mw,
            gen.pmin_mw,
        ));
    }
    output.push_str("0 / END OF GENERATOR DATA, BEGIN BRANCH DATA\n");
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

    #[test]
    fn export_psse_includes_load_data() {
        use gat_core::{Load, LoadId};

        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load1".to_string(),
            bus: BusId::new(1),
            active_power_mw: 100.0,
            reactive_power_mvar: 50.0,
        }));

        let output = export_to_psse_string(&network, "test").unwrap();
        assert!(output.contains("0 / END OF LOAD DATA"));
        assert!(output.contains("100.0")); // PL
        assert!(output.contains("50.0"));  // QL
    }

    #[test]
    fn export_psse_includes_generator_data() {
        use gat_core::{Gen, GenId};

        let mut network = Network::new();
        network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            voltage_kv: 138.0,
            ..Bus::default()
        }));
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen1".to_string(),
            bus: BusId::new(1),
            active_power_mw: 50.0,
            reactive_power_mvar: 25.0,
            voltage_setpoint_pu: Some(1.02),
            qmax_mvar: 100.0,
            qmin_mvar: -50.0,
            pmax_mw: 100.0,
            pmin_mw: 10.0,
            ..Gen::default()
        }));

        let output = export_to_psse_string(&network, "test").unwrap();
        assert!(output.contains("0 / END OF GENERATOR DATA"));
        assert!(output.contains("50.0")); // PG
        assert!(output.contains("1.02")); // VS
    }
}
