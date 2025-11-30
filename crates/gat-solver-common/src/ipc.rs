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
//! # Protocol Versions
//!
//! ## v1 (Legacy): Single wide RecordBatch
//!
//! All data in one batch with padding to max entity count. Requires entity count
//! fields (n_bus, n_gen, n_branch) to distinguish real data from padding.
//!
//! ## v2 (Current): Multiple RecordBatches
//!
//! Separate batches per entity type with natural lengths:
//! - Batch 0: Metadata (1 row)
//! - Batch 1: Bus data (n_bus rows)
//! - Batch 2: Generator data (n_gen rows)
//! - Batch 3: Branch data (n_branch rows)
//!
//! This eliminates padding and the associated bugs.
//!
//! # Protocol Version
//!
//! The `protocol_version` field in [`ProblemBatch`] enables schema evolution.
//! Increment [`PROTOCOL_VERSION`](crate::PROTOCOL_VERSION) when making breaking changes.

use crate::error::SolverResult;
use crate::problem::ProblemBatch;
use crate::solution::{SolutionBatch, SolutionStatus};
use arrow::array::{Array, Float64Array, Int32Array, Int64Array, StringArray};
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
        // Entity counts (to distinguish real data from padding)
        Field::new("n_bus", DataType::Int32, false),
        Field::new("n_gen", DataType::Int32, false),
        Field::new("n_branch", DataType::Int32, false),
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
        // Entity counts (to distinguish real data from padding)
        Field::new("n_bus", DataType::Int32, false),
        Field::new("n_gen", DataType::Int32, false),
        Field::new("n_branch", DataType::Int32, false),
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

    // Store entity counts for reader to know actual sizes
    let n_bus = problem.bus_id.len() as i32;
    let n_gen = problem.gen_id.len() as i32;
    let n_branch = problem.branch_id.len() as i32;

    // Create record batch with ALL fields to match schema
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            // Metadata fields (repeated for each row)
            Arc::new(Int32Array::from(vec![problem.protocol_version; n_rows])),
            Arc::new(Float64Array::from(vec![problem.base_mva; n_rows])),
            Arc::new(Float64Array::from(vec![problem.tolerance; n_rows])),
            Arc::new(Int32Array::from(vec![problem.max_iterations; n_rows])),
            // Entity counts (repeated for each row, reader uses first value)
            Arc::new(Int32Array::from(vec![n_bus; n_rows])),
            Arc::new(Int32Array::from(vec![n_gen; n_rows])),
            Arc::new(Int32Array::from(vec![n_branch; n_rows])),
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

        // Extract entity counts first (to know how much real data exists)
        let mut n_bus = 0usize;
        let mut n_gen = 0usize;
        let mut n_branch = 0usize;

        if let Some(col) = batch.column_by_name("n_bus") {
            if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                if !arr.is_empty() {
                    n_bus = arr.value(0) as usize;
                }
            }
        }
        if let Some(col) = batch.column_by_name("n_gen") {
            if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                if !arr.is_empty() {
                    n_gen = arr.value(0) as usize;
                }
            }
        }
        if let Some(col) = batch.column_by_name("n_branch") {
            if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                if !arr.is_empty() {
                    n_branch = arr.value(0) as usize;
                }
            }
        }

        // Helper to truncate arrays to their actual size
        fn truncate_i64(arr: &[i64], n: usize) -> Vec<i64> {
            arr.iter().take(n).copied().collect()
        }
        fn truncate_i32(arr: &[i32], n: usize) -> Vec<i32> {
            arr.iter().take(n).copied().collect()
        }
        fn truncate_f64(arr: &[f64], n: usize) -> Vec<f64> {
            arr.iter().take(n).copied().collect()
        }

        // Extract bus data (truncate to n_bus)
        if let Some(col) = batch.column_by_name("bus_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.bus_id = truncate_i64(arr.values(), n_bus);
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_min") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_v_min = truncate_f64(arr.values(), n_bus);
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_max") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_v_max = truncate_f64(arr.values(), n_bus);
            }
        }
        if let Some(col) = batch.column_by_name("bus_p_load") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_p_load = truncate_f64(arr.values(), n_bus);
            }
        }
        if let Some(col) = batch.column_by_name("bus_q_load") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_q_load = truncate_f64(arr.values(), n_bus);
            }
        }
        if let Some(col) = batch.column_by_name("bus_type") {
            if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                problem.bus_type = truncate_i32(arr.values(), n_bus);
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_mag") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_v_mag = truncate_f64(arr.values(), n_bus);
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_ang") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.bus_v_ang = truncate_f64(arr.values(), n_bus);
            }
        }

        // Extract generator data (truncate to n_gen)
        if let Some(col) = batch.column_by_name("gen_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.gen_id = truncate_i64(arr.values(), n_gen);
            }
        }
        if let Some(col) = batch.column_by_name("gen_bus_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.gen_bus_id = truncate_i64(arr.values(), n_gen);
            }
        }
        if let Some(col) = batch.column_by_name("gen_p_min") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_p_min = truncate_f64(arr.values(), n_gen);
            }
        }
        if let Some(col) = batch.column_by_name("gen_p_max") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_p_max = truncate_f64(arr.values(), n_gen);
            }
        }
        if let Some(col) = batch.column_by_name("gen_q_min") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_q_min = truncate_f64(arr.values(), n_gen);
            }
        }
        if let Some(col) = batch.column_by_name("gen_q_max") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_q_max = truncate_f64(arr.values(), n_gen);
            }
        }
        if let Some(col) = batch.column_by_name("gen_cost_c0") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_cost_c0 = truncate_f64(arr.values(), n_gen);
            }
        }
        if let Some(col) = batch.column_by_name("gen_cost_c1") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_cost_c1 = truncate_f64(arr.values(), n_gen);
            }
        }
        if let Some(col) = batch.column_by_name("gen_cost_c2") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.gen_cost_c2 = truncate_f64(arr.values(), n_gen);
            }
        }

        // Extract branch data (truncate to n_branch)
        if let Some(col) = batch.column_by_name("branch_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.branch_id = truncate_i64(arr.values(), n_branch);
            }
        }
        if let Some(col) = batch.column_by_name("branch_from") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.branch_from = truncate_i64(arr.values(), n_branch);
            }
        }
        if let Some(col) = batch.column_by_name("branch_to") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                problem.branch_to = truncate_i64(arr.values(), n_branch);
            }
        }
        if let Some(col) = batch.column_by_name("branch_r") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.branch_r = truncate_f64(arr.values(), n_branch);
            }
        }
        if let Some(col) = batch.column_by_name("branch_x") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                problem.branch_x = truncate_f64(arr.values(), n_branch);
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

    // Store entity counts for reader to know actual sizes
    let n_bus = solution.bus_id.len() as i32;
    let n_gen = solution.gen_id.len() as i32;
    let n_branch = solution.branch_id.len() as i32;

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
            // Entity counts (repeated for each row, reader uses first value)
            Arc::new(Int32Array::from(vec![n_bus; n_rows])),
            Arc::new(Int32Array::from(vec![n_gen; n_rows])),
            Arc::new(Int32Array::from(vec![n_branch; n_rows])),
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

        // Extract entity counts first (to know how much real data exists)
        let mut n_bus = 0usize;
        let mut n_gen = 0usize;
        let mut n_branch = 0usize;

        if let Some(col) = batch.column_by_name("n_bus") {
            if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                if !arr.is_empty() {
                    n_bus = arr.value(0) as usize;
                }
            }
        }
        if let Some(col) = batch.column_by_name("n_gen") {
            if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                if !arr.is_empty() {
                    n_gen = arr.value(0) as usize;
                }
            }
        }
        if let Some(col) = batch.column_by_name("n_branch") {
            if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                if !arr.is_empty() {
                    n_branch = arr.value(0) as usize;
                }
            }
        }

        // If entity counts are 0, fall back to full array length (legacy compatibility)
        let use_truncation = n_bus > 0 || n_gen > 0 || n_branch > 0;

        // Helper to truncate arrays to their actual size
        fn truncate_i64(arr: &[i64], n: usize, use_truncation: bool) -> Vec<i64> {
            if use_truncation {
                arr.iter().take(n).copied().collect()
            } else {
                arr.to_vec()
            }
        }
        fn truncate_f64(arr: &[f64], n: usize, use_truncation: bool) -> Vec<f64> {
            if use_truncation {
                arr.iter().take(n).copied().collect()
            } else {
                arr.to_vec()
            }
        }

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

        // Extract bus data (truncate to n_bus)
        if let Some(col) = batch.column_by_name("bus_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                solution.bus_id = truncate_i64(arr.values(), n_bus, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_mag") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.bus_v_mag = truncate_f64(arr.values(), n_bus, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("bus_v_ang") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.bus_v_ang = truncate_f64(arr.values(), n_bus, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("bus_lmp") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.bus_lmp = truncate_f64(arr.values(), n_bus, use_truncation);
            }
        }

        // Extract generator data (truncate to n_gen)
        if let Some(col) = batch.column_by_name("gen_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                solution.gen_id = truncate_i64(arr.values(), n_gen, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("gen_p") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.gen_p = truncate_f64(arr.values(), n_gen, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("gen_q") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.gen_q = truncate_f64(arr.values(), n_gen, use_truncation);
            }
        }

        // Extract branch data (truncate to n_branch)
        if let Some(col) = batch.column_by_name("branch_id") {
            if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                solution.branch_id = truncate_i64(arr.values(), n_branch, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("branch_p_from") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.branch_p_from = truncate_f64(arr.values(), n_branch, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("branch_q_from") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.branch_q_from = truncate_f64(arr.values(), n_branch, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("branch_p_to") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.branch_p_to = truncate_f64(arr.values(), n_branch, use_truncation);
            }
        }
        if let Some(col) = batch.column_by_name("branch_q_to") {
            if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                solution.branch_q_to = truncate_f64(arr.values(), n_branch, use_truncation);
            }
        }
    }

    Ok(solution)
}

