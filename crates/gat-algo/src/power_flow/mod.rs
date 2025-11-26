//! Power Flow Solvers
//!
//! This module provides power flow solution algorithms:
//!
//! - [`legacy`]: DC power flow, PTDF, state estimation, contingency analysis
//! - [`ac_pf`]: Full AC power flow using Newton-Raphson with Q-limit enforcement
//!
//! ## Q-Limit Enforcement
//!
//! The AC power flow solver supports generator reactive power limit enforcement
//! (PV-PQ bus switching). When enabled, generators that exceed their Q limits
//! have their buses converted from PV (voltage-controlled) to PQ mode, allowing
//! the voltage to vary while fixing Q at the limit.

use anyhow::Result;
use gat_core::{Network, Node};
use polars::prelude::*;
use std::path::Path;

pub mod ac_pf;
mod legacy;

#[cfg(test)]
mod q_limits;

// Re-export legacy power flow functions
pub use legacy::*;

// Export new AC power flow solver
pub use ac_pf::{AcPowerFlowSolution, AcPowerFlowSolver, BusType};

/// Write AC power flow solution to Parquet file
pub fn write_ac_pf_solution(
    network: &Network,
    solution: &AcPowerFlowSolution,
    output_path: &Path,
    partitions: &[String],
) -> Result<()> {
    // Build bus results dataframe
    let mut bus_ids: Vec<u32> = Vec::new();
    let mut bus_names: Vec<String> = Vec::new();
    let mut vm_values: Vec<f64> = Vec::new();
    let mut va_values: Vec<f64> = Vec::new();
    let mut bus_type_values: Vec<String> = Vec::new();

    for node in network.graph.node_weights() {
        if let Node::Bus(bus) = node {
            bus_ids.push(bus.id.value() as u32);
            bus_names.push(bus.name.clone());

            let vm = solution
                .bus_voltage_magnitude
                .get(&bus.id)
                .copied()
                .unwrap_or(1.0);
            let va = solution
                .bus_voltage_angle
                .get(&bus.id)
                .copied()
                .unwrap_or(0.0);
            let bus_type = solution
                .bus_types
                .get(&bus.id)
                .map(|t| match t {
                    BusType::Slack => "Slack",
                    BusType::PV => "PV",
                    BusType::PQ => "PQ",
                })
                .unwrap_or("PQ");

            vm_values.push(vm);
            va_values.push(va.to_degrees()); // Convert to degrees
            bus_type_values.push(bus_type.to_string());
        }
    }

    let mut df = DataFrame::new(vec![
        Series::new("bus_id", bus_ids),
        Series::new("bus_name", bus_names),
        Series::new("vm_pu", vm_values),
        Series::new("va_deg", va_values),
        Series::new("bus_type", bus_type_values),
    ])?;

    // Write to Parquet
    crate::io::persist_dataframe(&mut df, output_path, partitions, "pf_ac_qlim")?;

    println!(
        "AC power flow (Q-limits): {} buses, converged={}, iterations={}, max_mismatch={:.2e}, output={}",
        df.height(),
        solution.converged,
        solution.iterations,
        solution.max_mismatch,
        output_path.display()
    );

    Ok(())
}
