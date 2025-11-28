//! Arrow IPC serialization for solver communication.
//!
//! Provides helpers for serializing/deserializing problem and solution batches
//! to/from Arrow IPC format for subprocess communication.
//!
//! # Why Arrow IPC?
//!
//! Arrow IPC provides several advantages for solver communication:
//!
//! 1. **Zero-copy reads:** Data can be memory-mapped without deserialization
//! 2. **Language-agnostic:** Arrow has bindings for C++, Python, Julia, etc.
//! 3. **Columnar format:** Efficient for numerical arrays (bus voltages, power flows)
//! 4. **Schema evolution:** Fields can be added without breaking compatibility
//!
//! # Schema Design
//!
//! The IPC schema uses a "wide" format where all problem/solution data is in a
//! single RecordBatch. This simplifies parsing but requires padding arrays to
//! equal length. Alternative designs (multiple batches per entity type) are
//! possible but complicate streaming.
//!
//! ## Problem Schema Fields
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `bus_id` | Int64 | Bus identifiers (1-indexed) |
//! | `bus_v_min/max` | Float64 | Voltage limits (p.u.) |
//! | `gen_id` | Int64 | Generator identifiers |
//! | `gen_p_min/max` | Float64 | Active power limits (MW) |
//! | `branch_id` | Int64 | Branch identifiers |
//! | `branch_r/x` | Float64 | Resistance/reactance (p.u.) |
//!
//! ## Solution Schema Fields
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `status` | Utf8 | Solution status string |
//! | `objective` | Float64 | Objective value ($/hr) |
//! | `bus_v_mag/ang` | Float64 | Voltage solution (p.u., rad) |
//! | `bus_lmp` | Float64 | Locational marginal prices ($/MWh) |
//! | `gen_p/q` | Float64 | Generator dispatch (MW, MVAr) |
//!
//! # Protocol Version
//!
//! The `protocol_version` field in [`ProblemBatch`] enables schema evolution.
//! Increment [`PROTOCOL_VERSION`](crate::PROTOCOL_VERSION) when making breaking changes.