// ============================================================================
// Protocol v2: Multi-batch IPC (no padding)
// ============================================================================

/// Schemas for v2 multi-batch protocol
pub mod v2 {
    use super::*;

    /// Metadata batch schema (1 row)
    pub fn metadata_schema() -> Schema {
        Schema::new(vec![
            Field::new("protocol_version", DataType::Int32, false),
            Field::new("base_mva", DataType::Float64, false),
            Field::new("tolerance", DataType::Float64, false),
            Field::new("max_iterations", DataType::Int32, false),
        ])
    }

    /// Bus data batch schema (n_bus rows)
    pub fn bus_schema() -> Schema {
        Schema::new(vec![
            Field::new("bus_id", DataType::Int64, false),
            Field::new("bus_v_min", DataType::Float64, false),
            Field::new("bus_v_max", DataType::Float64, false),
            Field::new("bus_p_load", DataType::Float64, false),
            Field::new("bus_q_load", DataType::Float64, false),
            Field::new("bus_type", DataType::Int32, false),
            Field::new("bus_v_mag", DataType::Float64, false),
            Field::new("bus_v_ang", DataType::Float64, false),
        ])
    }

    /// Generator data batch schema (n_gen rows)
    pub fn gen_schema() -> Schema {
        Schema::new(vec![
            Field::new("gen_id", DataType::Int64, false),
            Field::new("gen_bus_id", DataType::Int64, false),
            Field::new("gen_p_min", DataType::Float64, false),
            Field::new("gen_p_max", DataType::Float64, false),
            Field::new("gen_q_min", DataType::Float64, false),
            Field::new("gen_q_max", DataType::Float64, false),
            Field::new("gen_cost_c0", DataType::Float64, false),
            Field::new("gen_cost_c1", DataType::Float64, false),
            Field::new("gen_cost_c2", DataType::Float64, false),
        ])
    }

