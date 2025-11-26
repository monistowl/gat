//! Arrow schema definitions for normalized power system data storage.
//!
//! This module defines the schemas for a multi-file Arrow dataset format:
//! - `system.arrow` - System-level metadata (base MVA, frequency, etc.)
//! - `buses.arrow` - Bus/node data with electrical characteristics
//! - `generators.arrow` - Generator data including cost models
//! - `loads.arrow` - Load/demand data
//! - `branches.arrow` - Lines and transformers
//!
//! The normalized format enables lossless roundtrips from MATPOWER, PandaPower,
//! and other power system formats while supporting efficient columnar access.

use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema};

/// Schema version for migration support (semver)
pub const SCHEMA_VERSION: &str = "2.0.0";

// =============================================================================
// System Schema
// =============================================================================

/// Create the system.arrow schema for system-level metadata.
///
/// This table has exactly one row containing global network parameters.
pub fn system_schema() -> Schema {
    Schema::new(vec![
        Field::new("base_mva", DataType::Float64, false),
        Field::new("base_frequency_hz", DataType::Float64, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("description", DataType::Utf8, true),
    ])
}

// =============================================================================
// Buses Schema
// =============================================================================

/// Create the buses.arrow schema for bus/node data.
///
/// Buses are the fundamental nodes in a power system graph. Each bus has
/// an ID, voltage level, and optional constraints for power flow analysis.
pub fn buses_schema() -> Schema {
    Schema::new(vec![
        // Primary key
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        // Electrical characteristics
        Field::new("voltage_kv", DataType::Float64, false),
        Field::new("voltage_pu", DataType::Float64, false),
        Field::new("angle_rad", DataType::Float64, false),
        // Bus type: PQ=1, PV=2, Slack=3, Isolated=4
        Field::new(
            "bus_type",
            DataType::Dictionary(Box::new(DataType::UInt8), Box::new(DataType::Utf8)),
            false,
        ),
        // Voltage constraints (for OPF)
        Field::new("vmin_pu", DataType::Float64, true),
        Field::new("vmax_pu", DataType::Float64, true),
        // Area/zone for multi-area analysis
        Field::new("area_id", DataType::Int64, true),
        Field::new("zone_id", DataType::Int64, true),
    ])
}

/// Valid bus types for dictionary encoding
pub const BUS_TYPES: &[&str] = &["PQ", "PV", "Slack", "Isolated"];

// =============================================================================
// Generators Schema
// =============================================================================

/// Create the generators.arrow schema for generator data.
///
/// Generators include conventional machines, renewable sources, and
/// synchronous condensers. Cost models support both piecewise linear
/// and polynomial representations for economic dispatch.
pub fn generators_schema() -> Schema {
    Schema::new(vec![
        // Primary key
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        // Foreign key to buses
        Field::new("bus", DataType::Int64, false),
        Field::new("status", DataType::Boolean, false),
        // Current operating point
        Field::new("active_power_mw", DataType::Float64, false),
        Field::new("reactive_power_mvar", DataType::Float64, false),
        // Capacity limits
        Field::new("pmin_mw", DataType::Float64, false),
        Field::new("pmax_mw", DataType::Float64, false),
        Field::new("qmin_mvar", DataType::Float64, false),
        Field::new("qmax_mvar", DataType::Float64, false),
        // Voltage control
        Field::new("voltage_setpoint_pu", DataType::Float64, true),
        Field::new("mbase_mva", DataType::Float64, true),
        // Cost model: 0=none, 1=piecewise, 2=polynomial
        Field::new("cost_model", DataType::Int32, false),
        Field::new("cost_startup", DataType::Float64, true),
        Field::new("cost_shutdown", DataType::Float64, true),
        // Cost coefficients (polynomial) or x-values (piecewise)
        Field::new(
            "cost_coeffs",
            DataType::List(Arc::new(Field::new("item", DataType::Float64, false))),
            true,
        ),
        // Piecewise y-values ($/hr at each MW point)
        Field::new(
            "cost_values",
            DataType::List(Arc::new(Field::new("item", DataType::Float64, false))),
            true,
        ),
        // Special flags
        Field::new("is_synchronous_condenser", DataType::Boolean, false),
    ])
}

/// Cost model type codes
pub const COST_MODEL_NONE: i32 = 0;
pub const COST_MODEL_PIECEWISE: i32 = 1;
pub const COST_MODEL_POLYNOMIAL: i32 = 2;

// =============================================================================
// Loads Schema
// =============================================================================

/// Create the loads.arrow schema for load/demand data.
///
/// Loads represent power consumption at buses. The simple schema covers
/// constant power loads; ZIP load models would require additional fields.
pub fn loads_schema() -> Schema {
    Schema::new(vec![
        // Primary key
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        // Foreign key to buses
        Field::new("bus", DataType::Int64, false),
        Field::new("status", DataType::Boolean, false),
        // Power demand
        Field::new("active_power_mw", DataType::Float64, false),
        Field::new("reactive_power_mvar", DataType::Float64, false),
    ])
}

// =============================================================================
// Branches Schema
// =============================================================================

/// Create the branches.arrow schema for lines and transformers.
///
/// Branches connect two buses and include transmission lines, cables,
/// and transformers. The unified schema handles both with type-specific
/// fields (tap_ratio, phase_shift for transformers).
pub fn branches_schema() -> Schema {
    Schema::new(vec![
        // Primary key
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        // Element type: line, transformer
        Field::new(
            "element_type",
            DataType::Dictionary(Box::new(DataType::UInt8), Box::new(DataType::Utf8)),
            false,
        ),
        // Foreign keys to buses
        Field::new("from_bus", DataType::Int64, false),
        Field::new("to_bus", DataType::Int64, false),
        Field::new("status", DataType::Boolean, false),
        // Impedance parameters (per-unit on system base)
        Field::new("resistance_pu", DataType::Float64, false),
        Field::new("reactance_pu", DataType::Float64, false),
        Field::new("charging_b_pu", DataType::Float64, false),
        // Transformer parameters (1.0 and 0.0 for lines)
        Field::new("tap_ratio", DataType::Float64, false),
        Field::new("phase_shift_rad", DataType::Float64, false),
        // Thermal ratings
        Field::new("rate_a_mva", DataType::Float64, true), // Normal
        Field::new("rate_b_mva", DataType::Float64, true), // Emergency
        Field::new("rate_c_mva", DataType::Float64, true), // Short-term
        // Angle limits (for OPF)
        Field::new("angle_min_rad", DataType::Float64, true),
        Field::new("angle_max_rad", DataType::Float64, true),
    ])
}

/// Valid branch element types for dictionary encoding
pub const BRANCH_TYPES: &[&str] = &["line", "transformer"];

// =============================================================================
// Schema Accessors
// =============================================================================

/// Get all table names in the normalized Arrow dataset
pub fn table_names() -> &'static [&'static str] {
    &["system", "buses", "generators", "loads", "branches"]
}