use crate::error::SolverResult;
use crate::problem::ProblemBatch;
use crate::solution::{SolutionBatch, SolutionStatus};
use arrow::array::{Float64Array, Int32Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use std::io::{Read, Write};
use std::sync::Arc;

/// Schema for problem data sent to solvers.
pub fn problem_schema() -> Schema {
    Schema::new(vec![
        // Metadata fields
        Field::new("protocol_version", DataType::Int32, false),
        Field::new("base_mva", DataType::Float64, false),
        Field::new("tolerance", DataType::Float64, false),
        Field::new("max_iterations", DataType::Int32, false),
        // Bus fields
        Field::new("bus_id", DataType::Int64, false),
        Field::new("bus_v_min", DataType::Float64, false),
        Field::new("bus_v_max", DataType::Float64, false),
        Field::new("bus_p_load", DataType::Float64, false),
        Field::new("bus_q_load", DataType::Float64, false),
        Field::new("bus_type", DataType::Int32, false),
        Field::new("bus_v_mag", DataType::Float64, false),
        Field::new("bus_v_ang", DataType::Float64, false),
        // Generator fields
        Field::new("gen_id", DataType::Int64, false),
        Field::new("gen_bus_id", DataType::Int64, false),
        Field::new("gen_p_min", DataType::Float64, false),
        Field::new("gen_p_max", DataType::Float64, false),
        Field::new("gen_q_min", DataType::Float64, false),
        Field::new("gen_q_max", DataType::Float64, false),
        Field::new("gen_cost_c0", DataType::Float64, false),
        Field::new("gen_cost_c1", DataType::Float64, false),
        Field::new("gen_cost_c2", DataType::Float64, false),
        // Branch fields
        Field::new("branch_id", DataType::Int64, false),
        Field::new("branch_from", DataType::Int64, false),
        Field::new("branch_to", DataType::Int64, false),
        Field::new("branch_r", DataType::Float64, false),
        Field::new("branch_x", DataType::Float64, false),
        Field::new("branch_b", DataType::Float64, false),
        Field::new("branch_rate", DataType::Float64, false),
        Field::new("branch_tap", DataType::Float64, false),
        Field::new("branch_shift", DataType::Float64, false),
    ])
}

/// Schema for solution data received from solvers.
pub fn solution_schema() -> Schema {
    Schema::new(vec![
        // Status fields
        Field::new("status", DataType::Utf8, false),
        Field::new("objective", DataType::Float64, false),
        Field::new("iterations", DataType::Int32, false),
        Field::new("solve_time_ms", DataType::Int64, false),
        Field::new("error_message", DataType::Utf8, true),
        // Bus results
        Field::new("bus_id", DataType::Int64, false),
        Field::new("bus_v_mag", DataType::Float64, false),
        Field::new("bus_v_ang", DataType::Float64, false),
        Field::new("bus_lmp", DataType::Float64, false),
        // Generator results
        Field::new("gen_id", DataType::Int64, false),
        Field::new("gen_p", DataType::Float64, false),
        Field::new("gen_q", DataType::Float64, false),
        // Branch results
        Field::new("branch_id", DataType::Int64, false),
        Field::new("branch_p_from", DataType::Float64, false),
        Field::new("branch_q_from", DataType::Float64, false),
        Field::new("branch_p_to", DataType::Float64, false),
        Field::new("branch_q_to", DataType::Float64, false),
    ])
}

/// Write a problem batch to Arrow IPC format.
pub fn write_problem<W: Write>(problem: &ProblemBatch, writer: W) -> SolverResult<()> {
    let schema = Arc::new(problem_schema());

    // Determine row count - use max of all arrays, minimum 1 for metadata
    let n_rows = [
        problem.bus_id.len(),
        problem.gen_id.len(),
        problem.branch_id.len(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
    .max(1);

    // Helper to pad arrays to n_rows
    fn pad_i64(arr: &[i64], n: usize) -> Vec<i64> {
        let mut v = arr.to_vec();
        v.resize(n, 0);
        v
    }
    fn pad_i32(arr: &[i32], n: usize) -> Vec<i32> {
        let mut v = arr.to_vec();
        v.resize(n, 0);
        v
    }
    fn pad_f64(arr: &[f64], n: usize) -> Vec<f64> {
        let mut v = arr.to_vec();
        v.resize(n, 0.0);
        v
    }

    // Create record batch with ALL fields to match schema
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            // Metadata fields (repeated for each row)
            Arc::new(Int32Array::from(vec![problem.protocol_version; n_rows])),
            Arc::new(Float64Array::from(vec![problem.base_mva; n_rows])),
            Arc::new(Float64Array::from(vec![problem.tolerance; n_rows])),
            Arc::new(Int32Array::from(vec![problem.max_iterations; n_rows])),
            // Bus fields
            Arc::new(Int64Array::from(pad_i64(&problem.bus_id, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.bus_v_min, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.bus_v_max, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.bus_p_load, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.bus_q_load, n_rows))),
            Arc::new(Int32Array::from(pad_i32(&problem.bus_type, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.bus_v_mag, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.bus_v_ang, n_rows))),
            // Generator fields
            Arc::new(Int64Array::from(pad_i64(&problem.gen_id, n_rows))),
            Arc::new(Int64Array::from(pad_i64(&problem.gen_bus_id, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.gen_p_min, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.gen_p_max, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.gen_q_min, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.gen_q_max, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.gen_cost_c0, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.gen_cost_c1, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.gen_cost_c2, n_rows))),
            // Branch fields
            Arc::new(Int64Array::from(pad_i64(&problem.branch_id, n_rows))),
            Arc::new(Int64Array::from(pad_i64(&problem.branch_from, n_rows))),
            Arc::new(Int64Array::from(pad_i64(&problem.branch_to, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.branch_r, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.branch_x, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.branch_b, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.branch_rate, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.branch_tap, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&problem.branch_shift, n_rows))),
        ],
    )?;

    let mut ipc_writer = StreamWriter::try_new(writer, &schema)?;
    ipc_writer.write(&batch)?;
    ipc_writer.finish()?;

    Ok(())
}

/// Read a problem batch from Arrow IPC format.
pub fn read_problem<R: Read>(reader: R) -> SolverResult<ProblemBatch> {
    let stream_reader = StreamReader::try_new(reader, None)?;

    let mut problem = ProblemBatch::default();

    for batch_result in stream_reader {
        let batch = batch_result?;

        // Extract bus data
        if let Some(col) = batch.column_by_name("bus_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.bus_id = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_min") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_v_min = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_max") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_v_max = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_p_load") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_p_load = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_q_load") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_q_load = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_type") {
            if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                problem.bus_type = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_mag") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_v_mag = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_ang") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_v_ang = arr.values().to_vec();
            }
        }

        // Extract generator data
        if let Some(col) = batch.column_by_name("gen_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.gen_id = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("gen_bus_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.gen_bus_id = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("gen_p_min") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_p_min = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("gen_p_max") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_p_max = arr.values().to_vec();
            }
        }

        // Extract branch data
        if let Some(col) = batch.column_by_name("branch_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.branch_id = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("branch_from") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.branch_from = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("branch_to") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.branch_to = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("branch_r") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.branch_r = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("branch_x") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.branch_x = arr.values().to_vec();
            }
        }
    }

    Ok(problem)
}