    /// Branch data batch schema (n_branch rows)
    pub fn branch_schema() -> Schema {
        Schema::new(vec![
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

    /// Solution metadata batch schema (1 row)
    pub fn solution_metadata_schema() -> Schema {
        Schema::new(vec![
            Field::new("status", DataType::Utf8, false),
            Field::new("objective", DataType::Float64, false),
            Field::new("iterations", DataType::Int32, false),
            Field::new("solve_time_ms", DataType::Int64, false),
            Field::new("error_message", DataType::Utf8, true),
        ])
    }

    /// Solution bus results batch schema (n_bus rows)
    pub fn solution_bus_schema() -> Schema {
        Schema::new(vec![
            Field::new("bus_id", DataType::Int64, false),
            Field::new("bus_v_mag", DataType::Float64, false),
            Field::new("bus_v_ang", DataType::Float64, false),
            Field::new("bus_lmp", DataType::Float64, false),
        ])
    }

    /// Solution generator results batch schema (n_gen rows)
    pub fn solution_gen_schema() -> Schema {
        Schema::new(vec![
            Field::new("gen_id", DataType::Int64, false),
            Field::new("gen_p", DataType::Float64, false),
            Field::new("gen_q", DataType::Float64, false),
        ])
    }

    /// Solution branch results batch schema (n_branch rows)
    pub fn solution_branch_schema() -> Schema {
        Schema::new(vec![
            Field::new("branch_id", DataType::Int64, false),
            Field::new("branch_p_from", DataType::Float64, false),
            Field::new("branch_q_from", DataType::Float64, false),
            Field::new("branch_p_to", DataType::Float64, false),
            Field::new("branch_q_to", DataType::Float64, false),
        ])
    }
}

/// Write a problem using v2 multi-batch protocol.
///
/// Writes 4 length-prefixed IPC streams: metadata, bus, gen, branch.
/// Each stream has its own schema - no padding needed.
///
/// Wire format: [len:u32][ipc_stream] repeated 4 times
pub fn write_problem_v2<W: Write>(problem: &ProblemBatch, mut writer: W) -> SolverResult<()> {
    // Helper to write a length-prefixed IPC stream
    fn write_batch<W: Write>(
        writer: &mut W,
        schema: Arc<Schema>,
        batch: RecordBatch,
    ) -> SolverResult<()> {
        let mut buf = Vec::new();
        let mut ipc_writer = StreamWriter::try_new(&mut buf, &schema)?;
        ipc_writer.write(&batch)?;
        ipc_writer.finish()?;

        // Write length prefix (u32 little-endian) then data
        let len = buf.len() as u32;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&buf)?;
        Ok(())
    }

    // Stream 0: Metadata (1 row)
    let meta_schema = Arc::new(v2::metadata_schema());
    let meta_batch = RecordBatch::try_new(
        meta_schema.clone(),
        vec![
            Arc::new(Int32Array::from(vec![problem.protocol_version])),
            Arc::new(Float64Array::from(vec![problem.base_mva])),
            Arc::new(Float64Array::from(vec![problem.tolerance])),
            Arc::new(Int32Array::from(vec![problem.max_iterations])),
        ],
    )?;
    write_batch(&mut writer, meta_schema, meta_batch)?;

    // Stream 1: Bus data (n_bus rows)
    let bus_schema = Arc::new(v2::bus_schema());
    let bus_batch = RecordBatch::try_new(
        bus_schema.clone(),
        vec![
            Arc::new(Int64Array::from(problem.bus_id.clone())),
            Arc::new(Float64Array::from(problem.bus_v_min.clone())),
            Arc::new(Float64Array::from(problem.bus_v_max.clone())),
            Arc::new(Float64Array::from(problem.bus_p_load.clone())),
            Arc::new(Float64Array::from(problem.bus_q_load.clone())),
            Arc::new(Int32Array::from(problem.bus_type.clone())),
            Arc::new(Float64Array::from(problem.bus_v_mag.clone())),
            Arc::new(Float64Array::from(problem.bus_v_ang.clone())),
        ],
    )?;
    write_batch(&mut writer, bus_schema, bus_batch)?;

    // Stream 2: Generator data (n_gen rows)
    let gen_schema = Arc::new(v2::gen_schema());
    let gen_batch = RecordBatch::try_new(
        gen_schema.clone(),
        vec![
            Arc::new(Int64Array::from(problem.gen_id.clone())),
            Arc::new(Int64Array::from(problem.gen_bus_id.clone())),
            Arc::new(Float64Array::from(problem.gen_p_min.clone())),
            Arc::new(Float64Array::from(problem.gen_p_max.clone())),
            Arc::new(Float64Array::from(problem.gen_q_min.clone())),
            Arc::new(Float64Array::from(problem.gen_q_max.clone())),
            Arc::new(Float64Array::from(problem.gen_cost_c0.clone())),
            Arc::new(Float64Array::from(problem.gen_cost_c1.clone())),
            Arc::new(Float64Array::from(problem.gen_cost_c2.clone())),
        ],
    )?;
    write_batch(&mut writer, gen_schema, gen_batch)?;

    // Stream 3: Branch data (n_branch rows)
    let branch_schema = Arc::new(v2::branch_schema());
    let branch_batch = RecordBatch::try_new(
        branch_schema.clone(),
        vec![
            Arc::new(Int64Array::from(problem.branch_id.clone())),
            Arc::new(Int64Array::from(problem.branch_from.clone())),
            Arc::new(Int64Array::from(problem.branch_to.clone())),
            Arc::new(Float64Array::from(problem.branch_r.clone())),
            Arc::new(Float64Array::from(problem.branch_x.clone())),
            Arc::new(Float64Array::from(problem.branch_b.clone())),
            Arc::new(Float64Array::from(problem.branch_rate.clone())),
            Arc::new(Float64Array::from(problem.branch_tap.clone())),
            Arc::new(Float64Array::from(problem.branch_shift.clone())),
        ],
    )?;
    write_batch(&mut writer, branch_schema, branch_batch)?;

    Ok(())
}

/// Read a problem using v2 multi-batch protocol.
///
/// Expects 4 length-prefixed IPC streams: metadata, bus, gen, branch.
/// Wire format: [len:u32][ipc_stream] repeated 4 times
pub fn read_problem_v2<R: Read>(mut reader: R) -> SolverResult<ProblemBatch> {
    use std::io::Cursor;

    // Helper to read a length-prefixed IPC stream and return the first batch
    fn read_batch<R: Read>(reader: &mut R) -> SolverResult<RecordBatch> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        // Read IPC data
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;

        // Parse IPC stream
        let cursor = Cursor::new(buf);
        let mut stream_reader = StreamReader::try_new(cursor, None)?;
        if let Some(batch_result) = stream_reader.next() {
            return Ok(batch_result?);
        }
        Err(crate::error::SolverError::Ipc(
            "Empty IPC stream".to_string(),
        ))
    }

    let mut problem = ProblemBatch::default();

    // Read 4 streams in order
    for batch_idx in 0..4 {
        let batch = read_batch(&mut reader)?;

        match batch_idx {
            0 => {
                // Metadata batch
                if let Some(col) = batch.column_by_name("protocol_version") {
                    if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                        if !arr.is_empty() {
                            problem.protocol_version = arr.value(0);
                        }
                    }
                }
                if let Some(col) = batch.column_by_name("base_mva") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        if !arr.is_empty() {
                            problem.base_mva = arr.value(0);
                        }
                    }
                }
                if let Some(col) = batch.column_by_name("tolerance") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        if !arr.is_empty() {
                            problem.tolerance = arr.value(0);
                        }
                    }
                }
                if let Some(col) = batch.column_by_name("max_iterations") {
                    if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                        if !arr.is_empty() {
                            problem.max_iterations = arr.value(0);
                        }
                    }
                }
            }
            1 => {
                // Bus batch - extract all arrays directly (no truncation needed!)
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
            }
            2 => {
                // Generator batch
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
                if let Some(col) = batch.column_by_name("gen_q_min") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.gen_q_min = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("gen_q_max") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.gen_q_max = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("gen_cost_c0") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.gen_cost_c0 = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("gen_cost_c1") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.gen_cost_c1 = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("gen_cost_c2") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.gen_cost_c2 = arr.values().to_vec();
                    }
                }
            }
            3 => {
                // Branch batch
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
                if let Some(col) = batch.column_by_name("branch_b") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.branch_b = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("branch_rate") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.branch_rate = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("branch_tap") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.branch_tap = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("branch_shift") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        problem.branch_shift = arr.values().to_vec();
                    }
                }
            }
            _ => {
                // Should not happen with fixed 4 iterations
            }
        }
    }

    Ok(problem)
}

