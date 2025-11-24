use super::*;
use gat_core::Node;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;
use zip::write::FileOptions;
use zip::ZipWriter;

#[test]
fn import_matpower_case_sample() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root should exist");
    let case_path = repo_root.join("test_data/matpower/ieee14.case");
    assert!(case_path.exists());

    let temp_dir = tempdir().expect("tmp dir");
    let output_path = temp_dir.path().join("grid.arrow");

    let network = import_matpower_case(case_path.to_str().unwrap(), output_path.to_str().unwrap())
        .expect("import should succeed");

    assert!(output_path.exists(), "arrow output file created");

    let loaded_network = load_grid_from_arrow(output_path.to_str().unwrap())
        .expect("loading arrow dataset should succeed");

    assert_eq!(
        loaded_network.graph.node_count(),
        network.graph.node_count()
    );
    assert_eq!(
        loaded_network.graph.edge_count(),
        network.graph.edge_count()
    );

    assert_eq!(network.graph.edge_count(), 20);
    let bus_count = network
        .graph
        .node_indices()
        .filter(|idx| matches!(network.graph[*idx], Node::Bus(_)))
        .count();
    assert_eq!(bus_count, 14);
    let generator_count = network
        .graph
        .node_indices()
        .filter(|idx| matches!(network.graph[*idx], Node::Gen(_)))
        .count();
    assert_eq!(generator_count, 5);
    let load_count = network
        .graph
        .node_indices()
        .filter(|idx| matches!(network.graph[*idx], Node::Load(_)))
        .count();
    assert!(load_count >= 1);
}

#[test]
fn import_psse_raw_sample() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root should exist");
    let raw_path = repo_root.join("test_data/psse/sample.raw");
    assert!(raw_path.exists());

    let temp_dir = tempdir().expect("tmp dir");
    let output_path = temp_dir.path().join("psse.arrow");

    let network = import_psse_raw(raw_path.to_str().unwrap(), output_path.to_str().unwrap())
        .expect("import should succeed");

    assert!(output_path.exists(), "arrow output file created");

    let loaded_network = load_grid_from_arrow(output_path.to_str().unwrap())
        .expect("loading arrow dataset should succeed");

    assert_eq!(
        loaded_network.graph.node_count(),
        network.graph.node_count()
    );
    assert_eq!(
        loaded_network.graph.edge_count(),
        network.graph.edge_count()
    );
    assert_eq!(network.graph.node_count(), 2);
    assert_eq!(network.graph.edge_count(), 1);
}

#[test]
fn import_cim_rdf_sample() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root should exist");
    let cim_path = repo_root.join("test_data/cim/simple.rdf");
    assert!(cim_path.exists());

    let temp_dir = tempdir().expect("tmp dir");
    let output_path = temp_dir.path().join("cim.arrow");

    let network = import_cim_rdf(cim_path.to_str().unwrap(), output_path.to_str().unwrap())
        .expect("import should succeed");

    assert!(output_path.exists(), "arrow output file created");

    let loaded_network = load_grid_from_arrow(output_path.to_str().unwrap())
        .expect("loading arrow dataset should succeed");

    assert_eq!(
        loaded_network.graph.node_count(),
        network.graph.node_count()
    );
    assert_eq!(
        loaded_network.graph.edge_count(),
        network.graph.edge_count()
    );
}

#[test]
fn import_cim_rdf_zip_sample() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root should exist");
    let cim_path = repo_root.join("test_data/cim/simple.rdf");
    assert!(cim_path.exists());

    let temp_dir = tempdir().expect("tmp dir");
    let zip_path = temp_dir.path().join("cim.zip");
    let mut zip_file = File::create(&zip_path).expect("zip file");
    let mut writer = ZipWriter::new(&mut zip_file);
    writer
        .start_file(
            "network.rdf",
            FileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .expect("start file");
    let contents = std::fs::read_to_string(cim_path).expect("read sample");
    writer
        .write_all(contents.as_bytes())
        .expect("write contents");
    writer.finish().expect("finish zip");

    let output_path = temp_dir.path().join("cim_zip.arrow");
    let network = import_cim_rdf(zip_path.to_str().unwrap(), output_path.to_str().unwrap())
        .expect("import should succeed");

    assert!(output_path.exists(), "arrow output file created");
    assert_eq!(network.graph.node_count(), 2);
    assert_eq!(network.graph.edge_count(), 1);
}

#[test]
fn test_cim_operational_limits_parsing() {
    use crate::importers::cim::parse_cim_documents;

    // Create a minimal CIM RDF with bus only (no limits for now)
    let cim_xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8"?>"#,
        "\n",
        r#"<rdf:RDF xmlns:cim="http://iec.ch/TC57/2013/CIM-schema-v2_4_0" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">"#,
        "\n",
        r#"  <cim:BusbarSection rdf:ID="bus1">"#,
        "\n",
        r#"    <cim:IdentifiedObject.name>Bus 1</cim:IdentifiedObject.name>"#,
        "\n",
        r#"  </cim:BusbarSection>"#,
        "\n",
        r#"</rdf:RDF>"#
    );

    // Parse and verify new return signature works
    let documents = vec![cim_xml.to_string()];
    let result = parse_cim_documents(&documents);

    assert!(result.is_ok());
    let (buses, _, _, _, limits, volt_limits, transformers) = result.unwrap();

    // Verify we got the bus
    assert_eq!(buses.len(), 1);

    // Verify the new limit types are returned correctly (empty for now)
    assert!(limits.is_empty());
    assert!(volt_limits.is_empty());
    assert!(transformers.is_empty());
}

#[test]
fn test_cim_validation_empty_network() {
    use crate::importers::validate_network_from_cim;
    use gat_core::Network;

    let network = Network::new();

    // Validation should fail for empty network
    let result = validate_network_from_cim(&network);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("no buses"));
}

#[test]
fn test_cim_validation_invalid_voltage() {
    use crate::importers::validate_network_from_cim;
    use gat_core::{Bus, BusId, Network, Node};

    let mut network = Network::new();
    let bus = Bus {
        id: BusId::new(1),
        name: "bus1".to_string(),
        voltage_kv: -10.0, // Invalid negative voltage
    };
    network.graph.add_node(Node::Bus(bus));

    // Validation should fail
    let result = validate_network_from_cim(&network);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("voltage"));
}

#[test]
fn test_cim_validation_passes() {
    use crate::importers::validate_network_from_cim;
    use gat_core::{Branch, BranchId, Bus, BusId, Edge, Network, Node};

    let mut network = Network::new();
    let bus1 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus1".to_string(),
        voltage_kv: 138.0,
    }));
    let bus2 = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(2),
        name: "bus2".to_string(),
        voltage_kv: 138.0,
    }));
    network.graph.add_edge(
        bus1,
        bus2,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "br1".to_string(),
            from_bus: BusId::new(1),
            to_bus: BusId::new(2),
            resistance: 0.01,
            reactance: 0.1,
        }),
    );

    let result = validate_network_from_cim(&network);
    assert!(result.is_ok());
}

#[test]
fn test_cim_validation_warnings() {
    use crate::importers::validate_cim_with_warnings;
    use gat_core::{Bus, BusId, Network, Node};

    let mut network = Network::new();
    let bus = Bus {
        id: BusId::new(1),
        name: "bus1".to_string(),
        voltage_kv: 1500.0, // Unusual high voltage, should generate warning
    };
    network.graph.add_node(Node::Bus(bus));

    let warnings = validate_cim_with_warnings(&network);
    assert!(!warnings.is_empty());
    assert!(warnings[0].issue.contains("Unusual voltage"));
}