/// Write a solution batch to Arrow IPC format.
pub fn write_solution<W: Write>(solution: &SolutionBatch, writer: W) -> SolverResult<()> {
    let schema = Arc::new(solution_schema());

    // Convert status to string
    let status_str = match solution.status {
        SolutionStatus::Optimal => "optimal",
        SolutionStatus::Infeasible => "infeasible",
        SolutionStatus::Unbounded => "unbounded",
        SolutionStatus::Timeout => "timeout",
        SolutionStatus::IterationLimit => "iteration_limit",
        SolutionStatus::NumericalError => "numerical_error",
        SolutionStatus::Error => "error",
        SolutionStatus::Unknown => "unknown",
    };

    // Determine row count - use max of all arrays, minimum 1 for header
    let n_rows = [
        solution.bus_id.len(),
        solution.gen_id.len(),
        solution.branch_id.len(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
    .max(1);

    // Helper to pad arrays to n_rows
    fn pad_i64(arr: &[i64], n: usize) -> Vec<i64> {
        let mut v = arr.to_vec();
        v.resize(n, 0);
        v
    }
    fn pad_f64(arr: &[f64], n: usize) -> Vec<f64> {
        let mut v = arr.to_vec();
        v.resize(n, 0.0);
        v
    }

    // Create record batch with ALL fields to match schema
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            // Header fields (repeated for each row)
            Arc::new(StringArray::from(vec![status_str; n_rows])),
            Arc::new(Float64Array::from(vec![solution.objective; n_rows])),
            Arc::new(Int32Array::from(vec![solution.iterations; n_rows])),
            Arc::new(Int64Array::from(vec![solution.solve_time_ms; n_rows])),
            Arc::new(StringArray::from(vec![
                solution.error_message.clone();
                n_rows
            ])),
            // Bus results
            Arc::new(Int64Array::from(pad_i64(&solution.bus_id, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.bus_v_mag, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.bus_v_ang, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.bus_lmp, n_rows))),
            // Generator results
            Arc::new(Int64Array::from(pad_i64(&solution.gen_id, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.gen_p, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.gen_q, n_rows))),
            // Branch results
            Arc::new(Int64Array::from(pad_i64(&solution.branch_id, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.branch_p_from, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.branch_q_from, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.branch_p_to, n_rows))),
            Arc::new(Float64Array::from(pad_f64(&solution.branch_q_to, n_rows))),
        ],
    )?;

    let mut ipc_writer = StreamWriter::try_new(writer, &schema)?;
    ipc_writer.write(&batch)?;
    ipc_writer.finish()?;

    Ok(())
}