/// Write a solution using v2 multi-batch protocol.
///
/// Writes 4 length-prefixed IPC streams: metadata, bus, gen, branch.
/// Wire format: [len:u32][ipc_stream] repeated 4 times
pub fn write_solution_v2<W: Write>(solution: &SolutionBatch, mut writer: W) -> SolverResult<()> {
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

    // Helper to write a length-prefixed IPC stream
    fn write_batch<W: Write>(
        writer: &mut W,
        schema: Arc<Schema>,
        batch: RecordBatch,
    ) -> SolverResult<()> {
        let mut buf = Vec::new();
        let mut ipc_writer = StreamWriter::try_new(&mut buf, &schema)?;
        ipc_writer.write(&batch)?;
        ipc_writer.finish()?;

        let len = buf.len() as u32;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&buf)?;
        Ok(())
    }

    // Stream 0: Metadata (1 row)
    let meta_schema = Arc::new(v2::solution_metadata_schema());
    let meta_batch = RecordBatch::try_new(
        meta_schema.clone(),
        vec![
            Arc::new(StringArray::from(vec![status_str])),
            Arc::new(Float64Array::from(vec![solution.objective])),
            Arc::new(Int32Array::from(vec![solution.iterations])),
            Arc::new(Int64Array::from(vec![solution.solve_time_ms])),
            Arc::new(StringArray::from(vec![solution.error_message.clone()])),
        ],
    )?;
    write_batch(&mut writer, meta_schema, meta_batch)?;

    // Stream 1: Bus results (n_bus rows)
    let bus_schema = Arc::new(v2::solution_bus_schema());
    let bus_batch = RecordBatch::try_new(
        bus_schema.clone(),
        vec![
            Arc::new(Int64Array::from(solution.bus_id.clone())),
            Arc::new(Float64Array::from(solution.bus_v_mag.clone())),
            Arc::new(Float64Array::from(solution.bus_v_ang.clone())),
            Arc::new(Float64Array::from(solution.bus_lmp.clone())),
        ],
    )?;
    write_batch(&mut writer, bus_schema, bus_batch)?;

    // Stream 2: Generator results (n_gen rows)
    let gen_schema = Arc::new(v2::solution_gen_schema());
    let gen_batch = RecordBatch::try_new(
        gen_schema.clone(),
        vec![
            Arc::new(Int64Array::from(solution.gen_id.clone())),
            Arc::new(Float64Array::from(solution.gen_p.clone())),
            Arc::new(Float64Array::from(solution.gen_q.clone())),
        ],
    )?;
    write_batch(&mut writer, gen_schema, gen_batch)?;

    // Stream 3: Branch results (n_branch rows)
    let branch_schema = Arc::new(v2::solution_branch_schema());
    let branch_batch = RecordBatch::try_new(
        branch_schema.clone(),
        vec![
            Arc::new(Int64Array::from(solution.branch_id.clone())),
            Arc::new(Float64Array::from(solution.branch_p_from.clone())),
            Arc::new(Float64Array::from(solution.branch_q_from.clone())),
            Arc::new(Float64Array::from(solution.branch_p_to.clone())),
            Arc::new(Float64Array::from(solution.branch_q_to.clone())),
        ],
    )?;
    write_batch(&mut writer, branch_schema, branch_batch)?;

    Ok(())
}

