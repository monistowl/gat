//! Roundtrip serialization tests for GAT schema types
//!
//! These tests verify that all schema types can be serialized to JSON
//! and deserialized back without loss of data.

use gat_schemas::*;

#[test]
fn test_bus_schema_roundtrip() {
    let schema = BusSchema {
        voltage_kv: 138.0,
        vmin_pu: 0.95,
        vmax_pu: 1.05,
        bus_type: 1,
    };

    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: BusSchema = serde_json::from_str(&json).unwrap();

    assert_eq!(schema, roundtrip);
}

#[test]
fn test_bus_schema_default_roundtrip() {
    let schema = BusSchema::default();
    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: BusSchema = serde_json::from_str(&json).unwrap();
    assert_eq!(schema, roundtrip);
}

#[test]
fn test_gen_schema_roundtrip() {
    let schema = GenSchema {
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_c0: 0.0,
        cost_c1: 10.0,
        cost_c2: 0.01,
    };

    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: GenSchema = serde_json::from_str(&json).unwrap();

    assert_eq!(schema, roundtrip);
}

#[test]
fn test_gen_schema_default_roundtrip() {
    let schema = GenSchema::default();
    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: GenSchema = serde_json::from_str(&json).unwrap();
    assert_eq!(schema, roundtrip);
}

#[test]
fn test_branch_schema_roundtrip() {
    let schema = BranchSchema {
        resistance_pu: 0.01,
        reactance_pu: 0.1,
        charging_b_pu: 0.02,
        rating_mva: 100.0,
        tap_ratio: 1.0,
        phase_shift_rad: 0.0,
    };

    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: BranchSchema = serde_json::from_str(&json).unwrap();

    assert_eq!(schema, roundtrip);
}

#[test]
fn test_branch_schema_default_roundtrip() {
    let schema = BranchSchema::default();
    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: BranchSchema = serde_json::from_str(&json).unwrap();
    assert_eq!(schema, roundtrip);
}

#[test]
fn test_load_schema_roundtrip() {
    let schema = LoadSchema {
        active_power_mw: 50.0,
        reactive_power_mvar: 10.0,
    };

    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: LoadSchema = serde_json::from_str(&json).unwrap();

    assert_eq!(schema, roundtrip);
}

#[test]
fn test_load_schema_default_roundtrip() {
    let schema = LoadSchema::default();
    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: LoadSchema = serde_json::from_str(&json).unwrap();
    assert_eq!(schema, roundtrip);
}

#[test]
fn test_bus_schema_json_format() {
    let schema = BusSchema {
        voltage_kv: 138.0,
        vmin_pu: 0.95,
        vmax_pu: 1.05,
        bus_type: 1,
    };

    let json = serde_json::to_string_pretty(&schema).unwrap();

    // Verify JSON contains expected fields
    assert!(json.contains("voltage_kv"));
    assert!(json.contains("138.0"));
    assert!(json.contains("vmin_pu"));
    assert!(json.contains("0.95"));
}

#[test]
fn test_gen_schema_json_format() {
    let schema = GenSchema {
        pmin_mw: 0.0,
        pmax_mw: 100.0,
        qmin_mvar: -50.0,
        qmax_mvar: 50.0,
        cost_c0: 0.0,
        cost_c1: 10.0,
        cost_c2: 0.01,
    };

    let json = serde_json::to_string_pretty(&schema).unwrap();

    // Verify JSON contains expected fields
    assert!(json.contains("pmin_mw"));
    assert!(json.contains("pmax_mw"));
    assert!(json.contains("100.0"));
    assert!(json.contains("cost_c1"));
}

#[test]
fn test_branch_schema_with_transformer_settings() {
    let schema = BranchSchema {
        resistance_pu: 0.005,
        reactance_pu: 0.05,
        charging_b_pu: 0.0,
        rating_mva: 200.0,
        tap_ratio: 1.05,
        phase_shift_rad: 0.1,
    };

    let json = serde_json::to_string(&schema).unwrap();
    let roundtrip: BranchSchema = serde_json::from_str(&json).unwrap();

    assert_eq!(schema, roundtrip);
    assert_eq!(roundtrip.tap_ratio, 1.05);
    assert_eq!(roundtrip.phase_shift_rad, 0.1);
}
