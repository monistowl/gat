//! Arrow IPC export for OPF results
//!
//! This module provides functions to serialize OPF results into Arrow IPC format,
//! enabling zero-copy data interchange between WASM and JavaScript.
//!
//! The resulting bytes can be loaded directly by:
//! - Apache Arrow JS (`@apache-arrow/ts`)
//! - DuckDB-WASM (for SQL queries on results)
//! - Arquero (for DataFrame transforms)

use arrow::array::{ArrayRef, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use std::collections::HashMap;
use std::sync::Arc;

/// Create an Arrow IPC stream containing generator dispatch results
///
/// Schema: gen_id (string), p_mw (float64), q_mvar (float64)
pub fn generators_to_arrow(
    generator_p: &HashMap<String, f64>,
    generator_q: &HashMap<String, f64>,
) -> Result<Vec<u8>, String> {
    let schema = Schema::new(vec![
        Field::new("gen_id", DataType::Utf8, false),
        Field::new("p_mw", DataType::Float64, false),
        Field::new("q_mvar", DataType::Float64, true),
    ]);

    // Collect in consistent order
    let mut gen_ids: Vec<&String> = generator_p.keys().collect();
    gen_ids.sort();

    let gen_id_array: StringArray = gen_ids.iter().map(|s| Some(s.as_str())).collect();
    let p_array: Float64Array = gen_ids
        .iter()
        .map(|id| generator_p.get(*id).copied())
        .collect();
    let q_array: Float64Array = gen_ids
        .iter()
        .map(|id| generator_q.get(*id).copied())
        .collect();

    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(gen_id_array) as ArrayRef,
            Arc::new(p_array) as ArrayRef,
            Arc::new(q_array) as ArrayRef,
        ],
    )
    .map_err(|e| format!("Failed to create RecordBatch: {}", e))?;

    let mut buf = Vec::new();
    {
        let mut writer =
            StreamWriter::try_new(&mut buf, &schema).map_err(|e| format!("Writer error: {}", e))?;
        writer
            .write(&batch)
            .map_err(|e| format!("Write error: {}", e))?;
        writer
            .finish()
            .map_err(|e| format!("Finish error: {}", e))?;
    }

    Ok(buf)
}

/// Create an Arrow IPC stream containing bus voltage results
///
/// Schema: bus_id (string), v_mag (float64), v_ang_deg (float64), lmp (float64)
pub fn buses_to_arrow(
    bus_voltage_mag: &HashMap<String, f64>,
    bus_voltage_ang: &HashMap<String, f64>,
    bus_lmp: &HashMap<String, f64>,
) -> Result<Vec<u8>, String> {
    let schema = Schema::new(vec![
        Field::new("bus_id", DataType::Utf8, false),
        Field::new("v_mag", DataType::Float64, true),
        Field::new("v_ang_deg", DataType::Float64, true),
        Field::new("lmp", DataType::Float64, true),
    ]);

    // Use all unique bus IDs
    let mut bus_ids: Vec<&String> = bus_voltage_mag
        .keys()
        .chain(bus_voltage_ang.keys())
        .chain(bus_lmp.keys())
        .collect();
    bus_ids.sort();
    bus_ids.dedup();

    let bus_id_array: StringArray = bus_ids.iter().map(|s| Some(s.as_str())).collect();
    let v_mag_array: Float64Array = bus_ids
        .iter()
        .map(|id| bus_voltage_mag.get(*id).copied())
        .collect();
    let v_ang_array: Float64Array = bus_ids
        .iter()
        .map(|id| bus_voltage_ang.get(*id).copied())
        .collect();
    let lmp_array: Float64Array = bus_ids
        .iter()
        .map(|id| bus_lmp.get(*id).copied())
        .collect();

    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(bus_id_array) as ArrayRef,
            Arc::new(v_mag_array) as ArrayRef,
            Arc::new(v_ang_array) as ArrayRef,
            Arc::new(lmp_array) as ArrayRef,
        ],
    )
    .map_err(|e| format!("Failed to create RecordBatch: {}", e))?;

    let mut buf = Vec::new();
    {
        let mut writer =
            StreamWriter::try_new(&mut buf, &schema).map_err(|e| format!("Writer error: {}", e))?;
        writer
            .write(&batch)
            .map_err(|e| format!("Write error: {}", e))?;
        writer
            .finish()
            .map_err(|e| format!("Finish error: {}", e))?;
    }

    Ok(buf)
}