/// Read a solution using v2 multi-batch protocol.
///
/// Expects 4 length-prefixed IPC streams: metadata, bus, gen, branch.
/// Wire format: [len:u32][ipc_stream] repeated 4 times
pub fn read_solution_v2<R: Read>(mut reader: R) -> SolverResult<SolutionBatch> {
    use std::io::Cursor;

    // Helper to read a length-prefixed IPC stream
    fn read_batch<R: Read>(reader: &mut R) -> SolverResult<RecordBatch> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;

        let cursor = Cursor::new(buf);
        let mut stream_reader = StreamReader::try_new(cursor, None)?;
        if let Some(batch_result) = stream_reader.next() {
            return Ok(batch_result?);
        }
        Err(crate::error::SolverError::Ipc(
            "Empty IPC stream".to_string(),
        ))
    }
    let mut solution = SolutionBatch::default();

    for batch_idx in 0..4 {
        let batch = read_batch(&mut reader)?;

        match batch_idx {
            0 => {
                // Metadata batch
                if let Some(col) = batch.column_by_name("status") {
                    if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                        if !arr.is_empty() {
                            solution.status = match arr.value(0) {
                                "optimal" => SolutionStatus::Optimal,
                                "infeasible" => SolutionStatus::Infeasible,
                                "unbounded" => SolutionStatus::Unbounded,
                                "timeout" => SolutionStatus::Timeout,
                                "iteration_limit" => SolutionStatus::IterationLimit,
                                "numerical_error" => SolutionStatus::NumericalError,
                                "error" => SolutionStatus::Error,
                                _ => SolutionStatus::Unknown,
                            };
                        }
                    }
                }
                if let Some(col) = batch.column_by_name("objective") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        if !arr.is_empty() {
                            solution.objective = arr.value(0);
                        }
                    }
                }
                if let Some(col) = batch.column_by_name("iterations") {
                    if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
                        if !arr.is_empty() {
                            solution.iterations = arr.value(0);
                        }
                    }
                }
                if let Some(col) = batch.column_by_name("solve_time_ms") {
                    if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                        if !arr.is_empty() {
                            solution.solve_time_ms = arr.value(0);
                        }
                    }
                }
                if let Some(col) = batch.column_by_name("error_message") {
                    if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                        if !arr.is_empty() && !arr.is_null(0) {
                            let msg = arr.value(0);
                            if !msg.is_empty() {
                                solution.error_message = Some(msg.to_string());
                            }
                        }
                    }
                }
            }
            1 => {
                // Bus results batch
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
            }
            2 => {
                // Generator results batch
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
            3 => {
                // Branch results batch
                if let Some(col) = batch.column_by_name("branch_id") {
                    if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
                        solution.branch_id = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("branch_p_from") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        solution.branch_p_from = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("branch_q_from") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        solution.branch_q_from = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("branch_p_to") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        solution.branch_p_to = arr.values().to_vec();
                    }
                }
                if let Some(col) = batch.column_by_name("branch_q_to") {
                    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
                        solution.branch_q_to = arr.values().to_vec();
                    }
                }
            }
            _ => {
                // Ignore extra batches for forward compatibility
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

    // =========================================================================
    // Protocol v2 tests
    // =========================================================================

    #[test]
    fn test_v2_problem_roundtrip_asymmetric() {
        // Create a problem with DIFFERENT entity counts - the key test case
        // that v1 required padding to handle
        let mut problem = ProblemBatch::new(ProblemType::DcOpf);

        // 3 buses
        problem.bus_id = vec![1, 2, 3];
        problem.bus_v_min = vec![0.95, 0.95, 0.95];
        problem.bus_v_max = vec![1.05, 1.05, 1.05];
        problem.bus_p_load = vec![50.0, 100.0, 75.0];
        problem.bus_q_load = vec![25.0, 50.0, 37.5];
        problem.bus_type = vec![3, 1, 1];
        problem.bus_v_mag = vec![1.0, 1.0, 1.0];
        problem.bus_v_ang = vec![0.0, 0.0, 0.0];

        // 1 generator (fewer than buses!)
        problem.gen_id = vec![0];
        problem.gen_bus_id = vec![1];
        problem.gen_p_min = vec![0.0];
        problem.gen_p_max = vec![100.0];
        problem.gen_q_min = vec![-50.0];
        problem.gen_q_max = vec![50.0];
        problem.gen_cost_c0 = vec![0.0];
        problem.gen_cost_c1 = vec![10.0];
        problem.gen_cost_c2 = vec![0.0];

        // 2 branches
        problem.branch_id = vec![0, 1];
        problem.branch_from = vec![1, 2];
        problem.branch_to = vec![2, 3];
        problem.branch_r = vec![0.01, 0.01];
        problem.branch_x = vec![0.1, 0.1];
        problem.branch_b = vec![0.0, 0.0];
        problem.branch_rate = vec![100.0, 100.0];
        problem.branch_tap = vec![1.0, 1.0];
        problem.branch_shift = vec![0.0, 0.0];

        // Write and read back using v2 protocol
        let mut buffer = Vec::new();
        write_problem_v2(&problem, &mut buffer).unwrap();

        let recovered = read_problem_v2(&buffer[..]).unwrap();

        // Verify all arrays have correct lengths (no padding!)
        assert_eq!(recovered.bus_id.len(), 3);
        assert_eq!(recovered.gen_id.len(), 1);
        assert_eq!(recovered.branch_id.len(), 2);

        // Verify data integrity
        assert_eq!(recovered.bus_id, vec![1, 2, 3]);
        assert_eq!(recovered.gen_id, vec![0]);
        assert_eq!(recovered.gen_p_max, vec![100.0]);
        assert_eq!(recovered.branch_from, vec![1, 2]);
    }

    #[test]
    fn test_v2_solution_roundtrip_asymmetric() {
        // Test case that caused the original bug: 2 buses, 1 gen
        let solution = SolutionBatch {
            status: SolutionStatus::Optimal,
            objective: 500.0,
            iterations: 10,
            solve_time_ms: 5,
            error_message: None,
            // 2 buses
            bus_id: vec![1, 2],
            bus_v_mag: vec![1.0, 1.0],
            bus_v_ang: vec![0.0, -0.087],
            bus_lmp: vec![10.0, 10.0],
            // 1 generator
            gen_id: vec![0],
            gen_p: vec![50.0],
            gen_q: vec![0.0],
            // 1 branch
            branch_id: vec![0],
            branch_p_from: vec![50.0],
            branch_q_from: vec![0.0],
            branch_p_to: vec![-50.0],
            branch_q_to: vec![0.0],
        };

        let mut buffer = Vec::new();
        write_solution_v2(&solution, &mut buffer).unwrap();

        let recovered = read_solution_v2(&buffer[..]).unwrap();

        // The bug was that gen_id became [0, 0] due to padding,
        // causing HashMap collision. With v2, lengths are preserved.
        assert_eq!(recovered.gen_id.len(), 1);
        assert_eq!(recovered.gen_id, vec![0]);
        assert_eq!(recovered.gen_p, vec![50.0]);
        assert_eq!(recovered.bus_id.len(), 2);
        assert_eq!(recovered.branch_id.len(), 1);
    }

    #[test]
    fn test_v2_empty_entities() {
        // Test with some empty entity arrays
        let mut problem = ProblemBatch::new(ProblemType::DcOpf);
        problem.bus_id = vec![1];
        problem.bus_v_min = vec![0.95];
        problem.bus_v_max = vec![1.05];
        problem.bus_p_load = vec![50.0];
        problem.bus_q_load = vec![25.0];
        problem.bus_type = vec![3];
        problem.bus_v_mag = vec![1.0];
        problem.bus_v_ang = vec![0.0];
        // No generators, no branches

        let mut buffer = Vec::new();
        write_problem_v2(&problem, &mut buffer).unwrap();

        let recovered = read_problem_v2(&buffer[..]).unwrap();

        assert_eq!(recovered.bus_id.len(), 1);
        assert_eq!(recovered.gen_id.len(), 0);
        assert_eq!(recovered.branch_id.len(), 0);
    }

    #[test]
    fn test_v2_solution_all_statuses() {
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
            write_solution_v2(&solution, &mut buffer).unwrap();

            let recovered = read_solution_v2(&buffer[..]).unwrap();
            assert_eq!(
                recovered.status, status,
                "v2: Failed to roundtrip status {:?}",
                status
            );
        }
    }

    #[test]
    fn test_v2_metadata_preserved() {
        let mut problem = ProblemBatch::new(ProblemType::AcOpf);
        problem.protocol_version = 42;
        problem.base_mva = 200.0;
        problem.tolerance = 1e-8;
        problem.max_iterations = 500;

        let mut buffer = Vec::new();
        write_problem_v2(&problem, &mut buffer).unwrap();

        let recovered = read_problem_v2(&buffer[..]).unwrap();
        assert_eq!(recovered.protocol_version, 42);
        assert!((recovered.base_mva - 200.0).abs() < 1e-10);
        assert!((recovered.tolerance - 1e-8).abs() < 1e-15);
        assert_eq!(recovered.max_iterations, 500);
    }
}
