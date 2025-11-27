//! Round-trip tests for format exporters
//!
//! These tests verify that we can:
//! 1. Import a format → Network
//! 2. Export Network → Arrow
//! 3. Load Arrow → Network
//! 4. Export Network → original format
//! 5. Re-import → Network
//! 6. Verify the networks are equivalent

#[cfg(test)]
mod tests {
    use crate::arrow_manifest::SourceInfo;
    use crate::exporters::formats::{
        export_network_to_cim, export_network_to_matpower, export_network_to_pandapower,
        export_network_to_powermodels, export_network_to_powermodels_string,
        export_network_to_psse,
    };
    use crate::exporters::{ArrowDirectoryWriter, ExportMetadata};
    use crate::importers::parse_powermodels_string;
    use crate::importers::{load_grid_from_arrow, parse_matpower};
    use anyhow::Result;
    use chrono::{TimeZone, Utc};
    use gat_core::{
        Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
    };
    use serde_json::Value;
    use std::{fs, io::Read, path::Path};
    use tempfile::TempDir;

    /// Test round-trip: MATPOWER → Arrow → MATPOWER → Arrow
    #[test]
    fn test_matpower_roundtrip() -> Result<()> {
        // Skip if test file doesn't exist
        let test_file = Path::new("../../test_data/matpower/ieee14.case");
        if !test_file.exists() {
            eprintln!("Skipping test: {} not found", test_file.display());
            return Ok(());
        }

        let temp_dir = TempDir::new()?;

        // Step 1: Import MATPOWER → Network
        let result1 = parse_matpower(test_file.to_str().unwrap())?;
        let network1 = result1.network;
        let stats1 = network1.stats();

        // Step 2: Export Network → Arrow
        let arrow_dir1 = temp_dir.path().join("step1.arrow");
        let writer = ArrowDirectoryWriter::new(&arrow_dir1)?;
        writer.write_network(&network1, None, None)?;

        // Step 3: Load Arrow → Network
        let network2 = load_grid_from_arrow(&arrow_dir1)?;
        let stats2 = network2.stats();

        // Verify stats match after Arrow round-trip
        assert_eq!(
            stats1.num_buses, stats2.num_buses,
            "Bus count mismatch after Arrow round-trip"
        );
        assert_eq!(
            stats1.num_gens, stats2.num_gens,
            "Generator count mismatch after Arrow round-trip"
        );
        assert_eq!(
            stats1.num_loads, stats2.num_loads,
            "Load count mismatch after Arrow round-trip"
        );
        assert_eq!(
            stats1.num_branches, stats2.num_branches,
            "Branch count mismatch after Arrow round-trip"
        );

        // Step 4: Export Network → MATPOWER
        let matpower_file = temp_dir.path().join("exported.m");
        export_network_to_matpower(&network2, &matpower_file, None)?;

        // Verify the file was created and has content
        assert!(
            matpower_file.exists(),
            "Exported MATPOWER file should exist"
        );
        let content = std::fs::read_to_string(&matpower_file)?;
        assert!(content.contains("mpc.baseMVA"), "Should contain baseMVA");
        assert!(content.contains("mpc.bus"), "Should contain bus data");
        assert!(content.contains("mpc.gen"), "Should contain generator data");
        assert!(content.contains("mpc.branch"), "Should contain branch data");

        // Step 5: Re-import MATPOWER → Network
        let result3 = parse_matpower(matpower_file.to_str().unwrap())?;
        let network3 = result3.network;
        let stats3 = network3.stats();

        // Verify stats match after full round-trip
        assert_eq!(
            stats1.num_buses, stats3.num_buses,
            "Bus count mismatch after full round-trip"
        );
        assert_eq!(
            stats1.num_gens, stats3.num_gens,
            "Generator count mismatch after full round-trip"
        );
        assert_eq!(
            stats1.num_loads, stats3.num_loads,
            "Load count mismatch after full round-trip"
        );
        assert_eq!(
            stats1.num_branches, stats3.num_branches,
            "Branch count mismatch after full round-trip"
        );

        // Verify total load is preserved (within tolerance)
        let load_diff = (stats1.total_load_mw - stats3.total_load_mw).abs();
        assert!(
            load_diff < 0.01,
            "Total load should be preserved (diff: {} MW)",
            load_diff
        );

        // Verify total generation capacity is preserved (within tolerance)
        let gen_diff = (stats1.total_gen_capacity_mw - stats3.total_gen_capacity_mw).abs();
        assert!(
            gen_diff < 0.01,
            "Total generation capacity should be preserved (diff: {} MW)",
            gen_diff
        );

        Ok(())
    }