/// Read a solution batch from Arrow IPC format.
pub fn read_solution<R: Read>(reader: R) -> SolverResult<SolutionBatch> {
    let stream_reader = StreamReader::try_new(reader, None)?;

    let mut solution = SolutionBatch::default();

    for batch_result in stream_reader {
        let batch = batch_result?;

        // Extract status from first column
        if let Some(status_col) = batch.column_by_name("status") {
            if let Some(status_array) = status_col.as_any().downcast_ref::<StringArray>() {
                if let Some(status_str) = status_array.value(0).to_string().as_str().into() {
                    solution.status = match status_str {
                        "optimal" => crate::SolutionStatus::Optimal,
                        "infeasible" => crate::SolutionStatus::Infeasible,
                        "unbounded" => crate::SolutionStatus::Unbounded,
                        "timeout" => crate::SolutionStatus::Timeout,
                        "iteration_limit" => crate::SolutionStatus::IterationLimit,
                        "numerical_error" => crate::SolutionStatus::NumericalError,
                        "error" => crate::SolutionStatus::Error,
                        _ => crate::SolutionStatus::Unknown,
                    };
                }
            }
        }

        // Extract objective
        if let Some(obj_col) = batch.column_by_name("objective") {
            if let Some(obj_array) = obj_col.as_any().downcast_ref::<Float64Array>() {
                solution.objective = obj_array.value(0);
            }
        }

        // Extract bus data
        if let Some(col) = batch.column_by_name("bus_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                solution.bus_id = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_mag") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.bus_v_mag = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_ang") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.bus_v_ang = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("bus_lmp") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.bus_lmp = arr.values().to_vec();
            }
        }

        // Extract generator data
        if let Some(col) = batch.column_by_name("gen_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                solution.gen_id = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("gen_p") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.gen_p = arr.values().to_vec();
            }
        }
        if let Some(col) = batch.column_by_name("gen_q") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.gen_q = arr.values().to_vec();
            }
        }
    }

    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::ProblemType;

    #[test]
    fn test_problem_schema() {
        let schema = problem_schema();
        assert!(schema.field_with_name("bus_id").is_ok());
        assert!(schema.field_with_name("gen_id").is_ok());
        assert!(schema.field_with_name("branch_id").is_ok());
    }

    #[test]
    fn test_solution_schema() {
        let schema = solution_schema();
        assert!(schema.field_with_name("status").is_ok());
        assert!(schema.field_with_name("objective").is_ok());
        assert!(schema.field_with_name("bus_v_mag").is_ok());
    }

    #[test]
    fn test_roundtrip_empty_problem() {
        let problem = ProblemBatch::new(ProblemType::DcOpf);
        let mut buffer = Vec::new();
        write_problem(&problem, &mut buffer).unwrap();
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_ipc_problem_roundtrip_with_data() {
        // Create a problem with some data
        let mut problem = ProblemBatch::new(ProblemType::AcOpf);
        problem.bus_id = vec![1, 2, 3];
        problem.bus_v_min = vec![0.95, 0.95, 0.95];
        problem.bus_v_max = vec![1.05, 1.05, 1.05];
        problem.bus_p_load = vec![50.0, 100.0, 75.0];
        problem.bus_q_load = vec![25.0, 50.0, 37.5];
        problem.bus_type = vec![3, 1, 1]; // Slack, PQ, PQ
        problem.bus_v_mag = vec![1.0, 1.0, 1.0];
        problem.bus_v_ang = vec![0.0, 0.0, 0.0];

        // Write to buffer
        let mut buffer = Vec::new();
        write_problem(&problem, &mut buffer).unwrap();

        // Read back
        let recovered = read_problem(&buffer[..]).unwrap();

        // Verify data preserved
        assert_eq!(recovered.bus_id, vec![1, 2, 3]);
        assert_eq!(recovered.bus_v_min, vec![0.95, 0.95, 0.95]);
        assert_eq!(recovered.bus_type, vec![3, 1, 1]);
    }

    #[test]
    fn test_ipc_solution_roundtrip() {
        let solution = SolutionBatch {
            status: SolutionStatus::Optimal,
            objective: 12345.67,
            iterations: 42,
            solve_time_ms: 123,
            error_message: None,
            bus_id: vec![],
            bus_v_mag: vec![],
            bus_v_ang: vec![],
            bus_lmp: vec![],
            gen_id: vec![],
            gen_p: vec![],
            gen_q: vec![],
            branch_id: vec![],
            branch_p_from: vec![],
            branch_q_from: vec![],
            branch_p_to: vec![],
            branch_q_to: vec![],
        };

        let mut buffer = Vec::new();
        write_solution(&solution, &mut buffer).unwrap();

        let recovered = read_solution(&buffer[..]).unwrap();
        assert_eq!(recovered.status, SolutionStatus::Optimal);
        assert!((recovered.objective - 12345.67).abs() < 1e-6);
    }

    #[test]
    fn test_ipc_solution_error_status() {
        let solution = SolutionBatch {
            status: SolutionStatus::Error,
            objective: 0.0,
            iterations: 0,
            solve_time_ms: 0,
            error_message: Some("Test error message".to_string()),
            ..Default::default()
        };

        let mut buffer = Vec::new();
        write_solution(&solution, &mut buffer).unwrap();

        let recovered = read_solution(&buffer[..]).unwrap();
        assert_eq!(recovered.status, SolutionStatus::Error);
    }

    #[test]
    fn test_ipc_all_solution_statuses() {
        let statuses = vec![
            SolutionStatus::Optimal,
            SolutionStatus::Infeasible,
            SolutionStatus::Unbounded,
            SolutionStatus::Timeout,
            SolutionStatus::IterationLimit,
            SolutionStatus::NumericalError,
            SolutionStatus::Error,
            SolutionStatus::Unknown,
        ];

        for status in statuses {
            let solution = SolutionBatch {
                status,
                ..Default::default()
            };

            let mut buffer = Vec::new();
            write_solution(&solution, &mut buffer).unwrap();

            let recovered = read_solution(&buffer[..]).unwrap();
            assert_eq!(
                recovered.status, status,
                "Failed to roundtrip status {:?}",
                status
            );
        }
    }
}