/// Create an Arrow IPC stream containing branch flow results
///
/// Schema: branch_id (string), p_flow_mw (float64), q_flow_mvar (float64)
pub fn branches_to_arrow(
    branch_p_flow: &HashMap<String, f64>,
    branch_q_flow: &HashMap<String, f64>,
) -> Result<Vec<u8>, String> {
    let schema = Schema::new(vec![
        Field::new("branch_id", DataType::Utf8, false),
        Field::new("p_flow_mw", DataType::Float64, false),
        Field::new("q_flow_mvar", DataType::Float64, true),
    ]);

    let mut branch_ids: Vec<&String> = branch_p_flow.keys().collect();
    branch_ids.sort();

    let branch_id_array: StringArray = branch_ids.iter().map(|s| Some(s.as_str())).collect();
    let p_array: Float64Array = branch_ids
        .iter()
        .map(|id| branch_p_flow.get(*id).copied())
        .collect();
    let q_array: Float64Array = branch_ids
        .iter()
        .map(|id| branch_q_flow.get(*id).copied())
        .collect();

    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(branch_id_array) as ArrayRef,
            Arc::new(p_array) as ArrayRef,
            Arc::new(q_array) as ArrayRef,
        ],
    )
    .map_err(|e| format!("Failed to create RecordBatch: {}", e))?;

    let mut buf = Vec::new();
    {
        let mut writer =
            StreamWriter::try_new(&mut buf, &schema).map_err(|e| format!("Writer error: {}", e))?;
        writer
            .write(&batch)
            .map_err(|e| format!("Write error: {}", e))?;
        writer
            .finish()
            .map_err(|e| format!("Finish error: {}", e))?;
    }

    Ok(buf)
}

/// OPF result in Arrow IPC format for JavaScript consumption
pub struct OpfArrowResult {
    /// Arrow IPC bytes for generator results
    pub generators: Vec<u8>,
    /// Arrow IPC bytes for bus results
    pub buses: Vec<u8>,
    /// Arrow IPC bytes for branch results
    pub branches: Vec<u8>,
    /// Summary metadata as JSON (for quick access without parsing Arrow)
    pub summary_json: String,
}

/// Summary data included as JSON alongside Arrow tables
#[derive(serde::Serialize)]
pub struct OpfSummary {
    pub converged: bool,
    pub objective_value: f64,
    pub solve_time_ms: u128,
    pub method: String,
    pub total_generation_mw: f64,
    pub total_load_mw: f64,
    pub total_losses_mw: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generators_to_arrow() {
        let mut p = HashMap::new();
        p.insert("Gen1".to_string(), 100.0);
        p.insert("Gen2".to_string(), 50.0);

        let mut q = HashMap::new();
        q.insert("Gen1".to_string(), 10.0);
        q.insert("Gen2".to_string(), 5.0);

        let bytes = generators_to_arrow(&p, &q).unwrap();
        assert!(!bytes.is_empty());
        // Arrow IPC stream starts with magic bytes
        assert_eq!(&bytes[0..4], b"\xff\xff\xff\xff");
    }

    #[test]
    fn test_buses_to_arrow() {
        let mut v_mag = HashMap::new();
        v_mag.insert("Bus1".to_string(), 1.0);
        v_mag.insert("Bus2".to_string(), 0.98);

        let mut v_ang = HashMap::new();
        v_ang.insert("Bus1".to_string(), 0.0);
        v_ang.insert("Bus2".to_string(), -5.2);

        let mut lmp = HashMap::new();
        lmp.insert("Bus1".to_string(), 25.0);
        lmp.insert("Bus2".to_string(), 26.5);

        let bytes = buses_to_arrow(&v_mag, &v_ang, &lmp).unwrap();
        assert!(!bytes.is_empty());
    }
}