/// Get schema for a table by name
pub fn schema_for_table(name: &str) -> Option<Schema> {
    match name {
        "system" => Some(system_schema()),
        "buses" => Some(buses_schema()),
        "generators" => Some(generators_schema()),
        "loads" => Some(loads_schema()),
        "branches" => Some(branches_schema()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_schema_construction() {
        let schema = system_schema();
        assert_eq!(schema.fields().len(), 4);
        assert_eq!(schema.field(0).name(), "base_mva");
        assert!(!schema.field(0).is_nullable());
        assert!(schema.field(2).is_nullable()); // name is nullable
    }

    #[test]
    fn test_buses_schema_construction() {
        let schema = buses_schema();
        assert_eq!(schema.fields().len(), 10);

        // Check primary key
        let id_field = schema.field_with_name("id").unwrap();
        assert_eq!(id_field.data_type(), &DataType::Int64);
        assert!(!id_field.is_nullable());

        // Check dictionary type for bus_type
        let bus_type_field = schema.field_with_name("bus_type").unwrap();
        assert!(matches!(
            bus_type_field.data_type(),
            DataType::Dictionary(_, _)
        ));
    }

    #[test]
    fn test_generators_schema_construction() {
        let schema = generators_schema();
        assert_eq!(schema.fields().len(), 18);

        // Check list type for cost_coeffs
        let coeffs_field = schema.field_with_name("cost_coeffs").unwrap();
        assert!(matches!(coeffs_field.data_type(), DataType::List(_)));
        assert!(coeffs_field.is_nullable());

        // Check cost_model is non-nullable
        let cost_model_field = schema.field_with_name("cost_model").unwrap();
        assert!(!cost_model_field.is_nullable());
    }

    #[test]
    fn test_loads_schema_construction() {
        let schema = loads_schema();
        assert_eq!(schema.fields().len(), 6);

        // All fields should be non-nullable for loads
        for field in schema.fields() {
            assert!(
                !field.is_nullable(),
                "Field {} should not be nullable",
                field.name()
            );
        }
    }

    #[test]
    fn test_branches_schema_construction() {
        let schema = branches_schema();
        assert_eq!(schema.fields().len(), 16);

        // Check foreign keys
        assert!(schema.field_with_name("from_bus").is_ok());
        assert!(schema.field_with_name("to_bus").is_ok());

        // Check optional ratings
        let rate_a = schema.field_with_name("rate_a_mva").unwrap();
        assert!(rate_a.is_nullable());
    }

    #[test]
    fn test_schema_for_table() {
        assert!(schema_for_table("system").is_some());
        assert!(schema_for_table("buses").is_some());
        assert!(schema_for_table("generators").is_some());
        assert!(schema_for_table("loads").is_some());
        assert!(schema_for_table("branches").is_some());
        assert!(schema_for_table("unknown").is_none());
    }

    #[test]
    fn test_table_names() {
        let names = table_names();
        assert_eq!(names.len(), 5);
        assert!(names.contains(&"system"));
        assert!(names.contains(&"buses"));
    }

    #[test]
    fn test_schema_version() {
        // Verify version is valid semver
        let parts: Vec<&str> = SCHEMA_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3);
        for part in parts {
            assert!(part.parse::<u32>().is_ok());
        }
    }
}