    /// Test that export creates valid MATPOWER syntax
    #[test]
    fn test_matpower_export_syntax() -> Result<()> {
        let mut network = Network::new();

        // Create a minimal 2-bus system
        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            vmin_pu: Some(0.95),
            vmax_pu: Some(1.05),
            area_id: Some(1),
            zone_id: Some(1),
        }));

        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            vmin_pu: Some(0.95),
            vmax_pu: Some(1.05),
            area_id: Some(1),
            zone_id: Some(1),
        }));

        // Add generator with quadratic cost
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen 1".to_string(),
            bus: BusId::new(1),
            active_power_mw: 100.0,
            reactive_power_mvar: 50.0,
            pmin_mw: 0.0,
            pmax_mw: 200.0,
            qmin_mvar: -50.0,
            qmax_mvar: 100.0,
            status: true,
            voltage_setpoint_pu: Some(1.05),
            cost_model: CostModel::quadratic(100.0, 20.0, 0.01),
            ..Gen::default()
        }));

        // Add load
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load 1".to_string(),
            bus: BusId::new(2),
            active_power_mw: 80.0,
            reactive_power_mvar: 40.0,
        }));

        // Add branch
        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Line 1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                charging_b_pu: 0.02,
                tap_ratio: 1.0,
                phase_shift_rad: 0.0,
                status: true,
                rating_a_mva: Some(250.0),
                ..Branch::default()
            }),
        );

        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("test.m");

        // Export to MATPOWER
        export_network_to_matpower(&network, &output_file, None)?;

        // Read and verify content
        let content = std::fs::read_to_string(&output_file)?;

        // Check for required sections
        assert!(
            content.contains("function mpc = case"),
            "Should have function header"
        );
        assert!(content.contains("mpc.version"), "Should have version");
        assert!(content.contains("mpc.baseMVA"), "Should have baseMVA");
        assert!(content.contains("mpc.bus = ["), "Should have bus matrix");
        assert!(content.contains("mpc.gen = ["), "Should have gen matrix");
        assert!(
            content.contains("mpc.branch = ["),
            "Should have branch matrix"
        );
        assert!(
            content.contains("mpc.gencost = ["),
            "Should have gencost matrix"
        );

        // Verify bus data format (should have 13 columns)
        let bus_section: Vec<&str> = content
            .lines()
            .skip_while(|l| !l.contains("mpc.bus = ["))
            .skip(1)
            .take_while(|l| !l.contains("];"))
            .collect();
        assert!(!bus_section.is_empty(), "Should have bus data rows");

        // Verify generator data format (should have 10 columns)
        let gen_section: Vec<&str> = content
            .lines()
            .skip_while(|l| !l.contains("mpc.gen = ["))
            .skip(1)
            .take_while(|l| !l.contains("];"))
            .collect();
        assert!(!gen_section.is_empty(), "Should have generator data rows");

        // Verify cost data is present
        let gencost_section: Vec<&str> = content
            .lines()
            .skip_while(|l| !l.contains("mpc.gencost = ["))
            .skip(1)
            .take_while(|l| !l.contains("];"))
            .collect();
        assert!(!gencost_section.is_empty(), "Should have gencost data rows");

        Ok(())
    }

    #[test]
    fn test_matpower_export_metadata() -> Result<()> {
        let network = build_sample_network();
        let metadata = sample_metadata();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("meta.m");

        export_network_to_matpower(&network, &output_file, Some(&metadata))?;
        let content = fs::read_to_string(&output_file)?;

        assert!(content.contains("%% Source: case14.raw (psse) hash deadbeef"));
        assert!(content.contains("%% Generated by GAT 0.5.0"));
        Ok(())
    }

    #[test]
    fn test_matpower_export_without_metadata() -> Result<()> {
        let network = build_sample_network();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("meta_none.m");

        export_network_to_matpower(&network, &output_file, None)?;
        let content = fs::read_to_string(&output_file)?;

        assert!(!content.contains("%% Source:"));
        assert!(!content.contains("%% Arrow dataset created at"));
        assert!(!content.contains("%% Generated by GAT 0.5.0"));
        Ok(())
    }

    fn build_sample_network() -> Network {
        let mut network = Network::new();
        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus 1".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            ..Default::default()
        }));

        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus 2".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            ..Default::default()
        }));

        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Export Gen".to_string(),
            bus: BusId::new(1),
            active_power_mw: 100.0,
            reactive_power_mvar: 30.0,
            pmin_mw: 0.0,
            pmax_mw: 120.0,
            qmin_mvar: -40.0,
            qmax_mvar: 80.0,
            status: true,
            cost_model: CostModel::NoCost,
            ..Gen::default()
        }));

        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Export Load".to_string(),
            bus: BusId::new(2),
            active_power_mw: 85.0,
            reactive_power_mvar: 35.0,
        }));

        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Interconnect".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                charging_b_pu: 0.02,
                status: true,
                rating_a_mva: Some(200.0),
                ..Branch::default()
            }),
        );

        network
    }

    fn sample_metadata() -> ExportMetadata {
        ExportMetadata {
            source: Some(SourceInfo {
                file: "case14.raw".to_string(),
                format: "psse".to_string(),
                file_hash: "deadbeef".to_string(),
            }),
            created_at: Some(Utc.with_ymd_and_hms(2025, 11, 26, 0, 0, 0).unwrap()),
            gat_version: Some("0.5.0".to_string()),
        }
    }

    #[test]
    fn test_psse_export_sections() -> Result<()> {
        let network = build_sample_network();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("psse.raw");
        let metadata = sample_metadata();
        export_network_to_psse(&network, &output_file, Some(&metadata))?;
        let mut content = String::new();
        fs::File::open(&output_file)?.read_to_string(&mut content)?;
        assert!(content.contains("BUS DATA FOLLOWS"));
        assert!(content.contains("GENERATOR DATA FOLLOWS"));
        assert!(content.contains("LOAD DATA FOLLOWS"));
        assert!(content.contains("BRANCH DATA FOLLOWS"));
        assert!(content.contains("1,'Bus 1'"));
        assert!(content.contains("Source: case14.raw (psse) hash deadbeef"));
        assert!(content.contains("GAT version 0.5.0"));
        Ok(())
    }

    #[test]
    fn test_psse_export_without_metadata() -> Result<()> {
        let network = build_sample_network();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("psse_plain.raw");

        export_network_to_psse(&network, &output_file, None)?;
        let content = fs::read_to_string(&output_file)?;

        assert!(content.contains("BUS DATA FOLLOWS"));
        assert!(!content.contains("Source:"));
        assert!(!content.contains("GAT version"));
        Ok(())
    }

    #[test]
    fn test_cim_export_contains_components() -> Result<()> {
        let network = build_sample_network();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("network.rdf");
        let metadata = sample_metadata();
        export_network_to_cim(&network, &output_file, Some(&metadata))?;
        let content = fs::read_to_string(&output_file)?;
        assert!(content.contains("<cim:BusbarSection"));
        assert!(content.contains("<cim:ACLineSegment"));
        assert!(content.contains("<cim:Load"));
        assert!(content.contains("<cim:SynchronousMachine"));
        assert!(content.contains("<!-- Source: case14.raw (psse) hash deadbeef -->"));
        Ok(())
    }

    #[test]
    fn test_cim_export_without_metadata() -> Result<()> {
        let network = build_sample_network();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("network_plain.rdf");

        export_network_to_cim(&network, &output_file, None)?;
        let content = fs::read_to_string(&output_file)?;

        assert!(content.contains("<cim:BusbarSection"));
        assert!(!content.contains("<!-- Source:"));
        Ok(())
    }

    #[test]
    fn test_pandapower_export_structure() -> Result<()> {
        let network = build_sample_network();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("network.json");
        let metadata = sample_metadata();
        export_network_to_pandapower(&network, &output_file, Some(&metadata))?;
        let content = fs::read_to_string(&output_file)?;
        let parsed: Value = serde_json::from_str(&content)?;
        let object = parsed
            .get("_object")
            .and_then(Value::as_object)
            .expect("missing _object");
        assert_eq!(
            object
                .get("bus")
                .and_then(|v| v.get("_module"))
                .and_then(Value::as_str),
            Some("pandas.core.frame")
        );
        assert!(object.get("line").is_some());
        assert!(object.get("gen").is_some());
        let meta = parsed
            .get("_meta")
            .and_then(Value::as_object)
            .expect("missing _meta");
        let source = meta
            .get("source")
            .and_then(Value::as_object)
            .expect("missing meta.source");
        assert_eq!(
            source.get("file").and_then(Value::as_str),
            Some("case14.raw")
        );
        assert_eq!(source.get("format").and_then(Value::as_str), Some("psse"));
        assert_eq!(source.get("hash").and_then(Value::as_str), Some("deadbeef"));
        Ok(())
    }

    #[test]
    fn test_pandapower_export_without_metadata() -> Result<()> {
        let network = build_sample_network();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("network_plain.json");

        export_network_to_pandapower(&network, &output_file, None)?;
        let content = fs::read_to_string(&output_file)?;
        let parsed: Value = serde_json::from_str(&content)?;

        assert!(parsed.get("_meta").is_none());
        Ok(())
    }

    // =========================================================================
    // PowerModels Export Tests
    // =========================================================================

    #[test]
    fn test_powermodels_export_structure() -> Result<()> {
        let network = build_sample_network();
        let json_str = export_network_to_powermodels_string(&network, None)?;
        let parsed: Value = serde_json::from_str(&json_str)?;

        // Verify required top-level fields
        assert!(parsed.get("baseMVA").is_some(), "Should have baseMVA");
        assert!(parsed.get("per_unit").is_some(), "Should have per_unit");
        assert!(parsed.get("bus").is_some(), "Should have bus dictionary");
        assert!(parsed.get("gen").is_some(), "Should have gen dictionary");
        assert!(parsed.get("load").is_some(), "Should have load dictionary");
        assert!(
            parsed.get("branch").is_some(),
            "Should have branch dictionary"
        );

        // Verify dictionary structure (keys are string indices)
        let bus = parsed.get("bus").unwrap().as_object().unwrap();
        assert!(!bus.is_empty(), "Bus dictionary should not be empty");

        // Verify bus structure
        for (_, bus_data) in bus.iter() {
            assert!(bus_data.get("index").is_some());
            assert!(bus_data.get("bus_type").is_some());
            assert!(bus_data.get("vm").is_some());
            assert!(bus_data.get("va").is_some());
        }

        Ok(())
    }

    #[test]
    fn test_powermodels_export_with_metadata() -> Result<()> {
        let network = build_sample_network();
        let metadata = sample_metadata();
        let json_str = export_network_to_powermodels_string(&network, Some(&metadata))?;
        let parsed: Value = serde_json::from_str(&json_str)?;

        // Verify metadata is included
        let meta = parsed.get("_meta").and_then(Value::as_object);
        assert!(meta.is_some(), "Should have _meta section");

        let meta = meta.unwrap();
        let source = meta.get("source").and_then(Value::as_object).unwrap();
        assert_eq!(
            source.get("file").and_then(Value::as_str),
            Some("case14.raw")
        );
        assert_eq!(source.get("format").and_then(Value::as_str), Some("psse"));
        assert_eq!(source.get("hash").and_then(Value::as_str), Some("deadbeef"));

        Ok(())
    }

    #[test]
    fn test_powermodels_export_without_metadata() -> Result<()> {
        let network = build_sample_network();
        let json_str = export_network_to_powermodels_string(&network, None)?;
        let parsed: Value = serde_json::from_str(&json_str)?;

        assert!(
            parsed.get("_meta").is_none(),
            "Should not have _meta section"
        );
        Ok(())
    }

    #[test]
    fn test_powermodels_export_file() -> Result<()> {
        let network = build_sample_network();
        let temp_dir = TempDir::new()?;
        let output_file = temp_dir.path().join("network.json");

        export_network_to_powermodels(&network, &output_file, None)?;

        assert!(output_file.exists(), "Output file should exist");
        let content = fs::read_to_string(&output_file)?;
        let parsed: Value = serde_json::from_str(&content)?;
        assert!(parsed.get("baseMVA").is_some());

        Ok(())
    }

    #[test]
    fn test_powermodels_roundtrip_counts() -> Result<()> {
        // Build a more complete network
        let original = build_network_with_cost();
        let orig_stats = original.stats();

        // Export to PowerModels JSON
        let json_str = export_network_to_powermodels_string(&original, None)?;

        // Import back
        let result = parse_powermodels_string(&json_str)?;
        let imported = result.network;
        let imp_stats = imported.stats();

        // Verify counts match
        assert_eq!(
            orig_stats.num_buses, imp_stats.num_buses,
            "Bus count mismatch"
        );
        assert_eq!(
            orig_stats.num_gens, imp_stats.num_gens,
            "Generator count mismatch"
        );
        assert_eq!(
            orig_stats.num_loads, imp_stats.num_loads,
            "Load count mismatch"
        );
        assert_eq!(
            orig_stats.num_branches, imp_stats.num_branches,
            "Branch count mismatch"
        );

        Ok(())
    }

    #[test]
    fn test_powermodels_roundtrip_values() -> Result<()> {
        let original = build_network_with_cost();

        // Export to PowerModels JSON
        let json_str = export_network_to_powermodels_string(&original, None)?;

        // Import back
        let result = parse_powermodels_string(&json_str)?;
        let imported = result.network;

        // Verify total load is preserved
        let orig_stats = original.stats();
        let imp_stats = imported.stats();
        let load_diff = (orig_stats.total_load_mw - imp_stats.total_load_mw).abs();
        assert!(
            load_diff < 0.01,
            "Load should be preserved (diff: {} MW)",
            load_diff
        );

        // Verify total generation capacity
        let gen_diff = (orig_stats.total_gen_capacity_mw - imp_stats.total_gen_capacity_mw).abs();
        assert!(
            gen_diff < 0.01,
            "Generation capacity should be preserved (diff: {} MW)",
            gen_diff
        );

        Ok(())
    }

    #[test]
    fn test_powermodels_roundtrip_cost_models() -> Result<()> {
        let original = build_network_with_cost();

        // Export to PowerModels JSON
        let json_str = export_network_to_powermodels_string(&original, None)?;

        // Import back
        let result = parse_powermodels_string(&json_str)?;
        let imported = result.network;

        // Find generators and verify cost models
        let orig_gens: Vec<_> = original
            .graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Gen(g) => Some(g),
                _ => None,
            })
            .collect();

        let imp_gens: Vec<_> = imported
            .graph
            .node_weights()
            .filter_map(|n| match n {
                Node::Gen(g) => Some(g),
                _ => None,
            })
            .collect();

        assert_eq!(orig_gens.len(), imp_gens.len());

        // Find gen by id and compare cost model
        for orig_gen in &orig_gens {
            let imp_gen = imp_gens.iter().find(|g| g.id == orig_gen.id);
            assert!(
                imp_gen.is_some(),
                "Gen {} not found after roundtrip",
                orig_gen.id.value()
            );

            let imp_gen = imp_gen.unwrap();
            match (&orig_gen.cost_model, &imp_gen.cost_model) {
                (CostModel::Polynomial(orig), CostModel::Polynomial(imp)) => {
                    assert_eq!(
                        orig.len(),
                        imp.len(),
                        "Polynomial coefficient count mismatch"
                    );
                    for (i, (o, im)) in orig.iter().zip(imp.iter()).enumerate() {
                        assert!(
                            (o - im).abs() < 1e-6,
                            "Coefficient {} mismatch: {} vs {}",
                            i,
                            o,
                            im
                        );
                    }
                }
                (CostModel::NoCost, CostModel::NoCost) => {}
                (CostModel::NoCost, CostModel::Polynomial(coeffs)) if coeffs.is_empty() => {}
                (o, i) => panic!("Cost model type mismatch: {:?} vs {:?}", o, i),
            }
        }

        Ok(())
    }

    #[test]
    fn test_powermodels_pglib_case14_roundtrip() -> Result<()> {
        // Load PGLib case14 from MATPOWER
        let pglib_path = Path::new("../../test_data/matpower/pglib/pglib_opf_case14_ieee.m");
        if !pglib_path.exists() {
            eprintln!("Skipping: {} not found", pglib_path.display());
            return Ok(());
        }

        // Import MATPOWER
        let mat_result = parse_matpower(pglib_path.to_str().unwrap())?;
        let original = mat_result.network;
        let orig_stats = original.stats();

        // Export to PowerModels
        let json_str = export_network_to_powermodels_string(&original, None)?;

        // Import back from PowerModels
        let pm_result = parse_powermodels_string(&json_str)?;
        let imported = pm_result.network;
        let imp_stats = imported.stats();

        // Verify structure preserved
        assert_eq!(orig_stats.num_buses, imp_stats.num_buses);
        assert_eq!(orig_stats.num_gens, imp_stats.num_gens);
        assert_eq!(orig_stats.num_loads, imp_stats.num_loads);
        assert_eq!(orig_stats.num_branches, imp_stats.num_branches);

        // Verify power values preserved
        let load_diff = (orig_stats.total_load_mw - imp_stats.total_load_mw).abs();
        assert!(load_diff < 0.1, "Load mismatch: {} MW", load_diff);

        let gen_diff = (orig_stats.total_gen_capacity_mw - imp_stats.total_gen_capacity_mw).abs();
        assert!(gen_diff < 0.1, "Gen capacity mismatch: {} MW", gen_diff);

        Ok(())
    }

    #[test]
    fn test_powermodels_matpower_to_powermodels_to_matpower() -> Result<()> {
        // Full roundtrip: MATPOWER → PowerModels → MATPOWER
        let pglib_path = Path::new("../../test_data/matpower/pglib/pglib_opf_case14_ieee.m");
        if !pglib_path.exists() {
            eprintln!("Skipping: {} not found", pglib_path.display());
            return Ok(());
        }

        let temp_dir = TempDir::new()?;

        // Step 1: Import MATPOWER
        let mat_result = parse_matpower(pglib_path.to_str().unwrap())?;
        let network1 = mat_result.network;
        let stats1 = network1.stats();

        // Step 2: Export to PowerModels
        let pm_file = temp_dir.path().join("case14.json");
        export_network_to_powermodels(&network1, &pm_file, None)?;

        // Step 3: Import PowerModels
        let pm_content = fs::read_to_string(&pm_file)?;
        let pm_result = parse_powermodels_string(&pm_content)?;
        let network2 = pm_result.network;
        let stats2 = network2.stats();

        // Step 4: Export back to MATPOWER
        let mat_file = temp_dir.path().join("case14_roundtrip.m");
        export_network_to_matpower(&network2, &mat_file, None)?;

        // Step 5: Import MATPOWER again
        let mat_result3 = parse_matpower(mat_file.to_str().unwrap())?;
        let network3 = mat_result3.network;
        let stats3 = network3.stats();

        // Verify counts preserved through entire chain
        assert_eq!(stats1.num_buses, stats2.num_buses);
        assert_eq!(stats2.num_buses, stats3.num_buses);

        assert_eq!(stats1.num_gens, stats2.num_gens);
        assert_eq!(stats2.num_gens, stats3.num_gens);

        assert_eq!(stats1.num_loads, stats2.num_loads);
        assert_eq!(stats2.num_loads, stats3.num_loads);

        assert_eq!(stats1.num_branches, stats2.num_branches);
        assert_eq!(stats2.num_branches, stats3.num_branches);

        // Verify power preserved through chain
        let load_diff = (stats1.total_load_mw - stats3.total_load_mw).abs();
        assert!(
            load_diff < 0.1,
            "Load should be preserved: {} MW diff",
            load_diff
        );

        Ok(())
    }

    /// Build a network with cost models for testing
    fn build_network_with_cost() -> Network {
        let mut network = Network::new();

        let bus1_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Slack Bus".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            vmin_pu: Some(0.95),
            vmax_pu: Some(1.05),
            area_id: Some(1),
            zone_id: Some(1),
        }));

        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Load Bus".to_string(),
            voltage_kv: 138.0,
            voltage_pu: 1.0,
            angle_rad: 0.0,
            vmin_pu: Some(0.95),
            vmax_pu: Some(1.05),
            area_id: Some(1),
            zone_id: Some(1),
        }));

        // Generator with polynomial cost
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen 1".to_string(),
            bus: BusId::new(1),
            active_power_mw: 100.0,
            reactive_power_mvar: 50.0,
            pmin_mw: 10.0,
            pmax_mw: 200.0,
            qmin_mvar: -100.0,
            qmax_mvar: 100.0,
            status: true,
            voltage_setpoint_pu: Some(1.0),
            mbase_mva: Some(100.0),
            cost_model: CostModel::Polynomial(vec![100.0, 20.0, 0.05]), // c0 + c1*P + c2*P^2
            ..Gen::default()
        }));

        // Load
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(1),
            name: "Load 1".to_string(),
            bus: BusId::new(2),
            active_power_mw: 90.0,
            reactive_power_mvar: 40.0,
        }));

        // Branch
        network.graph.add_edge(
            bus1_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                name: "Line 1-2".to_string(),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                charging_b_pu: 0.02,
                tap_ratio: 1.0,
                phase_shift_rad: 0.0,
                status: true,
                rating_a_mva: Some(100.0),
                element_type: "line".to_string(),
                ..Branch::default()
            }),
        );

        network
    }
}
